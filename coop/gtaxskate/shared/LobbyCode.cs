// Internet lobby code (design 2026-11-16-coop-internet-lobby, rev 2). Pure, C# 5, no I/O.
//
// A code is only a LOCATOR for the host's public IPv4 + UDP port, never a secret: the friend key
// stays the password. Payload = IPv4 (4 bytes) + port (2 bytes) + CRC-8 (1 byte) = 56 bits,
// padded with 4 zero bits to 60 bits = 12 Crockford base32 symbols, shown as SK8-XXXX-XXXX-XXXX.
using System;
using System.Text;

namespace GTAxSkate.Coop
{
    public enum LobbyCodeStatus
    {
        Ok = 0,
        Malformed = 1,      // wrong length or a symbol outside the alphabet
        BadChecksum = 2,    // mistyped: CRC-8 or the zero padding do not match
        ReservedAddress = 3,// decoded IPv4 is not a global address
        BadPort = 4
    }

    public static class LobbyCode
    {
        public const string Prefix = "SK8";
        public const int MinPort = 1024;
        public const int SymbolCount = 12;
        private const string Alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"; // Crockford: no I, L, O, U

        // True when the address is a global unicast IPv4 (none of the reserved ranges below).
        public static bool IsGlobalIPv4(byte[] b)
        {
            if (b == null || b.Length != 4) return false;
            if (b[0] == 0) return false;                                        // 0.0.0.0/8
            if (b[0] == 10) return false;                                       // 10/8
            if (b[0] == 100 && b[1] >= 64 && b[1] <= 127) return false;         // 100.64/10 (CGNAT)
            if (b[0] == 127) return false;                                      // 127/8
            if (b[0] == 169 && b[1] == 254) return false;                       // 169.254/16
            if (b[0] == 172 && b[1] >= 16 && b[1] <= 31) return false;          // 172.16/12
            if (b[0] == 192 && b[1] == 0 && b[2] == 0) return false;            // 192.0.0/24
            if (b[0] == 192 && b[1] == 0 && b[2] == 2) return false;            // 192.0.2/24
            if (b[0] == 192 && b[1] == 88 && b[2] == 99) return false;          // 192.88.99/24 (6to4 relay, Copilot review)
            if (b[0] == 192 && b[1] == 168) return false;                       // 192.168/16
            if (b[0] == 198 && (b[1] == 18 || b[1] == 19)) return false;        // 198.18/15
            if (b[0] == 198 && b[1] == 51 && b[2] == 100) return false;         // 198.51.100/24
            if (b[0] == 203 && b[1] == 0 && b[2] == 113) return false;          // 203.0.113/24
            if (b[0] >= 224) return false;                                      // 224/4 multicast, 240/4 reserved
            return true;
        }

        public static bool IsGlobalIPv4(string text)
        {
            byte[] b;
            return TryParseIPv4(text, out b) && IsGlobalIPv4(b);
        }

        // Strict dotted quad (no hex, no short forms, no whitespace inside).
        public static bool TryParseIPv4(string text, out byte[] bytes)
        {
            bytes = null;
            if (string.IsNullOrEmpty(text)) return false;
            string[] parts = text.Trim().Split('.');
            if (parts.Length != 4) return false;
            var b = new byte[4];
            for (int i = 0; i < 4; i++)
            {
                string p = parts[i];
                if (p.Length == 0 || p.Length > 3) return false;
                int v = 0;
                for (int k = 0; k < p.Length; k++)
                {
                    if (p[k] < '0' || p[k] > '9') return false;
                    v = v * 10 + (p[k] - '0');
                }
                if (v > 255) return false;
                b[i] = (byte)v;
            }
            bytes = b;
            return true;
        }

        public static bool TryEncode(string ipv4, int port, out string code)
        {
            code = null;
            byte[] ip;
            if (!TryParseIPv4(ipv4, out ip) || !IsGlobalIPv4(ip)) return false;
            if (port < MinPort || port > 65535) return false;
            code = EncodeRaw(ip, port);
            return true;
        }

