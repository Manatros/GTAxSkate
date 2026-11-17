// Bounded, dependency-free reader for optional FriendsCoopClient.json configuration.
// Enforces CoopSecurityPolicy fail-closed rules even if a config file is edited by hand.
using System;
using System.IO;
using System.Text;

namespace GTAxSkate.Coop
{
    public static class ClientConfigLoader
    {
        public static bool TryParseClientConfigJson(
            string jsonText,
            out ClientConnectRequest request,
            out string error)
        {
            request = null;
            if (string.IsNullOrEmpty(jsonText) || jsonText.Length > 16384)
            {
                error = "FriendsCoopClient.json is empty or exceeds 16 KB limit.";
                return false;
            }

            bool blockPublic = ExtractBool(jsonText, "BlockPublicServerBrowser", true);
            string hostLabel = ExtractString(jsonText, "TrustedHostLabel");
            string hostIp = ExtractString(jsonText, "TrustedHostIp");
            string friendKey = ExtractString(jsonText, "FriendKey");
            int port = ExtractInt(jsonText, "Port", 4499);

            if (!blockPublic)
            {
                error = "Config rejected: BlockPublicServerBrowser must stay true.";
                return false;
            }
            if (string.IsNullOrEmpty(hostIp))
            {
                error = "Config rejected: TrustedHostIp is missing.";
                return false;
            }

            var candidate = new ClientConnectRequest(
                LobbyDiscoverySource.DirectFriendInvite,
                hostIp,
                port,
                false,
                // Drawn on screen: only a label without digits, dots, colons, '@' or slashes survives (else "Friend").
                string.IsNullOrEmpty(hostLabel) ? "VPN host" : CoopSecurityPolicy.ScreenLabel(hostLabel),
                friendKey);

            PolicyDecision decision = CoopSecurityPolicy.EvaluateOutboundConnect(candidate);
            if (!decision.Allowed)
            {
                error = decision.Reason;
                return false;
            }

            request = candidate;
            error = string.Empty;
            return true;
        }

        public static bool TryLoadFromGtaRoot(
            string gtaRoot,
            out ClientConnectRequest request,
            out string error)
        {
            request = null;
            if (string.IsNullOrEmpty(gtaRoot))
            {
                error = "GTA root path is empty.";
                return false;
            }
            string path = Path.Combine(Path.Combine(Path.Combine(gtaRoot, "scripts"), "RageCoop"), "FriendsCoopClient.json");
            if (!File.Exists(path))
            {
                error = "Optional FriendsCoopClient.json not configured yet.";
                return false;
            }
            string text = File.ReadAllText(path, Encoding.UTF8);
            return TryParseClientConfigJson(text, out request, out error);
        }

        private static string ExtractString(string json, string key)
        {
            string token = "\"" + key + "\"";
            int idx = json.IndexOf(token, StringComparison.OrdinalIgnoreCase);
            if (idx < 0) return string.Empty;
            int colon = json.IndexOf(':', idx + token.Length);
            if (colon < 0) return string.Empty;
            int q1 = json.IndexOf('"', colon + 1);
            if (q1 < 0) return string.Empty;
            var sb = new StringBuilder();
            for (int i = q1 + 1; i < json.Length && sb.Length < 256; i++)
            {
                char c = json[i];
                if (c == '\\' && i + 1 < json.Length)
                {
                    sb.Append(json[++i]);
                    continue;
                }
                if (c == '"') return sb.ToString();
                sb.Append(c);
            }
            return string.Empty;
        }

        private static bool ExtractBool(string json, string key, bool fallback)
        {
            string token = "\"" + key + "\"";
            int idx = json.IndexOf(token, StringComparison.OrdinalIgnoreCase);
            if (idx < 0) return fallback;
            int colon = json.IndexOf(':', idx + token.Length);
            if (colon < 0) return fallback;
            int end = Math.Min(json.Length, colon + 16);
            string tail = json.Substring(colon + 1, end - (colon + 1)).Trim().ToLowerInvariant();
            if (tail.StartsWith("true", StringComparison.Ordinal)) return true;
            if (tail.StartsWith("false", StringComparison.Ordinal)) return false;
            return fallback;
        }

        private static int ExtractInt(string json, string key, int fallback)
        {
            string token = "\"" + key + "\"";
            int idx = json.IndexOf(token, StringComparison.OrdinalIgnoreCase);
            if (idx < 0) return fallback;
            int colon = json.IndexOf(':', idx + token.Length);
            if (colon < 0) return fallback;
            int i = colon + 1;
            while (i < json.Length && (json[i] == ' ' || json[i] == '\t')) i++;
            int val = 0;
            bool any = false;
            while (i < json.Length && json[i] >= '0' && json[i] <= '9')
            {
                any = true;
                if (val > 65535) break;
                val = val * 10 + (json[i] - '0');
                i++;
            }
            return any ? val : fallback;
        }
    }
}
