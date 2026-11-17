// Round 3 of the streamer-safe / internet-lobby tests (Copilot round-2 review, build/test-review/2026-11-17-streamer/round2-peer.md):
// UPnP cleanup after a partial failure, single-flight router workers across services, SSDP sender checks, the
// local-server restart decision and case-insensitive public host names. Partial class of StreamerSafeTests.
using System;
using System.Collections.Generic;
using System.Threading;
using GTAxSkate.Coop;

public static partial class StreamerSafeTests
{
    private static void Round3Tests(string dir)
    {
        UpnpPartialFailureTests();
        UpnpSingleFlightTests();
        SsdpSpoofTests();
        LocalServerDecisionTests();
        Round3SafeTextTests();
    }

    // ---- 5. SSDP: only the default gateway's own answers count -------------------------------------------------------
    private static void SsdpSpoofTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";
        const string Attacker = "192.168.178.99";
        string bogusLocation = "http://" + Gateway + ":49000/bogus.xml";
        string bogus = "HTTP/1.1 200 OK\r\nLOCATION: " + bogusLocation + "\r\nST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n";

        // a LAN host forges a reply whose LOCATION names the gateway: its datagram sender is not the gateway
        FakeIgd t = FakeIgd.Standard();
        t.SsdpFirst.Add(new SsdpReply(Attacker, bogus));
        OnlineHostStatus st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open && !t.Gets.Contains(bogusLocation), "a reply from another sender is ignored even when its LOCATION host is the gateway: " + st.State);

        // the gateway itself answers twice: the first location is no IGD description, the second is; both are tried in order
        t = FakeIgd.Standard();
        t.SsdpFirst.Add(new SsdpReply(Gateway, bogus));
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open && t.Gets.Count == 2 && t.Gets[0] == bogusLocation && t.Gets[1] == DescUrl(Gateway), "every matching location is tried in order, not just the first: " + string.Join(",", t.Gets.ToArray()));

        // only forged replies: no device
        t = new FakeIgd();
        t.SsdpFirst.Add(new SsdpReply(Attacker, Ssdp(Gateway)));
        t.Pages[DescUrl(Gateway)] = IgdDescription;
        Check(!new UpnpIgdGateway(t).IsAvailable() && t.Gets.Count == 0, "forged replies alone never make a device");

        // the gateway answers, but with a LOCATION on another host
        t = new FakeIgd();
        t.SsdpFirst.Add(new SsdpReply(Gateway, Ssdp(Repeater)));
        t.Pages[DescUrl(Repeater)] = IgdDescription;
        Check(!new UpnpIgdGateway(t).IsAvailable() && t.Gets.Count == 0, "a location on another host is ignored even from the gateway");

        // no default gateway known: no device, nothing fetched (never accept an arbitrary location)
        t = FakeIgd.Standard();
        t.GatewayHost = null;
        Check(!new UpnpIgdGateway(t).IsAvailable() && t.Gets.Count == 0 && t.Posts.Count == 0, "without a default gateway no UPnP device is accepted");
    }

    // ---- 2. restart decision for the local server -------------------------------------------------------------------
    private static void LocalServerDecisionTests()
    {
        string a = CoopInvite.GeneratePassword(), b = CoopInvite.GeneratePassword();
        string fa = LocalServerKey.Fingerprint(a);
        Check(LocalServerKey.Decide(fa, a) == LocalServerAction.Reuse, "the same key reuses the server");
        Check(LocalServerKey.Decide(fa, b) == LocalServerAction.Refuse, "another key is refused, never restarted (round 4: a host action never kills a running server)");
        Check(LocalServerKey.Decide(null, b) == LocalServerAction.Refuse && LocalServerKey.Decide(string.Empty, b) == LocalServerAction.Refuse, "an unknown fingerprint is refused too");
    }

    // A gateway whose calls can block, to hold a worker inside a slow router call.
    private sealed class GateGateway : IUpnpGateway
    {
        public readonly object Lock = new object();
        public readonly List<UpnpMappingInfo> Mappings = new List<UpnpMappingInfo>();
        public ManualResetEvent AddGate, RemoveGate;
        public Exception AddThrowsAfterCreate;
        public int AddCalls;
        public bool IsAvailable() { return true; }
        public UpnpMappingInfo[] List() { lock (Lock) return Mappings.ToArray(); }
        public UpnpMappingInfo Add(int port, string protocol, string internalClient, string description)
        {
            lock (Lock) AddCalls++;
            if (AddGate != null) AddGate.WaitOne();
            var m = new UpnpMappingInfo { ExternalPort = port, Protocol = protocol, InternalClient = internalClient, Description = description, ExternalIp = "8.8.4.4" };
            lock (Lock) Mappings.Add(m);
            if (AddThrowsAfterCreate != null) throw AddThrowsAfterCreate;
            return m;
        }
        public UpnpMappingInfo Get(int port, string protocol)
        {
            lock (Lock) return Mappings.Find(delegate(UpnpMappingInfo m) { return m.ExternalPort == port && m.Protocol == protocol; });
        }
        public int RemoveFailures, RemoveCalls;   // the next RemoveFailures removals throw
        public void Remove(int port, string protocol)
        {
            if (RemoveGate != null) RemoveGate.WaitOne();
            lock (Lock)
            {
                RemoveCalls++;
                if (RemoveFailures > 0) { RemoveFailures--; throw new InvalidOperationException("router refused the delete"); }
            }
            lock (Lock) Mappings.RemoveAll(delegate(UpnpMappingInfo m) { return m.ExternalPort == port && m.Protocol == protocol; });
        }
        public int Count(string description) { lock (Lock) return Mappings.FindAll(delegate(UpnpMappingInfo m) { return m.Description == description; }).Count; }
    }

    // Retries Begin until it is accepted (a refused Begin is Failed at once); returns the final status.
    private static OnlineHostStatus BeginWhenFree(UpnpHostPortService s, int port, string desc, int withinMs)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (true)
        {
            s.Begin(port, desc, "192.168.178.23");
            OnlineHostStatus st = s.Poll();
            if (st.State != OnlineHostState.Failed || sw.ElapsedMilliseconds > withinMs) return st;
            Thread.Sleep(10);
        }
    }

    // ---- 1. Anything that fails after a successful AddPortMapping removes our mapping again -------------------------
    private static void UpnpPartialFailureTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";
        FakeIgd t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4500", "UDP", "192.168.178.60", "Foreign" });
        t.Replies["GetExternalIPAddress"] = new KeyValuePair<int, string>(500, SoapFault(606, "Action not authorized"));
        OnlineHostStatus st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message == UpnpIgdGateway.FritzNotAuthorized, "fault 606 on the external address read fails the attempt: " + st.Message);
        Check(t.Count("AddPortMapping") == 1 && t.Count("DeletePortMapping") >= 1 && t.Mappings.TrueForAll(delegate(string[] m) { return m[3] != Desc; }),
            "the mapping that AddPortMapping created is deleted again");
        Check(t.Mappings.Count == 1 && t.Mappings[0][3] == "Foreign", "a foreign mapping is never touched by the cleanup");

        t = FakeIgd.Standard();
        t.Replies["GetExternalIPAddress"] = new KeyValuePair<int, string>(0, null);
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && t.Mappings.Count == 0, "no answer to the external address read: failed, nothing left open");

        // service level: any exception after the router created the mapping
        var g = new GateGateway { AddThrowsAfterCreate = new InvalidOperationException("late failure") };
        var svc = new UpnpHostPortService(delegate() { return g; }, 5000);
        svc.Begin(4499, Desc, "192.168.178.23");
        st = WaitUpnp(svc);
        Check(st.State == OnlineHostState.Failed && g.Count(Desc) == 0, "a gateway that throws after creating the mapping leaves nothing open");
    }

    // ---- 3. Router workers are single flight across services ---------------------------------------------------------
    private static void UpnpSingleFlightTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";

        // a removal worker is still inside a slow router call: a new hosting waits (is refused) until it finished
        var g = new GateGateway { RemoveGate = new ManualResetEvent(false) };
        var first = new UpnpHostPortService(delegate() { return g; }, 5000);
        first.Begin(4499, Desc, "192.168.178.23");
        Check(WaitUpnp(first).State == OnlineHostState.Open && g.Count(Desc) == 1, "first hosting opens the port");
        first.Release(0);                       // removal worker starts and blocks inside Remove
        Thread.Sleep(100);
        var second = new UpnpHostPortService(delegate() { return g; }, 5000);
        second.Begin(4499, Desc, "192.168.178.23");
        OnlineHostStatus st = second.Poll();
        Check(st.State == OnlineHostState.Failed && g.AddCalls == 1, "a new hosting is refused while the previous removal is still running: " + st.State);
        g.RemoveGate.Set();
        st = BeginWhenFree(second, 4499, Desc, 5000);
        Check(st.State == OnlineHostState.Opening || st.State == OnlineHostState.Open, "after the removal finished the new hosting starts: " + st.State);
        st = WaitUpnp(second);
        Check(st.State == OnlineHostState.Open && g.Count(Desc) == 1, "and its mapping is not deleted by the old removal");
        second.Release(3000);

        // a timed-out worker still inside Add: another service must not start, and its late cleanup cannot hit a newer mapping
        g = new GateGateway { AddGate = new ManualResetEvent(false) };
        var slow = new UpnpHostPortService(delegate() { return g; }, 50);
        slow.Begin(4499, Desc, "192.168.178.23");
        Thread.Sleep(150);
        Check(slow.Poll().State == OnlineHostState.Failed, "the slow attempt timed out");
        var other = new UpnpHostPortService(delegate() { return g; }, 5000);
        other.Begin(4499, Desc, "192.168.178.23");
        st = other.Poll();
        Check(st.State == OnlineHostState.Failed && g.AddCalls == 1, "another service is refused while the timed-out worker is alive: " + st.State);
        g.AddGate.Set();                        // the old worker finishes, sees it was abandoned and removes what it added
        st = BeginWhenFree(other, 4499, Desc, 5000);
        Check(st.State == OnlineHostState.Opening || st.State == OnlineHostState.Open, "the next hosting starts once the old worker is done: " + st.State);
        st = WaitUpnp(other);
        Check(st.State == OnlineHostState.Open && g.Count(Desc) == 1, "the replacement mapping survives the old worker's cleanup");
        other.Release(3000);
    }

    // ---- 6. public host names in any case ---------------------------------------------------------------------------
    private static void Round3SafeTextTests()
    {
        SafeText.ClearSecrets();
        SafeText.SetUserNameForTests("RoboStreamer");
        string H = SafeText.Hidden;
        Func<string, string> f = SafeText.ForScreen;
        Check(f("host GAME.EXAMPLE.COM refused") == "host " + H + " refused", "an upper-case public host name: " + f("host GAME.EXAMPLE.COM refused"));
        Check(f("SKATE-HOST.EXAMPLE.ORG") == H && f("Skate-Host.Example.Com") == H && f("PLAY.MYGAME.XYZ down") == H + " down", "mixed and upper-case suffixes: " + f("Skate-Host.Example.Com"));
        Check(f("GAME.EXAMPLE.NET") == H && f("MY-SERVER.DE") == H, "fully upper-case host names with ambiguous suffixes are masked: " + f("GAME.EXAMPLE.NET") + " / " + f("MY-SERVER.DE"));
        foreach (string plain in new[] { "System.IO.IOException", "RageCoop.Server.exe", "Settings.xml", "Press Ctrl.S", "Version 1.5.12.15" })
            Check(f(plain) == plain, "still readable: " + plain + " -> " + f(plain));
        SafeText.SetUserNameForTests(null);
    }
}
