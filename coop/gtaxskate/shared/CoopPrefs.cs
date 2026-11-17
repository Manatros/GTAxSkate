// Strict reader/writer for scripts\RageCoop\FriendsCoopPrefs.json, the persisted
// "multiplayer enabled" choice. Separate from FriendsCoopClient.json (credentials),
// which is never rewritten. Anything but the exact top-level literal true means OFF.
using System;
using System.IO;
using System.Text;

namespace GTAxSkate.Coop
{
    public static class CoopPrefs
    {
        public const string EnabledKey = "MultiplayerEnabled";
        public const int MaxBytes = 16384;

        // Flat-object strict parser. Returns true only for {"MultiplayerEnabled": true}
        // (other keys with scalar values are tolerated). Duplicate key, quoted/numeric/null
        // value, nested containers, trailing junk, malformed JSON or >16 KB => false.
        public static bool ParseEnabled(string json)
        {
            if (string.IsNullOrEmpty(json) || json.Length > MaxBytes) return false;
            int i = 0;
            SkipWs(json, ref i);
            if (i >= json.Length || json[i] != '{') return false;
            i++;
            bool seen = false, result = false;
            SkipWs(json, ref i);
            if (i < json.Length && json[i] == '}') { i++; return Finish(json, i) && false; }
            while (true)
            {
                SkipWs(json, ref i);
                string key;
                if (!ReadString(json, ref i, out key)) return false;
                SkipWs(json, ref i);
                if (i >= json.Length || json[i] != ':') return false;
                i++;
                SkipWs(json, ref i);
                if (i >= json.Length) return false;
                bool isTarget = string.Equals(key, EnabledKey, StringComparison.Ordinal);
                if (isTarget && seen) return false; // duplicate
                char c = json[i];
                if (c == '"')
                {
                    string ignored;
                    if (!ReadString(json, ref i, out ignored)) return false;
                    if (isTarget) { seen = true; result = false; return false; }
                }
                else if (Literal(json, ref i, "true"))
                {
                    if (isTarget) { seen = true; result = true; }
                }
                else if (Literal(json, ref i, "false"))
                {
                    if (isTarget) { seen = true; result = false; }
                }
                else if (Literal(json, ref i, "null"))
                {
                    if (isTarget) return false;
                }
                else if (c == '-' || (c >= '0' && c <= '9'))
                {
                    if (isTarget) return false;
                    if (!SkipNumber(json, ref i)) return false; // strict RFC 8259 number or the whole file is rejected
                }
                else
                {
                    return false; // nested object/array or garbage
                }
                SkipWs(json, ref i);
                if (i >= json.Length) return false;
                if (json[i] == ',') { i++; continue; }
                if (json[i] == '}') { i++; break; }
                return false;
            }
            return Finish(json, i) && result;
        }

        public static bool Read(string path)
        {
            try
            {
                if (string.IsNullOrEmpty(path)) return false;
                var info = new FileInfo(path);
                if (!info.Exists || info.Length > MaxBytes) return false;
                byte[] bytes;
                using (var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                {
                    bytes = new byte[(int)Math.Min(fs.Length, MaxBytes)];
                    int total = 0;
                    while (total < bytes.Length)
                    {
                        int n = fs.Read(bytes, total, bytes.Length - total);
                        if (n <= 0) break;
                        total += n;
                    }
                    if (total != bytes.Length) return false;
                }
                string text = new UTF8Encoding(false, false).GetString(bytes);
                if (text.Length > 0 && text[0] == '﻿') text = text.Substring(1);
                return ParseEnabled(text);
            }
            catch
            {
                return false;
            }
        }

        // Atomic write: temp file in the same directory, then replace/move.
        public static bool Write(string path, bool enabled, out string error)
        {
            error = string.Empty;
            if (string.IsNullOrEmpty(path)) { error = "Preferences path is not known."; return false; }
            string temp = null;
            try
            {
                string dir = Path.GetDirectoryName(path);
                if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);
                temp = path + ".tmp";
                File.WriteAllText(temp, "{\r\n  \"" + EnabledKey + "\": " + (enabled ? "true" : "false") + "\r\n}\r\n", new UTF8Encoding(false));
                if (File.Exists(path)) File.Replace(temp, path, null);
                else File.Move(temp, path);
                return true;
            }
            catch (Exception ex)
            {
                error = ex.Message;
                try { if (temp != null && File.Exists(temp)) File.Delete(temp); } catch { }
                return false;
            }
        }

        // RFC 8259: -? (0 | [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?  The token must end at a delimiter.
        private static bool SkipNumber(string s, ref int i)
        {
            int j = i;
            if (j < s.Length && s[j] == '-') j++;
            if (j >= s.Length || s[j] < '0' || s[j] > '9') return false;
            if (s[j] == '0') j++;
            else while (j < s.Length && s[j] >= '0' && s[j] <= '9') j++;
            if (j < s.Length && s[j] == '.')
            {
                j++;
                int digits = j;
                while (j < s.Length && s[j] >= '0' && s[j] <= '9') j++;
                if (j == digits) return false;
            }
            if (j < s.Length && (s[j] == 'e' || s[j] == 'E'))
            {
                j++;
                if (j < s.Length && (s[j] == '+' || s[j] == '-')) j++;
                int digits = j;
                while (j < s.Length && s[j] >= '0' && s[j] <= '9') j++;
                if (j == digits) return false;
            }
            if (j < s.Length)
            {
                char c = s[j];
                if (c != ',' && c != '}' && c != ' ' && c != '\t' && c != '\r' && c != '\n') return false;
            }
            i = j;
            return true;
        }

        private static bool Finish(string json, int i)
        {
            SkipWs(json, ref i);
            return i == json.Length;
        }

        private static void SkipWs(string s, ref int i)
        {
            while (i < s.Length && (s[i] == ' ' || s[i] == '\t' || s[i] == '\r' || s[i] == '\n')) i++;
        }

        private static bool Literal(string s, ref int i, string word)
        {
            if (string.CompareOrdinal(s, i, word, 0, word.Length) != 0) return false;
            int end = i + word.Length;
            if (end < s.Length && char.IsLetterOrDigit(s[end])) return false; // "trueish" is not true
            i = end;
            return true;
        }

        private static bool ReadString(string s, ref int i, out string value)
        {
            value = null;
            if (i >= s.Length || s[i] != '"') return false;
            var sb = new StringBuilder();
            for (int j = i + 1; j < s.Length; j++)
            {
                char c = s[j];
                if (c == '"') { value = sb.ToString(); i = j + 1; return true; }
                if (c < 0x20) return false;
                if (c == '\\')
                {
                    if (j + 1 >= s.Length) return false;
                    char n = s[++j];
                    if (n == 'u')
                    {
                        if (j + 4 >= s.Length) return false;
                        int code;
                        if (!int.TryParse(s.Substring(j + 1, 4), System.Globalization.NumberStyles.HexNumber,
                            System.Globalization.CultureInfo.InvariantCulture, out code)) return false;
                        sb.Append((char)code);
                        j += 4;
                    }
                    else if ("\"\\/bfnrt".IndexOf(n) >= 0) sb.Append(n);
                    else return false;
                    continue;
                }
                sb.Append(c);
            }
            return false;
        }
    }
}
