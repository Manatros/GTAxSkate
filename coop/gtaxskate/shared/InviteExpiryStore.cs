// Crash-proof clipboard expiry for the host invite (design 2026-11-17-streamer-safe-coop, round 2). The in-game 10-minute
// timer stops when GTA stops ticking, so a crash would leave the invite on the clipboard. The expiry is therefore also
// written to %LOCALAPPDATA%\SkateV\coop-invite-expiry.json: ONLY a SHA-256 of the invite text and the expiry time
// (never the invite itself). The next adapter start reads the clipboard and clears it only when its text hashes to the
// stored value and the time has passed. Pure, C# 5; never throws.
using System;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;

namespace GTAxSkate.Coop
{
    public static class InviteExpiryStore
    {
        public const string FileName = "coop-invite-expiry.json";
        public static string DirectoryOverride;   // tests point this at a scratch folder

        private static string FilePath()
        {
#if COOP_FIXTURE
            if (DirectoryOverride == null) return null;   // fixture runs never touch the real profile
#endif
            string dir = DirectoryOverride ?? Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "SkateV");
            return string.IsNullOrEmpty(dir) ? null : Path.Combine(dir, FileName);
        }

        public static string Hash(string text)
        {
            using (SHA256 sha = SHA256.Create())
            {
                byte[] h = sha.ComputeHash(Encoding.UTF8.GetBytes(text ?? string.Empty));
                var sb = new StringBuilder();
                for (int i = 0; i < h.Length; i++) sb.Append(h[i].ToString("x2"));
                return sb.ToString();
            }
        }

        public static void Write(string inviteText, DateTime expiresUtc)
        {
            try
            {
                string path = FilePath();
                if (path == null || string.IsNullOrEmpty(inviteText)) return;
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                string json = "{\"sha256\":\"" + Hash(inviteText) + "\",\"expiresUtc\":\"" +
                    expiresUtc.ToUniversalTime().ToString("o", CultureInfo.InvariantCulture) + "\"}";
                string tmp = path + ".tmp";
                File.WriteAllText(tmp, json, new UTF8Encoding(false));
                if (File.Exists(path)) File.Delete(path);
                File.Move(tmp, path);
            }
            catch { }
        }

        public static bool TryRead(out string sha256, out DateTime expiresUtc)
        {
            sha256 = null; expiresUtc = DateTime.MinValue;
            try
            {
                string path = FilePath();
                if (path == null || !File.Exists(path)) return false;
                string json = File.ReadAllText(path, Encoding.UTF8);
                sha256 = Field(json, "sha256");
                string when = Field(json, "expiresUtc");
                DateTime parsed;
                if (string.IsNullOrEmpty(sha256) || sha256.Length != 64 ||
                    !DateTime.TryParse(when, CultureInfo.InvariantCulture,
                        DateTimeStyles.AdjustToUniversal | DateTimeStyles.AssumeUniversal, out parsed))
                {
                    sha256 = null;
                    Delete();   // unreadable: nothing to act on
                    return false;
                }
                expiresUtc = parsed;
                return true;
            }
            catch { sha256 = null; return false; }
        }

        public static void Delete()
        {
            try
            {
                string path = FilePath();
                if (path != null && File.Exists(path)) File.Delete(path);
            }
            catch { }
        }

        // Deletes the record only when it still belongs to this invite (a newer hosting may have replaced it meanwhile).
        public static void DeleteIfHash(string sha256)
        {
            string current; DateTime expires;
            if (!TryRead(out current, out expires)) return;
            if (string.Equals(current, sha256, StringComparison.Ordinal)) Delete();
        }

        // The file is ours and flat; a tiny extractor avoids a JSON dependency in the game process.
        private static string Field(string json, string name)
        {
            string key = "\"" + name + "\":\"";
            int i = json.IndexOf(key, StringComparison.Ordinal);
            if (i < 0) return null;
            i += key.Length;
            int j = json.IndexOf('"', i);
            return j < 0 ? null : json.Substring(i, j - i);
        }
    }
}
