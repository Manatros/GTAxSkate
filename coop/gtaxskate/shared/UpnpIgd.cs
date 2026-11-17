// Our own minimal UPnP IGD client (design 2026-11-16-coop-internet-lobby, round 2). The Windows COM API
// (HNetCfg.NATUPnP) returned no mapping collection when the network profile is "Public" (test 2026-11-17 10:14), so
// the protocol is spoken directly: SSDP discovery, the device description, and SOAP AddPortMapping /
// DeletePortMapping / GetExternalIPAddress / GetGenericPortMappingEntry. All I/O sits behind IUpnpTransport, so the
// logic runs against recorded XML/SOAP samples in tests. Only ever called from UpnpHostPortService's worker threads.
//
//  - The router is the one that ANSWERED from this PC's default gateway (UDP sender address) AND whose SSDP LOCATION
//    host equals that gateway (several FRITZ!Boxes may answer on one LAN, and any LAN host can send a forged reply).
//    All replies of the search window are collected and each matching location is tried in order. No default
//    gateway, or no match, means "no UPnP device": never a guess, never an arbitrary location.
//  - The description and control URLs must stay on that same host (a LAN device cannot redirect us elsewhere).
//  - XML is parsed with DTD processing prohibited and no resolver.
//  - Errors become UpnpUserException with a player-facing message; no address is ever put into a message or trace.
using System;
using System.Collections.Generic;
using System.IO;
using System.Security;
using System.Xml;

namespace GTAxSkate.Coop
{
    // Player-facing failure from the gateway (the service shows UserMessage when set, and traces TraceNote).
    public sealed class UpnpUserException : Exception
    {
        public readonly string UserMessage;
        public readonly string TraceNote;
        public UpnpUserException(string userMessage, string traceNote) : base(traceNote)
        {
            UserMessage = userMessage; TraceNote = traceNote;
        }
    }

    // One SSDP datagram: the UDP sender's IPv4 text and the raw response.
    public sealed class SsdpReply
    {
        public readonly string Sender;
        public readonly string Text;
        public SsdpReply(string sender, string text) { Sender = sender; Text = text; }
    }

    public interface IUpnpTransport
    {
        // This PC's default gateway (IPv4 text), null when unknown.
        string DefaultGatewayIPv4();
        // SSDP M-SEARCH for every target; collects every reply of the whole window (with its UDP sender address).
        List<SsdpReply> Search(string[] searchTargets, int timeoutMs);
        // Returns the body, or null on any failure.
        string HttpGet(string url, int timeoutMs);
        // Returns the HTTP status (0 = no response); the body also for 4xx/5xx (UPnP faults come as 500).
        int HttpPost(string url, string soapAction, string body, int timeoutMs, out string responseBody);
    }

    public sealed class UpnpIgdGateway : IUpnpGateway
    {
        public const int SearchTimeoutMs = 2500;
        public const int HttpTimeoutMs = 2500;
        public const string FritzNotAuthorized =
            "Your FRITZ!Box/router does not allow this PC to open ports. Allow 'independent port sharing' for this PC in the router, or use Tailscale.";
        public const string NoPublicIPv4 =
            "Your internet line has no public IPv4 address (DS-Lite / carrier NAT), so friends cannot reach you over IPv4. Use Tailscale, or let a friend with a public IPv4 host.";
        private const int MaxListEntries = 256;
        private static readonly string[] SearchTargets =
        {
            "urn:schemas-upnp-org:device:InternetGatewayDevice:1",
            "urn:schemas-upnp-org:device:InternetGatewayDevice:2"
        };

        private readonly IUpnpTransport transport;
        private bool discovered;
        private bool available;
        private string controlUrl;
        private string serviceType;

        public UpnpIgdGateway(IUpnpTransport transport)
        {
            if (transport == null) throw new ArgumentNullException("transport");
            this.transport = transport;
        }

        public bool IsAvailable()
        {
            if (discovered) return available;
            discovered = true;
            available = Discover();
            return available;
        }

