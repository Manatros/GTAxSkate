// Round 4 of the streamer-safe / internet-lobby tests (Copilot round-3 review, build/test-review/2026-11-17-streamer/round3-peer.md):
// public host names in every case, a removal that reads the router entry first (description + internal client), and
// a failed or timed-out removal that stays pending and is retried. The "never restart a running local server" rule
// is covered in LocalServerDecisionTests (Round 3) and RunStopLocalServerTests (adapter tests). Partial class of StreamerSafeTests.
using System;
using System.Collections.Generic;
using System.Threading;
using GTAxSkate.Coop;

public static partial class StreamerSafeTests
{
    private static void Round4Tests(string dir)
    {
        UpnpHostPortService.ClearPendingForTests();
        try
        {
            Round4SafeTextTests();
            UpnpOwnershipGuardTests();
            UpnpPendingCleanupTests();
        }
        finally { UpnpHostPortService.ClearPendingForTests(); }
    }

    private static bool WaitFor(Func<bool> condition, int ms)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < ms) { if (condition()) return true; Thread.Sleep(5); }
        return condition();
    }

    // ---- 2. every suffix is case-insensitive -----------------------------------------------------------------------
    private static void Round4SafeTextTests()
    {
        SafeText.ClearSecrets();
        SafeText.SetUserNameForTests("RoboStreamer");
        string H = SafeText.Hidden;
        Func<string, string> f = SafeText.ForScreen;
        Check(f("host Game.Example.NET refused") == "host " + H + " refused", "mixed-case .NET suffix: " + f("host Game.Example.NET refused"));
        Check(f("gAmE.example.Com") == H && f("Skate.Host.Me") == H && f("play.x.Io down") == H + " down" && f("my-box.DE") == H && f("a.b.Co") == H,
            "mixed-case ambiguous and common suffixes: " + f("gAmE.example.Com") + " / " + f("Skate.Host.Me"));
        Check(f("game.example.net") == H && f("GAME.EXAMPLE.NET") == H && f("game.example.COM") == H, "lower, upper and mixed all masked");
        // accepted false positive (round 4 decision): a masked word is cheaper than a leaked host name
        Check(f("System.Net") == H, "System.Net is masked (accepted false positive): " + f("System.Net"));
        foreach (string plain in new[] { "System.IO.IOException", "RageCoop.Server.exe", "Settings.xml", "Press Ctrl.S", "Version 1.5.12.15" })
            Check(f(plain) == plain, "still readable: " + plain + " -> " + f(plain));
        SafeText.SetUserNameForTests(null);
    }

    // ---- 3a. delete only our own entry on our own PC --------------------------------------------------------------
    private static void UpnpOwnershipGuardTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";
        const string Pc = "192.168.178.23";

        // IGD level: the entry is read first (GetSpecificPortMappingEntry), a mismatch is never deleted
        FakeIgd t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4499", "UDP", Pc, "Replacement app" });
        int removed = UpnpHostPortService.RemoveOwn(new UpnpIgdGateway(t), 4499, Desc, Pc);
        Check(removed == 0 && t.Count("GetSpecificPortMappingEntry") == 1 && t.Count("DeletePortMapping") == 0 && t.Mappings.Count == 1,
            "a replacement mapping with another description is read and left alone");

        t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4499", "UDP", "192.168.178.99", Desc });
        removed = UpnpHostPortService.RemoveOwn(new UpnpIgdGateway(t), 4499, Desc, Pc);
        Check(removed == 0 && t.Count("DeletePortMapping") == 0 && t.Mappings.Count == 1, "our description on another device's address is not deleted");

        t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4499", "UDP", Pc, Desc });
        removed = UpnpHostPortService.RemoveOwn(new UpnpIgdGateway(t), 4499, Desc, Pc);
        Check(removed == 1 && t.Count("GetSpecificPortMappingEntry") == 1 && t.Count("DeletePortMapping") == 1 && t.Mappings.Count == 0, "our own mapping on this PC is read, then deleted");

        t = FakeIgd.Standard();
        removed = UpnpHostPortService.RemoveOwn(new UpnpIgdGateway(t), 4499, Desc, Pc);
        Check(removed == 0 && t.Count("DeletePortMapping") == 0, "no entry (fault 714 on the read) means nothing to delete");

        // service level: somebody replaced our mapping after it opened; Release must not delete the replacement
        UpnpHostPortService.ClearPendingForTests();
        var g = new GateGateway();
        var svc = new UpnpHostPortService(delegate() { return g; }, 5000);
        svc.Begin(4499, Desc, Pc);
        Check(WaitUpnp(svc).State == OnlineHostState.Open, "hosting opens");
        lock (g.Lock) { g.Mappings.Clear(); g.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "UDP", InternalClient = Pc, Description = "Another app", ExternalIp = "8.8.4.4" }); }
        Check(svc.Release(3000), "release finishes");
        Check(g.Count("Another app") == 1 && g.RemoveCalls == 0 && UpnpHostPortService.PendingCleanupCount == 0,
            "Release leaves the replacement mapping alone and has nothing left to retry");

        // stateless removal (successor copy) honours the same guard
        g = new GateGateway();
        lock (g.Lock) g.Mappings.Add(new UpnpMappingInfo { ExternalPort = 4499, Protocol = "UDP", InternalClient = "192.168.178.99", Description = Desc, ExternalIp = "8.8.4.4" });
        Check(UpnpHostPortService.RemoveOwnAsync(delegate() { return g; }, 4499, Desc, Pc, 3000) && g.RemoveCalls == 0 && g.Count(Desc) == 1, "stateless removal skips an entry on another address");
        UpnpHostPortService.ClearPendingForTests();
    }

    // ---- 3b. a failed or timed-out removal stays pending and is retried -------------------------------------------------
    private static void UpnpPendingCleanupTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";
        const string Pc = "192.168.178.23";
        UpnpHostPortService.ClearPendingForTests();

        // Release: the router refuses the delete -> pending; the next host attempt retries it before adding
        var g = new GateGateway { RemoveFailures = 1 };
        var first = new UpnpHostPortService(delegate() { return g; }, 5000);
        first.Begin(4499, Desc, Pc);
        Check(WaitUpnp(first).State == OnlineHostState.Open && g.Count(Desc) == 1, "first hosting opens");
        first.Release(3000);
        Check(g.Count(Desc) == 1 && UpnpHostPortService.PendingCleanupCount == 1, "a failed removal leaves the mapping and keeps the cleanup pending");
        var second = new UpnpHostPortService(delegate() { return g; }, 5000);
        OnlineHostStatus st = BeginWhenFree(second, 4499, Desc, 5000);
        Check(st.State == OnlineHostState.Opening || st.State == OnlineHostState.Open, "the next hosting starts: " + st.State);
        st = WaitUpnp(second);
        Check(st.State == OnlineHostState.Open && g.RemoveCalls == 2 && g.Count(Desc) == 1 && UpnpHostPortService.PendingCleanupCount == 0,
            "the host attempt retried the pending removal (2 delete calls), added a fresh mapping and cleared the pending mark");
        second.Release(3000);
        Check(g.Count(Desc) == 0 && UpnpHostPortService.PendingCleanupCount == 0, "and the stop removes it");

        // Stop / adapter start: RetryPendingAsync
        g = new GateGateway { RemoveFailures = 1 };
        var s3 = new UpnpHostPortService(delegate() { return g; }, 5000);
        s3.Begin(4499, Desc, Pc);
        WaitUpnp(s3);
        s3.Release(3000);
        Check(g.Count(Desc) == 1 && UpnpHostPortService.PendingCleanupCount == 1, "again a failed removal is pending");
        UpnpHostPortService.RetryPendingAsync(delegate() { return g; }, 3000);
        Check(g.Count(Desc) == 0 && UpnpHostPortService.PendingCleanupCount == 0, "the retry at adapter start / stop removes it");

        // a failed attempt (Add threw after creating) whose cleanup fails is pending too
        g = new GateGateway { AddThrowsAfterCreate = new InvalidOperationException("late failure"), RemoveFailures = 1 };
        var s4 = new UpnpHostPortService(delegate() { return g; }, 5000);
        s4.Begin(4499, Desc, Pc);
        Check(WaitUpnp(s4).State == OnlineHostState.Failed && g.Count(Desc) == 1 && UpnpHostPortService.PendingCleanupCount == 1,
            "a failed attempt whose cleanup also failed keeps it pending");
        UpnpHostPortService.RetryPendingAsync(delegate() { return g; }, 3000);
        Check(g.Count(Desc) == 0 && UpnpHostPortService.PendingCleanupCount == 0, "and the retry cleans it up");

        // a timed-out removal (router never answers) is pending from the start
        g = new GateGateway { RemoveGate = new ManualResetEvent(false) };
        var s5 = new UpnpHostPortService(delegate() { return g; }, 5000);
        s5.Begin(4499, Desc, Pc);
        WaitUpnp(s5);
        Check(!s5.Release(100) && UpnpHostPortService.PendingCleanupCount == 1, "a removal that has not finished within the wait is pending");
        g.RemoveGate.Set();
        Check(WaitFor(delegate() { return g.Count(Desc) == 0 && UpnpHostPortService.PendingCleanupCount == 0; }, 3000), "when the router answers late, the removal completes and clears the mark");

        // bounded: a router that always refuses is retried 5 times, then given up
        UpnpHostPortService.ClearPendingForTests();
        g = new GateGateway { RemoveFailures = 1000 };
        var s6 = new UpnpHostPortService(delegate() { return g; }, 5000);
        s6.Begin(4499, Desc, Pc);
        WaitUpnp(s6);
        s6.Release(3000);
        for (int i = 0; i < 8; i++) UpnpHostPortService.RetryPendingAsync(delegate() { return g; }, 3000);
        Check(g.RemoveCalls == 1 + UpnpHostPortService.MaxCleanupRetries && UpnpHostPortService.PendingCleanupCount == 0,
            "retries are bounded (1 + " + UpnpHostPortService.MaxCleanupRetries + " delete calls, then dropped): " + g.RemoveCalls);

        // bounded list: never more than MaxPendingCleanups entries
        UpnpHostPortService.ClearPendingForTests();
        for (int port = 5000; port < 5012; port++)
            UpnpHostPortService.RemoveOwnAsync(delegate() { throw new InvalidOperationException("no router"); }, port, Desc, Pc, 3000);
        Check(UpnpHostPortService.PendingCleanupCount == UpnpHostPortService.MaxPendingCleanups, "the pending list is bounded: " + UpnpHostPortService.PendingCleanupCount);
        UpnpHostPortService.ClearPendingForTests();
    }
}
