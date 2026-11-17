// Offline tests for the internet lobby building blocks (design 2026-11-16-coop-internet-lobby, rev 2):
// LobbyCode, PublicHandshakeGate, the pinned-public policy rule and UpnpHostPortService behind a fake gateway.
// No network, no router, no COM, no game: the UPnP gateway is an in-memory fixture.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text.RegularExpressions;
using System.Threading;
using GTAxSkate.Coop;

public static class InternetLobbyTests
{
    private static int checks;
    private static void Check(bool condition, string message)
    {
        checks++;
        if (!condition) throw new Exception("FAIL: " + message);
    }

    private sealed class FakeGateway : IUpnpGateway
    {
        public readonly object Lock = new object();
        public bool Available = true;
        public Exception AvailableThrows;
        public string ExternalIp = "8.8.4.4";
        public readonly List<UpnpMappingInfo> Mappings = new List<UpnpMappingInfo>();
        public readonly List<string> Removed = new List<string>();
        public ManualResetEvent AddGate;          // when set, Add blocks until released (timeout fixture)
        public ApartmentState WorkerApartment = ApartmentState.Unknown;
        public bool WorkerIsBackground;
        public int WorkerThreadId;

        public bool IsAvailable()
        {
            WorkerApartment = Thread.CurrentThread.GetApartmentState();
            WorkerIsBackground = Thread.CurrentThread.IsBackground;
            WorkerThreadId = Thread.CurrentThread.ManagedThreadId;
            if (AvailableThrows != null) throw AvailableThrows;
            return Available;
        }
        public UpnpMappingInfo[] List() { lock (Lock) return Mappings.ToArray(); }
        public UpnpMappingInfo Add(int port, string protocol, string internalClient, string description)
        {
            if (AddGate != null) AddGate.WaitOne();
            var m = new UpnpMappingInfo { ExternalPort = port, Protocol = protocol, InternalClient = internalClient, Description = description, ExternalIp = ExternalIp };
            lock (Lock) Mappings.Add(m);
            return m;
        }
        public UpnpMappingInfo Get(int port, string protocol)
        {
            lock (Lock) return Mappings.Find(delegate(UpnpMappingInfo m) { return m.ExternalPort == port && m.Protocol == protocol; });
        }
        public void Remove(int port, string protocol)
        {
            lock (Lock)
            {
                for (int i = Mappings.Count - 1; i >= 0; i--)
                    if (Mappings[i].ExternalPort == port && Mappings[i].Protocol == protocol)
                    {
                        Removed.Add(Mappings[i].Description);
                        Mappings.RemoveAt(i);
                    }
            }
        }
        public int Count(string description)
        {
            lock (Lock) { int n = 0; foreach (var m in Mappings) if (m.Description == description) n++; return n; }
        }
    }

