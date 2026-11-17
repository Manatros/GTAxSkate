// Neutral, path-safe mod-parity & interference warning checker for private Friend Coop.
// Compares DLC packs, SK8V binaries, and script/handling mods across connected friends
// and surfaces warnings for mods that can cause desync, missing maps, or physics conflicts.
using System;
using System.Collections.Generic;
using System.Text;

namespace GTAxSkate.Coop
{
    public enum ModImpactClass
    {
        CosmeticLocalOnly = 0,     // ENB/ReShade, HUD preset, audio-only: safe locally
        GameplayInterference = 1,  // Script/trainer/traffic/ped mods: may interfere
        CriticalMapOrPhysics = 2   // SK8V runtime/DLCs, custom YMAP/handling/gameconfig: desync risk
    }

    public sealed class ModEntry
    {
        public readonly string RelativeId; // Neutral relative path (no drive letters/usernames)
        public readonly string Digest;     // Short lowercase hex digest/version tag (<= 16 chars)
        public readonly ModImpactClass Impact;

        public ModEntry(string relativeId, string digest, ModImpactClass impact)
        {
            RelativeId = SanitizeRelativeId(relativeId);
            Digest = SanitizeDigest(digest);
            Impact = impact;
        }

        public static string SanitizeRelativeId(string raw)
        {
            if (string.IsNullOrEmpty(raw)) return string.Empty;
            string s = raw.Replace('\\', '/').Trim();
            // Reject drive-letter prefixes or UNC/user-profile roots; keep filename/relative tail only.
            int colon = s.IndexOf(':');
            if (colon >= 0) s = s.Substring(colon + 1);
            string lower = s.ToLowerInvariant();
            string[] prefixes = new[] { "/users/", "users/" };
            for (int p = 0; p < prefixes.Length; p++)
            {
                if (lower.StartsWith(prefixes[p], StringComparison.Ordinal))
                {
                    int slash = s.LastIndexOf('/');
                    s = slash >= 0 ? s.Substring(slash + 1) : s;
                }
            }
            s = s.TrimStart('/');
            var clean = new List<char>(s.Length);
            for (int i = 0; i < s.Length; i++)
            {
                char c = s[i];
                if (c >= 0x20 && c <= 0x7E) clean.Add(c);
            }
            string ascii = new string(clean.ToArray());
            if (ascii.Length > 64)
                ascii = ascii.Substring(0, 31) + "~" + ascii.Substring(ascii.Length - 32);
            return ascii;
        }

        private static string SanitizeDigest(string raw)
        {
            if (string.IsNullOrEmpty(raw)) return "none";
            string clean = raw.Trim().ToLowerInvariant();
            var sb = new StringBuilder(Math.Min(clean.Length, 16));
            for (int i = 0; i < clean.Length && sb.Length < 16; i++)
            {
                char c = clean[i];
                if ((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || c == '.' || c == '-')
                    sb.Append(c);
            }
            return sb.Length == 0 ? "none" : sb.ToString();
        }
    }

    public sealed class ModMismatchItem
    {
        public readonly string RelativeId;
        public readonly ModImpactClass Impact;
        public readonly string Detail; // "missing locally", "extra locally", "different version"

        internal ModMismatchItem(string relativeId, ModImpactClass impact, string detail)
        {
            RelativeId = relativeId;
            Impact = impact;
            Detail = detail;
        }
    }

    public sealed class ModCompatibilityReport
    {
        public readonly string PeerLabel;
        public readonly ModMismatchItem[] Mismatches;
        public readonly string WarningSummary;

        public bool HasWarnings { get { return Mismatches != null && Mismatches.Length > 0; } }

        internal ModCompatibilityReport(string peerLabel, ModMismatchItem[] mismatches, string warningSummary)
        {
            PeerLabel = peerLabel;
            Mismatches = mismatches;
            WarningSummary = warningSummary;
        }
    }

    public static class ModCompatibilityChecker
    {
        public const int MaxModEntries = 32;

