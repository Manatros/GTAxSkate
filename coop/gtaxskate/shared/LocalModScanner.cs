// Privacy-safe local GTA V / SK8V mod scanner for Friends-Only Coop parity checks.
// Scans known mod locations inside a GTA folder, hashes payloads locally, and
// emits strictly neutral relative IDs (no drive letters, usernames, or host paths).
using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;

namespace GTAxSkate.Coop
{
    public static class LocalModScanner
    {
        private const long FullHashLimitBytes = 8 * 1024 * 1024;
        private const int PartialWindowBytes = 65536;

        public static ModEntry[] ScanGtaRoot(string gtaRoot)
        {
            if (string.IsNullOrEmpty(gtaRoot) || !Directory.Exists(gtaRoot))
                return new ModEntry[0];

            var found = new List<ModEntry>();

            // 1. Core SK8V runtime binaries (required to match across friends).
            AddIfExists(found, Path.Combine(gtaRoot, "SkateVLegacy.asi"), "SkateVLegacy.asi");
            AddIfExists(found, Path.Combine(gtaRoot, "SkateVRuntime.dll"), "SkateVRuntime.dll");

            // 2. Optional Skate School + StartPark tutorial expansion manifest & assets.
            AddIfExists(found,
                Path.Combine(Path.Combine(Path.Combine(gtaRoot, "SK8V"), "tutorial"), "m0-tutorial-manifest.json"),
                "SK8V/tutorial/m0-tutorial-manifest.json");
            AddIfExists(found,
                Path.Combine(gtaRoot, "SK8V/addons/skate_school/collision/DIST_SkateSchool.svwc".Replace('/', Path.DirectorySeparatorChar)),
                "SK8V/addons/skate_school/collision/DIST_SkateSchool.svwc");
            AddIfExists(found,
                Path.Combine(gtaRoot, "SK8V/addons/skate_school/collision/DIST_StartPark.svwc".Replace('/', Path.DirectorySeparatorChar)),
                "SK8V/addons/skate_school/collision/DIST_StartPark.svwc");
            AddIfExists(found,
                Path.Combine(gtaRoot, "SK8V/addons/skate_school/splines/DIST_SkateSchool_rails.json".Replace('/', Path.DirectorySeparatorChar)),
                "SK8V/addons/skate_school/splines/DIST_SkateSchool_rails.json");

            // 3. Shared update.rpf overrides (handling/gameconfig/dlclist).
            AddIfExists(found,
                Path.Combine(Path.Combine(gtaRoot, "mods"), "update/update.rpf".Replace('/', Path.DirectorySeparatorChar)),
                "mods/update/update.rpf");

            // 4. Add-on DLC packs (only retain mods classified as interfering/desync-risk).
            string dlcRoot = Path.Combine(gtaRoot, "mods/update/x64/dlcpacks".Replace('/', Path.DirectorySeparatorChar));
            if (Directory.Exists(dlcRoot))
            {
                string[] dirs = Directory.GetDirectories(dlcRoot);
                Array.Sort(dirs, StringComparer.OrdinalIgnoreCase);
                for (int i = 0; i < dirs.Length; i++)
                {
                    string packName = Path.GetFileName(dirs[i]);
                    if (string.IsNullOrEmpty(packName)) continue;
                    string rpf = Path.Combine(dirs[i], "dlc.rpf");
                    AddIfInterfering(found, rpf, "dlcpacks/" + packName + "/dlc.rpf");
                }
            }

            string rootDlc = Path.Combine(gtaRoot, "dlcpacks");
            if (Directory.Exists(rootDlc))
            {
                string[] dirs = Directory.GetDirectories(rootDlc);
                Array.Sort(dirs, StringComparer.OrdinalIgnoreCase);
                for (int i = 0; i < dirs.Length; i++)
                {
                    string packName = Path.GetFileName(dirs[i]);
                    if (string.IsNullOrEmpty(packName)) continue;
                    string relId = "dlcpacks/" + packName + "/dlc.rpf";
                    string rpf = Path.Combine(dirs[i], "dlc.rpf");
                    string normalizedRelId = ModEntry.SanitizeRelativeId(relId);
                    string modsDigest = null;
                    for (int j = 0; j < found.Count; j++)
                    {
                        if (string.Equals(found[j].RelativeId, normalizedRelId, StringComparison.OrdinalIgnoreCase))
                        {
                            modsDigest = found[j].Digest;
                            break;
                        }
                    }
                    if (modsDigest == null)
                    {
                        AddIfInterfering(found, rpf, relId);
                    }
                    else if (File.Exists(rpf))
                    {
                        try
                        {
                            if (!string.Equals(ComputeFastDigest(rpf), modsDigest, StringComparison.OrdinalIgnoreCase))
                                AddIfInterfering(found, rpf, "root-dlcpacks/" + packName + "/dlc.rpf");
                        }
                        catch
                        {
                            AddIfInterfering(found, rpf, "root-dlcpacks/" + packName + "/dlc.rpf");
                        }
                    }
                }
            }

            // 5. Root ASIs & Scripts folder plugins: ignore RageCoop itself and harmless local-only plugins.
            string[] rootAsis = Directory.GetFiles(gtaRoot, "*.asi");
            Array.Sort(rootAsis, StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < rootAsis.Length; i++)
            {
                string name = Path.GetFileName(rootAsis[i]);
                if (string.IsNullOrEmpty(name) || name.Equals("SkateVLegacy.asi", StringComparison.OrdinalIgnoreCase))
                    continue;
                AddIfInterfering(found, rootAsis[i], name);
            }

            string scriptsDir = Path.Combine(gtaRoot, "scripts");
            if (Directory.Exists(scriptsDir))
            {
                string[] files = Directory.GetFiles(scriptsDir);
                Array.Sort(files, StringComparer.OrdinalIgnoreCase);
                for (int i = 0; i < files.Length; i++)
                {
                    string ext = Path.GetExtension(files[i]);
                    if (!string.Equals(ext, ".dll", StringComparison.OrdinalIgnoreCase) &&
                        !string.Equals(ext, ".asi", StringComparison.OrdinalIgnoreCase) &&
                        !string.Equals(ext, ".cs", StringComparison.OrdinalIgnoreCase))
                        continue;
                    string name = Path.GetFileName(files[i]);
                    if (string.IsNullOrEmpty(name) || name.StartsWith("RageCoop", StringComparison.OrdinalIgnoreCase))
                        continue;
                    AddIfInterfering(found, files[i], "scripts/" + name);
                }
            }

            return ModCompatibilityChecker.NormalizeManifest(found.ToArray());
        }

