// Round 2 of the streamer-safe / internet-lobby tests (live test 2026-11-17 10:14 + Copilot/Gemini peer reviews):
// own UPnP IGD client against recorded XML/SOAP samples, sanitizer widening and false-positive guards, Discord invite
// formatting, local server key tracking, crash-proof invite expiry, trace unwrapping. Partial class of StreamerSafeTests.
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Threading;
using GTAxSkate.Coop;

public static partial class StreamerSafeTests
{
    private static void Round2Tests(string dir)
    {
        Round2InviteTests();
        Round2SafeTextTests();
        UpnpIgdTests();
        ServerKeyAndTraceTests();
        ExpiryStoreTests(dir);
    }

    // ---- Discord / chat formatting (peer review: block quotes, spoilers, emphasis, fenced blocks) -----------------
    private static void Round2InviteTests()
    {
        string code;
        Check(LobbyCode.TryEncode("8.8.4.4", 4499, out code), "code encodes");
        string line = "GTAxSkate coop invite " + code + " " + Password;
        Func<string, InviteStatus> parse = delegate(string t) { CoopInviteData x; return CoopInvite.Parse(t, out x); };
        string a = code.Substring(0, 8), b = code.Substring(8);
        Check(parse("> " + line) == InviteStatus.Ok, "a quoted line");
        Check(parse("> GTAxSkate coop invite " + a + "\n> " + b + " " + Password) == InviteStatus.Ok, "a multi-line Discord block quote (> on every line)");
        Check(parse(">>> GTAxSkate coop invite " + a + "\r\n" + b + " " + Password) == InviteStatus.Ok, "a >>> block quote");
        Check(parse("> > " + line) == InviteStatus.Ok, "nested quote markers");
        Check(parse("||" + line + "||") == InviteStatus.Ok, "spoiler bars");
        Check(parse("|| " + line + " ||") == InviteStatus.Ok, "spoiler bars with spaces");
        Check(parse("**" + line + "**") == InviteStatus.Ok, "bold");
        Check(parse("*" + line + "*") == InviteStatus.Ok, "italic (asterisk)");
        Check(parse("_" + line + "_") == InviteStatus.Ok, "italic (underscore)");
        Check(parse("__" + line + "__") == InviteStatus.Ok, "underline");
        Check(parse("~~" + line + "~~") == InviteStatus.Ok, "strike-through");
        Check(parse("```\n" + line + "\n```") == InviteStatus.Ok, "a fenced block");
        Check(parse("```txt\n" + line + "\n```") == InviteStatus.Ok, "a fenced block with a language tag");
        Check(parse("```text\r\n" + line + "\r\n```") == InviteStatus.Ok, "a fenced block with a language tag and CRLF");
        Check(parse("```" + line + "```") == InviteStatus.Ok, "a one-line fence (no tag to strip)");
        Check(parse("> ```txt\n> " + line + "\n> ```") == InviteStatus.Ok, "a quoted fenced block");
        Check(parse("||**" + line + "**||") == InviteStatus.Ok, "spoiler around bold");
        Check(parse("> hello there") == InviteStatus.WrongFormat && parse("||hello||") == InviteStatus.WrongFormat, "other formatted text is still rejected");
        string lan = "GTAxSkate coop invite lan:192.168.1.5:4499 " + Password;
        Check(parse("> " + lan) == InviteStatus.Ok && parse("||" + lan + "||") == InviteStatus.Ok, "LAN invites survive the same formatting");
        CoopInviteData d;
        Check(CoopInvite.Parse("> GTAxSkate coop invite " + a + "\n> " + b + " " + Password, out d) == InviteStatus.Ok && d.Password == Password && d.Host == "8.8.4.4" && d.Port == 4499, "the parsed data is exact");
    }