        private bool Discover()
        {
            string gateway = transport.DefaultGatewayIPv4();
            if (string.IsNullOrEmpty(gateway)) return false;   // unknown gateway: no device, never an arbitrary location
            List<SsdpReply> replies = transport.Search(SearchTargets, SearchTimeoutMs);
            if (replies == null) return false;
            var tried = new List<string>();
            foreach (SsdpReply reply in replies)
            {
                if (reply == null || !string.Equals(reply.Sender, gateway, StringComparison.Ordinal)) continue;   // forged / other device
                string location = LocationOf(reply.Text);
                if (location == null || !HostMatches(location, gateway) || tried.Contains(location)) continue;
                tried.Add(location);
                string xml = transport.HttpGet(location, HttpTimeoutMs);
                if (string.IsNullOrEmpty(xml)) continue;
                string url, type;
                if (!FindWanService(xml, location, out url, out type)) continue;
                controlUrl = url; serviceType = type;
                return true;
            }
            return false;
        }

        // A plain http LOCATION on the default gateway; an unknown gateway matches nothing.
        internal static bool HostMatches(string location, string gateway)
        {
            Uri u;
            if (!Uri.TryCreate(location, UriKind.Absolute, out u) || u.Scheme != Uri.UriSchemeHttp) return false;
            if (string.IsNullOrEmpty(gateway)) return false;
            return string.Equals(u.Host, gateway, StringComparison.OrdinalIgnoreCase);
        }

        internal static string LocationOf(string ssdpResponse)
        {
            if (ssdpResponse == null) return null;
            foreach (string line in ssdpResponse.Split('\n'))
            {
                string t = line.Trim();
                if (t.Length > 9 && t.StartsWith("LOCATION:", StringComparison.OrdinalIgnoreCase))
                    return t.Substring(9).Trim();
            }
            return null;
        }

        private static XmlDocument LoadXml(string xml)
        {
            var settings = new XmlReaderSettings();
            settings.DtdProcessing = DtdProcessing.Prohibit;
            settings.XmlResolver = null;
            var doc = new XmlDocument();
            doc.XmlResolver = null;
            using (XmlReader r = XmlReader.Create(new StringReader(xml), settings)) doc.Load(r);
            return doc;
        }

        private static void Collect(XmlNode node, string localName, List<XmlNode> into)
        {
            if (node.NodeType == XmlNodeType.Element && node.LocalName == localName) into.Add(node);
            foreach (XmlNode c in node.ChildNodes) Collect(c, localName, into);
        }

        private static string ChildText(XmlNode node, string localName)
        {
            foreach (XmlNode c in node.ChildNodes)
                if (c.NodeType == XmlNodeType.Element && c.LocalName == localName) return c.InnerText == null ? null : c.InnerText.Trim();
            return null;
        }

        // WANIPConnection:2/1 first, else WANPPPConnection:1; controlURL resolved against URLBase / the description URL.
        internal static bool FindWanService(string descriptionXml, string descriptionUrl, out string controlUrl, out string serviceType)
        {
            controlUrl = null; serviceType = null;
            XmlDocument doc;
            try { doc = LoadXml(descriptionXml); } catch (XmlException) { return false; }
            var services = new List<XmlNode>();
            Collect(doc, "service", services);
            string[] preferred =
            {
                "urn:schemas-upnp-org:service:WANIPConnection:2",
                "urn:schemas-upnp-org:service:WANIPConnection:1",
                "urn:schemas-upnp-org:service:WANPPPConnection:1"
            };
            string baseText = null;
            var bases = new List<XmlNode>();
            Collect(doc, "URLBase", bases);
            if (bases.Count > 0) baseText = bases[0].InnerText.Trim();
            Uri baseUri;
            if (!Uri.TryCreate(string.IsNullOrEmpty(baseText) ? descriptionUrl : baseText, UriKind.Absolute, out baseUri)) return false;
            Uri descUri;
            if (!Uri.TryCreate(descriptionUrl, UriKind.Absolute, out descUri)) return false;
            foreach (string want in preferred)
            {
                foreach (XmlNode s in services)
                {
                    string type = ChildText(s, "serviceType");
                    string ctl = ChildText(s, "controlURL");
                    if (!string.Equals(type, want, StringComparison.Ordinal) || string.IsNullOrEmpty(ctl)) continue;
                    Uri resolved;
                    if (!Uri.TryCreate(baseUri, ctl, out resolved)) continue;
                    // Same host as the description fetched from the default gateway; never follow a redirect elsewhere.
                    if (resolved.Scheme != Uri.UriSchemeHttp ||
                        !string.Equals(resolved.Host, descUri.Host, StringComparison.OrdinalIgnoreCase)) continue;
                    controlUrl = resolved.ToString(); serviceType = type;
                    return true;
                }
            }
            return false;
        }

