// The friend key the locally started RageCoop.Server.exe was launched with (as a SHA-256 fingerprint; the key itself
// is never stored). A second host attempt generates a new password, so a still-running server with the old one must be
// replaced: it would refuse the new connection as "wrong transport password" (test 2026-11-17 10:14).
using System;
using System.Security.Cryptography;
using System.Text;

namespace GTAxSkate.Coop
{
    public enum LocalServerAction { Reuse = 0, Refuse = 2 }

    public static class LocalServerKey
    {
        public static string Fingerprint(string key)
        {
            using (SHA256 sha = SHA256.Create())
            {
                byte[] h = sha.ComputeHash(Encoding.UTF8.GetBytes(key ?? string.Empty));
                var sb = new StringBuilder();
                for (int i = 0; i < h.Length; i++) sb.Append(h[i].ToString("x2"));
                return sb.ToString();
            }
        }

        // True when a running server may be reused for this key. Unknown fingerprint (a server from before this
        // build, or lost state) counts as different: restarting is always safe.
        public static bool CanReuse(string runningFingerprint, string key)
        {
            return !string.IsNullOrEmpty(runningFingerprint) &&
                string.Equals(runningFingerprint, Fingerprint(key), StringComparison.Ordinal);
        }

        public const string StillRunningNotice =
            "A coop server from your last session is still running. Choose 'Stop local server' first.";

        // What a new hosting does with a still-running local server (Copilot round 3): the same key reuses it; any
        // other key (or an unknown one) is refused. A host action never kills a running server: friends may still be
        // on it, and only the player's explicit "Stop local server" row ends it.
        public static LocalServerAction Decide(string runningFingerprint, string key)
        {
            return CanReuse(runningFingerprint, key) ? LocalServerAction.Reuse : LocalServerAction.Refuse;
        }
    }
}
