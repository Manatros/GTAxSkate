// Locks RageCoop's own client UI while GTAxSkate coop manages the session (design 2026-11-17-streamer-safe-coop,
// item 6): RageCoop's F9 menu shows a server address and the player's name and has a typed server list.
// While multiplayer is switched on, scripts\RageCoop\Data\RageCoop.Client.Settings.xml carries
//   MenuKey = F24 (no normal keyboard has it), MasterServer empty (no browser fetch),
//   LastServerAddress = 127.0.0.1:4499, Username = Skater;
// the user's previous values are kept in a sidecar file and put back when multiplayer is switched off
// or the coop is uninstalled. RageCoop reads this file at game start; the bridge applies the same values live.
using System;
using System.IO;
using System.Text;
using System.Xml;

namespace GTAxSkate.Coop
{
    public static class RageCoopClientSettings
    {
        public const string LockedMenuKey = "F24";
        public const string LockedMasterServer = "";
        public const string LockedServerAddress = "127.0.0.1:4499";
        public const string LockedUsername = "Skater";
        public const string BackupSuffix = ".gtaxskate-backup.xml";
        public static readonly string[] Keys = { "MenuKey", "MasterServer", "LastServerAddress", "Username" };

        private static string LockedValue(string key)
        {
            switch (key)
            {
                case "MenuKey": return LockedMenuKey;
                case "MasterServer": return LockedMasterServer;
                case "LastServerAddress": return LockedServerAddress;
                default: return LockedUsername;
            }
        }

        public static string BackupPath(string settingsPath) { return settingsPath + BackupSuffix; }

        // Writes the locked values; remembers the previous ones once (a second Apply never overwrites the backup).
        public static bool Apply(string settingsPath, out string error)
        {
            error = string.Empty;
            if (string.IsNullOrEmpty(settingsPath)) { error = "Settings path is not known."; return false; }
            try
            {
                string backupPath = BackupPath(settingsPath);
                bool existed = File.Exists(settingsPath);
                var doc = new XmlDocument();
                XmlElement root;
                if (existed)
                {
                    doc.Load(settingsPath);
                    root = doc.DocumentElement;
                    if (root == null) throw new InvalidDataException("empty settings file");
                }
                else
                {
                    doc.AppendChild(doc.CreateXmlDeclaration("1.0", "utf-8", null));
                    root = doc.CreateElement("Settings");
                    doc.AppendChild(root);
                }

                if (!File.Exists(backupPath))
                {
                    var backup = new XmlDocument();
                    backup.AppendChild(backup.CreateXmlDeclaration("1.0", "utf-8", null));
                    XmlElement b = backup.CreateElement("Backup");
                    b.SetAttribute("fileExisted", existed ? "1" : "0");
                    backup.AppendChild(b);
                    for (int i = 0; i < Keys.Length; i++)
                    {
                        XmlElement previous = root[Keys[i]];
                        XmlElement entry = backup.CreateElement(Keys[i]);
                        entry.SetAttribute("present", previous != null ? "1" : "0");
                        if (previous != null) entry.InnerText = previous.InnerText;
                        b.AppendChild(entry);
                    }
                    WriteAtomic(backup, backupPath);
                }

                for (int i = 0; i < Keys.Length; i++)
                {
                    XmlElement e = root[Keys[i]];
                    if (e == null) { e = doc.CreateElement(Keys[i]); root.AppendChild(e); }
                    e.InnerText = LockedValue(Keys[i]);
                }
                WriteAtomic(doc, settingsPath);
                return true;
            }
            catch (Exception ex)
            {
                error = ex.Message;
                return false;
            }
        }

        // Puts the previous values back and removes the sidecar. No backup = nothing of ours to undo.
        public static bool Restore(string settingsPath, out string error)
        {
            error = string.Empty;
            if (string.IsNullOrEmpty(settingsPath)) { error = "Settings path is not known."; return false; }
            try
            {
                string backupPath = BackupPath(settingsPath);
                if (!File.Exists(backupPath)) return true;
                var backup = new XmlDocument();
                backup.Load(backupPath);
                XmlElement b = backup.DocumentElement;
                bool fileExisted = b != null && b.GetAttribute("fileExisted") == "1";
                if (File.Exists(settingsPath))
                {
                    var doc = new XmlDocument();
                    doc.Load(settingsPath);
                    XmlElement root = doc.DocumentElement;
                    if (root != null)
                    {
                        for (int i = 0; i < Keys.Length; i++)
                        {
                            XmlElement entry = b == null ? null : b[Keys[i]];
                            XmlElement current = root[Keys[i]];
                            if (entry != null && entry.GetAttribute("present") == "1")
                            {
                                if (current == null) { current = doc.CreateElement(Keys[i]); root.AppendChild(current); }
                                current.InnerText = entry.InnerText;
                            }
                            else if (current != null)
                            {
                                root.RemoveChild(current);
                            }
                        }
                        if (!fileExisted && root.ChildNodes.Count == 0) File.Delete(settingsPath);
                        else WriteAtomic(doc, settingsPath);
                    }
                }
                File.Delete(backupPath);
                return true;
            }
            catch (Exception ex)
            {
                error = ex.Message;
                return false;
            }
        }

        // True when the file currently carries all four locked values.
        public static bool IsLocked(string settingsPath)
        {
            try
            {
                if (!File.Exists(settingsPath)) return false;
                var doc = new XmlDocument();
                doc.Load(settingsPath);
                XmlElement root = doc.DocumentElement;
                if (root == null) return false;
                for (int i = 0; i < Keys.Length; i++)
                {
                    XmlElement e = root[Keys[i]];
                    if (e == null || e.InnerText != LockedValue(Keys[i])) return false;
                }
                return true;
            }
            catch { return false; }
        }

        private static void WriteAtomic(XmlDocument doc, string path)
        {
            string dir = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(dir)) Directory.CreateDirectory(dir);
            string temp = path + ".tmp";
            try
            {
                var settings = new XmlWriterSettings { Indent = true, Encoding = new UTF8Encoding(false) };
                using (XmlWriter w = XmlWriter.Create(temp, settings)) doc.Save(w);
                if (File.Exists(path)) File.Replace(temp, path, null);
                else File.Move(temp, path);
            }
            finally
            {
                try { if (File.Exists(temp)) File.Delete(temp); } catch { }
            }
        }
    }
}