    // ---- SafeText: wider net and fewer false positives --------------------------------------------------------------
    private static void Round2SafeTextTests()
    {
        SafeText.ClearSecrets();
        SafeText.SetUserNameForTests("RoboStreamer");
        string H = SafeText.Hidden;
        Func<string, string> f = SafeText.ForScreen;

        // bare host names, but never file names or namespaces
        Check(f("host skate-host.example refused") == "host " + H + " refused", "a bare host name: " + f("host skate-host.example refused"));
        Check(f("see www.example.com.") == "see " + H + ".", "a public host name");
        Check(f("my-pc.LOCAL and fritz.box and nas.home.arpa") == H + " and " + H + " and " + H, "home-network suffixes in any case");
        foreach (string plain in new[] { "RageCoop.Server.exe started", "GTAxSkate.Coop.Client.dll loaded", "Settings.xml", "coop-client.log", "FriendsCoopPrefs.json", "System.IO.IOException", "RageCoop.Client.Main", "SkateVLegacy.asi", "x.ini and y.txt", "Press Ctrl.S" })
            Check(f(plain) == plain, "file names and namespaces stay readable: " + plain + " -> " + f(plain));

        // line-broken lobby codes
        Check(f("SK8-ABCD\r\nEFGH-JKMN") == H && f("SK8-ABCD-\nEFGH\n-JKMN") == H && f("sk8\nabcd\tefgh jkmn") == H, "a lobby code broken across lines");

        // Windows paths with spaces in the user folder
        SafeText.SetUserNameForTests("John Smith");
        Check(f(@"C:\Users\John Smith\Documents\file.txt") == H, "a path with the user name containing a space: " + f(@"C:\Users\John Smith\Documents\file.txt"));
        Check(f(@"\\server\share\John Smith\secret.txt") == H, "a UNC path with a spaced user folder");
        Check(f("hello John Smith, welcome") == "hello " + H + ", welcome", "the spaced user name alone");
        SafeText.SetUserNameForTests("RoboStreamer");
        Check(f(@"open C:\Users\Jane Doe\My Games\x.txt now") == "open " + H + " now", "any spaced folder names inside a path: " + f(@"open C:\Users\Jane Doe\My Games\x.txt now"));
        Check(f(@"files in C:\Games\GTA V failed") == "files in " + H + " V failed" || f(@"files in C:\Games\GTA V failed").StartsWith("files in " + H), "a sentence after a path is not swallowed whole");
        Check(f("Slot a: 5 and x: y") == "Slot a: 5 and x: y", "a lone letter and a colon is not a drive");

        // IPv6 forms
        Check(f("0:0:0:0:0:ffff:192.0.2.1 x") == H + " x", "uncompressed IPv4-mapped IPv6: " + f("0:0:0:0:0:ffff:192.0.2.1 x"));
        Check(f("2001:db8:3:4:5:6:192.0.2.1 x") == H + " x", "an IPv4-embedded IPv6 prefix: " + f("2001:db8:3:4:5:6:192.0.2.1 x"));
        Check(f("::ffff:192.0.2.1") == H, "compressed IPv4-mapped IPv6");
        Check(f("[fe80::1%12]:4499") == H && f("fe80::1%eth0 up") == H + " up", "IPv6 zone ids and bracketed ports: " + f("[fe80::1%12]:4499") + " / " + f("fe80::1%eth0 up"));
        Check(f("at 12:30:45 pm") == "at 12:30:45 pm", "times stay");

        // mail-like and URLs with credentials
        Check(f("user@my-pc.local:4499 down") == H + " down", "user@host:port: " + f("user@my-pc.local:4499 down"));
        Check(f("https://user:pass@myhost.com:8080/x failed") == H + " failed", "a URL with credentials: " + f("https://user:pass@myhost.com:8080/x failed"));
        Check(f("ftp://10.1.2.3/pub ok") == H + " ok", "any URL");

        // false positives
        Check(f("RageCoop 1.5.12.15 loaded") == "RageCoop 1.5.12.15 loaded" && f("v1.5.12.15") == "v1.5.12.15" && f("version 1.5.12.15") == "version 1.5.12.15" && f("Version: 1.5.12.15") == "Version: 1.5.12.15", "versions stay: " + f("RageCoop 1.5.12.15 loaded"));
        Check(f("addr 1.5.12.15") == "addr " + H, "a 4-part number in an address context is still masked");
        Check(f("1.5.1000.2 stays") == "1.5.1000.2 stays", "an impossible octet is not an address");
        Check(f("Error:10054 and Port:4499 and HRESULT:12345") == "Error:10054 and Port:4499 and HRESULT:12345", "word:number diagnostics stay: " + f("Error:10054 and Port:4499 and HRESULT:12345"));
        Check(f("mypc:4499") == H, "an unknown word:port is a host");
        foreach (string common in new[] { "Host", "User", "Friend", "Solo", "Player", "Admin", "Skater", "Guest" })
        {
            SafeText.SetUserNameForTests(common);
            string ui = "[CONNECTED - PRIVATE COOP] Host: Friend [LAN] | Solo Skater Admin Guest Player User";
            Check(f(ui) == ui, "a common-word Windows user name (" + common + ") does not corrupt UI text");
            Check(f(@"C:\Users\" + common + @"\x.txt") == H, "but its path is still masked (" + common + ")");
        }
        SafeText.SetUserNameForTests(null);
    }

    // ---- UPnP IGD client against recorded XML/SOAP -------------------------------------------------------------------
    private const string Gateway = "192.168.178.1", Repeater = "192.168.178.2";
    private static string DescUrl(string host) { return "http://" + host + ":49000/igddesc.xml"; }
    private static string Ssdp(string host)
    {
        return "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nEXT:\r\nLOCATION: " + DescUrl(host) + "\r\nSERVER: FRITZ!Box 6670 Cable UPnP/1.0 AVM FRITZ!Box 6670 Cable 154.07.57\r\n" +
            "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\nUSN: uuid:75802409-bccb-40e7-8e6c-fa095ecce13e::urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n";
    }
    private const string IgdDescription =
        "<?xml version=\"1.0\"?>\r\n<root xmlns=\"urn:schemas-upnp-org:device-1-0\"><specVersion><major>1</major><minor>0</minor></specVersion>" +
        "<device><deviceType>urn:schemas-upnp-org:device:InternetGatewayDevice:1</deviceType><friendlyName>FRITZ!Box 6670 Cable</friendlyName>" +
        "<serviceList><service><serviceType>urn:schemas-any-com:service:Any:1</serviceType><controlURL>/igdupnp/control/any</controlURL></service></serviceList>" +
        "<deviceList><device><deviceType>urn:schemas-upnp-org:device:WANDevice:1</deviceType><deviceList><device>" +
        "<deviceType>urn:schemas-upnp-org:device:WANConnectionDevice:1</deviceType><serviceList>" +
        "<service><serviceType>urn:schemas-upnp-org:service:WANIPConnection:1</serviceType><serviceId>urn:upnp-org:serviceId:WANIPConn1</serviceId>" +
        "<controlURL>/igdupnp/control/WANIPConn1</controlURL><eventSubURL>/igdupnp/control/WANIPConn1</eventSubURL><SCPDURL>/igdconnSCPD.xml</SCPDURL></service>" +
        "</serviceList></device></deviceList></device></deviceList></device></root>";
    private const string SoapNs = "xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"";
    private static string SoapOk(string action, string inner)
    {
        return "<?xml version=\"1.0\"?><s:Envelope " + SoapNs + "><s:Body><u:" + action + "Response xmlns:u=\"urn:schemas-upnp-org:service:WANIPConnection:1\">" + inner + "</u:" + action + "Response></s:Body></s:Envelope>";
    }
    private static string SoapFault(int code, string text)
    {
        return "<?xml version=\"1.0\"?><s:Envelope " + SoapNs + "><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring><detail>" +
            "<UPnPError xmlns=\"urn:schemas-upnp-org:control-1-0\"><errorCode>" + code + "</errorCode><errorDescription>" + text + "</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>";
    }

    private sealed class FakeIgd : IUpnpTransport
    {
        public string GatewayHost = Gateway;
        public readonly List<string> SsdpResponses = new List<string>();        // answered by the router named in their LOCATION
        public readonly List<SsdpReply> SsdpFirst = new List<SsdpReply>();      // explicit senders, delivered before the others
        public readonly Dictionary<string, string> Pages = new Dictionary<string, string>();
        public readonly List<string> Gets = new List<string>();
        public readonly List<string> Posts = new List<string>();      // "Action|url|body"
        public readonly Dictionary<string, KeyValuePair<int, string>> Replies = new Dictionary<string, KeyValuePair<int, string>>();   // by action
        public readonly List<string[]> Mappings = new List<string[]>();   // port, protocol, internal client, description
        public string ExternalIp = "8.8.4.4";
        public int SearchCalls;

        public static FakeIgd Standard()
        {
            var t = new FakeIgd();
            t.SsdpResponses.Add(Ssdp(Gateway));
            t.Pages[DescUrl(Gateway)] = IgdDescription;
            return t;
        }

        public string DefaultGatewayIPv4() { return GatewayHost; }
        public List<SsdpReply> Search(string[] targets, int timeoutMs)
        {
            SearchCalls++;
            var l = new List<SsdpReply>(SsdpFirst);
            foreach (string r in SsdpResponses)
            {
                Uri u;
                string loc = UpnpIgdGateway.LocationOf(r);
                l.Add(new SsdpReply(loc != null && Uri.TryCreate(loc, UriKind.Absolute, out u) ? u.Host : "0.0.0.0", r));
            }
            return l;
        }
        public string HttpGet(string url, int timeoutMs) { Gets.Add(url); string v; return Pages.TryGetValue(url, out v) ? v : null; }

        private static string Arg(string body, string name)
        {
            int i = body.IndexOf("<" + name + ">", StringComparison.Ordinal);
            if (i < 0) return string.Empty;
            i += name.Length + 2;
            int j = body.IndexOf("</", i, StringComparison.Ordinal);
            return body.Substring(i, j - i);
        }

        public int HttpPost(string url, string soapAction, string body, int timeoutMs, out string responseBody)
        {
            string action = soapAction.Substring(soapAction.IndexOf('#') + 1).Trim('"');
            Posts.Add(action + "|" + url + "|" + body);
            KeyValuePair<int, string> canned;
            if (Replies.TryGetValue(action, out canned)) { responseBody = canned.Value; return canned.Key; }
            switch (action)
            {
                case "GetGenericPortMappingEntry":
                {
                    int i = int.Parse(Arg(body, "NewPortMappingIndex"));
                    if (i >= Mappings.Count) { responseBody = SoapFault(713, "SpecifiedArrayIndexInvalid"); return 500; }
                    string[] m = Mappings[i];
                    responseBody = SoapOk(action, "<NewRemoteHost></NewRemoteHost><NewExternalPort>" + m[0] + "</NewExternalPort><NewProtocol>" + m[1] + "</NewProtocol><NewInternalPort>" + m[0] +
                        "</NewInternalPort><NewInternalClient>" + m[2] + "</NewInternalClient><NewEnabled>1</NewEnabled><NewPortMappingDescription>" + m[3] + "</NewPortMappingDescription><NewLeaseDuration>0</NewLeaseDuration>");
                    return 200;
                }
                case "GetSpecificPortMappingEntry":
                {
                    string port = Arg(body, "NewExternalPort"), proto = Arg(body, "NewProtocol");
                    string[] m = Mappings.Find(delegate(string[] x) { return x[0] == port && x[1] == proto; });
                    if (m == null) { responseBody = SoapFault(714, "NoSuchEntryInArray"); return 500; }
                    responseBody = SoapOk(action, "<NewInternalPort>" + m[0] + "</NewInternalPort><NewInternalClient>" + m[2] + "</NewInternalClient><NewEnabled>1</NewEnabled><NewPortMappingDescription>" + m[3] + "</NewPortMappingDescription><NewLeaseDuration>0</NewLeaseDuration>");
                    return 200;
                }
                case "AddPortMapping":
                    Mappings.Add(new[] { Arg(body, "NewExternalPort"), Arg(body, "NewProtocol"), Arg(body, "NewInternalClient"), Arg(body, "NewPortMappingDescription") });
                    responseBody = SoapOk(action, string.Empty); return 200;
                case "DeletePortMapping":
                {
                    string port = Arg(body, "NewExternalPort"), proto = Arg(body, "NewProtocol");
                    int removed = Mappings.RemoveAll(delegate(string[] m) { return m[0] == port && m[1] == proto; });
                    if (removed == 0) { responseBody = SoapFault(714, "NoSuchEntryInArray"); return 500; }
                    responseBody = SoapOk(action, string.Empty); return 200;
                }
                case "GetExternalIPAddress":
                    responseBody = SoapOk(action, "<NewExternalIPAddress>" + ExternalIp + "</NewExternalIPAddress>"); return 200;
            }
            responseBody = SoapFault(401, "Invalid Action"); return 500;
        }
        public int Count(string action) { int n = 0; foreach (string p in Posts) if (p.StartsWith(action + "|", StringComparison.Ordinal)) n++; return n; }
    }

    private static OnlineHostStatus WaitUpnp(UpnpHostPortService s)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < 5000)
        {
            OnlineHostStatus st = s.Poll();
            if (st.State != OnlineHostState.Opening) return st;
            Thread.Sleep(5);
        }
        throw new Exception("FAIL: UPnP service did not finish");
    }

