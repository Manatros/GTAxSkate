using System;
using System.IO;
using System.Text;
using GTAxSkate.Coop;

// Pure tests for the canonical install-root/server-path helper and the strict prefs reader/writer.
// Compiled together with shared/*.cs (CoopInstallPaths is internal, so the test lives in the same assembly).
// Every fixture lives under the workspace build folder (or the OS temp dir); the real GTA folder is never touched.
internal static class CoopOptInTests
{
    private static int checks;
    private static void Check(bool value, string text) { checks++; if (!value) throw new Exception("FAILED: " + text); }

    public static int Main()
    {
        string tmpRoot = Environment.GetEnvironmentVariable("GTAXSKATE_COOP_TEST_TMP");
        if (string.IsNullOrEmpty(tmpRoot)) tmpRoot = Path.Combine(Path.GetTempPath(), "gtaxskate-coop-optin-tests");
        string dir = Path.Combine(tmpRoot, Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        try
        {
            PathTests(dir);
            PrefsParseTests();
            PrefsFileTests(dir);
            Console.WriteLine("PASS: " + checks + " opt-in path/preference checks; no game, server or network touched.");
            return 0;
        }
        catch (Exception ex) { Console.Error.WriteLine(ex); return 1; }
        finally { try { Directory.Delete(dir, true); } catch { } }
    }

    private static string FakeGta(string parent, string name)
    {
        string root = Path.Combine(parent, name);
        Directory.CreateDirectory(Path.Combine(root, "scripts"));
        File.WriteAllText(Path.Combine(root, "GTA5.exe"), "x");
        return root;
    }

    private static void PathTests(string dir)
    {
        string root = FakeGta(dir, "GTA V Legacy é");   // spaces + non-ASCII
        string other = FakeGta(dir, "Other GTA");
        string shadow = Path.Combine(dir, "clr-cache", "Assembly", "dl3", "ABC");
        Directory.CreateDirectory(shadow);
        string exe = Path.Combine(root, "GTA5.exe");

        Check(CoopInstallPaths.ResolveGameRoot(exe, shadow) == root, "process module wins over an arbitrary shadow-copy base");
        Check(CoopInstallPaths.ResolveGameRoot(exe, Path.Combine(other, "scripts")) == root, "process root is authoritative: never falls through to another install");
        Check(CoopInstallPaths.ResolveGameRoot(null, root) == root, "unavailable module metadata: base at the GTA root");
        Check(CoopInstallPaths.ResolveGameRoot("", root + Path.DirectorySeparatorChar) == root, "trailing separator is normalised");
        Check(CoopInstallPaths.ResolveGameRoot(null, Path.Combine(root, "scripts")) == root, "base at <GTA>\\scripts strips exactly one scripts segment");
        Check(CoopInstallPaths.ResolveGameRoot(null, Path.Combine(root, "scripts") + Path.DirectorySeparatorChar) == root, "scripts with trailing separator");
        Check(CoopInstallPaths.ResolveGameRoot(null, Path.Combine(root, "SCRIPTS")) == root, "scripts segment is case-insensitive");
        Check(CoopInstallPaths.ResolveGameRoot(Path.Combine(shadow, "notgta.exe"), Path.Combine(root, "scripts")) == root, "process dir without GTA5.exe falls back to the normalised base");
        Check(CoopInstallPaths.ResolveGameRoot(dir + "\\bad|name\\GTA5.exe", Path.Combine(root, "scripts")) == root, "invalid module path (exception) falls back to the base");
        Check(CoopInstallPaths.ResolveGameRoot(null, shadow) == null, "shadow-copy cache is never accepted as a root");
        Check(CoopInstallPaths.ResolveGameRoot(null, Path.Combine(Path.Combine(root, "scripts"), "RageCoop")) == null, "only a trailing scripts segment is stripped");
        Check(CoopInstallPaths.ResolveGameRoot(null, null) == null && CoopInstallPaths.ResolveGameRoot("", "") == null, "no input -> unresolved");

        string oldCwd = Directory.GetCurrentDirectory();
        try
        {
            Directory.SetCurrentDirectory(other);
            Check(CoopInstallPaths.ResolveGameRoot(null, shadow) == null, "working directory is never an authority");
            Check(CoopInstallPaths.ResolveGameRoot(exe, shadow) == root, "changed working directory does not matter");
        }
        finally { Directory.SetCurrentDirectory(oldCwd); }

        Check(CoopInstallPaths.HostServerExePath(root) == Path.Combine(root, "scripts", "RageCoop", "HostServer", "RageCoop.Server.exe"),
            "canonical server path");
        Check(CoopInstallPaths.ClientResourceDllPath(root) == Path.Combine(root, "scripts", "RageCoop", "Resources", "GTAxSkate.Coop.Client", "GTAxSkate.Coop.Client.dll"),
            "canonical bootstrap client-resource path");
        Check(CoopInstallPaths.PrefsPath(root) == Path.Combine(root, "scripts", "RageCoop", "FriendsCoopPrefs.json"), "prefs path sits next to the client config");
        Check(CoopInstallPaths.ClientConfigPath(root) == Path.Combine(root, "scripts", "RageCoop", "FriendsCoopClient.json"), "config path");

        // Installed-data regression: SHVDN's base is <GTA>\scripts. The old fallback appended scripts\RageCoop\HostServer
        // again (scripts\scripts\...) and missed; the canonical path finds the real installed executable.
        string serverDir = Path.Combine(root, "scripts", "RageCoop", "HostServer");
        Directory.CreateDirectory(serverDir);
        string scriptsBase = Path.Combine(root, "scripts");
        string oldFormula = Path.Combine(scriptsBase, "scripts\\RageCoop\\HostServer");
        Check(!Directory.Exists(oldFormula), "old formula misses on scripts-base (scripts\\scripts\\...)");
        Check(!File.Exists(CoopInstallPaths.HostServerExePath(root)), "existing but empty HostServer is not a server (executable validated, not the directory)");
        File.WriteAllText(CoopInstallPaths.HostServerExePath(root), "exe");
        string resolved = CoopInstallPaths.ResolveGameRoot(null, scriptsBase);
        Check(resolved != null && File.Exists(CoopInstallPaths.HostServerExePath(resolved)), "canonical path finds the installed server from the scripts base");

        string exePath = CoopInstallPaths.HostServerExePath(root);
        Check(CoopInstallPaths.SamePath(exePath, exePath.ToUpperInvariant()), "orphan comparison is case-insensitive");
        Check(CoopInstallPaths.SamePath(exePath, Path.Combine(root, "scripts", "..", "scripts", "RageCoop", "HostServer", "RageCoop.Server.exe")), "orphan comparison normalises dot segments");
        Check(!CoopInstallPaths.SamePath(exePath, Path.Combine(other, "scripts", "RageCoop", "HostServer", "RageCoop.Server.exe")), "a different install's server is not 'ours'");
        Check(!CoopInstallPaths.SamePath(exePath, null) && !CoopInstallPaths.SamePath("", exePath), "empty path never matches");
    }

    private static void PrefsParseTests()
    {
        Check(CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": true}"), "exact true => on");
        Check(CoopPrefs.ParseEnabled("  {\r\n \"MultiplayerEnabled\" : true ,\r\n \"Other\": 5 }\r\n"), "whitespace and extra scalar keys tolerated");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": false}"), "false => off");
        Check(!CoopPrefs.ParseEnabled("{}"), "missing key => off");
        Check(!CoopPrefs.ParseEnabled(""), "empty => off");
        Check(!CoopPrefs.ParseEnabled(null), "null => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": \"true\"}"), "quoted true => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": 1}"), "numeric => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": null}"), "null value => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": trueish}"), "prefix match => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": True}"), "wrong-case literal => off");
        Check(!CoopPrefs.ParseEnabled("{\"multiplayerenabled\": true}"), "key is case-sensitive");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": true, \"MultiplayerEnabled\": true}"), "duplicate key (true,true) => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": false, \"MultiplayerEnabled\": true}"), "duplicate key (false,true) => off");
        Check(!CoopPrefs.ParseEnabled("{\"a\": {\"MultiplayerEnabled\": true}}"), "nested object => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": [true]}"), "array => off");
        Check(!CoopPrefs.ParseEnabled("[{\"MultiplayerEnabled\": true}]"), "top-level array => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": true"), "truncated => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": true} trailing"), "trailing junk => off");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\": true,}"), "trailing comma => off");
        foreach (string badNum in new[] { "1e", "1e+", "1.", ".5", "-", "+1", "01", "00", "1.e5", "-.5", "1e5e5", "1.2.3", "0x10", "--1", "1-", "1e-", "NaN", "Infinity", "-Infinity", "1+2", "1e5." })
            Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\":true,\"other\":" + badNum + "}"), "malformed number '" + badNum + "' => off");
        foreach (string goodNum in new[] { "0", "-0", "7", "-12", "0.5", "10", "1e5", "1E+5", "2.5e-3", "-1.5E10" })
            Check(CoopPrefs.ParseEnabled("{\"other\":" + goodNum + ",\"MultiplayerEnabled\":true}"), "valid JSON number '" + goodNum + "' tolerated");
        Check(!CoopPrefs.ParseEnabled("{\"MultiplayerEnabled\" true}"), "missing colon => off");
        Check(!CoopPrefs.ParseEnabled("{'MultiplayerEnabled': true}"), "single quotes => off");
        string big = "{\"MultiplayerEnabled\": true, \"pad\": \"" + new string('x', 17000) + "\"}";
        Check(!CoopPrefs.ParseEnabled(big), "over 16 KB => off");

        // Credentials JSON is never consent: a legacy/credentials file parsed as prefs stays off.
        string legacy = "{\"Schema\":1,\"BlockPublicServerBrowser\":true,\"RequireMutualHmacProof\":true,\"ShowSessionStatusAlways\":true," +
            "\"TrustedHostLabel\":\"VPN-Host\",\"TrustedHostIp\":\"100.88.12.34\",\"Port\":4499,\"FriendKey\":\"SkateFriends-Secret-99\"}";
        Check(!CoopPrefs.ParseEnabled(legacy), "legacy credentials JSON with ShowSessionStatusAlways=true and a valid endpoint/key is OFF");

        // A multiplayer key inside the credentials file must not bypass connection validation either.
        ClientConnectRequest request; string error;
        Check(!ClientConfigLoader.TryParseClientConfigJson("{\"MultiplayerEnabled\": true}", out request, out error) && request == null,
            "consent is not a connection: no endpoint/key still means no valid request");
        Check(!ClientConfigLoader.TryParseClientConfigJson("{\"MultiplayerEnabled\": true,\"BlockPublicServerBrowser\":false,\"TrustedHostIp\":\"100.88.12.34\"}", out request, out error),
            "config true never bypasses the public-browser guard");
    }

    private static void PrefsFileTests(string dir)
    {
        string prefs = Path.Combine(dir, "scripts", "RageCoop", "FriendsCoopPrefs.json");
        Check(!CoopPrefs.Read(prefs), "missing file => off");
        string error;
        Check(CoopPrefs.Write(prefs, true, out error) && CoopPrefs.Read(prefs), "write creates directories and round-trips true");
        Check(!File.Exists(prefs + ".tmp"), "no temp file left behind");
        Check(CoopPrefs.Write(prefs, false, out error) && !CoopPrefs.Read(prefs), "atomic replace of an existing file round-trips false");
        Check(CoopPrefs.ParseEnabled(File.ReadAllText(prefs).Replace("false", "true")), "file content is plain JSON");

        File.WriteAllBytes(prefs, new byte[] { 0xEF, 0xBB, 0xBF }.Concat("{\"MultiplayerEnabled\": true}"));
        Check(CoopPrefs.Read(prefs), "UTF-8 BOM is tolerated");
        File.WriteAllText(prefs, "{\"MultiplayerEnabled\": tru");
        Check(!CoopPrefs.Read(prefs), "malformed file => off, silently");
        File.WriteAllText(prefs, "{\"MultiplayerEnabled\": true, \"pad\": \"" + new string('x', 17000) + "\"}");
        Check(!CoopPrefs.Read(prefs), "oversized file => off");

        // Failure to save must report, not throw.
        string blocker = Path.Combine(dir, "blocker");
        File.WriteAllText(blocker, "file");
        Check(!CoopPrefs.Write(Path.Combine(blocker, "sub", "p.json"), true, out error) && error.Length > 0, "unwritable location reports an error");
        Check(!CoopPrefs.Write(null, true, out error) && error.Length > 0, "unknown path reports an error");
    }

    private static byte[] Concat(this byte[] head, string text)
    {
        byte[] body = new UTF8Encoding(false).GetBytes(text);
        byte[] all = new byte[head.Length + body.Length];
        Buffer.BlockCopy(head, 0, all, 0, head.Length);
        Buffer.BlockCopy(body, 0, all, head.Length, body.Length);
        return all;
    }
}