    private static bool WaitFor(Func<bool> condition, int ms)
    {
        var sw = Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < ms)
        {
            if (condition()) return true;
            Thread.Sleep(5);
        }
        return condition();
    }

    private static OnlineHostStatus WaitDone(UpnpHostPortService s, int ms)
    {
        OnlineHostStatus st = s.Poll();
        var sw = Stopwatch.StartNew();
        while (st.State == OnlineHostState.Opening && sw.ElapsedMilliseconds < ms) { Thread.Sleep(5); st = s.Poll(); }
        return st;
    }

    public static int Main()
    {
        TestLobbyCode();
        TestGate();
        TestPolicy();
        TestUpnp();
        Console.WriteLine("PASS: " + checks + " internet lobby checks (LobbyCode, throttle, policy, UPnP fixture); no network, router, COM or game touched.");
        return 0;
    }

    private static void TestLobbyCode()
    {
        var rnd = new Random(20261116);
        int roundTrips = 0;
        string shape = @"^SK8-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$";
        while (roundTrips < 1000)
        {
            var ip = new byte[4];
            rnd.NextBytes(ip);
            if (!LobbyCode.IsGlobalIPv4(ip)) continue;
            int port = 1024 + rnd.Next(65535 - 1024 + 1);
            string text = ip[0] + "." + ip[1] + "." + ip[2] + "." + ip[3];
            string code;
            Check(LobbyCode.TryEncode(text, port, out code), "global address encodes: " + text);
            Check(Regex.IsMatch(code, shape), "code shape " + code);
            string ip2; int port2;
            Check(LobbyCode.Decode(code, out ip2, out port2) == LobbyCodeStatus.Ok && ip2 == text && port2 == port, "round trip " + code);
            roundTrips++;
        }
        Check(roundTrips == 1000, "1,000 random round trips");

        // Pinned vector (stable wire format).
        string known;
        Check(LobbyCode.TryEncode("8.8.8.8", 4499, out known), "known vector encodes");
        Check(known == "SK8-1040-G20H-JC2G", "pinned wire format for 8.8.8.8:4499 (independently computed CRC-8/0x07), got " + known);
        string knownIp; int knownPort;
        Check(LobbyCode.Decode(known, out knownIp, out knownPort) == LobbyCodeStatus.Ok && knownIp == "8.8.8.8" && knownPort == 4499, "known vector decodes");

        // Every single-symbol change is caught (CRC-8 or the zero padding), for several codes and all positions.
        string[] samples = { known, "SK8-" + Strip(MakeCode("93.184.216.34", 4499)), MakeCode("1.1.1.1", 65535), MakeCode("223.5.5.5", 1024) };
        foreach (string sample in samples)
        {
            string body = LobbyCode.Normalize(sample);
            const string alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
            for (int pos = 0; pos < body.Length; pos++)
                foreach (char c in alphabet)
                {
                    if (c == body[pos]) continue;
                    string changed = body.Substring(0, pos) + c + body.Substring(pos + 1);
                    string i3; int p3;
                    LobbyCodeStatus st = LobbyCode.Decode(changed, out i3, out p3);
                    Check(st != LobbyCodeStatus.Ok, "single symbol change at " + pos + " (" + c + ") is rejected in " + sample);
                }
        }

        // Folding: case, dashes, spaces, O->0, I/L->1, optional prefix.
        string good = MakeCode("8.8.4.4", 4499);
        string plain = LobbyCode.Normalize(good);
        string a1, a2; int b1, b2;
        Check(LobbyCode.Decode(good.ToLowerInvariant(), out a1, out b1) == LobbyCodeStatus.Ok && a1 == "8.8.4.4" && b1 == 4499, "lower case decodes");
        Check(LobbyCode.Decode(plain, out a2, out b2) == LobbyCodeStatus.Ok && a2 == "8.8.4.4", "no dashes, no prefix decodes");
        Check(LobbyCode.Decode("  " + good.Replace("-", " ") + "  ", out a2, out b2) == LobbyCodeStatus.Ok, "spaces for dashes decode");
        string folded = plain.Replace('0', 'O').Replace('1', 'l');
        Check(LobbyCode.Decode(folded, out a2, out b2) == LobbyCodeStatus.Ok && a2 == "8.8.4.4", "O/0 and l/1 fold, folded=" + folded);
        Check(LobbyCode.Decode(plain.Replace('1', 'I'), out a2, out b2) == LobbyCodeStatus.Ok, "I folds to 1");
        Check(LobbyCode.Decode("", out a2, out b2) == LobbyCodeStatus.Malformed, "empty is malformed");
        Check(LobbyCode.Decode(null, out a2, out b2) == LobbyCodeStatus.Malformed, "null is malformed");
        Check(LobbyCode.Decode(plain.Substring(1), out a2, out b2) == LobbyCodeStatus.Malformed, "short is malformed");
        Check(LobbyCode.Decode(plain + "0", out a2, out b2) == LobbyCodeStatus.Malformed, "long is malformed");
        Check(LobbyCode.Decode("U" + plain.Substring(1), out a2, out b2) == LobbyCodeStatus.Malformed, "U is not in the alphabet");
        Check(LobbyCode.Decode(plain.Substring(0, 5) + "*" + plain.Substring(6), out a2, out b2) == LobbyCodeStatus.Malformed, "punctuation is malformed");

        // Reserved ranges: first and last address of each range are rejected on encode and decode; neighbours are fine.
        string[] reservedEdges =
        {
            "0.0.0.0", "0.255.255.255", "10.0.0.0", "10.255.255.255", "100.64.0.0", "100.127.255.255", "127.0.0.1", "127.255.255.255",
            "169.254.0.0", "169.254.255.255", "172.16.0.0", "172.31.255.255", "192.0.0.0", "192.0.0.255", "192.0.2.0", "192.0.2.255",
            "192.88.99.0", "192.88.99.255", "192.168.0.0", "192.168.255.255", "198.18.0.0", "198.19.255.255", "198.51.100.0", "198.51.100.255",
            "203.0.113.0", "203.0.113.255", "224.0.0.0", "239.255.255.255", "240.0.0.0", "255.255.255.255"
        };
        foreach (string r in reservedEdges)
        {
            byte[] rb; string rc;
            Check(LobbyCode.TryParseIPv4(r, out rb), "parses " + r);
            Check(!LobbyCode.IsGlobalIPv4(rb), r + " is not global");
            Check(!LobbyCode.TryEncode(r, 4499, out rc), r + " is not encoded");
            string raw = LobbyCode.EncodeRaw(rb, 4499), ri; int rp;
            Check(LobbyCode.Decode(raw, out ri, out rp) == LobbyCodeStatus.ReservedAddress, r + " decodes as ReservedAddress");
        }
        string[] globalEdges = { "1.0.0.0", "9.255.255.255", "11.0.0.0", "100.63.255.255", "100.128.0.0", "126.255.255.255", "128.0.0.0",
            "169.253.255.255", "169.255.0.0", "172.15.255.255", "172.32.0.0", "192.0.1.0", "192.0.3.0", "192.167.255.255", "192.169.0.0",
            "198.17.255.255", "198.20.0.0", "198.51.99.255", "198.51.101.0", "203.0.112.255", "203.0.114.0", "223.255.255.255" };
        foreach (string g in globalEdges)
        {
            string gc;
            Check(LobbyCode.TryEncode(g, 4499, out gc), g + " is global and encodes");
        }
        string x;
        Check(!LobbyCode.TryEncode("8.8.8.8", 80, out x) && !LobbyCode.TryEncode("8.8.8.8", 0, out x) && !LobbyCode.TryEncode("8.8.8.8", 65536, out x), "ports below 1024 or above 65535 are rejected");
        Check(!LobbyCode.TryEncode("8.8.8", 4499, out x) && !LobbyCode.TryEncode("8.8.8.256", 4499, out x) && !LobbyCode.TryEncode("a.b.c.d", 4499, out x) && !LobbyCode.TryEncode("", 4499, out x), "malformed addresses are rejected");
        byte[] low = { 8, 8, 8, 8 }; string lowCode = LobbyCode.EncodeRaw(low, 80), li; int lp;
        Check(LobbyCode.Decode(lowCode, out li, out lp) == LobbyCodeStatus.BadPort, "a low port in a code is rejected");
    }

    private static string MakeCode(string ip, int port)
    {
        string code;
        if (!LobbyCode.TryEncode(ip, port, out code)) throw new Exception("test setup: " + ip);
        return code;
    }
    private static string Strip(string code) { return code.Substring(4); }

    private static void TestGate()
    {
        var g = new PublicHandshakeGate();
        string reason;
        long t = 1000;
        Check(g.TryBegin("1.2.3.4", t, out reason), "fresh address passes");
        for (int i = 0; i < 4; i++) { g.RecordFailure("1.2.3.4", t += 1000); Check(g.TryBegin("1.2.3.4", t, out reason), "still open after " + (i + 1) + " failures"); }
        g.RecordFailure("1.2.3.4", t += 1000);
        Check(g.LastEvent == "address lockout started", "5th failure starts the address lockout");
        Check(!g.TryBegin("1.2.3.4", t + 1000, out reason), "address refused after 5 failures");
        Check(g.TryBegin("5.6.7.8", t + 1000, out reason), "another address is unaffected");
        Check(!g.TryBegin("1.2.3.4", t + PublicHandshakeGate.PerAddressLockMs - 1, out reason), "still refused just before 10 minutes");
        Check(g.TryBegin("1.2.3.4", t + PublicHandshakeGate.PerAddressLockMs + 1, out reason), "allowed again after 10 minutes");

        // Failures spread beyond the window do not add up; a success clears the count.
        var g2 = new PublicHandshakeGate();
        long u = 0;
        for (int i = 0; i < 8; i++) { g2.RecordFailure("9.9.9.9", u); u += PublicHandshakeGate.PerAddressWindowMs / 2 + 1000; }
        Check(g2.TryBegin("9.9.9.9", u, out reason), "slow failures never lock");
        var g3 = new PublicHandshakeGate();
        for (int i = 0; i < 4; i++) g3.RecordFailure("7.7.7.7", i);
        g3.RecordSuccess("7.7.7.7");
        g3.RecordFailure("7.7.7.7", 10);
        Check(g3.TryBegin("7.7.7.7", 11, out reason) && g3.TrackedAddresses == 1, "a correct password clears the failure count");

        // Global cap: 31 failures within a minute from many addresses lock every public handshake for a minute.
        var gg = new PublicHandshakeGate();
        long w = 100000;
        for (int i = 0; i < 30; i++) { gg.RecordFailure("50.0." + (i / 200) + "." + (i % 200), w + i * 100); }
        Check(gg.TryBegin("60.0.0.1", w + 5000, out reason), "30 failures do not lock yet");
        gg.RecordFailure("50.1.0.1", w + 5100);
        Check(gg.LastEvent == "global lockout started", "31st failure starts the global lockout");
        Check(!gg.TryBegin("60.0.0.1", w + 5200, out reason) && !gg.TryBegin("61.0.0.1", w + 6000, out reason), "all public handshakes refused during the global lockout");
        Check(gg.TryBegin("60.0.0.1", w + 5100 + PublicHandshakeGate.GlobalLockMs + 1, out reason), "global lockout ends after a minute");

        // The table never grows past 256 addresses.
        var gt = new PublicHandshakeGate();
        for (int i = 0; i < 2000; i++)
        {
            gt.RecordFailure("20." + (i / 250) + "." + (i % 250) + ".1", 1000000 + i * 5000L);   // spaced out: no global lockout
            Check(gt.TrackedAddresses <= PublicHandshakeGate.MaxTrackedAddresses, "table bounded at " + i);
        }
        Check(gt.TrackedAddresses == PublicHandshakeGate.MaxTrackedAddresses, "table is full at the cap");
    }

    private static void TestPolicy()
    {
        string key = "SkateFriends-Secret-99";
        // Pinned (valid code) public host is allowed; unpinned public stays blocked; reserved ranges stay blocked even when pinned.
        Check(CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
            LobbyDiscoverySource.DirectFriendInvite, "8.8.4.4", 4499, true, "Friend-Online", key)).Allowed, "pinned global public host allowed");
        Check(!CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
            LobbyDiscoverySource.DirectFriendInvite, "8.8.4.4", 4499, false, "Friend-Online", key)).Allowed, "unpinned public host blocked");
        foreach (string reserved in new[] { "198.51.100.25", "203.0.113.9", "192.0.2.5", "198.18.0.1", "224.0.0.5", "240.0.0.1" })
            Check(!CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
                LobbyDiscoverySource.DirectFriendInvite, reserved, 4499, true, "Friend-Online", key)).Allowed, "pinned reserved address blocked: " + reserved);
        Check(!CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
            LobbyDiscoverySource.DirectFriendInvite, "8.8.4.4", 4499, true, "Friend-Online", "short")).Allowed, "short password blocked even for a pinned host");
        Check(CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
            LobbyDiscoverySource.DirectFriendInvite, "100.88.12.34", 4499, false, "Alex-VPN", key)).Allowed, "VPN friends still connect unpinned");
        Check(!CoopSecurityPolicy.EvaluateOutboundConnect(new ClientConnectRequest(
            LobbyDiscoverySource.PublicServerBrowser, "8.8.4.4", 4499, true, "Browser", key)).Allowed, "server browsers stay blocked");
    }

    private static UpnpHostPortService Service(FakeGateway g, int timeoutMs)
    {
        return new UpnpHostPortService(delegate() { return g; }, timeoutMs);
    }

    private static void TestUpnp()
    {
        const string own = "GTAxSkate Coop abcd1234";

        // Success on an MTA background worker; Begin returns at once.
        var g = new FakeGateway();
        var s = Service(g, 8000);
        var sw = Stopwatch.StartNew();
        s.Begin(4499, own, "192.168.1.20");
        Check(sw.ElapsedMilliseconds < 500, "Begin does not block the caller");
        OnlineHostStatus st = WaitDone(s, 3000);
        Check(st.State == OnlineHostState.Open && st.ExternalIp == "8.8.4.4", "UPnP success -> Open with the external address");
        Check(g.Count(own) == 1 && g.Mappings[0].Protocol == "UDP" && g.Mappings[0].ExternalPort == 4499 && g.Mappings[0].InternalClient == "192.168.1.20",
            "only UDP 4499 to the LAN address is mapped");
        Check(g.WorkerApartment == ApartmentState.MTA && g.WorkerIsBackground && g.WorkerThreadId != Thread.CurrentThread.ManagedThreadId,
            "gateway runs on a background MTA worker, not the caller");
        Check(st.Message.IndexOf("cannot test", StringComparison.OrdinalIgnoreCase) >= 0, "success message says the host cannot self-test");
        Check(s.Release(3000) && g.Count(own) == 0 && s.Poll().State == OnlineHostState.Idle, "Release removes our mapping");

        // No UPnP device.
        var gNone = new FakeGateway { Available = false };
        var sNone = Service(gNone, 8000);
        sNone.Begin(4499, own, "192.168.1.20");
        st = WaitDone(sNone, 3000);
        Check(st.State == OnlineHostState.Failed && st.Message.Contains("Forward UDP 4499") && st.Message.Contains("Tailscale"), "no device -> Failed with the manual-forward hint");
        Check(gNone.Mappings.Count == 0, "no device -> nothing mapped");

        // COM error.
        var gErr = new FakeGateway { AvailableThrows = new InvalidOperationException("COM exploded") };
        var sErr = Service(gErr, 8000);
        sErr.Begin(4499, own, "192.168.1.20");
        st = WaitDone(sErr, 3000);
        Check(st.State == OnlineHostState.Failed && !st.Message.Contains("COM exploded"), "COM error -> Failed without leaking details to the player");

        // No LAN address.
        var sNoLan = Service(new FakeGateway(), 8000);
        sNoLan.Begin(4499, own, null);
        Check(WaitDone(sNoLan, 3000).State == OnlineHostState.Failed, "no LAN address -> Failed");

        // Timeout: Poll never blocks, the late worker removes what it added.
        var gSlow = new FakeGateway { AddGate = new ManualResetEvent(false) };
        var sSlow = Service(gSlow, 150);
        sSlow.Begin(4499, own, "192.168.1.20");
        Check(sSlow.Poll().State == OnlineHostState.Opening, "still Opening before the timeout");
        Thread.Sleep(250);
        sw.Restart();
        st = sSlow.Poll();
        Check(sw.ElapsedMilliseconds < 100 && st.State == OnlineHostState.Failed, "timeout -> Failed, Poll did not block");
        // Single flight (Copilot review 2026-11-17): no new attempt while the timed-out worker is still inside the router call.
        sSlow.Begin(4499, own, "192.168.1.20");
        Check(sSlow.Poll().State == OnlineHostState.Failed && gSlow.Count(own) == 0, "retry refused while the previous worker is alive");
        gSlow.AddGate.Set();   // the stalled router finally answers
        Check(WaitFor(delegate { return gSlow.Count(own) == 0 && gSlow.Removed.Count == 1; }, 3000), "late success after the timeout is taken back (own mapping removed)");
        Check(sSlow.Poll().State == OnlineHostState.Failed, "the late result does not revive a timed-out attempt");

        // CGNAT and private external addresses.
        foreach (string ext in new[] { "100.70.1.1", "10.1.2.3", "192.168.0.7", "0.0.0.0", "" })
        {
            var gc = new FakeGateway { ExternalIp = ext };
            var sc = Service(gc, 8000);
            sc.Begin(4499, own, "192.168.1.20");
            st = WaitDone(sc, 3000);
            Check(st.State == OnlineHostState.Failed && st.ExternalIp == "", "non-global external address '" + ext + "' -> Failed, no code");
            Check(WaitFor(delegate { return gc.Count(own) == 0; }, 3000), "our now-useless mapping is removed for '" + ext + "'");
        }
        var gCg = new FakeGateway { ExternalIp = "100.70.1.1" };
        var sCg = Service(gCg, 8000);
        sCg.Begin(4499, own, "192.168.1.20");
        Check(WaitDone(sCg, 3000).Message.Contains("Tailscale"), "CGNAT message points to Tailscale");

        // Foreign mapping on the port: untouched, nothing added.
        var gF = new FakeGateway();
        gF.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "UDP", InternalClient = "192.168.1.99", Description = "Someone else's console", ExternalIp = "8.8.4.4" });
        var sF = Service(gF, 8000);
        sF.Begin(4499, own, "192.168.1.20");
        st = WaitDone(sF, 3000);
        Check(st.State == OnlineHostState.Failed && st.Message.Contains("already forwarded"), "foreign mapping on the port -> Failed with a clear message");
        Check(gF.Mappings.Count == 1 && gF.Removed.Count == 0, "foreign mapping is never touched");
        Check(sF.Release(2000) && gF.Mappings.Count == 1 && gF.Removed.Count == 0, "Release after a failure leaves foreign mappings alone");

        // Stale own mapping (crash recovery) is replaced; foreign mappings on other ports and TCP stay.
        var gS = new FakeGateway();
        gS.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "UDP", InternalClient = "192.168.1.10", Description = own, ExternalIp = "8.8.4.4" });
        gS.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "TCP", InternalClient = "192.168.1.50", Description = "NAS", ExternalIp = "8.8.4.4" });
        gS.Mappings.Add(new UpnpMappingInfo { ExternalPort = 25565, Protocol = "TCP", InternalClient = "192.168.1.51", Description = "Minecraft", ExternalIp = "8.8.4.4" });
        gS.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4500, Protocol = "UDP", InternalClient = "192.168.1.52", Description = "GTAxSkate Coop someoneelse", ExternalIp = "8.8.4.4" });
        var sS = Service(gS, 8000);
        sS.Begin(4499, own, "192.168.1.20");
        st = WaitDone(sS, 3000);
        Check(st.State == OnlineHostState.Open, "stale own mapping does not block hosting");
        Check(gS.Removed.Count == 1 && gS.Removed[0] == own && gS.Count(own) == 1 && gS.Mappings.Find(m => m.Description == own).InternalClient == "192.168.1.20", "stale own mapping removed, then re-added for this PC");
        Check(sS.Release(3000), "release finishes");
        Check(gS.Count(own) == 0 && gS.Count("NAS") == 1 && gS.Count("Minecraft") == 1 && gS.Count("GTAxSkate Coop someoneelse") == 1,
            "release removed only our own mapping; TCP, other ports and other installs' mappings stay");

        // Stateless recovery by description (another loaded copy / crash): same own-only rule.
        var gR = new FakeGateway();
        gR.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "UDP", InternalClient = "192.168.1.10", Description = own });
        gR.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "TCP", InternalClient = "192.168.1.11", Description = own });
        Check(UpnpHostPortService.RemoveOwnAsync(delegate() { return gR; }, 4499, own, 3000), "stateless removal finishes");
        Check(gR.Mappings.Count == 1 && gR.Mappings[0].Protocol == "TCP", "stateless removal deletes only the UDP mapping with our description");
        var gNoDev = new FakeGateway { Available = false };
        Check(UpnpHostPortService.RemoveOwnAsync(delegate() { return gNoDev; }, 4499, own, 3000), "stateless removal without a device is a no-op");

        // Release while still Opening: the worker takes its own mapping back.
        var gO = new FakeGateway { AddGate = new ManualResetEvent(false) };
        var sO = Service(gO, 8000);
        sO.Begin(4499, own, "192.168.1.20");
        Thread.Sleep(50);
        sw.Restart();
        sO.Release(0);
        Check(sw.ElapsedMilliseconds < 100, "Release(0) never blocks");
        gO.AddGate.Set();
        Check(WaitFor(delegate { return gO.Count(own) == 0 && gO.Removed.Count == 1; }, 3000), "Release during Opening: late mapping is removed");
        Check(sO.Poll().State == OnlineHostState.Idle, "released service is Idle");

        // A new attempt after a failure is allowed.
        gNone.Available = true;
        sNone.Begin(4499, own, "192.168.1.20");
        Check(WaitDone(sNone, 3000).State == OnlineHostState.Open, "retry after a failure works");
        sNone.Release(3000);
        Check(UpnpHostPortService.DescriptionFor("abcd1234") == own && UpnpHostPortService.DescriptionFor("") == "GTAxSkate Coop install", "mapping description carries the install id");
    }
}
