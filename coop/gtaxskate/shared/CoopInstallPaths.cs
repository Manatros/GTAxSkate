// Canonical GTA V install-root and coop file layout, independent of where SHVDN
// shadow-copies our assemblies. Pure string logic (plus one thin production
// wrapper); never launches processes or writes files.
// INTERNAL on purpose: this file is compiled into both GTAxSkate.Coop.Client.dll
// and GTAxSkate.Coop.Bootstrap.dll (the bootstrap needs it before the client
// DLL can be resolved), so each assembly keeps a private copy.
using System;
using System.Diagnostics;
using System.IO;

namespace GTAxSkate.Coop
{
    internal static class CoopInstallPaths
    {
        internal const string GameExe = "GTA5.exe";

        // Pure resolver. processExePath: main module of the host process (may be null);
        // appDomainBase: AppDomain.CurrentDomain.BaseDirectory (may be null).
        internal static string ResolveGameRoot(string processExePath, string appDomainBase)
        {
            try
            {
                if (!string.IsNullOrEmpty(processExePath))
                {
                    string dir = Path.GetDirectoryName(Path.GetFullPath(processExePath));
                    if (!string.IsNullOrEmpty(dir) && File.Exists(Path.Combine(dir, GameExe)))
                        return TrimEnd(dir);
                }
            }
            catch { }
            try
            {
                if (!string.IsNullOrEmpty(appDomainBase))
                {
                    string baseDir = TrimEnd(Path.GetFullPath(appDomainBase));
                    if (File.Exists(Path.Combine(baseDir, GameExe)))
                        return baseDir;
                    // SHVDN uses <GTA>\scripts as ApplicationBase: strip exactly one trailing "scripts".
                    string name = Path.GetFileName(baseDir);
                    if (string.Equals(name, "scripts", StringComparison.OrdinalIgnoreCase))
                    {
                        string parent = Path.GetDirectoryName(baseDir);
                        if (!string.IsNullOrEmpty(parent) && File.Exists(Path.Combine(parent, GameExe)))
                            return TrimEnd(parent);
                    }
                }
            }
            catch { }
            return null;
        }

        // Production wrapper. A process-module query failure falls back to the AppDomain base.
        internal static string ResolveGameRoot()
        {
            string processExe = null;
            try
            {
                using (Process p = Process.GetCurrentProcess())
                    processExe = p.MainModule != null ? p.MainModule.FileName : null;
            }
            catch { }
            string baseDir = null;
            try { baseDir = AppDomain.CurrentDomain.BaseDirectory; } catch { }
            return ResolveGameRoot(processExe, baseDir);
        }

        internal static string RageCoopDir(string root)
        {
            return Path.Combine(Path.Combine(root, "scripts"), "RageCoop");
        }

        internal static string ClientConfigPath(string root)
        {
            return Path.Combine(RageCoopDir(root), "FriendsCoopClient.json");
        }

        internal static string PrefsPath(string root)
        {
            return Path.Combine(RageCoopDir(root), "FriendsCoopPrefs.json");
        }

        internal static string ClientResourceDllPath(string root)
        {
            return Path.Combine(Path.Combine(Path.Combine(RageCoopDir(root), "Resources"), "GTAxSkate.Coop.Client"), "GTAxSkate.Coop.Client.dll");
        }

        internal static string HostServerDir(string root)
        {
            return Path.Combine(RageCoopDir(root), "HostServer");
        }

        internal static string HostServerExePath(string root)
        {
            return Path.Combine(HostServerDir(root), "RageCoop.Server.exe");
        }

        // Case-insensitive, normalised path equality for the orphan-server comparison.
        internal static bool SamePath(string a, string b)
        {
            if (string.IsNullOrEmpty(a) || string.IsNullOrEmpty(b)) return false;
            try
            {
                return string.Equals(TrimEnd(Path.GetFullPath(a)), TrimEnd(Path.GetFullPath(b)), StringComparison.OrdinalIgnoreCase);
            }
            catch
            {
                return false;
            }
        }

        private static string TrimEnd(string path)
        {
            string root = Path.GetPathRoot(path);
            while (path.Length > (root == null ? 0 : root.Length) &&
                   (path[path.Length - 1] == Path.DirectorySeparatorChar || path[path.Length - 1] == Path.AltDirectorySeparatorChar))
                path = path.Substring(0, path.Length - 1);
            return path;
        }
    }
}