        // ---- SOAP -----------------------------------------------------------------------------------------

        private static string Envelope(string type, string action, string argsXml)
        {
            return "<?xml version=\"1.0\"?>\r\n" +
                "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">" +
                "<s:Body><u:" + action + " xmlns:u=\"" + type + "\">" + argsXml + "</u:" + action + "></s:Body></s:Envelope>";
        }

        private static string Arg(string name, string value)
        {
            return "<" + name + ">" + SecurityElement.Escape(value ?? string.Empty) + "</" + name + ">";
        }

        // faultCode: 0 = ok, otherwise the UPnP error code (606 also for HTTP 401/403), -1 = unknown fault.
        // Throws UpnpUserException(null, ...) when the router did not answer at all.
        private XmlDocument Call(string action, string argsXml, out int faultCode)
        {
            faultCode = 0;
            string body;
            int status = transport.HttpPost(controlUrl, "\"" + serviceType + "#" + action + "\"",
                Envelope(serviceType, action, argsXml), HttpTimeoutMs, out body);
            if (status == 0) throw new UpnpUserException(null, "upnp no response to " + action);
            XmlDocument doc = null;
            if (!string.IsNullOrEmpty(body))
            {
                try { doc = LoadXml(body); } catch (XmlException) { doc = null; }
            }
            if (status == 401 || status == 403) { faultCode = 606; return doc; }
            if (status >= 400 || (doc != null && HasFault(doc)))
            {
                faultCode = doc == null ? -1 : FaultCode(doc);
                if (faultCode == 0) faultCode = -1;
            }
            return doc;
        }

        private static bool HasFault(XmlDocument doc)
        {
            var l = new List<XmlNode>();
            Collect(doc, "Fault", l);
            return l.Count > 0;
        }

        internal static int FaultCode(XmlDocument doc)
        {
            var l = new List<XmlNode>();
            Collect(doc, "errorCode", l);
            int code;
            if (l.Count > 0 && int.TryParse(l[0].InnerText.Trim(), out code)) return code;
            return 0;
        }

        private static string Value(XmlDocument doc, string localName)
        {
            if (doc == null) return null;
            var l = new List<XmlNode>();
            Collect(doc, localName, l);
            return l.Count == 0 ? null : l[0].InnerText.Trim();
        }

        private static Exception FaultToException(string action, int code, int port)
        {
            if (code == 606)
                return new UpnpUserException(FritzNotAuthorized, "upnp fault 606 (not authorized) on " + action);
            if (code == 718)
                return new UpnpUserException("UDP port " + port + " is already forwarded to another device or app on your router. Free it, or forward it to this PC yourself.",
                    "upnp fault 718 (port held by another mapping)");
            return new UpnpUserException(null, "upnp fault " + code + " on " + action);
        }