        private static void AddIfInterfering(List<ModEntry> target, string fullPath, string relativeId)
        {
            if (ModCompatibilityChecker.ClassifyModPath(relativeId) == ModImpactClass.CosmeticLocalOnly)
                return;
            AddIfExists(target, fullPath, relativeId);
        }

        private static void AddIfExists(List<ModEntry> target, string fullPath, string relativeId)
        {
            if (target.Count >= ModCompatibilityChecker.MaxModEntries || !File.Exists(fullPath))
                return;
            try
            {
                string digest = ComputeFastDigest(fullPath);
                target.Add(new ModEntry(relativeId, digest, ModCompatibilityChecker.ClassifyModPath(relativeId)));
            }
            catch
            {
                // Ignore transient file locks defensively rather than blocking connect.
            }
        }

        // Uses exact SHA-256 up to 8 MB and bounded length+edge SHA-256 above 8 MB (e.g., dlc.rpf)
        // so connect checks complete in milliseconds without stalling the game thread.
        public static string ComputeFastDigest(string fullPath)
        {
            var info = new FileInfo(fullPath);
            using (var sha = SHA256.Create())
            using (var fs = new FileStream(fullPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            {
                byte[] hash;
                if (info.Length <= FullHashLimitBytes)
                {
                    hash = sha.ComputeHash(fs);
                }
                else
                {
                    byte[] header = BitConverter.GetBytes(info.Length);
                    byte[] head = new byte[PartialWindowBytes];
                    byte[] tail = new byte[PartialWindowBytes];
                    int hRead = fs.Read(head, 0, head.Length);
                    fs.Seek(-PartialWindowBytes, SeekOrigin.End);
                    int tRead = fs.Read(tail, 0, tail.Length);
                    sha.TransformBlock(header, 0, header.Length, null, 0);
                    sha.TransformBlock(head, 0, hRead, null, 0);
                    sha.TransformFinalBlock(tail, 0, tRead);
                    hash = sha.Hash;
                }
                var sb = new StringBuilder(12);
                for (int i = 0; i < 6 && i < hash.Length; i++)
                    sb.Append(hash[i].ToString("x2"));
                return sb.ToString();
            }
        }
    }
}
