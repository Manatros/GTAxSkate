// One sanitizer for everything our code draws or notifies (design 2026-11-17-streamer-safe-coop, item 6).
// Anything on screen ends up in a stream or video, so no address, code, password, path or user name may pass
// through. Pure, C# 5, no I/O. Applied inside Notify, overlay lines, banners and panels; it is a safety net
// behind the rule "never put a secret into a string we draw", not a replacement for it.
//
// Order matters: registered secrets, URLs, mail-like user@host, lobby codes (also line-broken), the user name,
// Windows paths (spaces allowed inside folder names), environment variables, IPv6 (before IPv4, so an IPv4 tail is
// not cut out of a mapped address), IPv4 (version numbers and ports of known words are spared), host:port, and bare
// host names that end in a known domain suffix. Round 2 widened the net and added the false-positive guards.
using System;
using System.Collections.Generic;
using System.Text;
using System.Text.RegularExpressions;

namespace GTAxSkate.Coop
{
    public static class SafeText
    {
        public const string Hidden = "[hidden]";
        public const string GroupHost = "host";        // the password and invite this PC is hosting with
        public const string GroupJoin = "join";        // the password and invite this PC is joining with
        public const string GroupSession = "session";  // in-game session join codes

        private const int MaxSecrets = 24;
        private const int MinSecretLength = 4;
        private static readonly object Gate = new object();
        private static readonly List<KeyValuePair<string, string>> Secrets = new List<KeyValuePair<string, string>>();
        // Tests replace the machine's real user name with a known one.
        private static string userNameOverride;
        internal static void SetUserNameForTests(string name) { userNameOverride = name; }

        // Words that are plain UI text as well as possible Windows user names: never masked as a name (a path still is).
        private static readonly string[] CommonNameWords =
            { "host", "user", "friend", "solo", "player", "admin", "administrator", "skater", "guest", "owner", "server", "client", "test", "default", "public", "mode", "copy", "join", "code", "online", "friends" };
        // word:digits where the word is a label, not a host name ("Error:10054", "Port:4499").
        private static readonly string[] NonHostWords =
            { "error", "err", "code", "port", "hresult", "status", "exit", "exitcode", "line", "result", "errno", "win32", "socket", "server", "host", "udp", "tcp", "id", "pid", "tick", "frame", "slot", "count", "size", "port" };
        // 4-part dotted numbers after these words are versions, not addresses.
        private static readonly Regex VersionContextRx = new Regex(
            @"(?:(?<![A-Za-z0-9])v|\bversion|\bver\.?|\bbuild|\brelease|\brev|\bRageCoop|\bGTAxSkate|\bSK8V|\bScriptHookV?|\bSHVDN|\bAPI)\s*:?\s*$",
            RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);