        // No range checks: tests use it to build well-formed codes for reserved addresses.
        internal static string EncodeRaw(byte[] ip, int port)
        {
            var payload = new byte[7];
            Array.Copy(ip, payload, 4);
            payload[4] = (byte)(port >> 8);
            payload[5] = (byte)port;
            payload[6] = Crc8(payload, 6);
            // 56 payload bits followed by 4 zero padding bits, five bits per symbol.
            var sb = new StringBuilder();
            sb.Append(Prefix);
            for (int s = 0; s < SymbolCount; s++)
            {
                int value = 0;
                for (int bit = 0; bit < 5; bit++)
                {
                    int pos = s * 5 + bit;
                    int bitValue = pos < 56 ? (payload[pos / 8] >> (7 - pos % 8)) & 1 : 0;
                    value = (value << 1) | bitValue;
                }
                if (s % 4 == 0) sb.Append('-');
                sb.Append(Alphabet[value]);
            }
            return sb.ToString();
        }

        // Folds case, dashes/spaces, O->0, I/L->1 and an optional SK8 prefix. Returns null when the
        // text cannot be a code (wrong length or a symbol outside the alphabet).
        public static string Normalize(string text)
        {
            if (string.IsNullOrEmpty(text)) return null;
            var sb = new StringBuilder();
            foreach (char raw in text)
            {
                if (raw == '-' || raw == ' ' || raw == '_' || raw == '\t' || raw == '\r' || raw == '\n') continue;
                char c = char.ToUpperInvariant(raw);
                if (c == 'O') c = '0';
                else if (c == 'I' || c == 'L') c = '1';
                if (Alphabet.IndexOf(c) < 0) return null;
                sb.Append(c);
            }
            string s = sb.ToString();
            if (s.Length == SymbolCount + Prefix.Length && s.StartsWith(Prefix, StringComparison.Ordinal))
                s = s.Substring(Prefix.Length);
            return s.Length == SymbolCount ? s : null;
        }

        public static LobbyCodeStatus Decode(string text, out string ipv4, out int port)
        {
            ipv4 = null; port = 0;
            string s = Normalize(text);
            if (s == null) return LobbyCodeStatus.Malformed;
            var payload = new byte[7];
            for (int sym = 0; sym < SymbolCount; sym++)
            {
                int value = Alphabet.IndexOf(s[sym]);
                for (int bit = 0; bit < 5; bit++)
                {
                    int bitValue = (value >> (4 - bit)) & 1;
                    int pos = sym * 5 + bit;
                    if (pos < 56) payload[pos / 8] |= (byte)(bitValue << (7 - pos % 8));
                    else if (bitValue != 0) return LobbyCodeStatus.BadChecksum; // padding must be zero
                }
            }
            if (Crc8(payload, 6) != payload[6]) return LobbyCodeStatus.BadChecksum;
            var ip = new byte[] { payload[0], payload[1], payload[2], payload[3] };
            int p = (payload[4] << 8) | payload[5];
            if (!IsGlobalIPv4(ip)) return LobbyCodeStatus.ReservedAddress;
            if (p < MinPort) return LobbyCodeStatus.BadPort;
            ipv4 = ip[0] + "." + ip[1] + "." + ip[2] + "." + ip[3];
            port = p;
            return LobbyCodeStatus.Ok;
        }

        public static string Describe(LobbyCodeStatus status)
        {
            switch (status)
            {
                case LobbyCodeStatus.Ok: return "Lobby code accepted.";
                case LobbyCodeStatus.Malformed: return "Lobby code mistyped: it has 12 letters and digits (SK8-XXXX-XXXX-XXXX).";
                case LobbyCodeStatus.BadChecksum: return "Lobby code mistyped: the check characters do not match.";
                case LobbyCodeStatus.ReservedAddress: return "Lobby code is not an internet address (LAN/VPN friends use the pinned-friend entry).";
                case LobbyCodeStatus.BadPort: return "Lobby code carries an invalid port.";
                default: return "Lobby code rejected.";
            }
        }

        // CRC-8, polynomial 0x07, initial value 0. Detects every burst up to 8 bits, so any single
        // changed base32 symbol (5 contiguous bits) is caught.
        internal static byte Crc8(byte[] data, int length)
        {
            int crc = 0;
            for (int i = 0; i < length; i++)
            {
                crc ^= data[i];
                for (int k = 0; k < 8; k++)
                    crc = (crc & 0x80) != 0 ? ((crc << 1) ^ 0x07) & 0xFF : (crc << 1) & 0xFF;
            }
            return (byte)crc;
        }
    }
}