        // Classifies ONLY mods that genuinely desync or interfere with Friends-Only Coop.
        // Client-only visuals, stability adjusters, trainers, UI, audio, and unknown benign scripts
        // return CosmeticLocalOnly so friends never get false-positive warnings.
        public static ModImpactClass ClassifyModPath(string relativeId)
        {
            string lower = ModEntry.SanitizeRelativeId(relativeId).ToLowerInvariant();
            if (lower.Length == 0) return ModImpactClass.CosmeticLocalOnly;

            // 1. Explicitly benign client-side graphics, shaders, stability infra, UI, camera, audio, and SP trainers.
            if (lower.Contains("reshade") || lower.Contains("dxgi.dll") || lower.Contains("d3d11") ||
                lower.Contains("enb") || lower.Contains("visualv") || lower.Contains("timecycle") ||
                lower.Contains("vulkan") || lower.Contains("shadows.asi") || lower.Contains("loadingscreen") ||
                lower.Contains("rudenude") || lower.Contains("tvnude") || lower.Contains("nsfwposter") ||
                lower.Contains("scripthook") || lower.Contains("dinput8") || lower.Contains("openiv") ||
                lower.Contains("heapadjuster") || lower.Contains("packfilelimit") || lower.Contains("nativeui") ||
                lower.Contains("lemonui") || lower.Contains("menyoo") || lower.Contains("trainerv") ||
                lower.Contains("simpletrainer") || lower.Contains("opencamera") || lower.Contains("speedo") ||
                lower.Contains("audio") || lower.Contains("radio") || lower.Contains("music") ||
                lower.Contains("hud") || lower.Contains(".oiv.bak") || lower.EndsWith(".ini", StringComparison.Ordinal))
                return ModImpactClass.CosmeticLocalOnly;

            // 2. Critical shared maps, SK8V runtime, physics, handling, or spawnable asset DLCs (missing/mismatch = crash or void).
            if (lower.Contains("skatev") || lower.Contains("sk8v/tutorial") || lower.Contains("m0-tutorial") ||
                lower.Contains("addons/skate_school") || lower.Contains("dlcpacks/skate_school") ||
                lower.Contains("dlcpacks/skate") || lower.Contains("dlcpacks/addonpeds") ||
                lower.Contains("dlcpacks/gta5korn") || lower.Contains("dlcpacks/predator") ||
                lower.Contains("dlcpacks/fast") || lower.Contains("parkassets") ||
                lower.Contains("handling.meta") || lower.Contains("vehicles.meta") ||
                lower.Contains("gameconfig.xml") || lower.Contains("dlclist.xml") || lower.Contains("update.rpf") ||
                lower.EndsWith(".ymap", StringComparison.Ordinal) || lower.EndsWith(".svs3", StringComparison.Ordinal) ||
                (lower.EndsWith(".rpf", StringComparison.Ordinal) && lower.Contains("dlcpacks/")))
                return ModImpactClass.CriticalMapOrPhysics;

            // 3. World-state, traffic AI, police/wanted, interior, or mission scripts that fight over synced entities.
            if (lower.Contains("traffic") || lower.Contains("betterchases") || lower.Contains("dispatch") ||
                lower.Contains("police") || lower.Contains("wanted") || lower.Contains("interior") ||
                lower.Contains("population") || lower.Contains("pedpersonality") || lower.Contains("vehicleaitasks") ||
                lower.Contains("chaos") || lower.Contains("zombie") || lower.Contains("mission") ||
                lower.Contains("multiplayer") || lower.Contains("fivem") || lower.Contains("ragemp"))
                return ModImpactClass.GameplayInterference;

            // 4. Default fail-open for unrecognized client helpers/scripts so harmless local mods are never flagged.
            return ModImpactClass.CosmeticLocalOnly;
        }

        public static ModEntry[] NormalizeManifest(ModEntry[] entries)
        {
            if (entries == null || entries.Length == 0) return new ModEntry[0];
            var byId = new SortedDictionary<string, ModEntry>(StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < entries.Length && byId.Count < MaxModEntries; i++)
            {
                ModEntry e = entries[i];
                if (e == null || string.IsNullOrEmpty(e.RelativeId)) continue;
                byId[e.RelativeId] = e;
            }
            var result = new ModEntry[byId.Count];
            int idx = 0;
            foreach (ModEntry val in byId.Values) result[idx++] = val;
            return result;
        }

        public static ModCompatibilityReport CompareManifests(
            ModEntry[] localRaw, ModEntry[] remoteRaw, string peerLabel)
        {
            string label = string.IsNullOrEmpty(peerLabel) ? "Friend" : peerLabel.Trim();
            ModEntry[] local = NormalizeManifest(localRaw);
            ModEntry[] remote = NormalizeManifest(remoteRaw);

            var localMap = new Dictionary<string, ModEntry>(StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < local.Length; i++) localMap[local[i].RelativeId] = local[i];

            var remoteMap = new Dictionary<string, ModEntry>(StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < remote.Length; i++) remoteMap[remote[i].RelativeId] = remote[i];

            var diffs = new List<ModMismatchItem>();

            foreach (ModEntry r in remote)
            {
                if (r.Impact == ModImpactClass.CosmeticLocalOnly) continue;
                ModEntry l;
                if (!localMap.TryGetValue(r.RelativeId, out l))
                {
                    diffs.Add(new ModMismatchItem(r.RelativeId, r.Impact, "host/friend has it, you do not"));
                }
                else if (!string.Equals(l.Digest, r.Digest, StringComparison.OrdinalIgnoreCase))
                {
                    diffs.Add(new ModMismatchItem(r.RelativeId, r.Impact, "different version (" + l.Digest + " vs " + r.Digest + ")"));
                }
            }

            foreach (ModEntry l in local)
            {
                if (l.Impact == ModImpactClass.CosmeticLocalOnly) continue;
                if (!remoteMap.ContainsKey(l.RelativeId))
                {
                    diffs.Add(new ModMismatchItem(l.RelativeId, l.Impact, "only installed on your PC"));
                }
            }

            if (diffs.Count == 0)
            {
                return new ModCompatibilityReport(label, new ModMismatchItem[0],
                    "Mods match with " + label + " (no interfering mod differences).");
            }

            diffs.Sort(delegate(ModMismatchItem a, ModMismatchItem b)
            {
                int cmp = ((int)b.Impact).CompareTo((int)a.Impact);
                return cmp != 0 ? cmp : string.Compare(a.RelativeId, b.RelativeId, StringComparison.OrdinalIgnoreCase);
            });

            var parts = new List<string>(Math.Min(diffs.Count, 5));
            for (int i = 0; i < diffs.Count && i < 5; i++)
            {
                string tag = diffs[i].Impact == ModImpactClass.CriticalMapOrPhysics ? "HIGH DESYNC RISK" : "MAY INTERFERE";
                parts.Add(diffs[i].RelativeId + " [" + tag + ": " + diffs[i].Detail + "]");
            }
            string more = diffs.Count > 5 ? " (+" + (diffs.Count - 5) + " more)" : string.Empty;
            string summary = "WARNING vs " + label + " - mod mismatch detected: " +
                string.Join("; ", parts.ToArray()) + more;
            if (summary.Length > 960) summary = summary.Substring(0, 957) + "...";

            return new ModCompatibilityReport(label, diffs.ToArray(), summary);
        }
    }
}
