using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

// Metadata-only check of the BUILT coop DLLs (reflection-only; nothing is executed).
// Proves the B152 split and the single controller identity:
//   * GTAxSkate.Coop.Client.dll defines CoopRuntimeControl exactly once and zero GTA.Script types;
//   * GTAxSkate.Coop.Bootstrap.dll defines the only GTA.Script type, defines NO copy of the controller
//     (or of the adapter/logic/prefs types) and references GTAxSkate.Coop.Client;
//   * CoopInstallPaths is internal in both (a private copy per assembly, never an exported duplicate).
// usage: BuildIdentityCheck.exe <client.dll> <bootstrap.dll> <referenceDir>...
internal static class BuildIdentityCheck
{
    private static int checks;
    private static void Check(bool value, string text) { checks++; if (!value) throw new Exception("FAILED: " + text); }

    private static string[] refDirs;

    public static int Main(string[] args)
    {
        try
        {
            if (args.Length < 3) throw new Exception("usage: <client.dll> <bootstrap.dll> <referenceDir>...");
            refDirs = new string[args.Length - 2];
            Array.Copy(args, 2, refDirs, 0, refDirs.Length);
            AppDomain.CurrentDomain.ReflectionOnlyAssemblyResolve += Resolve;

            Assembly client = Assembly.ReflectionOnlyLoadFrom(args[0]);
            Assembly boot = Assembly.ReflectionOnlyLoadFrom(args[1]);
            Type[] clientTypes = Types(client), bootTypes = Types(boot);

            Check(Count(clientTypes, "GTAxSkate.Coop.RageCoopAdapter.CoopRuntimeControl") == 1, "client DLL defines CoopRuntimeControl exactly once");
            Check(Count(bootTypes, "GTAxSkate.Coop.RageCoopAdapter.CoopRuntimeControl") == 0, "bootstrap DLL defines no copy of CoopRuntimeControl");
            foreach (string name in new[] { "ClientAdapter", "CoopBootstrapLogic", "ShvdnWorldBridge", "CoopPanelView" })
                Check(Count(bootTypes, "GTAxSkate.Coop.RageCoopAdapter." + name) == 0, "bootstrap DLL defines no copy of " + name);
            foreach (string name in new[] { "CoopPrefs", "ClientConfigLoader", "CoopSecurityPolicy" })
                Check(Count(bootTypes, "GTAxSkate.Coop." + name) == 0, "bootstrap DLL defines no copy of " + name);
            Check(Find(clientTypes, "GTAxSkate.Coop.RageCoopAdapter.CoopRuntimeControl").IsPublic, "controller is public in the client DLL");

            int clientScripts = 0, bootScripts = 0;
            string bootScriptName = null;
            foreach (Type t in clientTypes) if (DerivesFrom(t, "GTA.Script")) clientScripts++;
            foreach (Type t in bootTypes) if (DerivesFrom(t, "GTA.Script")) { bootScripts++; bootScriptName = t.FullName; }
            Check(clientScripts == 0, "client DLL has zero GTA.Script types (B152)");
            Check(bootScripts == 1 && bootScriptName == "GTAxSkate.Coop.RageCoopAdapter.CoopBootstrapScript", "bootstrap DLL has exactly one GTA.Script type: CoopBootstrapScript");

            bool refsClient = false;
            foreach (AssemblyName r in boot.GetReferencedAssemblies())
                if (string.Equals(r.Name, "GTAxSkate.Coop.Client", StringComparison.OrdinalIgnoreCase)) refsClient = true;
            Check(refsClient, "bootstrap DLL references GTAxSkate.Coop.Client (one controller via the normal assembly reference)");

            Type bootPaths = Find(bootTypes, "GTAxSkate.Coop.CoopInstallPaths");
            Type clientPaths = Find(clientTypes, "GTAxSkate.Coop.CoopInstallPaths");
            Check(bootPaths != null && !bootPaths.IsPublic && clientPaths != null && !clientPaths.IsPublic,
                "CoopInstallPaths is a private (internal) copy in each DLL");

            Console.WriteLine("PASS: " + checks + " built-DLL identity checks (metadata only).");
            return 0;
        }
        catch (Exception ex) { Console.Error.WriteLine(ex); return 1; }
    }

    private static Type[] Types(Assembly a)
    {
        try { return a.GetTypes(); }
        catch (ReflectionTypeLoadException ex)
        {
            var good = new List<Type>();
            foreach (Type t in ex.Types) if (t != null) good.Add(t);
            if (ex.LoaderExceptions.Length > 0) throw new Exception("Could not load all types of " + a.GetName().Name + ": " + ex.LoaderExceptions[0].Message);
            return good.ToArray();
        }
    }

    private static int Count(Type[] types, string fullName)
    {
        int n = 0;
        foreach (Type t in types) if (t.FullName == fullName) n++;
        return n;
    }

    private static Type Find(Type[] types, string fullName)
    {
        foreach (Type t in types) if (t.FullName == fullName) return t;
        return null;
    }

    private static bool DerivesFrom(Type t, string baseFullName)
    {
        for (Type b = t.BaseType; b != null; b = b.BaseType)
            if (b.FullName == baseFullName) return true;
        return false;
    }

    private static Assembly Resolve(object sender, ResolveEventArgs e)
    {
        string simple = new AssemblyName(e.Name).Name;
        foreach (string dir in refDirs)
        {
            string candidate = Path.Combine(dir, simple + ".dll");
            if (File.Exists(candidate)) return Assembly.ReflectionOnlyLoadFrom(candidate);
        }
        try { return Assembly.ReflectionOnlyLoad(e.Name); } catch { }
        string fx = Path.Combine(Environment.GetEnvironmentVariable("WINDIR") ?? "C:\\Windows", "Microsoft.NET\\Framework64\\v4.0.30319");
        string fxCandidate = Path.Combine(fx, simple + ".dll");
        return File.Exists(fxCandidate) ? Assembly.ReflectionOnlyLoadFrom(fxCandidate) : null;
    }
}