        private static readonly Regex UrlRx = new Regex(
            @"(?<![A-Za-z0-9])[A-Za-z][A-Za-z0-9+.\-]*://[^\s""'<>]+",
            RegexOptions.CultureInvariant);
        private static readonly Regex MailLikeRx = new Regex(
            @"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+(?::\d{1,5})?",
            RegexOptions.CultureInvariant);
        // SK8 + three groups of four; any run of dashes, underscores or whitespace (also line breaks) between them.
        private static readonly Regex LobbyCodeRx = new Regex(
            @"(?<![A-Za-z0-9])SK8[-_\s]*(?:[0-9A-Za-z]{4}[-_\s]*){2}[0-9A-Za-z]{4}(?![A-Za-z0-9])",
            RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
        // C:\a\b c\d.txt and \\host\share\x: a non-final folder may hold spaces (a user folder "John Smith"), the last
        // segment stops at whitespace so the sentence after a path stays readable.
        private static readonly Regex WindowsPathRx = new Regex(
            @"(?<![A-Za-z0-9])(?:[A-Za-z]:(?=[\\/])|\\\\[^\s\\/:*?""<>|]+)(?:[\\/][^\\/:*?""<>|\r\n]+(?=[\\/]))*(?:[\\/][^\s\\/:*?""<>|]*)?",
            RegexOptions.CultureInvariant);
        private static readonly Regex EnvVarRx = new Regex(@"%[A-Za-z_][A-Za-z0-9_]*%", RegexOptions.CultureInvariant);
        private static readonly Regex Ipv4Rx = new Regex(
            @"(?<![A-Za-z0-9.])(?:\d{1,3}\.){3}\d{1,3}(?::\d{1,5})?(?![A-Za-z0-9])",
            RegexOptions.CultureInvariant);
        // Groups of hex digits and colons, an optional dotted IPv4 tail, an optional zone id (%eth0, %12), optional
        // brackets and port.
        private static readonly Regex Ipv6Rx = new Regex(
            @"(?<![A-Za-z0-9:])\[?(?:[0-9A-Fa-f]{0,4}:){2,7}[0-9A-Fa-f]{0,4}(?:\.\d{1,3}){0,3}(?:%[A-Za-z0-9_.~\-]+)?\]?(?::\d{1,5})?(?![A-Za-z0-9:])",
            RegexOptions.CultureInvariant);
        private static readonly Regex HostPortRx = new Regex(
            @"(?<![A-Za-z0-9_.:@\-])([A-Za-z0-9_.\-]+):\d{4,5}(?![A-Za-z0-9])",
            RegexOptions.CultureInvariant);
        // Bare DNS names: labels ending in a well-known domain suffix, in ANY case (Game.Example.NET and gAmE.example.Com
        // leak as easily as game.example.com; Copilot rounds 2 and 3). Suffixes that are also file or namespace endings
        // (net, io, co, me, de, at, it, no ...) are masked too: a false positive such as "System.Net" costs one hidden
        // word on screen, a missed host name costs the stream. File names (RageCoop.Server.exe, Settings.xml, x.json,
        // y.log) never end in one of these.
        private const string Suffixes =
            @"home\.arpa|local|lan|home|internal|localdomain|example|invalid|box|fritz|intranet|corp|private|ddns|" +
            @"com|org|info|biz|tv|gg|xyz|app|dev|online|site|tech|cloud|uk|fr|nl|dk|fi|eu|us|ca|au|nz|jp|kr|cn|ru|br|" +
            @"net|io|co|me|de|at|it|no|es|be|se|ch|pl";
        private const string HostLabels = @"(?<![A-Za-z0-9_.@/\\\-])(?:[A-Za-z0-9](?:[A-Za-z0-9\-]{0,61}[A-Za-z0-9])?\.)+";
        // not followed by another label: "game.example.net" is one host name, not "game.example" plus ".net"
        private const string HostEnd = @"(?![A-Za-z0-9_\-]|\.[A-Za-z0-9])";
        private static readonly Regex HostNameRx = new Regex(
            HostLabels + "(?:" + Suffixes + ")" + HostEnd,
            RegexOptions.CultureInvariant | RegexOptions.IgnoreCase);

        public static void AddSecret(string group, string secret)
        {
            if (string.IsNullOrEmpty(secret) || secret.Trim().Length < MinSecretLength) return;
            string trimmed = secret.Trim();
            lock (Gate)
            {
                for (int i = 0; i < Secrets.Count; i++)
                    if (Secrets[i].Key == group && string.Equals(Secrets[i].Value, trimmed, StringComparison.Ordinal)) return;
                if (Secrets.Count >= MaxSecrets) Secrets.RemoveAt(0);
                Secrets.Add(new KeyValuePair<string, string>(group ?? string.Empty, trimmed));
            }
        }

        public static void ClearGroup(string group)
        {
            lock (Gate) { Secrets.RemoveAll(delegate(KeyValuePair<string, string> kv) { return kv.Key == (group ?? string.Empty); }); }
        }

        public static void ClearSecrets()
        {
            lock (Gate) { Secrets.Clear(); }
        }

        public static int SecretCount { get { lock (Gate) { return Secrets.Count; } } }

        public static string ForScreen(string text)
        {
            if (string.IsNullOrEmpty(text)) return string.Empty;
            string s = text;

            string[] secrets;
            lock (Gate)
            {
                secrets = new string[Secrets.Count];
                for (int i = 0; i < Secrets.Count; i++) secrets[i] = Secrets[i].Value;
            }
            Array.Sort(secrets, delegate(string a, string b) { return b.Length.CompareTo(a.Length); }); // longest (the invite) first
            for (int i = 0; i < secrets.Length; i++) s = ReplaceSecret(s, secrets[i]);

            s = UrlRx.Replace(s, Hidden);
            s = MailLikeRx.Replace(s, Hidden);
            s = LobbyCodeRx.Replace(s, Hidden);
            s = MaskUserName(s);            // before paths: a user folder with a space ("John Smith") is masked whole
            s = WindowsPathRx.Replace(s, Hidden);
            s = EnvVarRx.Replace(s, Hidden);
            s = Ipv6Rx.Replace(s, MaskIpv6);
            s = Ipv4Rx.Replace(s, delegate(Match m) { return MaskIpv4(s, m); });
            s = HostPortRx.Replace(s, delegate(Match m)
            {
                return Array.IndexOf(NonHostWords, m.Groups[1].Value.ToLowerInvariant()) >= 0 ? m.Value : Hidden;
            });
            s = HostNameRx.Replace(s, Hidden);
            return s;
        }

        // "12:30:45" style times are not addresses; an IPv6 literal has "::", all eight groups, or seven colons / six
        // plus a dotted IPv4 tail.
        private static string MaskIpv6(Match m)
        {
            string v = m.Value;
            int colons = 0;
            for (int i = 0; i < v.Length; i++) if (v[i] == ':') colons++;
            bool dotted = v.IndexOf('.') >= 0;
            bool literal = v.IndexOf("::", StringComparison.Ordinal) >= 0 || colons >= 7 || (dotted && colons >= 6);
            return literal ? Hidden : v;
        }

        private static string MaskIpv4(string whole, Match m)
        {
            string v = m.Value;
            int colon = v.IndexOf(':');
            string ip = colon >= 0 ? v.Substring(0, colon) : v;
            string[] parts = ip.Split('.');
            for (int i = 0; i < parts.Length; i++)
            {
                int octet;
                if (!int.TryParse(parts[i], out octet) || octet > 255) return v;   // not an address (1.5.1000.2)
            }
            if (colon < 0)
            {
                int start = Math.Max(0, m.Index - 24);
                string before = whole.Substring(start, m.Index - start);
                if (VersionContextRx.IsMatch(before)) return v;   // "v1.5.12.15", "version 1.5.12.15", "RageCoop 1.5.12.15"
            }
            return Hidden;
        }

        private static string ReplaceSecret(string s, string secret)
        {
            if (s.IndexOf(secret, StringComparison.OrdinalIgnoreCase) < 0) return s;
            // Short secrets (join codes) only match as whole words so ordinary words are not eaten.
            string pattern = secret.Length < 8
                ? @"(?<![A-Za-z0-9])" + Regex.Escape(secret) + @"(?![A-Za-z0-9])"
                : Regex.Escape(secret);
            return Regex.Replace(s, pattern, Hidden, RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
        }

        private static string MaskUserName(string s)
        {
            string name = userNameOverride;
            if (name == null)
            {
                try { name = Environment.UserName; } catch { name = null; }
            }
            if (string.IsNullOrEmpty(name) || name.Length < 3) return s;
            if (Array.IndexOf(CommonNameWords, name.Trim().ToLowerInvariant()) >= 0) return s;   // plain UI words stay readable
            return Regex.Replace(s, @"(?<![A-Za-z0-9])" + Regex.Escape(name) + @"(?![A-Za-z0-9])", Hidden,
                RegexOptions.IgnoreCase | RegexOptions.CultureInvariant);
        }
    }
}
