// UPnP gateway for UpnpHostPortService: our own minimal IGD client (shared/UpnpIgd.cs) over plain sockets. The Windows
// COM API (HNetCfg.NATUPnP) is deliberately not used: it returns no mapping collection on a "Public" network profile
// (test 2026-11-17 10:14). Only ever used from the service's background worker threads.
using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Text;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    internal sealed class NatUpnpGateway : IUpnpGateway
    {
        private readonly UpnpIgdGateway inner;

        public NatUpnpGateway()
        {
            inner = new UpnpIgdGateway(new NetUpnpTransport(PickLanIPv4()));
        }

        public bool IsAvailable() { return inner.IsAvailable(); }
        public UpnpMappingInfo[] List() { return inner.List(); }
        public UpnpMappingInfo Add(int port, string protocol, string internalClient, string description)
        {
            return inner.Add(port, protocol, internalClient, description);
        }
        public UpnpMappingInfo Get(int port, string protocol) { return inner.Get(port, protocol); }
        public void Remove(int port, string protocol) { inner.Remove(port, protocol); }

        // The default gateway (IPv4) of the adapter that owns lanIPv4, else of the first adapter with one.
        internal static string PickDefaultGatewayIPv4(string lanIPv4)
        {
            string fallback = null;
            try
            {
                foreach (NetworkInterface nic in NetworkInterface.GetAllNetworkInterfaces())
                {
                    if (nic.OperationalStatus != OperationalStatus.Up ||
                        nic.NetworkInterfaceType == NetworkInterfaceType.Loopback ||
                        nic.NetworkInterfaceType == NetworkInterfaceType.Tunnel) continue;
                    IPInterfaceProperties props = nic.GetIPProperties();
                    string gw = null;
                    foreach (GatewayIPAddressInformation g in props.GatewayAddresses)
                        if (g.Address.AddressFamily == AddressFamily.InterNetwork && !g.Address.Equals(IPAddress.Any)) { gw = g.Address.ToString(); break; }
                    if (gw == null) continue;
                    if (fallback == null) fallback = gw;
                    if (lanIPv4 != null)
                        foreach (UnicastIPAddressInformation u in props.UnicastAddresses)
                            if (u.Address.AddressFamily == AddressFamily.InterNetwork && u.Address.ToString() == lanIPv4) return gw;
                }
            }
            catch { }
            return fallback;
        }

        // The LAN IPv4 of the adapter that has a default gateway (the one the router sees); null when none.
        internal static string PickLanIPv4()
        {
            // The address Windows routes outbound traffic from (respects route metrics and VPN adapters);
            // a UDP connect only selects the route, nothing is sent (Copilot review 2026-11-17).
            try
            {
                using (var s = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp))
                {
                    s.Connect(new IPEndPoint(IPAddress.Parse("192.0.2.1"), 9));
                    var local = s.LocalEndPoint as IPEndPoint;
                    if (local != null)
                    {
                        byte[] a = local.Address.GetAddressBytes();
                        bool privateLan = a[0] == 10 || (a[0] == 172 && a[1] >= 16 && a[1] <= 31) || (a[0] == 192 && a[1] == 168);
                        if (privateLan) return local.Address.ToString();
                    }
                }
            }
            catch { }
            try
            {
                foreach (NetworkInterface nic in NetworkInterface.GetAllNetworkInterfaces())
                {
                    if (nic.OperationalStatus != OperationalStatus.Up ||
                        nic.NetworkInterfaceType == NetworkInterfaceType.Loopback ||
                        nic.NetworkInterfaceType == NetworkInterfaceType.Tunnel) continue;
                    IPInterfaceProperties props = nic.GetIPProperties();
                    bool hasGateway = false;
                    foreach (GatewayIPAddressInformation gw in props.GatewayAddresses)
                        if (gw.Address.AddressFamily == AddressFamily.InterNetwork && !gw.Address.Equals(IPAddress.Any))
                            hasGateway = true;
                    if (!hasGateway) continue;
                    foreach (UnicastIPAddressInformation u in props.UnicastAddresses)
                        if (u.Address.AddressFamily == AddressFamily.InterNetwork)
                            return u.Address.ToString();
                }
            }
            catch { }
            return null;
        }
    }

    // Real sockets for UpnpIgdGateway. No system proxy, no redirects, small bodies; never logs addresses.
    internal sealed class NetUpnpTransport : IUpnpTransport
    {
        private readonly string lanIPv4;
        public NetUpnpTransport(string lanIPv4) { this.lanIPv4 = lanIPv4; }

        public string DefaultGatewayIPv4() { return NatUpnpGateway.PickDefaultGatewayIPv4(lanIPv4); }

        // The whole window is collected (no early stop): UpnpIgdGateway keeps only replies whose UDP sender is the default gateway.
        public List<SsdpReply> Search(string[] searchTargets, int timeoutMs)
        {
            var found = new List<SsdpReply>();
            IPAddress local;
            if (lanIPv4 == null || !IPAddress.TryParse(lanIPv4, out local)) local = IPAddress.Any;
            using (var sock = new Socket(AddressFamily.InterNetwork, SocketType.Dgram, ProtocolType.Udp))
            {
                sock.Bind(new IPEndPoint(local, 0));
                sock.ReceiveTimeout = 250;
                var dest = new IPEndPoint(IPAddress.Parse("239.255.255.250"), 1900);
                var buf = new byte[4096];
                var sw = System.Diagnostics.Stopwatch.StartNew();
                long nextSend = 0;
                while (sw.ElapsedMilliseconds < timeoutMs)
                {
                    if (sw.ElapsedMilliseconds >= nextSend)
                    {
                        nextSend = sw.ElapsedMilliseconds + 1000;   // UDP: ask again once a second
                        foreach (string st in searchTargets)
                        {
                            string req = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: " + st + "\r\n\r\n";
                            try { sock.SendTo(Encoding.ASCII.GetBytes(req), dest); } catch (SocketException) { }
                        }
                    }
                    int n;
                    EndPoint from = new IPEndPoint(IPAddress.Any, 0);
                    try { n = sock.ReceiveFrom(buf, ref from); }
                    catch (SocketException) { continue; }   // receive timeout: loop (and resend when due)
                    var ip = from as IPEndPoint;
                    if (ip == null || found.Count >= 64) continue;   // bounded: a flood cannot grow the list
                    found.Add(new SsdpReply(ip.Address.ToString(), Encoding.ASCII.GetString(buf, 0, n)));
                }
            }
            return found;
        }

        private static HttpWebRequest Make(string url, string method, int timeoutMs)
        {
            var req = (HttpWebRequest)WebRequest.Create(url);
            req.Method = method;
            req.Proxy = null;
            req.AllowAutoRedirect = false;
            req.Timeout = timeoutMs;
            req.ReadWriteTimeout = timeoutMs;
            req.KeepAlive = false;
            return req;
        }

        private static string ReadBody(WebResponse resp)
        {
            using (Stream s = resp.GetResponseStream())
            {
                var ms = new MemoryStream();
                var buf = new byte[4096];
                int n;
                while ((n = s.Read(buf, 0, buf.Length)) > 0 && ms.Length < 262144) ms.Write(buf, 0, n);
                return Encoding.UTF8.GetString(ms.ToArray());
            }
        }

        public string HttpGet(string url, int timeoutMs)
        {
            try
            {
                using (WebResponse r = Make(url, "GET", timeoutMs).GetResponse()) return ReadBody(r);
            }
            catch { return null; }
        }

        public int HttpPost(string url, string soapAction, string body, int timeoutMs, out string responseBody)
        {
            responseBody = null;
            try
            {
                HttpWebRequest req = Make(url, "POST", timeoutMs);
                req.ContentType = "text/xml; charset=\"utf-8\"";
                req.Headers["SOAPACTION"] = soapAction;
                byte[] data = Encoding.UTF8.GetBytes(body);
                req.ContentLength = data.Length;
                using (Stream s = req.GetRequestStream()) s.Write(data, 0, data.Length);
                using (var r = (HttpWebResponse)req.GetResponse())
                {
                    responseBody = ReadBody(r);
                    return (int)r.StatusCode;
                }
            }
            catch (WebException we)
            {
                var hr = we.Response as HttpWebResponse;
                if (hr == null) return 0;
                try { responseBody = ReadBody(hr); } catch { }
                int code = (int)hr.StatusCode;
                hr.Close();
                return code;
            }
            catch { return 0; }
        }
    }
}
