// Offline tests for streamer-safe coop (design 2026-11-17-streamer-safe-coop): CoopInvite, the password generator,
// SafeText.ForScreen, on-screen host labels and the banner, ClipboardWorker behind a fake backend and the
// RageCoop client-settings UI lock. No network, no game, no real clipboard.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Threading;
using GTAxSkate.Coop;

public static partial class StreamerSafeTests
{
    private static int checks;
    private static void Check(bool condition, string message)
    {
        checks++;
        if (!condition) throw new Exception("FAIL: " + message);
    }

    private const string Password = "7H2KQ9XD4MZP0VBN3TRE";

    public static int Main()
    {
        try
        {
            string dir = Path.Combine(Environment.GetEnvironmentVariable("GTAXSKATE_COOP_TEST_TMP") ?? Path.GetTempPath(), "streamer-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(dir);
            try
            {
                InviteTests();
                PasswordTests();
                SafeTextTests();
                LabelAndBannerTests();
                ClipboardWorkerTests();
                SettingsLockTests(dir);
                Round2Tests(dir);
                Round3Tests(dir);
                Round4Tests(dir);
            }
            finally { try { Directory.Delete(dir, true); } catch { } }
            Console.WriteLine("PASS: " + checks + " streamer-safe checks (invite, password, sanitizer, labels, clipboard worker, settings lock, UPnP IGD client, rounds 2-4); no network, game or real clipboard.");
            return 0;
        }
        catch (Exception e) { Console.Error.WriteLine(e); return 1; }
    }

    // ---- CoopInvite ----------------------------------------------------------------------------------------------
    private static void InviteTests()
    {
        string code;
        Check(LobbyCode.TryEncode("8.8.4.4", 4499, out code), "code encodes");
        string online = CoopInvite.BuildOnline(code, Password);
        Check(online == "GTAxSkate coop invite " + code + " " + Password, "online invite format");
        CoopInviteData d;
        Check(CoopInvite.Parse(online, out d) == InviteStatus.Ok && d.Kind == InviteLocatorKind.Online && d.Host == "8.8.4.4" && d.Port == 4499 && d.Password == Password,
            "online round trip");
        var req = CoopInvite.ToConnectRequest(d);
        Check(req.ExplicitPinnedFriendHost && req.EndpointAddress == "8.8.4.4" && req.Port == 4499 && req.FriendSharedKey == Password && req.HostLabel == "Online host" &&
            req.Source == LobbyDiscoverySource.DirectFriendInvite && CoopSecurityPolicy.EvaluateOutboundConnect(req).Allowed, "an online invite becomes a pinned public friend request that the policy admits");

        string lan = CoopInvite.BuildLocal(InviteLocatorKind.Lan, "192.168.178.23", 4499, Password);
        Check(lan == "GTAxSkate coop invite lan:192.168.178.23:4499 " + Password, "LAN invite format");
        Check(CoopInvite.Parse(lan, out d) == InviteStatus.Ok && d.Kind == InviteLocatorKind.Lan && d.Host == "192.168.178.23" && d.Port == 4499 && d.Password == Password, "LAN round trip");
        req = CoopInvite.ToConnectRequest(d);
        Check(!req.ExplicitPinnedFriendHost && req.HostLabel == "LAN host" && CoopSecurityPolicy.EvaluateOutboundConnect(req).Allowed, "a LAN invite is never pinned-public");
        string vpn = CoopInvite.BuildLocal(InviteLocatorKind.Vpn, "100.101.102.103", 4510, Password);
        Check(CoopInvite.Parse(vpn, out d) == InviteStatus.Ok && d.Kind == InviteLocatorKind.Vpn && d.Host == "100.101.102.103" && d.Port == 4510 &&
            CoopInvite.ToConnectRequest(d).HostLabel == "VPN host", "VPN round trip");

        // Builders refuse what the parser would refuse.
        Check(CoopInvite.BuildOnline("SK8-0000-0000-0000", Password) == null, "bad lobby code is not built");
        Check(CoopInvite.BuildOnline(code, "short") == null && CoopInvite.BuildOnline(code, null) == null, "short or missing password is not built");
        Check(CoopInvite.BuildLocal(InviteLocatorKind.Lan, "8.8.4.4", 4499, Password) == null, "a public address is not a LAN locator");
        Check(CoopInvite.BuildLocal(InviteLocatorKind.Lan, "100.64.1.1", 4499, Password) == null, "a VPN address is not a LAN locator");
        Check(CoopInvite.BuildLocal(InviteLocatorKind.Vpn, "192.168.1.5", 4499, Password) == null, "a LAN address is not a VPN locator");
        Check(CoopInvite.BuildLocal(InviteLocatorKind.Lan, "192.168.1.5", 80, Password) == null && CoopInvite.BuildLocal(InviteLocatorKind.Lan, "192.168.1.5", 70000, Password) == null, "ports outside 1024..65535 are refused");
        Check(CoopInvite.BuildLocal(InviteLocatorKind.Online, "192.168.1.5", 4499, Password) == null, "Online has no local builder");

        // Parser rejections.
        Func<string, InviteStatus> parse = delegate(string t) { CoopInviteData x; return CoopInvite.Parse(t, out x); };
        Check(parse(null) == InviteStatus.Empty && parse("") == InviteStatus.Empty && parse("  \r\n\t ") == InviteStatus.Empty, "empty clipboard");
        Check(parse("hello world") == InviteStatus.WrongFormat, "random text");
        Check(parse("GTAxSkate invite " + code + " " + Password) == InviteStatus.WrongFormat, "wrong prefix (missing coop)");
        Check(parse("SkateMods coop invite " + code + " " + Password) == InviteStatus.WrongFormat, "wrong prefix (other product)");
        Check(parse("GTAxSkate coop invite " + code) == InviteStatus.WrongFormat, "no password token");
        string typo = code.Substring(0, 5) + (code[5] == '2' ? '3' : '2') + code.Substring(6);
        Check(parse("GTAxSkate coop invite " + typo + " " + Password) == InviteStatus.BadLocator, "mistyped lobby code (checksum)");
        Check(parse("GTAxSkate coop invite " + LobbyCode.EncodeRaw(new byte[] { 10, 0, 0, 5 }, 4499) + " " + Password) == InviteStatus.BadLocator, "reserved-range lobby code");
        Check(parse("GTAxSkate coop invite lan:8.8.4.4:4499 " + Password) == InviteStatus.BadLocator, "public IP in a lan: locator");
        Check(parse("GTAxSkate coop invite vpn:192.168.1.5:4499 " + Password) == InviteStatus.BadLocator, "LAN IP in a vpn: locator");
        Check(parse("GTAxSkate coop invite lan:192.168.1.5 " + Password) == InviteStatus.BadLocator, "locator without a port");
        Check(parse("GTAxSkate coop invite lan:192.168.1.5:80 " + Password) == InviteStatus.BadLocator, "locator with a reserved port");
        Check(parse("GTAxSkate coop invite lan:192.168.1.5:99999 " + Password) == InviteStatus.BadLocator, "locator with an impossible port");
        Check(parse("GTAxSkate coop invite lan:localhost:4499 " + Password) == InviteStatus.BadLocator, "a host name is not a locator");
        Check(parse("GTAxSkate coop invite somewhere " + Password) == InviteStatus.BadLocator, "unknown locator");
        Check(parse("GTAxSkate coop invite " + code + " short") == InviteStatus.BadPassword, "short password");
        Check(parse("GTAxSkate coop invite " + code + " " + new string('A', 65)) == InviteStatus.BadPassword, "overlong password");
        Check(parse("GTAxSkate coop invite " + code + " " + Password.Substring(0, 12) + "!!!!!!!!") == InviteStatus.BadPassword, "password with symbols");
        Check(parse(new string('x', 600)) == InviteStatus.WrongFormat, "oversized clipboard");

        // Whitespace / chat tolerance (Discord, mail, Notepad).
        Check(parse("  \r\n GTAxSkate   coop\tinvite  " + code + "  \r\n" + Password + " \r\n") == InviteStatus.Ok, "extra whitespace and newlines");
        Check(parse("gtaxskate COOP Invite " + code + " " + Password) == InviteStatus.Ok, "prefix case is ignored");
        Check(parse("`GTAxSkate coop invite " + code + " " + Password + "`") == InviteStatus.Ok, "backticks");
        Check(parse("\"GTAxSkate coop invite " + code + " " + Password + "\"") == InviteStatus.Ok, "quotes");
        Check(parse("<GTAxSkate coop invite " + code + " " + Password + ">") == InviteStatus.Ok, "angle brackets");
        Check(parse("GTAxSkate coop invite " + code.Substring(0, 8) + "\r\n" + code.Substring(8) + " " + Password) == InviteStatus.Ok, "a lobby code split by a line break");
        Check(parse("GTAxSkate coop invite " + code.ToLowerInvariant() + " " + Password) == InviteStatus.Ok, "a lower-case lobby code");
        Check(CoopInvite.Describe(InviteStatus.Empty).StartsWith("Copy the invite your friend sent you") && CoopInvite.Describe(InviteStatus.BadLocator).Contains("not a valid invite"), "player-facing texts");
        Check(CoopInvite.Describe(InviteStatus.BadPassword).IndexOf(Password, StringComparison.Ordinal) < 0, "texts carry no secret");
    }

    // ---- password generator --------------------------------------------------------------------------------------
    private static void PasswordTests()
    {
        const string alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        var seen = new HashSet<string>();
        var symbols = new int[32];
        for (int i = 0; i < 1000; i++)
        {
            string p = CoopInvite.GeneratePassword();
            Check(p.Length == 20, "length 20");
            Check(CoopInvite.IsValidPassword(p), "valid for the invite format");
            foreach (char c in p) { int at = alphabet.IndexOf(c); Check(at >= 0, "Crockford alphabet only: " + c); symbols[at]++; }
            seen.Add(p);
        }
        Check(seen.Count == 1000, "1,000 draws are unique");
        foreach (int n in symbols) Check(n > 400 && n < 900, "no symbol is wildly over- or under-used (20,000 symbols over 32 values, got " + n + ")");
        Check(CoopSecurityPolicy.MinFriendKeyLength <= CoopInvite.PasswordLength, "generated passwords satisfy the friend-key minimum");
        Check(CoopInvite.IsValidPassword("abcDEF123456") && !CoopInvite.IsValidPassword("abcDEF12345") && !CoopInvite.IsValidPassword("abcDEF 123456") && !CoopInvite.IsValidPassword(null), "validity rules");
        // The generated password works as the friend key of the mutual proof (no characters the HMAC code dislikes).
        byte[] n1 = Guid.NewGuid().ToByteArray(), n2 = Guid.NewGuid().ToByteArray();
        string pw = CoopInvite.GeneratePassword();
        Check(CoopSecurityPolicy.VerifyMutualProof(pw, n1, n2, "CLI", CoopSecurityPolicy.ComputeMutualProof(pw, n1, n2, "CLI")), "a generated password authenticates");
    }

    // ---- SafeText ------------------------------------------------------------------------------------------------
    private static void SafeTextTests()
    {
        SafeText.ClearSecrets();
        SafeText.SetUserNameForTests("RoboStreamer");
        string H = SafeText.Hidden;
        Func<string, string> f = SafeText.ForScreen;

        Check(f(null) == string.Empty && f("") == string.Empty, "null and empty");
        Check(f("Connected to 192.168.1.20 now") == "Connected to " + H + " now", "IPv4");
        Check(f("port 100.64.0.1:4499 open") == "port " + H + " open", "IPv4 with a port");
        Check(f("a 8.8.4.4, b 203.0.113.9.") == "a " + H + ", b " + H + ".", "several IPv4 addresses and trailing punctuation");
        Check(f("[fe80::1]:4499 and 2001:db8::7 and ::1") == H + " and " + H + " and " + H, "IPv6 forms");
        Check(f("2001:0db8:0000:0000:0000:ff00:0042:8329 end") == H + " end", "a full IPv6 address");
        Check(f("host my-pc.local:4499 refused") == "host " + H + " refused", "host:port");
        Check(f("localhost:4499") == H, "localhost:port");
        string pathOut = f(@"open C:\Users\RoboStreamer\Documents\GTA\log.txt now");
        Check(pathOut == "open " + H + " now", "a Windows path: " + pathOut);
        Check(f(@"D:/Games/GTA/x.dll failed") == H + " failed", "a forward-slash path");
        Check(f(@"share \\nas\public\gta.rpf missing") == "share " + H + " missing", "a UNC path");
        Check(f("%USERPROFILE% and %USERNAME%") == H + " and " + H, "environment variable references");
        Check(f("hello RoboStreamer, welcome") == "hello " + H + ", welcome" && f("hello robostreamer") == "hello " + H, "the Windows user name, any case");
        Check(f("RoboStreamers are fine") == "RoboStreamers are fine", "the user name only as a whole word");
        Check(f("lobby SK8-ABCD-EFGH-JKMN ready") == "lobby " + H + " ready" && f("sk8 abcd efgh jkmn") == H, "lobby codes");
        // Ordinary text stays readable.
        foreach (string plain in new[] { "[SOLO] Offline Story Mode (no other player connected)", "Skaters: 2", "Slot 3 of 10", "at 12:30 pm", "Connecting to Online host...",
            "Invite copied to the clipboard (hidden for streaming). Send it to your friend in a private message.", "[CONNECTED - PRIVATE COOP] Host: Friend [LAN] | Skaters: 2",
            "Version 1.5 build 12", "UDP 4499", "Press F11" })
            Check(f(plain) == plain, "plain text unchanged: " + plain);
        // Secrets.
        SafeText.AddSecret(SafeText.GroupHost, Password);
        SafeText.AddSecret(SafeText.GroupHost, "GTAxSkate coop invite lan:192.168.1.5:4499 " + Password);
        SafeText.AddSecret(SafeText.GroupSession, "ABC234");
        Check(f("pw " + Password + " end") == "pw " + H + " end" && f("pw " + Password.ToLowerInvariant()) == "pw " + H, "a registered password, any case");
        Check(f("code ABC234.") == "code " + H + "." && f("ABC2345 stays") == "ABC2345 stays" && f("xABC234") == "xABC234", "a short secret matches whole words only");
        Check(f("whole GTAxSkate coop invite lan:192.168.1.5:4499 " + Password).IndexOf("coop invite", StringComparison.Ordinal) < 0, "the registered invite text is masked as a whole");
        Check(SafeText.SecretCount == 3, "three secrets registered");
        SafeText.AddSecret(SafeText.GroupHost, Password); SafeText.AddSecret(SafeText.GroupHost, "ab");
        Check(SafeText.SecretCount == 3, "duplicates and tiny secrets are ignored");
        SafeText.ClearGroup(SafeText.GroupHost);
        Check(f("pw " + Password) == "pw " + Password && f("code ABC234") == "code " + H, "clearing a group removes only its secrets");
        SafeText.ClearSecrets();
        Check(SafeText.SecretCount == 0, "all secrets cleared");
        for (int i = 0; i < 40; i++) SafeText.AddSecret("g", "secret-number-" + i);
        Check(SafeText.SecretCount <= 24, "the secret list is bounded");
        SafeText.ClearSecrets();
        Check(f(f("a 8.8.4.4 b")) == f("a 8.8.4.4 b"), "idempotent");
        SafeText.SetUserNameForTests(null);
    }

    // ---- labels and banner --------------------------------------------------------------------------------------
    private static void LabelAndBannerTests()
    {
        foreach (string fixedLabel in new[] { "Friend", "LocalHost", "Online host", "LAN host", "VPN host" })
            Check(CoopSecurityPolicy.ScreenLabel(fixedLabel) == fixedLabel, "fixed label kept: " + fixedLabel);
        Check(CoopSecurityPolicy.ScreenLabel(null) == "Friend" && CoopSecurityPolicy.ScreenLabel("") == "Friend" && CoopSecurityPolicy.ScreenLabel("   ") == "Friend", "empty label -> Friend");
        Check(CoopSecurityPolicy.ScreenLabel("Alex-VPN") == "Friend" && CoopSecurityPolicy.ScreenLabel("Friends-Tailscale") == "Friend" && CoopSecurityPolicy.ScreenLabel("HomePC") == "Friend",
            "a configured free-text label is never shown (only the fixed set)");
        foreach (string bad in new[] { "Alex 2", "192.168.1.5", "pc.local", "host:4499", "me@home", @"C:\x", "a/b", "x\\y", "Alex\u00e9", "has\ttab" })
            Check(CoopSecurityPolicy.ScreenLabel(bad) == "Friend", "label with a digit, dot, colon, @, slash or non-ASCII -> Friend: " + bad);
        Check(CoopSecurityPolicy.ScreenLabel(new string('a', 80)) == "Friend", "an overlong free-text label -> Friend");

        var tracker = new SessionConnectionTracker();
        byte[] cn = Guid.NewGuid().ToByteArray(), sn = Guid.NewGuid().ToByteArray();
        PolicyDecision decision;
        // An empty label must not fall back to the address any more.
        Check(tracker.BeginFriendConnect(new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, "100.88.12.34", 4499, false, "", Password), cn, 1, out decision), "connect starts");
        string banner = tracker.BuildStatusBanner();
        Check(banner == "[CHECKING] Verifying private friend host: Friend [VPN]", "checking banner: " + banner);
        Check(tracker.CompleteMutualServerVerification(cn, sn, CoopSecurityPolicy.ComputeMutualProof(Password, cn, sn, "SRV"), "ABC234", 2, 2), "verified");
        banner = tracker.BuildStatusBanner();
        Check(banner == "[CONNECTED - PRIVATE COOP] Host: Friend [VPN] | Skaters: 2", "connected banner shows label + trust tag + count only: " + banner);
        foreach (var pair in new[] { new KeyValuePair<string, EndpointTrustClass>("127.0.0.1", EndpointTrustClass.Loopback), new KeyValuePair<string, EndpointTrustClass>("192.168.1.5", EndpointTrustClass.PrivateLan),
            new KeyValuePair<string, EndpointTrustClass>("8.8.4.4", EndpointTrustClass.PublicInternet) })
        {
            var t = new SessionConnectionTracker();
            t.BeginFriendConnect(new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, pair.Key, 4499, true, "Online host", Password), cn, 1, out decision);
            string tag = pair.Value == EndpointTrustClass.Loopback ? "[LOCALHOST]" : pair.Value == EndpointTrustClass.PrivateLan ? "[LAN]" : "[ONLINE]";
            Check(t.BuildStatusBanner().Contains(tag) && !t.BuildStatusBanner().Contains(pair.Key), "trust tag " + tag + " without the address");
        }
        var blocked = new SessionConnectionTracker();
        Check(!blocked.BeginFriendConnect(new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, "8.8.4.4", 4499, false, "x", Password), cn, 1, out decision), "unpinned public blocked");
        Check(blocked.BuildStatusBanner().StartsWith("[BLOCKED:") && blocked.BuildStatusBanner().IndexOf("8.8.4.4", StringComparison.Ordinal) < 0, "blocked banner names no address");
        blocked.FailBlocked("connection to 10.1.2.3:4499 failed in C:\\Users\\" + Environment.UserName + "\\x", 3);
        Check(blocked.BuildStatusBanner().IndexOf("10.1.2.3", StringComparison.Ordinal) < 0 && blocked.BuildStatusBanner().IndexOf("Users", StringComparison.Ordinal) < 0, "banner text is sanitized too");
        // A configured file keeps working and its address is not shown.
        ClientConnectRequest cfg; string err;
        Check(ClientConfigLoader.TryParseClientConfigJson("{\"BlockPublicServerBrowser\":true,\"TrustedHostLabel\":\"Bob 192.168.1.9\",\"TrustedHostIp\":\"100.64.2.3\",\"Port\":4499,\"FriendKey\":\"" + Password + "\"}", out cfg, out err), "config parses");
        Check(cfg.HostLabel == "Friend" && cfg.EndpointAddress == "100.64.2.3", "a configured label with a digit is shown as Friend, the address stays internal");
        Check(ClientConfigLoader.TryParseClientConfigJson("{\"BlockPublicServerBrowser\":true,\"TrustedHostLabel\":\"Alex-VPN\",\"TrustedHostIp\":\"100.64.2.3\",\"Port\":4499,\"FriendKey\":\"" + Password + "\"}", out cfg, out err) && cfg.HostLabel == "Friend", "even a clean configured label is shown as Friend");
        var rows = new CoopInGameMenuState().BuildVisibleItems(false, false, true, cfg.HostLabel, false, false, "ABC234", "Online lobby: open", true);
        foreach (var row in rows) Check(row.Label.IndexOf("ABC234", StringComparison.Ordinal) < 0 && !System.Text.RegularExpressions.Regex.IsMatch(row.Label, @"\d{1,3}\.\d{1,3}"), "no menu row carries a code or an address: " + row.Label);
    }

    // ---- ClipboardWorker -----------------------------------------------------------------------------------------
    private sealed class FakeClipboard : IClipboardBackend
    {
        public readonly object Lock = new object();
        public string Text = string.Empty;
        public bool FailGet, FailSet, FailClear, IgnoreSet;
        public ApartmentState Apartment = ApartmentState.Unknown;
        public readonly HashSet<int> ThreadIds = new HashSet<int>();
        public readonly List<string> Log = new List<string>();
        public int DelayMs;
        private void Enter() { lock (Lock) { Apartment = Thread.CurrentThread.GetApartmentState(); ThreadIds.Add(Thread.CurrentThread.ManagedThreadId); } if (DelayMs > 0) Thread.Sleep(DelayMs); }
        public string GetText() { Enter(); lock (Lock) { Log.Add("get"); if (FailGet) throw new InvalidOperationException("clipboard busy"); return Text; } }
        public void SetTextHidden(string text) { Enter(); lock (Lock) { Log.Add("set:" + text); if (FailSet) throw new InvalidOperationException("clipboard busy"); if (!IgnoreSet) Text = text; } }
        public void Clear() { Enter(); lock (Lock) { Log.Add("clear"); if (FailClear) throw new InvalidOperationException("clipboard busy"); Text = string.Empty; } }
    }

    private static ClipboardOpResult Wait(ClipboardWorker w, int ticket)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < 5000)
        {
            ClipboardOpResult r = w.Poll(ticket);
            if (r.State != ClipboardOpState.Pending) return r;
            Thread.Sleep(2);
        }
        throw new Exception("FAIL: clipboard operation did not finish");
    }

    private static void ClipboardWorkerTests()
    {
        var fake = new FakeClipboard();
        var worker = new ClipboardWorker(fake);
        // Copy: real completion status, on an STA background thread, never on the caller's thread.
        int t = worker.Begin(ClipboardOpKind.CopyHidden, "invite one", true);
        Check(t > 0, "a ticket is returned at once");
        ClipboardOpResult r = Wait(worker, t);
        Check(r.State == ClipboardOpState.Done && r.Changed && fake.Text == "invite one", "copy completes and reports Done");
        Check(fake.Apartment == ApartmentState.STA && !fake.ThreadIds.Contains(Thread.CurrentThread.ManagedThreadId), "the backend runs on an STA thread that is not the caller");
        Check(worker.Poll(t).State == ClipboardOpState.Failed, "a finished result is consumed once; a later poll says Failed");
        Check(worker.Poll(0).State == ClipboardOpState.Failed && worker.Poll(9999).State == ClipboardOpState.Failed, "unknown tickets fail");

        // Read.
        fake.Text = "from the friend";
        r = Wait(worker, worker.Begin(ClipboardOpKind.Read, null, true));
        Check(r.State == ClipboardOpState.Done && r.Text == "from the friend", "read returns the text");
        fake.Text = null;
        r = Wait(worker, worker.Begin(ClipboardOpKind.Read, null, true));
        Check(r.State == ClipboardOpState.Done && r.Text == string.Empty, "an empty clipboard reads as empty text");

        // Failures are reported, never swallowed.
        fake.FailSet = true;
        Check(Wait(worker, worker.Begin(ClipboardOpKind.CopyHidden, "x", true)).State == ClipboardOpState.Failed, "a backend exception on copy -> Failed");
        fake.FailSet = false; fake.IgnoreSet = true; fake.Text = "other app text";
        Check(Wait(worker, worker.Begin(ClipboardOpKind.CopyHidden, "mine", true)).State == ClipboardOpState.Failed, "a copy that did not stick (read-back differs) -> Failed");
        fake.IgnoreSet = false; fake.FailGet = true;
        Check(Wait(worker, worker.Begin(ClipboardOpKind.Read, null, true)).State == ClipboardOpState.Failed, "a backend exception on read -> Failed");
        fake.FailGet = false;

        // Compare-and-clear.
        fake.Text = "  my invite \r\n";
        r = Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, "my invite", true));
        Check(r.State == ClipboardOpState.Done && !r.Changed && fake.Text == "  my invite \r\n", "compare-and-clear is exact: added boundary whitespace means the clipboard changed, so it is not erased");
        fake.Text = "my invite";
        r = Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, "my invite", true));
        Check(r.State == ClipboardOpState.Done && r.Changed && fake.Text == string.Empty, "cleared when it still holds exactly the same text");
        fake.Text = "user copied something else";
        r = Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, "my invite", true));
        Check(r.State == ClipboardOpState.Done && !r.Changed && fake.Text == "user copied something else", "never cleared when the clipboard changed");
        fake.Text = string.Empty;
        r = Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, "my invite", true));
        Check(r.State == ClipboardOpState.Done && !r.Changed, "an empty clipboard stays untouched");
        r = Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, string.Empty, true));
        Check(r.State == ClipboardOpState.Done && !r.Changed, "an empty expectation clears nothing");
        fake.Text = "x"; fake.FailClear = true;
        Check(Wait(worker, worker.Begin(ClipboardOpKind.ClearIfEquals, "x", true)).State == ClipboardOpState.Failed, "a failing clear is reported");
        fake.FailClear = false;

        // Operations run in the order they were begun and a pending ticket stays Pending until done.
        var slow = new FakeClipboard { DelayMs = 30 };
        var w2 = new ClipboardWorker(slow);
        int a = w2.Begin(ClipboardOpKind.CopyHidden, "first", true);
        int b = w2.Begin(ClipboardOpKind.CopyHidden, "second", true);
        int c = w2.Begin(ClipboardOpKind.Read, null, true);
        Check(w2.Poll(c).State == ClipboardOpState.Pending, "the caller never waits: results are Pending first");
        Check(Wait(w2, c).Text == "second", "operations run strictly in begin order");
        Check(Wait(w2, a).State == ClipboardOpState.Done && Wait(w2, b).State == ClipboardOpState.Done, "earlier tickets are all Done");
        // Fire and forget keeps no result but still runs, and Shutdown lets a queued clear finish.
        slow.Text = "keep me not";
        int ff = w2.Begin(ClipboardOpKind.ClearIfEquals, "keep me not", false);
        Check(ff > 0 && w2.Poll(ff).State == ClipboardOpState.Failed, "a fire-and-forget op has no pollable result");
        w2.Shutdown(3000);
        Check(slow.Text == string.Empty, "Shutdown waited for the queued clear");
        Check(w2.Begin(ClipboardOpKind.Read, null, true) == 0, "a stopped worker accepts nothing");

        // Many operations stay ordered and bounded; results of unpolled tickets are pruned.
        var fake3 = new FakeClipboard();
        var w3 = new ClipboardWorker(fake3);
        int last = 0;
        for (int i = 0; i < 200; i++) last = w3.Begin(ClipboardOpKind.CopyHidden, "v" + i, true);
        Check(Wait(w3, last).State == ClipboardOpState.Done && fake3.Text == "v199", "200 queued copies end with the last one");
        Check(w3.Poll(1).State == ClipboardOpState.Failed, "old unpolled results were pruned");
        w3.Dispose();
        worker.Dispose();
        w2.Dispose();
    }

    // ---- RageCoop client settings lock ---------------------------------------------------------------------------
    private static string Read(string path) { return File.ReadAllText(path, Encoding.UTF8); }

    private static void SettingsLockTests(string dir)
    {
        string error;
        // An existing user file: other values survive, the four keys are locked, the originals come back.
        string path = Path.Combine(Path.Combine(dir, "a"), "RageCoop.Client.Settings.xml");
        Directory.CreateDirectory(Path.GetDirectoryName(path));
        File.WriteAllText(path, "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Settings>\r\n  <Username>Real Name 42</Username>\r\n  <Password></Password>\r\n  <LastServerAddress>203.0.113.9:4499</LastServerAddress>\r\n" +
            "  <MasterServer>https://masterserver.ragecoop.com/</MasterServer>\r\n  <MenuKey>F9</MenuKey>\r\n  <Voice>false</Voice>\r\n</Settings>\r\n", new UTF8Encoding(false));
        Check(!RageCoopClientSettings.IsLocked(path), "not locked at first");
        Check(RageCoopClientSettings.Apply(path, out error) && error == string.Empty, "apply succeeds");
        string text = Read(path);
        Check(RageCoopClientSettings.IsLocked(path) && text.Contains("<MenuKey>F24</MenuKey>") && text.Contains("<Username>Skater</Username>") && text.Contains("<LastServerAddress>127.0.0.1:4499</LastServerAddress>") &&
            text.Contains("<Voice>false</Voice>") && text.IndexOf("Real Name", StringComparison.Ordinal) < 0 && text.IndexOf("203.0.113.9", StringComparison.Ordinal) < 0 && text.IndexOf("masterserver", StringComparison.Ordinal) < 0,
            "the four values are locked, others kept, the user's name and address are gone from the file");
        Check(File.Exists(RageCoopClientSettings.BackupPath(path)), "a sidecar backup holds the originals");
        // A second apply (every start) must not overwrite the backup with the locked values.
        Check(RageCoopClientSettings.Apply(path, out error), "apply twice");
        Check(RageCoopClientSettings.Restore(path, out error) && error == string.Empty, "restore succeeds");
        text = Read(path);
        Check(text.Contains("<Username>Real Name 42</Username>") && text.Contains("<MenuKey>F9</MenuKey>") && text.Contains("<LastServerAddress>203.0.113.9:4499</LastServerAddress>") &&
            text.Contains("masterserver.ragecoop.com") && !File.Exists(RageCoopClientSettings.BackupPath(path)) && !RageCoopClientSettings.IsLocked(path), "restore puts the user's own values back and removes the sidecar");
        Check(RageCoopClientSettings.Restore(path, out error) && error == string.Empty, "restore without a backup is a no-op");

        // No file at all: created for the session, removed again.
        string fresh = Path.Combine(Path.Combine(dir, "b"), "Data", "RageCoop.Client.Settings.xml");
        Check(RageCoopClientSettings.Apply(fresh, out error) && RageCoopClientSettings.IsLocked(fresh), "a missing file is created locked");
        Check(RageCoopClientSettings.Restore(fresh, out error) && !File.Exists(fresh) && !File.Exists(RageCoopClientSettings.BackupPath(fresh)), "and removed again when it did not exist before");

        // A file without the keys: the keys are removed again, the rest stays.
        string partial = Path.Combine(Path.Combine(dir, "c"), "RageCoop.Client.Settings.xml");
        Directory.CreateDirectory(Path.GetDirectoryName(partial));
        File.WriteAllText(partial, "<Settings><Voice>true</Voice></Settings>", new UTF8Encoding(false));
        Check(RageCoopClientSettings.Apply(partial, out error) && RageCoopClientSettings.IsLocked(partial), "keys are added to a file that lacks them");
        Check(RageCoopClientSettings.Restore(partial, out error), "restore ok");
        text = Read(partial);
        Check(text.Contains("<Voice>true</Voice>") && !text.Contains("MenuKey") && !text.Contains("Username") && File.Exists(partial), "added keys are removed again, the rest stays");

        // Broken input never throws.
        string broken = Path.Combine(Path.Combine(dir, "d"), "RageCoop.Client.Settings.xml");
        Directory.CreateDirectory(Path.GetDirectoryName(broken));
        File.WriteAllText(broken, "<<< not xml", new UTF8Encoding(false));
        Check(!RageCoopClientSettings.Apply(broken, out error) && error.Length > 0 && Read(broken) == "<<< not xml", "a broken file is left alone and reported");
        Check(!RageCoopClientSettings.Apply(null, out error) && !RageCoopClientSettings.Restore("", out error) && !RageCoopClientSettings.IsLocked(null), "no path -> false, no exception");
        Check(RageCoopClientSettings.LockedMenuKey == "F24" && RageCoopClientSettings.LockedUsername == "Skater" && RageCoopClientSettings.LockedMasterServer == string.Empty &&
            RageCoopClientSettings.LockedServerAddress == "127.0.0.1:4499", "locked values match the design");
    }
}