        public UpnpMappingInfo[] List()
        {
            var list = new List<UpnpMappingInfo>();
            if (!IsAvailable()) return list.ToArray();
            for (int i = 0; i < MaxListEntries; i++)
            {
                int fault;
                XmlDocument doc = Call("GetGenericPortMappingEntry", Arg("NewPortMappingIndex", i.ToString()), out fault);
                if (fault == 606) throw FaultToException("GetGenericPortMappingEntry", 606, 0);
                if (fault != 0 || doc == null) break;   // 713 = end of list; other faults: enumeration unsupported, go on
                int port;
                if (!int.TryParse(Value(doc, "NewExternalPort") ?? string.Empty, out port)) break;
                list.Add(new UpnpMappingInfo
                {
                    ExternalPort = port,
                    Protocol = Value(doc, "NewProtocol"),
                    InternalClient = Value(doc, "NewInternalClient"),
                    Description = Value(doc, "NewPortMappingDescription"),
                    ExternalIp = string.Empty
                });
            }
            return list.ToArray();
        }

        // Two separate steps: (1) AddPortMapping creates the mapping, (2) GetExternalIPAddress reads the address.
        // Once step 1 succeeded, ANY failure of step 2 deletes our mapping again before the error is reported
        // (Copilot round 2: a fault 606 on the address read used to leave the port open while the UI said "failed").
        public UpnpMappingInfo Add(int port, string protocol, string internalClient, string description)
        {
            if (!IsAvailable()) throw new InvalidOperationException("no UPnP device");
            AddMapping(port, protocol, internalClient, description);
            try
            {
                string ext = ReadExternalIp(port);
                return new UpnpMappingInfo { ExternalPort = port, Protocol = protocol, InternalClient = internalClient, Description = description, ExternalIp = ext };
            }
            catch
            {
                try { Remove(port, protocol); } catch { }   // best effort: the original failure is what gets reported
                throw;
            }
        }

        private void AddMapping(int port, string protocol, string internalClient, string description)
        {
            int fault;
            Call("AddPortMapping",
                Arg("NewRemoteHost", "") + Arg("NewExternalPort", port.ToString()) + Arg("NewProtocol", protocol) +
                Arg("NewInternalPort", port.ToString()) + Arg("NewInternalClient", internalClient) + Arg("NewEnabled", "1") +
                Arg("NewPortMappingDescription", description) + Arg("NewLeaseDuration", "0"), out fault);
            if (fault != 0) throw FaultToException("AddPortMapping", fault, port);
        }

        // Empty when the router does not say (the service reports that as "no public IPv4"); throws when not authorized.
        private string ReadExternalIp(int port)
        {
            try
            {
                int f2;
                XmlDocument doc = Call("GetExternalIPAddress", string.Empty, out f2);
                if (f2 == 606) throw FaultToException("GetExternalIPAddress", 606, port);
                if (f2 == 0) return Value(doc, "NewExternalIPAddress") ?? string.Empty;
            }
            catch (UpnpUserException e)
            {
                if (e.UserMessage != null) throw;   // not authorized: report it
            }
            return string.Empty;
        }

        // GetSpecificPortMappingEntry: null when there is no such entry (714); faults and a missing answer throw.
        public UpnpMappingInfo Get(int port, string protocol)
        {
            if (!IsAvailable()) throw new InvalidOperationException("no UPnP device");
            int fault;
            XmlDocument doc = Call("GetSpecificPortMappingEntry",
                Arg("NewRemoteHost", "") + Arg("NewExternalPort", port.ToString()) + Arg("NewProtocol", protocol), out fault);
            if (fault == 714) return null;
            if (fault != 0) throw FaultToException("GetSpecificPortMappingEntry", fault, port);
            string client = Value(doc, "NewInternalClient");
            if (client == null) return null;
            return new UpnpMappingInfo
            {
                ExternalPort = port, Protocol = protocol, InternalClient = client,
                Description = Value(doc, "NewPortMappingDescription") ?? string.Empty, ExternalIp = string.Empty
            };
        }

        public void Remove(int port, string protocol)
        {
            if (!IsAvailable()) return;
            int fault;
            Call("DeletePortMapping", Arg("NewRemoteHost", "") + Arg("NewExternalPort", port.ToString()) + Arg("NewProtocol", protocol), out fault);
            if (fault == 0 || fault == 714) return;   // 714: no such entry (already gone)
            throw FaultToException("DeletePortMapping", fault, port);
        }
    }
}
