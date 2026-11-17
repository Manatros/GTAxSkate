// Standalone ScriptHookVDotNet3 GTA.Script entry point for Friends-Only Coop
// (GTAxSkate.Coop.Bootstrap.dll). Kept separate from GTAxSkate.Coop.Client.dll
// so RageCoop server-pushed client resources remain bootstrap-free: this is the ONLY
// GTA.Script type; all logic lives in CoopBootstrapLogic (client DLL, no GTA.Script).
//
// Multiplayer is opt-in (default OFF, persisted in scripts\RageCoop\FriendsCoopPrefs.json).
// While off this script only reads input and draws the small F11 panel on request.
using System;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Windows.Forms;
using GTA;
using GTA.UI;
using GTAxSkate.Coop;
using RageCoop.Client.Scripting;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class CoopBootstrapScript : Script
    {
        private CoopBootstrapLogic logic;
        private bool initialized;

        static CoopBootstrapScript()
        {
            AppDomain.CurrentDomain.AssemblyResolve += OnAssemblyResolve;
        }

        public CoopBootstrapScript()
        {
            Tick += OnGameTick;
            KeyDown += OnGameKeyDown;
            Aborted += OnScriptAborted;
        }

        // Canonical client resource path under the verified GTA root:
        //   <root>\scripts\RageCoop\Resources\GTAxSkate.Coop.Client\GTAxSkate.Coop.Client.dll
        // Never this assembly's own Location: under SHVDN's shadow copy that is a CLR cache directory.
        private static Assembly OnAssemblyResolve(object sender, ResolveEventArgs args)
        {
            try
            {
                string simpleName = new AssemblyName(args.Name).Name;
                if (!string.Equals(simpleName, "GTAxSkate.Coop.Client", StringComparison.OrdinalIgnoreCase))
                    return null;

                Assembly[] loaded = AppDomain.CurrentDomain.GetAssemblies();
                for (int i = 0; i < loaded.Length; i++)
                {
                    if (string.Equals(loaded[i].GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                        return loaded[i];
                }

                string root = CoopInstallPaths.ResolveGameRoot();
                if (string.IsNullOrEmpty(root)) return null;
                string candidate = CoopInstallPaths.ClientResourceDllPath(root);
                if (File.Exists(candidate))
                    return Assembly.LoadFrom(candidate);
            }
            catch { }
            return null;
        }

        private sealed class ScriptHost : ICoopBootstrapHost
        {
            public bool IsWorldReady()
            {
                try
                {
                    Player player = Game.Player;
                    Ped ped = player != null ? Game.LocalPlayerPed : null;
                    return ped != null && ped.Exists();
                }
                catch
                {
                    return false;
                }
            }

            public bool IsWorldAndRageCoopReady()
            {
                try
                {
                    if (!IsWorldReady()) return false;
                    return API.CurrentVersion != null && API.Logger != null;
                }
                catch
                {
                    return false;
                }
            }

            public long NowMs() { return DateTime.UtcNow.Ticks / TimeSpan.TicksPerMillisecond; }

            public ClientAdapter CreateAdapter() { return new ClientAdapter(); }

            public bool IsTransportConnected()
            {
                try { return API.IsOnServer; } catch { return false; }
            }

            public void DisconnectTransport()
            {
                try { API.Disconnect(); } catch { }
            }

            public void LogWarning(string message)
            {
                try
                {
                    var log = API.Logger;
                    if (log != null) log.Warning(message);
                }
                catch { }
            }

            public void Notify(string message)
            {
                // Last line of defence (the view and the logic already sanitize): nothing reaches the screen raw.
                message = SafeText.ForScreen(message);
                if (string.IsNullOrEmpty(message)) return;
                try
                {
                    GTA.Native.Function.Call(GTA.Native.Hash.BEGIN_TEXT_COMMAND_THEFEED_POST, "STRING");
                    GTA.Native.Function.Call(GTA.Native.Hash.ADD_TEXT_COMPONENT_SUBSTRING_PLAYER_NAME, message);
                    GTA.Native.Function.Call(GTA.Native.Hash.END_TEXT_COMMAND_THEFEED_POST_TICKER, false, false);
                }
                catch { }
            }

            public void DrawPanel(CoopPanelView view)
            {
                if (view == null) return;
                ShvdnWorldBridge.BlockGameMenuKeys();
                try
                {
                    float height = 78f + (view.Items.Length * 28f);
                    var panel = new ContainerElement(new PointF(14f, 40f), new SizeF(480f, height), Color.FromArgb(225, 14, 18, 24));
                    panel.Items.Add(new ContainerElement(new PointF(0f, 0f), new SizeF(480f, 3f), Color.FromArgb(245, 242, 184, 48)));
                    panel.Items.Add(new TextElement(SafeText.ForScreen(view.Title), new PointF(12f, 9f), 0.34f, Color.Gold, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    float y = 36f;
                    for (int i = 0; i < view.Items.Length; i++)
                    {
                        if (i == view.Selected)
                            panel.Items.Add(new ContainerElement(new PointF(6f, y - 2f), new SizeF(468f, 25f), Color.FromArgb(135, 42, 118, 210)));
                        panel.Items.Add(new TextElement((i == view.Selected ? "> " : "  ") + SafeText.ForScreen(view.Items[i]),
                            new PointF(12f, y), 0.30f, Color.White, GTA.UI.Font.ChaletLondon, Alignment.Left));
                        y += 28f;
                    }
                    if (view.Status.Length > 0)
                        panel.Items.Add(new TextElement(SafeText.ForScreen(view.Status), new PointF(12f, y + 2f), 0.25f, Color.Orange, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    if (view.Help.Length > 0)
                        panel.Items.Add(new TextElement(SafeText.ForScreen(view.Help), new PointF(12f, y + 24f), 0.22f, Color.LightGray, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    panel.Draw();
                }
                catch { }
            }
        }

        private void EnsureInitialized()
        {
            if (initialized) return;
            initialized = true;
            // Reads the persisted choice once per script domain; missing/invalid file => OFF.
            try
            {
                string root = CoopInstallPaths.ResolveGameRoot();
                CoopRuntimeControl.Initialize(string.IsNullOrEmpty(root) ? null : CoopInstallPaths.PrefsPath(root));
            }
            catch { }
            logic = new CoopBootstrapLogic(new ScriptHost());
        }

        private void OnGameTick(object sender, EventArgs e)
        {
            EnsureInitialized();
            if (logic != null) logic.Tick();
        }

        private void OnGameKeyDown(object sender, KeyEventArgs e)
        {
            if (e == null) return;
            EnsureInitialized();
            if (logic != null) logic.EnqueueKey((int)e.KeyCode);
        }

        private void OnScriptAborted(object sender, EventArgs e)
        {
            if (logic != null) logic.Abort();
        }
    }
}
