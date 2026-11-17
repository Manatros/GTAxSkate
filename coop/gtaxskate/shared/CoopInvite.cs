// Invite text for streamer-safe coop (design 2026-11-17-streamer-safe-coop). Pure, C# 5, no I/O.
//
// An invite travels ONLY through the clipboard: nothing secret is ever typed into GTA's on-screen keyboard
// or drawn by our code. Format (one line, whitespace tolerant when pasted from a chat):
//
//     GTAxSkate coop invite <locator> <password>
//
//   locator  SK8-XXXX-XXXX-XXXX            internet lobby code (LobbyCode)
//            lan:<ipv4>:<port>             private LAN host
//            vpn:<ipv4>:<port>             VPN host (100.64.0.0/10, for example Tailscale)
//   password 12..64 letters/digits; generated passwords are 20 Crockford base32 symbols (about 100 bits).
using System;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

namespace GTAxSkate.Coop
{
    public enum InviteLocatorKind
    {
        Online = 0,
        Lan = 1,
        Vpn = 2
    }

    public enum InviteStatus
    {
        Ok = 0,
        Empty = 1,
        WrongFormat = 2,
        BadLocator = 3,
        BadPassword = 4
    }

    public sealed class CoopInviteData
    {
        public readonly InviteLocatorKind Kind;
        public readonly string Host;     // IPv4 text (decoded from the lobby code for Online)
        public readonly int Port;
        public readonly string Password;

        public CoopInviteData(InviteLocatorKind kind, string host, int port, string password)
        {
            Kind = kind; Host = host ?? string.Empty; Port = port; Password = password ?? string.Empty;
        }
    }

    public static class CoopInvite
    {
        public const string Prefix = "GTAxSkate coop invite";
        public const int PasswordLength = 20;
        public const int MinPasswordLength = CoopSecurityPolicy.MinFriendKeyLength;
        public const int MaxPasswordLength = 64;
        public const int MaxInviteChars = 512;
        private const string Alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"; // Crockford base32, 32 symbols = exactly 5 bits per byte & 31

        // Fresh random password per hosting: 20 symbols x 5 bits = 100 bits from the OS CSPRNG, no modulo bias.
        public static string GeneratePassword()
        {
            var raw = new byte[PasswordLength];
            // RandomNumberGenerator.Create() is the OS CSPRNG (RNGCryptoServiceProvider on .NET Framework, which the
            // game runs; the type itself is obsolete for the net6 server build that compiles this file too).
            using (RandomNumberGenerator rng = RandomNumberGenerator.Create())
                rng.GetBytes(raw);
            var sb = new StringBuilder(PasswordLength);
            for (int i = 0; i < raw.Length; i++) sb.Append(Alphabet[raw[i] & 31]);
            return sb.ToString();
        }