    private static OnlineHostStatus RunUpnp(FakeIgd t, string desc)
    {
        var svc = new UpnpHostPortService(delegate() { return new UpnpIgdGateway(t); }, 5000);
        svc.Begin(4499, desc, "192.168.178.23");
        return WaitUpnp(svc);
    }

    private static void UpnpIgdTests()
    {
        const string Desc = "GTAxSkate Coop abcd1234";
        string note;

        // success: discovery by the default gateway's LOCATION, description, SOAP actions with the right arguments
        FakeIgd t = FakeIgd.Standard();
        OnlineHostStatus st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open && st.ExternalIp == "8.8.4.4", "a router with a public IPv4 opens the port: " + st.State + " " + st.TraceNote);
        Check(t.Gets.Count == 1 && t.Gets[0] == DescUrl(Gateway), "the description was fetched from the gateway");
        string add = t.Posts.Find(delegate(string p) { return p.StartsWith("AddPortMapping|", StringComparison.Ordinal); });
        Check(add != null && add.Contains("http://192.168.178.1:49000/igdupnp/control/WANIPConn1") && add.Contains("<NewExternalPort>4499</NewExternalPort>") && add.Contains("<NewProtocol>UDP</NewProtocol>") &&
            add.Contains("<NewInternalPort>4499</NewInternalPort>") && add.Contains("<NewInternalClient>192.168.178.23</NewInternalClient>") && add.Contains("<NewEnabled>1</NewEnabled>") &&
            add.Contains("<NewPortMappingDescription>" + Desc + "</NewPortMappingDescription>") && add.Contains("<NewLeaseDuration>0</NewLeaseDuration>"), "AddPortMapping carries port, protocol, client, description");
        Check(t.Count("GetExternalIPAddress") == 1 && t.Mappings.Count == 1, "the external address was asked once, the mapping exists");
        Check(st.Message.IndexOf("8.8.4.4", StringComparison.Ordinal) < 0 && st.TraceNote.IndexOf("8.8.4.4", StringComparison.Ordinal) < 0 && st.TraceNote.IndexOf("192.168", StringComparison.Ordinal) < 0, "no address in the message or the trace");

        // our own removal: only the mapping carrying our description
        t.Mappings.Insert(0, new[] { "4499", "TCP", "192.168.178.50", "Some other app" });
        t.Mappings.Add(new[] { "4500", "UDP", "192.168.178.60", "Foreign" });
        int removed = UpnpHostPortService.RemoveOwn(new UpnpIgdGateway(t), 4499, Desc);
        Check(removed == 1 && t.Mappings.Count == 2 && t.Mappings.TrueForAll(delegate(string[] m) { return m[3] != Desc; }), "RemoveOwn deletes only the UDP mapping with our description");

        // several FRITZ!Boxes answer: the repeater comes first, the gateway wins
        t = FakeIgd.Standard();
        t.SsdpResponses.Clear(); t.SsdpResponses.Add(Ssdp(Repeater)); t.SsdpResponses.Add(Ssdp(Gateway));
        t.Pages[DescUrl(Repeater)] = IgdDescription.Replace("WANIPConn1", "REPEATER");
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open && !t.Gets.Contains(DescUrl(Repeater)) && t.Posts[0].Contains(Gateway), "only the answer from the default gateway is used");
        t = FakeIgd.Standard();
        t.SsdpResponses.Clear(); t.SsdpResponses.Add(Ssdp(Repeater));
        t.Pages[DescUrl(Repeater)] = IgdDescription;
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && t.Gets.Count == 0 && t.Posts.Count == 0, "a router that is not the default gateway is never used");

        // fault 606: not authorized
        t = FakeIgd.Standard();
        t.Replies["AddPortMapping"] = new KeyValuePair<int, string>(500, SoapFault(606, "Action not authorized"));
        t.Replies["GetGenericPortMappingEntry"] = new KeyValuePair<int, string>(500, SoapFault(713, "SpecifiedArrayIndexInvalid"));
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message == "Your FRITZ!Box/router does not allow this PC to open ports. Allow 'independent port sharing' for this PC in the router, or use Tailscale.", "fault 606 -> the FRITZ!Box message: " + st.Message);
        t = FakeIgd.Standard();
        t.Replies["GetGenericPortMappingEntry"] = new KeyValuePair<int, string>(500, SoapFault(606, "Action not authorized"));
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message == UpnpIgdGateway.FritzNotAuthorized, "fault 606 while listing -> the same message");
        t = FakeIgd.Standard();
        t.Replies["AddPortMapping"] = new KeyValuePair<int, string>(401, "Unauthorized");
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message == UpnpIgdGateway.FritzNotAuthorized, "HTTP 401 on a SOAP call is the same refusal");

        // DS-Lite: the router answers, but has no public IPv4
        foreach (string empty in new[] { string.Empty, "0.0.0.0" })
        {
            t = FakeIgd.Standard();
            t.ExternalIp = empty;
            st = RunUpnp(t, Desc);
            Check(st.State == OnlineHostState.Failed && st.Message == "Your internet line has no public IPv4 address (DS-Lite / carrier NAT), so friends cannot reach you over IPv4. Use Tailscale, or let a friend with a public IPv4 host.",
                "an empty/unspecified external IPv4 -> the DS-Lite message: " + st.Message);
            Check(t.Mappings.Count == 0 && t.Count("DeletePortMapping") == 1, "the mapping created for the attempt is removed again");
        }
        t = FakeIgd.Standard();
        t.ExternalIp = "100.72.1.9";
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message.Contains("carrier-grade NAT") && t.Mappings.Count == 0, "a private/CGNAT external address keeps the CGNAT message");

        // no IGD at all
        t = new FakeIgd();
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message.Contains("Your router did not open the port") && t.Posts.Count == 0, "no UPnP device answered");
        t = FakeIgd.Standard();
        t.Pages[DescUrl(Gateway)] = "<root><device/></root>";
        Check(!new UpnpIgdGateway(t).IsAvailable(), "a description without a WAN connection service is no device");

        // foreign mapping on our port
        t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4499", "UDP", "192.168.178.77", "Console" });
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.Message.Contains("already forwarded") && t.Count("AddPortMapping") == 0 && t.Mappings.Count == 1, "a foreign mapping on the port is left alone and reported");

        // stale own mapping is replaced
        t = FakeIgd.Standard();
        t.Mappings.Add(new[] { "4499", "UDP", "192.168.178.99", Desc });
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open && t.Mappings.Count == 1 && t.Mappings[0][2] == "192.168.178.23", "a stale own mapping is replaced");

        // hostile descriptions
        t = FakeIgd.Standard();
        t.Pages[DescUrl(Gateway)] = IgdDescription.Replace("/igdupnp/control/WANIPConn1", "http://203.0.113.5/steal");
        Check(!new UpnpIgdGateway(t).IsAvailable(), "a control URL on another host is rejected");
        t = FakeIgd.Standard();
        t.Pages[DescUrl(Gateway)] = "<?xml version=\"1.0\"?><!DOCTYPE root [<!ENTITY x SYSTEM \"file:///C:/Windows/win.ini\">]>" + IgdDescription.Substring(IgdDescription.IndexOf("<root", StringComparison.Ordinal)).Replace("FRITZ!Box 6670 Cable", "&x;");
        Check(!new UpnpIgdGateway(t).IsAvailable(), "a DTD in the description is refused (no entity expansion)");
        t = FakeIgd.Standard();
        t.SsdpResponses.Clear(); t.SsdpResponses.Add("HTTP/1.1 200 OK\r\nLOCATION: ftp://192.168.178.1/x\r\n\r\n");
        Check(!new UpnpIgdGateway(t).IsAvailable(), "only plain http LOCATION headers count");

        // WANPPPConnection is used when there is no WANIPConnection
        t = FakeIgd.Standard();
        t.Pages[DescUrl(Gateway)] = IgdDescription.Replace("WANIPConnection:1", "WANPPPConnection:1");
        string url, type;
        Check(UpnpIgdGateway.FindWanService(t.Pages[DescUrl(Gateway)], DescUrl(Gateway), out url, out type) && type == "urn:schemas-upnp-org:service:WANPPPConnection:1" && url == "http://192.168.178.1:49000/igdupnp/control/WANIPConn1", "WANPPPConnection is accepted");
        Check(UpnpIgdGateway.FindWanService(IgdDescription.Replace("<controlURL>/igdupnp", "<URLBase>http://192.168.178.1:5000/</URLBase><controlURL>/igdupnp").Replace("<specVersion>", "<URLBase>http://192.168.178.1:5000/</URLBase><specVersion>"),
            DescUrl(Gateway), out url, out type) && url.StartsWith("http://192.168.178.1:5000/"), "URLBase is honoured");

        // listing is bounded and tolerant: a router without enumeration still gets the add
        t = FakeIgd.Standard();
        t.Replies["GetGenericPortMappingEntry"] = new KeyValuePair<int, string>(500, SoapFault(501, "Action Failed"));
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Open, "a router that cannot enumerate mappings still opens the port: " + st.TraceNote);

        // an unanswered router is a plain failure, not a hang
        t = FakeIgd.Standard();
        t.Replies["GetGenericPortMappingEntry"] = new KeyValuePair<int, string>(0, null);
        st = RunUpnp(t, Desc);
        Check(st.State == OnlineHostState.Failed && st.TraceNote.IndexOf("192.168", StringComparison.Ordinal) < 0, "no answer from the router -> failed");
        Check(UpnpIgdGateway.HostMatches(DescUrl(Gateway), Gateway) && !UpnpIgdGateway.HostMatches(DescUrl(Repeater), Gateway) && !UpnpIgdGateway.HostMatches(DescUrl(Repeater), null) && !UpnpIgdGateway.HostMatches(DescUrl(Gateway), null), "gateway host matching (an unknown gateway matches nothing)");
        note = UpnpIgdGateway.LocationOf(Ssdp(Gateway));
        Check(note == DescUrl(Gateway) && UpnpIgdGateway.LocationOf("HTTP/1.1 200 OK\r\n\r\n") == null && UpnpIgdGateway.LocationOf(null) == null, "LOCATION header parsing");
    }

    // ---- local server key, trace unwrapping ---------------------------------------------------------------------------
    private static void ServerKeyAndTraceTests()
    {
        string a = CoopInvite.GeneratePassword(), b = CoopInvite.GeneratePassword();
        Check(LocalServerKey.Fingerprint(a).Length == 64 && LocalServerKey.Fingerprint(a) != LocalServerKey.Fingerprint(b) && LocalServerKey.Fingerprint(a).IndexOf(a, StringComparison.OrdinalIgnoreCase) < 0, "a fingerprint is a SHA-256 hex, never the key");
        Check(LocalServerKey.CanReuse(LocalServerKey.Fingerprint(a), a), "a server started with this key is reused");
        Check(!LocalServerKey.CanReuse(LocalServerKey.Fingerprint(a), b), "a new hosting password replaces the running server");
        Check(!LocalServerKey.CanReuse(null, a) && !LocalServerKey.CanReuse(string.Empty, a), "an unknown fingerprint counts as different (restart is always safe)");
        Check(LocalServerKey.CanReuse(LocalServerKey.Fingerprint(null), null) == true, "no key on both sides matches");

        var inner = new InvalidOperationException("secret 8.8.4.4 C:\\Users\\Someone\\x");
        string d = CoopTrace.Describe(new TargetInvocationException(inner));
        Check(d.StartsWith("InvalidOperationException 0x") && d.IndexOf("8.8.4.4", StringComparison.Ordinal) < 0 && d.IndexOf("TargetInvocation", StringComparison.Ordinal) < 0 && d.IndexOf("Someone", StringComparison.Ordinal) < 0,
            "a TargetInvocationException is unwrapped to the inner type + HRESULT, never the message: " + d);
        Check(CoopTrace.Describe(new AggregateException(inner)).StartsWith("InvalidOperationException 0x"), "an AggregateException is unwrapped too");
        Check(CoopTrace.Describe(new TargetInvocationException(new TargetInvocationException(new System.IO.IOException("x")))).StartsWith("IOException 0x") && CoopTrace.Describe(null) == "none", "nested wrappers and null");
    }

    // ---- crash-proof invite expiry ----------------------------------------------------------------------------------
    private static void ExpiryStoreTests(string dir)
    {
        string scratch = Path.Combine(dir, "expiry");
        Directory.CreateDirectory(scratch);
        InviteExpiryStore.DirectoryOverride = scratch;
        try
        {
            string sha; DateTime when;
            Check(!InviteExpiryStore.TryRead(out sha, out when), "nothing stored yet");
            string invite = "GTAxSkate coop invite lan:192.168.1.5:4499 " + Password;
            DateTime expires = DateTime.UtcNow.AddMinutes(10);
            InviteExpiryStore.Write(invite, expires);
            string path = Path.Combine(scratch, InviteExpiryStore.FileName);
            string raw = File.ReadAllText(path);
            Check(raw.IndexOf(Password, StringComparison.Ordinal) < 0 && raw.IndexOf("192.168", StringComparison.Ordinal) < 0 && raw.IndexOf("coop invite", StringComparison.Ordinal) < 0, "the file holds no invite text, password or address");
            Check(InviteExpiryStore.TryRead(out sha, out when) && sha == InviteExpiryStore.Hash(invite) && Math.Abs((when - expires).TotalSeconds) < 2 && when.Kind == DateTimeKind.Utc, "hash and expiry time round-trip");
            Check(InviteExpiryStore.Hash(invite) != InviteExpiryStore.Hash(invite + " ") && InviteExpiryStore.Hash(invite).Length == 64, "the hash is exact (a changed text never matches)");
            InviteExpiryStore.Write(invite + "2", expires);
            InviteExpiryStore.TryRead(out sha, out when);
            Check(sha == InviteExpiryStore.Hash(invite + "2"), "a later invite replaces the record");
            InviteExpiryStore.Delete();
            Check(!File.Exists(path) && !InviteExpiryStore.TryRead(out sha, out when), "deleted");
            File.WriteAllText(path, "not json at all");
            Check(!InviteExpiryStore.TryRead(out sha, out when) && !File.Exists(path), "a corrupt record is dropped");
            InviteExpiryStore.Write(string.Empty, expires);
            Check(!File.Exists(path), "an empty invite is never recorded");
        }
        finally { InviteExpiryStore.DirectoryOverride = null; }
    }
}