        public static bool IsValidPassword(string password)
        {
            if (password == null || password.Length < MinPasswordLength || password.Length > MaxPasswordLength) return false;
            for (int i = 0; i < password.Length; i++)
            {
                char c = password[i];
                bool ok = (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
                if (!ok) return false;
            }
            return true;
        }

        // Online invite from a lobby code. Null when the code or password is not usable.
        public static string BuildOnline(string lobbyCode, string password)
        {
            string ip; int port;
            if (!IsValidPassword(password) || LobbyCode.Decode(lobbyCode, out ip, out port) != LobbyCodeStatus.Ok) return null;
            string canonical;
            if (!LobbyCode.TryEncode(ip, port, out canonical)) return null;
            return Prefix + " " + canonical + " " + password;
        }

        // LAN / VPN invite. Null when the address does not belong to the stated kind or the port/password is bad.
        public static string BuildLocal(InviteLocatorKind kind, string ipv4, int port, string password)
        {
            if (kind == InviteLocatorKind.Online || !IsValidPassword(password) || !ValidLocalEndpoint(kind, ipv4, port)) return null;
            return Prefix + " " + (kind == InviteLocatorKind.Lan ? "lan:" : "vpn:") + ipv4.Trim() + ":" + port + " " + password;
        }

        private static bool ValidLocalEndpoint(InviteLocatorKind kind, string ipv4, int port)
        {
            byte[] bytes;
            if (!LobbyCode.TryParseIPv4(ipv4, out bytes)) return false;
            if (port < LobbyCode.MinPort || port > 65535) return false;
            EndpointTrustClass trust = CoopSecurityPolicy.ClassifyEndpoint(ipv4.Trim());
            return kind == InviteLocatorKind.Lan ? trust == EndpointTrustClass.PrivateLan
                : trust == EndpointTrustClass.PrivateVpnOverlay;
        }

        public static InviteStatus Parse(string text, out CoopInviteData data)
        {
            data = null;
            if (string.IsNullOrWhiteSpace(text)) return InviteStatus.Empty;
            if (text.Length > MaxInviteChars) return InviteStatus.WrongFormat;
            string trimmed = StripChatFormatting(text).Trim(' ', '\t', '\r', '\n', '`', '"', '\'', '<', '>');
            var tokens = new List<string>(trimmed.Split(new[] { ' ', '\t', '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries));
            // Italic underscores (_text_) stick to token ends; no locator or password symbol is an underscore at an end.
            for (int i = 0; i < tokens.Count; i++) tokens[i] = tokens[i].Trim('_');
            tokens.RemoveAll(delegate(string t) { return t.Length == 0; });
            if (tokens.Count < 5 ||
                !string.Equals(tokens[0], "GTAxSkate", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(tokens[1], "coop", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(tokens[2], "invite", StringComparison.OrdinalIgnoreCase))
                return InviteStatus.WrongFormat;

            string password = tokens[tokens.Count - 1];
            var locatorBuilder = new StringBuilder();
            for (int i = 3; i < tokens.Count - 1; i++) locatorBuilder.Append(tokens[i]); // a lobby code split by a line break
            string locator = locatorBuilder.ToString();

            InviteLocatorKind kind;
            string host; int port;
            if (!ParseLocator(locator, out kind, out host, out port)) return InviteStatus.BadLocator;
            if (!IsValidPassword(password)) return InviteStatus.BadPassword;
            data = new CoopInviteData(kind, host, port, password);
            return InviteStatus.Ok;
        }

        private static readonly Regex FenceWithLanguage = new Regex(@"```[A-Za-z0-9_+.#-]*[ \t]*(?=\n)", RegexOptions.CultureInvariant);

        // Chats wrap pasted text: block quotes ("> " on every line), spoilers (||x||), bold/italic/underline/strike
        // (**, *, __, ~~) and fenced code blocks with or without a language tag. None of these characters is part of a
        // locator or a password, so they become spaces; ">" only at the start of a line.
        internal static string StripChatFormatting(string text)
        {
            string t = text.Replace("\r\n", "\n").Replace('\r', '\n');
            t = FenceWithLanguage.Replace(t, " ");   // "```txt" on its own line: the tag goes with the fence
            t = t.Replace("```", " ").Replace('`', ' ');
            string[] lines = t.Split('\n');
            for (int i = 0; i < lines.Length; i++) lines[i] = lines[i].TrimStart(' ', '\t', '>');
            t = string.Join("\n", lines);
            return t.Replace("||", " ").Replace("**", " ").Replace("__", " ").Replace("~~", " ").Replace('*', ' ').Replace('|', ' ').Replace('~', ' ');
        }

        private static bool ParseLocator(string locator, out InviteLocatorKind kind, out string host, out int port)
        {
            kind = InviteLocatorKind.Online; host = null; port = 0;
            if (string.IsNullOrEmpty(locator)) return false;
            bool lan = locator.StartsWith("lan:", StringComparison.OrdinalIgnoreCase);
            bool vpn = locator.StartsWith("vpn:", StringComparison.OrdinalIgnoreCase);
            if (lan || vpn)
            {
                string rest = locator.Substring(4);
                int colon = rest.LastIndexOf(':');
                if (colon <= 0 || colon >= rest.Length - 1) return false;
                string ip = rest.Substring(0, colon);
                int p;
                if (!int.TryParse(rest.Substring(colon + 1), System.Globalization.NumberStyles.None,
                    System.Globalization.CultureInfo.InvariantCulture, out p)) return false;
                InviteLocatorKind wanted = lan ? InviteLocatorKind.Lan : InviteLocatorKind.Vpn;
                if (!ValidLocalEndpoint(wanted, ip, p)) return false;
                kind = wanted; host = ip.Trim(); port = p;
                return true;
            }
            if (!locator.StartsWith(LobbyCode.Prefix, StringComparison.OrdinalIgnoreCase)) return false;
            string decodedIp; int decodedPort;
            if (LobbyCode.Decode(locator, out decodedIp, out decodedPort) != LobbyCodeStatus.Ok) return false;
            kind = InviteLocatorKind.Online; host = decodedIp; port = decodedPort;
            return true;
        }

        // Fixed, address-free labels (also the only labels the banner ever shows for an invited host).
        public static string LabelFor(InviteLocatorKind kind)
        {
            switch (kind)
            {
                case InviteLocatorKind.Lan: return "LAN host";
                case InviteLocatorKind.Vpn: return "VPN host";
                default: return "Online host";
            }
        }

        // Online: a pinned public friend host (the lobby code is the pin). LAN/VPN: never a public address.
        public static ClientConnectRequest ToConnectRequest(CoopInviteData data)
        {
            if (data == null) return null;
            return new ClientConnectRequest(LobbyDiscoverySource.DirectFriendInvite, data.Host, data.Port,
                data.Kind == InviteLocatorKind.Online, LabelFor(data.Kind), data.Password);
        }

        public static string Describe(InviteStatus status)
        {
            switch (status)
            {
                case InviteStatus.Ok: return "Invite accepted.";
                case InviteStatus.Empty: return "Copy the invite your friend sent you, then choose this again.";
                default: return "That is not a valid invite. Copy the invite your friend sent you, then choose this again.";
            }
        }
    }
}
