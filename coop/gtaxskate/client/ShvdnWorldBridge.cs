// Live GTA V Legacy (SHVDN 3.7) & RAGECOOP 1.5 bridge for Host-Authoritative Coop.
// Compiled ONLY into GTAxSkate.Coop.Client.dll; offline unit tests use IGtaWorldBridge mocks.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Net;
using System.Net.NetworkInformation;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Windows.Forms;
using GTA;
using GTA.Chrono;
using GTA.Math;
using GTA.Native;
using GTA.UI;
using RageCoop.Client.Scripting;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class ShvdnWorldBridge : IGtaWorldBridge, IDisposable
    {
        private readonly ClientAdapter owner;
        private Blip hostObjectiveBlip;
        private bool appliedPeerStandby;
        private readonly Dictionary<int, bool> lastRemoteRagdollState = new Dictionary<int, bool>();
        private string lastShopNotice = string.Empty;
        // The local server's handle and its lock live in AppDomain data, not in copy-local statics: when a
        // second loaded copy of this assembly displaces the first, the new owner must still be able to stop
        // the server the first copy started (Codex re-check 2026-11-16 of 8e79400). Process is a BCL type,
        // so both copies see one type identity.
        private const string HostServerSlot = "GTAxSkate.Coop.HostServerProcess.v1";
        private const string HostServerGateSlot = "GTAxSkate.Coop.HostServerGate.v1";
        private static object hostServerGate
        {
            get
            {
                lock (AppDomain.CurrentDomain)
                {
                    object gate = AppDomain.CurrentDomain.GetData(HostServerGateSlot);
                    if (gate == null)
                    {
                        gate = new object();
                        AppDomain.CurrentDomain.SetData(HostServerGateSlot, gate);
                    }
                    return gate;
                }
            }
        }
        // SHA-256 of the friend key the running server was started with (string, BCL type); see LocalServerKey.
        private const string HostServerKeySlot = "GTAxSkate.Coop.HostServerKey.v1";
        private static Process hostServerProcess
        {
            get { return AppDomain.CurrentDomain.GetData(HostServerSlot) as Process; }
            set
            {
                AppDomain.CurrentDomain.SetData(HostServerSlot, value);
                if (value == null) AppDomain.CurrentDomain.SetData(HostServerKeySlot, null);
            }
        }

        // Internet lobby. The UPnP service is per bridge, but what a successor copy needs to remove our mapping
        // (port + description) is kept in AppDomain data as BCL types only, like the server handle above.
        private const string OnlineMappingSlot = "GTAxSkate.Coop.OnlineMapping.v1";
        private UpnpHostPortService upnp;
        // Streamer-safe coop: clipboard worker (invites travel only through the clipboard) and the RageCoop UI lock.
        private ClipboardWorker clipboard;
        private const string UiBackupSlot = "GTAxSkate.Coop.RageCoopUiBackup.v1";   // string[4] of the user's own values (BCL type only)

        public ShvdnWorldBridge(ClientAdapter adapter)
        {
            owner = adapter;
            // Adapter start: a router removal that failed in an earlier run gets another chance (own description only).
            try { UpnpHostPortService.RetryPendingAsync(delegate() { return new NatUpnpGateway(); }, 0); } catch { }
        }

        public void Dispose()
        {
            // Game exit / disable: give the router a moment to drop our mapping (only when one exists).
            ReleaseOnlineMapping(1000);
            StopLocalHostServerIfStarted();
            // A queued "clear the clipboard if it still holds our invite" gets a moment to run.
            ClipboardWorker worker = clipboard;
            if (worker != null) worker.Shutdown(1000);
        }

        public bool IsGameLoading()
        {
            try
            {
                Player player = Game.Player;
                if (player == null) return true;
                Ped ped = Game.LocalPlayerPed;
                return ped == null || !ped.Exists();
            }
            catch
            {
                return true;
            }
        }

        public HostWorldSyncState CaptureHostWorldState(int epoch)
        {
            if (IsGameLoading()) return null;
            try
            {
                Ped ped = Game.LocalPlayerPed;
                Vector3 pos = ped != null && ped.Exists() ? ped.Position : Vector3.Zero;
                bool inVeh = ped != null && ped.Exists() && ped.IsInVehicle();
                var tod = GameClock.TimeOfDay;
                int weather = Math.Max(0, Math.Min(HostWorldSyncState.MaxWeatherId, (int)World.Weather));
                bool mission = Game.IsMissionActive;
                bool cutscene = Game.IsCutsceneActive;
                int wanted = Math.Max(0, Math.Min(HostWorldSyncState.MaxWantedLevel, Game.Player.Wanted.WantedLevel));
                Vector3 wc = Game.Player.Wanted.LastPositionSpottedByPolice;

                bool hasObj = false;
                Vector3 objPos = Vector3.Zero;
                int objSprite = 1;
                int objColor = 5; // Yellow mission marker
                Blip[] allBlips = World.GetAllBlips();
                if (allBlips != null)
                {
                    for (int i = 0; i < allBlips.Length; i++)
                    {
                        Blip b = allBlips[i];
                        if (b == null || !b.Exists()) continue;
                        if (b.ShowRoute || b.Color == BlipColor.Yellow || b.Color == BlipColor.Green)
                        {
                            hasObj = true;
                            objPos = b.Position;
                            objSprite = Math.Max(0, Math.Min(2048, (int)b.Sprite));
                            objColor = Math.Max(0, Math.Min(255, (int)b.Color));
                            break;
                        }
                    }
                }

                return new HostWorldSyncState(
                    epoch, tod.Hour, tod.Minute, GameClock.IsPaused, weather,
                    mission, cutscene, inVeh, wanted,
                    ClampCoord(wc.X), ClampCoord(wc.Y), ClampCoord(wc.Z),
                    ClampCoord(pos.X), ClampCoord(pos.Y), ClampCoord(pos.Z),
                    hasObj, ClampCoord(objPos.X), ClampCoord(objPos.Y), ClampCoord(objPos.Z),
                    objSprite, objColor, !cutscene);
            }
            catch
            {
                return null;
            }
        }

        public LocalRagdollSample CaptureLocalRagdoll()
        {
            if (IsGameLoading()) return null;
            try
            {
                Ped ped = Game.LocalPlayerPed;
                if (ped == null || !ped.Exists()) return null;
                Vector3 p = ped.Position;
                Vector3 v = ped.Velocity;
                return new LocalRagdollSample(
                    ped.IsRagdoll || ped.IsRunningRagdollTask,
                    ClampCoord(p.X), ClampCoord(p.Y), ClampCoord(p.Z),
                    ClampSpeed(v.X), ClampSpeed(v.Y), ClampSpeed(v.Z));
            }
            catch
            {
                return null;
            }
        }

        public void ApplyPeerWorldState(HostWorldSyncState state)
        {
            if (state == null) return;
            try
            {
                // Suppress local ambient spawns safely via per-frame multipliers (never SET_VEHICLE_POPULATION_BUDGET(0)).
                Function.Call(Hash.SET_VEHICLE_DENSITY_MULTIPLIER_THIS_FRAME, 0.0f);
                Function.Call(Hash.SET_RANDOM_VEHICLE_DENSITY_MULTIPLIER_THIS_FRAME, 0.0f);
                Function.Call(Hash.SET_PED_DENSITY_MULTIPLIER_THIS_FRAME, 0.0f);
                Function.Call(Hash.SET_SCENARIO_PED_DENSITY_MULTIPLIER_THIS_FRAME, 0.0f, 0.0f);

                GameClock.TimeOfDay = GameClockTime.FromHms(state.ClockHour, state.ClockMinute, 0);
                GameClock.IsPaused = state.ClockPaused;
                if (Enum.IsDefined(typeof(Weather), state.WeatherId))
                    World.Weather = (Weather)state.WeatherId;

                if (Game.Player.Wanted.WantedLevel != state.WantedLevel)
                {
                    Game.Player.Wanted.SetWantedLevel(state.WantedLevel, false);
                    Game.Player.Wanted.ApplyWantedLevelChangeNow(false);
                }
                if (state.WantedLevel > 0)
                    Function.Call(Hash.SET_PLAYER_WANTED_CENTRE_POSITION, Game.Player.Handle, state.WantedCoordX, state.WantedCoordY, state.WantedCoordZ, false, false);

                Ped local = Game.LocalPlayerPed;
                if (state.CutsceneActive)
                {
                    appliedPeerStandby = true;
                    Game.Player.SetControlState(false, SetPlayerControlFlags.None);
                    if (local != null && local.Exists()) local.IsInvincible = true;
                }
                else if (appliedPeerStandby)
                {
                    ExitPeerCutsceneStandbyIfNeeded(state, true);
                }

                UpdateObjectiveBlip(state);
            }
            catch { }
        }

        public void ApplyPeerShopState(HostShopSyncState state)
        {
            if (state == null) return;
            try
            {
                if (!string.IsNullOrEmpty(state.StoreSyncNotice) && state.StoreSyncNotice != lastShopNotice)
                {
                    lastShopNotice = state.StoreSyncNotice;
                    Notify("~b~Host Shop:~w~ " + state.StoreSyncNotice);
                }
            }
            catch { }
        }

        public void ApplyRemotePlayerRagdoll(int sourceSlot, string peerUsername, RagdollSyncState state)
        {
            if (state == null) return;
            try
            {
                bool prev = false;
                lastRemoteRagdollState.TryGetValue(sourceSlot, out prev);
                lastRemoteRagdollState[sourceSlot] = state.IsRagdoll;

                // Apply ragdoll fall impulse ONLY on ragdoll entry transition to avoid fighting EntityPool.DoSync interpolation.
                if (state.IsRagdoll == prev) return;
                if (API.Players == null) return;

                foreach (var kv in API.Players)
                {
                    RageCoop.Client.Player rp = kv.Value;
                    if (rp == null || rp.Character == null) continue;
                    Ped ped = rp.Character.MainPed;
                    if (ped == null || !ped.Exists()) continue;

                    Vector3 d = ped.Position - new Vector3(state.PosX, state.PosY, state.PosZ);
                    if (d.Length() > 12.0f) continue;

                    if (state.IsRagdoll)
                    {
                        ped.CanRagdoll = true;
                        ped.SetToRagdollWithFall(
                            Math.Max(300, state.DurationMs),
                            Math.Max(300, state.DurationMs),
                            RagdollFallType.Male,
                            new Vector3(state.VelX, state.VelY, state.VelZ),
                            1.0f);
                    }
                    else
                    {
                        ped.CancelRagdoll();
                    }
                    break;
                }
            }
            catch { }
        }

        public void ExitPeerCutsceneStandbyIfNeeded(HostWorldSyncState lastWorldState, bool allowGroundRegroup)
        {
            try
            {
                appliedPeerStandby = false;
                Game.Player.SetControlState(true, SetPlayerControlFlags.None);
                Ped local = Game.LocalPlayerPed;
                if (local == null || !local.Exists()) return;
                local.IsInvincible = false;

                if (allowGroundRegroup && lastWorldState != null && !lastWorldState.CutsceneActive)
                {
                    Vector3 hostPos = new Vector3(lastWorldState.HostPosX, lastWorldState.HostPosY, lastWorldState.HostPosZ);
                    if (hostPos.Length() > 1.0f && (local.Position - hostPos).Length() > 35.0f)
                    {
                        Vector3 candidate = new Vector3(hostPos.X + 2.0f, hostPos.Y + 2.0f, hostPos.Z);
                        var groundOut = new OutputArgument();
                        if (Function.Call<bool>(Hash.GET_GROUND_Z_FOR_3D_COORD, candidate.X, candidate.Y, candidate.Z + 15.0f, groundOut, false))
                        {
                            float gz = groundOut.GetResult<float>();
                            local.Position = new Vector3(candidate.X, candidate.Y, gz + 0.35f);
                        }
                    }
                }
            }
            catch { }
        }

        public void RestoreSoloDefaults()
        {
            try
            {
                lastRemoteRagdollState.Clear();
                lastShopNotice = string.Empty;
                if (appliedPeerStandby) ExitPeerCutsceneStandbyIfNeeded(null, false);
                if (hostObjectiveBlip != null && hostObjectiveBlip.Exists())
                {
                    hostObjectiveBlip.Delete();
                    hostObjectiveBlip = null;
                }
            }
            catch { }
        }

        public bool ConnectTransport(string endpointAddress, int port, string friendKey)
        {
            try
            {
                string address = endpointAddress + ":" + port;
                // RAGECOOP 1.5's public API.Connect(address) sends Settings.Password (usually empty);
                // our server admits only SHA-256(friendKey) (ServerAdapter.AdmitPrivateTransport), so
                // the key goes in through the internal ToggleConnection(address, username, password,
                // publicKey). RageCoop.Client.dll is a hash-pinned stock binary (build.ps1), so the
                // signature is fixed. RageCoop then asks once "Server's IP can be spoofed..." (Enter).
                MethodInfo toggle = null;
                if (!string.IsNullOrEmpty(friendKey))
                {
                    Type networking = typeof(API).Assembly.GetType("RageCoop.Client.Networking", false);
                    if (networking != null)
                        foreach (MethodInfo m in networking.GetMethods(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
                        {
                            ParameterInfo[] p = m.GetParameters();
                            if (m.Name == "ToggleConnection" && p.Length == 4 && p[0].ParameterType == typeof(string) &&
                                p[1].ParameterType == typeof(string) && p[2].ParameterType == typeof(string))
                            { toggle = m; break; }
                        }
                }
                if (toggle == null)
                {
                    if (!string.IsNullOrEmpty(friendKey))
                        try { if (API.Logger != null) API.Logger.Error("GTAxSkate Coop: RageCoop ToggleConnection not found; connecting without the friend key (the server will refuse)."); } catch { }
                    API.Connect(address);
                    return true;
                }
                // ToggleConnection is a toggle: while connected or connecting it disconnects instead
                // (API.Connect refuses those states; keep that guard).
                bool connecting = false;
                const BindingFlags any = BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;
                PropertyInfo cp = toggle.DeclaringType.GetProperty("IsConnecting", any);
                FieldInfo cf = cp == null ? toggle.DeclaringType.GetField("IsConnecting", any) : null;
                object cv = cp != null ? cp.GetValue(null, null) : (cf != null ? cf.GetValue(null) : null);
                if (cv is bool) connecting = (bool)cv;
                if (API.IsOnServer || connecting)
                    throw new InvalidOperationException("Cannot connect to server when another connection is active");
                // The neutral name is passed explicitly as well: name tags, the player list and join notices never carry a Windows or account name.
                toggle.Invoke(null, new object[] { address, RageCoopClientSettings.LockedUsername, friendKey, null });
                // Accepted: IsConnecting stays true while RageCoop's connect task runs. Cancelled
                // warning: ToggleConnection resets IsConnecting and returns.
                object after = cp != null ? cp.GetValue(null, null) : (cf != null ? cf.GetValue(null) : null);
                return API.IsOnServer || !(after is bool) || (bool)after;
            }
            catch (TargetInvocationException tie)
            {
                Exception ex = tie.InnerException ?? tie;
                try { if (API.Logger != null) API.Logger.Error("GTAxSkate Coop ConnectTransport failed: " + ex); } catch { }
                CoopTrace.Write("client", "transport connect call failed: " + CoopTrace.Describe(ex));
                Notify("Connect failed (" + ex.GetType().Name + "). See the coop log.");
                return false;
            }
            catch (Exception ex)
            {
                try { if (API.Logger != null) API.Logger.Error("GTAxSkate Coop ConnectTransport failed: " + ex); } catch { }
                CoopTrace.Write("client", "transport connect call failed: " + CoopTrace.Describe(ex));
                Notify("Connect failed (" + ex.GetType().Name + "). See the coop log.");
                return false;
            }
        }

        private static FieldInfo peerField;
        private static MethodInfo flushMethod;
        private static bool flushResolved;

        public void FlushTransport()
        {
            try
            {
                if (!flushResolved)
                {
                    flushResolved = true;
                    Type networking = typeof(API).Assembly.GetType("RageCoop.Client.Networking", false);
                    peerField = networking == null ? null : networking.GetField("Peer", BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    flushMethod = peerField == null ? null : peerField.FieldType.GetMethod("FlushSendQueue", Type.EmptyTypes);
                    if (flushMethod == null)
                        try { if (API.Logger != null) API.Logger.Error("GTAxSkate Coop: RageCoop Peer.FlushSendQueue not found."); } catch { }
                }
                if (flushMethod == null) return;
                object peer = peerField.GetValue(null);
                if (peer != null) flushMethod.Invoke(peer, null);
            }
            catch { }
        }

        public bool IsTransportConnecting()
        {
            try
            {
                Type networking = typeof(API).Assembly.GetType("RageCoop.Client.Networking", false);
                if (networking == null) return true;  // unknown: let the deadline decide
                const BindingFlags any = BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;
                PropertyInfo p = networking.GetProperty("IsConnecting", any);
                object v = p != null ? p.GetValue(null, null) : null;
                if (v == null) { FieldInfo f = networking.GetField("IsConnecting", any); v = f != null ? f.GetValue(null) : null; }
                return !(v is bool) || (bool)v;
            }
            catch { return true; }
        }

        public void DisconnectTransport()
        {
            try
            {
                API.Disconnect();
            }
            catch { }
        }

        public bool EnsureLocalHostServerRunning(int port, string friendKey, out string statusMessage)
        {
            statusMessage = string.Empty;
            lock (hostServerGate)
            {
                try
                {
                    if (hostServerProcess != null && !hostServerProcess.HasExited)
                    {
                        // Same key: reuse. Another key: never kill a running server from a host action (friends may still
                        // be on it); the player stops it explicitly with the "Stop local server" row (Copilot round 3).
                        if (LocalServerKey.Decide(AppDomain.CurrentDomain.GetData(HostServerKeySlot) as string, friendKey) == LocalServerAction.Reuse)
                            return true;
                        CoopTrace.Write("client", "new hosting refused: a local server with another key is still running");
                        statusMessage = LocalServerKey.StillRunningNotice;
                        return false;
                    }

                    // One canonical root (GTA5.exe's directory), never the SHVDN shadow-copy location.
                    string gameRoot = CoopInstallPaths.ResolveGameRoot();
                    if (string.IsNullOrEmpty(gameRoot))
                    {
                        statusMessage = "GTA install root could not be resolved (no GTA5.exe next to the game process).";
                        return false;
                    }
                    string serverDir = CoopInstallPaths.HostServerDir(gameRoot);
                    string serverExe = CoopInstallPaths.HostServerExePath(gameRoot);
                    if (!File.Exists(serverExe))
                    {
                        statusMessage = "Local server executable is missing: scripts\\RageCoop\\HostServer\\RageCoop.Server.exe (re-run the coop setup).";
                        return false;
                    }
                    try { if (API.Logger != null) API.Logger.Info("GTAxSkate Coop host server: scripts\\RageCoop\\HostServer\\RageCoop.Server.exe (root from game process)."); } catch { }

                    // Clean up any orphaned RageCoop.Server.exe left over from an earlier crash holding the UDP port.
                    try
                    {
                        Process[] stale = Process.GetProcessesByName("RageCoop.Server");
                        for (int i = 0; i < stale.Length; i++)
                        {
                            try
                            {
                                if (CoopInstallPaths.SamePath(stale[i].MainModule.FileName, serverExe))
                                {
                                    stale[i].Kill();
                                    stale[i].WaitForExit(500);
                                }
                            }
                            catch { }
                            finally { stale[i].Dispose(); }
                        }
                    }
                    catch { }

                    string settingsPath = Path.Combine(serverDir, "Settings.xml");
                    if (!File.Exists(settingsPath))
                    {
                        string xml =
                            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n" +
                            "<Settings xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">\r\n" +
                            "  <Port>" + port + "</Port>\r\n" +
                            "  <MaxPlayers>10</MaxPlayers>\r\n" +
                            "  <AnnounceSelf>false</AnnounceSelf>\r\n" +
                            "  <UseUPnP>false</UseUPnP>\r\n" +
                            "  <WelcomeMessage>GTAxSkate Friends-Only Coop (Private Lobby)</WelcomeMessage>\r\n" +
                            "</Settings>\r\n";
                        File.WriteAllText(settingsPath, xml, new UTF8Encoding(false));
                    }

                    var psi = new ProcessStartInfo
                    {
                        FileName = serverExe,
                        WorkingDirectory = serverDir,
                        UseShellExecute = false,
                        CreateNoWindow = true,
                        WindowStyle = ProcessWindowStyle.Hidden
                    };
                    psi.EnvironmentVariables["GTAXSKATE_COOP_FRIEND_KEY"] = friendKey ?? string.Empty;

                    try
                    {
                        hostServerProcess = Process.Start(psi);
                    }
                    catch (Exception launchEx)
                    {
                        hostServerProcess = null;
                        statusMessage = "Local server launch failed (" + CoopTrace.Describe(launchEx) + ").";
                        return false;
                    }
                    if (hostServerProcess == null)
                    {
                        statusMessage = "Local server launch failed: no process was created.";
                        return false;
                    }
                    // Wait until RageCoop.Server.exe binds its UDP port (up to ~2.5s) before API.Connect runs.
                    for (int attempt = 0; attempt < 50; attempt++)
                    {
                        if (hostServerProcess.WaitForExit(50) || hostServerProcess.HasExited)
                        {
                            int exitCode = -1;
                            try { exitCode = hostServerProcess.ExitCode; } catch { }
                            statusMessage = "Local server exited early (exit code " + exitCode + "; port " + port + " may already be in use).";
                            hostServerProcess.Dispose();
                            hostServerProcess = null;
                            return false;
                        }
                        if (IsUdpPortBound(port))
                        {
                            AppDomain.CurrentDomain.SetData(HostServerKeySlot, LocalServerKey.Fingerprint(friendKey));
                            return true;
                        }
                    }
                    statusMessage = "Local server did not bind UDP port " + port + " in time.";
                    try { if (!hostServerProcess.HasExited) hostServerProcess.Kill(); } catch { }
                    hostServerProcess.Dispose();
                    hostServerProcess = null;
                    return false;
                }
                catch (Exception ex)
                {
                    statusMessage = "Local server error (" + CoopTrace.Describe(ex) + ").";
                    return false;
                }
            }
        }

        private string InstallId()
        {
            string root = null;
            try { root = CoopInstallPaths.ResolveGameRoot(); } catch { }
            using (var sha = System.Security.Cryptography.SHA256.Create())
            {
                byte[] h = sha.ComputeHash(Encoding.UTF8.GetBytes(Environment.MachineName + "|" + (root ?? string.Empty).ToLowerInvariant()));
                var sb = new StringBuilder();
                for (int i = 0; i < 4; i++) sb.Append(h[i].ToString("x2"));
                return sb.ToString();
            }
        }

        public void BeginOnlineHosting(int udpPort)
        {
            try
            {
                string description = UpnpHostPortService.DescriptionFor(InstallId());
                string lan = NatUpnpGateway.PickLanIPv4();
                AppDomain.CurrentDomain.SetData(OnlineMappingSlot, udpPort + "|" + (lan ?? string.Empty) + "|" + description);
                upnp = new UpnpHostPortService(delegate() { return new NatUpnpGateway(); }, UpnpHostPortService.DefaultTimeoutMs);
                upnp.Begin(udpPort, description, lan);
            }
            catch (Exception e)
            {
                CoopTrace.Write("client", "online hosting could not start: " + CoopTrace.Describe(e));
            }
        }

        public OnlineHostStatus PollOnlineHosting()
        {
            UpnpHostPortService service = upnp;
            return service == null ? OnlineHostStatus.Idle : service.Poll();
        }

        // Removes only this install's own mapping (by description). waitMs 0 never blocks the game thread.
        private void ReleaseOnlineMapping(int waitMs)
        {
            try
            {
                string recorded = AppDomain.CurrentDomain.GetData(OnlineMappingSlot) as string;
                AppDomain.CurrentDomain.SetData(OnlineMappingSlot, null);
                UpnpHostPortService service = upnp;
                upnp = null;
                if (service != null) { service.Release(waitMs); return; }
                if (string.IsNullOrEmpty(recorded))
                {
                    UpnpHostPortService.RetryPendingAsync(delegate() { return new NatUpnpGateway(); }, 0);   // earlier failed removals
                    return;
                }
                string[] f = recorded.Split(new[] { '|' }, 3);   // port | lan | description
                int port;
                if (f.Length != 3 || !int.TryParse(f[0], out port)) return;
                UpnpHostPortService.RemoveOwnAsync(delegate() { return new NatUpnpGateway(); }, port, f[2], f[1].Length == 0 ? null : f[1], waitMs);
            }
            catch (Exception e)
            {
                CoopTrace.Write("client", "online mapping release failed: " + CoopTrace.Describe(e));
            }
        }

        private readonly object clipboardGate = new object();

        private ClipboardWorker Worker()
        {
            lock (clipboardGate)
            {
                if (clipboard == null) clipboard = new ClipboardWorker(new WindowsClipboardBackend());
                return clipboard;
            }
        }

        public int BeginClipboard(ClipboardOpKind kind, string text, bool track)
        {
            try { return Worker().Begin(kind, text, track); }
            catch { return 0; }
        }

        public ClipboardOpResult PollClipboard(int ticket)
        {
            try { return Worker().Poll(ticket); }
            catch { return ClipboardOpResult.FailedResult; }
        }

        // WinForms clipboard on the worker's STA thread. A copied invite is marked so Windows keeps it out of the
        // clipboard history (Win+V), the cloud clipboard and clipboard monitors.
        private sealed class WindowsClipboardBackend : IClipboardBackend
        {
            private const int Attempts = 5, RetryDelayMs = 80;

            public string GetText()
            {
                Exception last = null;
                for (int i = 0; i < Attempts; i++)
                {
                    try { return Clipboard.ContainsText() ? Clipboard.GetText() : string.Empty; }
                    catch (Exception e) { last = e; Thread.Sleep(RetryDelayMs); }
                }
                throw last;
            }

            public void SetTextHidden(string text)
            {
                var data = new DataObject();
                data.SetData(DataFormats.UnicodeText, true, text);
                data.SetData("ExcludeClipboardContentFromMonitorProcessing", new MemoryStream(new byte[] { 1, 0, 0, 0 }));
                data.SetData("CanIncludeInClipboardHistory", new MemoryStream(new byte[4]));
                data.SetData("CanUploadToCloudClipboard", new MemoryStream(new byte[4]));
                Exception last = null;
                for (int i = 0; i < Attempts; i++)
                {
                    try { Clipboard.SetDataObject(data, true, 3, RetryDelayMs); return; }
                    catch (Exception e) { last = e; Thread.Sleep(RetryDelayMs); }
                }
                throw last;
            }

            public void Clear()
            {
                Exception last = null;
                for (int i = 0; i < Attempts; i++)
                {
                    try { Clipboard.Clear(); return; }
                    catch (Exception e) { last = e; Thread.Sleep(RetryDelayMs); }
                }
                throw last;
            }
        }

        public void GetLocalAddresses(out string lanIPv4, out string vpnIPv4)
        {
            lanIPv4 = null; vpnIPv4 = null;
            try { lanIPv4 = NatUpnpGateway.PickLanIPv4(); } catch { }
            try
            {
                foreach (NetworkInterface nic in NetworkInterface.GetAllNetworkInterfaces())
                {
                    if (nic.OperationalStatus != OperationalStatus.Up || nic.NetworkInterfaceType == NetworkInterfaceType.Loopback) continue;
                    foreach (UnicastIPAddressInformation ua in nic.GetIPProperties().UnicastAddresses)
                    {
                        if (ua.Address.AddressFamily != System.Net.Sockets.AddressFamily.InterNetwork) continue;
                        byte[] b = ua.Address.GetAddressBytes();
                        if (b[0] == 100 && b[1] >= 64 && b[1] <= 127) { vpnIPv4 = ua.Address.ToString(); return; }   // Tailscale / CGNAT range
                    }
                }
            }
            catch { }
        }

        // RageCoop's Main.Settings (internal static). Reflection only: RageCoop.Client.dll is a hash-pinned stock binary.
        private static object RageCoopSettingsObject()
        {
            Type main = typeof(API).Assembly.GetType("RageCoop.Client.Main", false);
            if (main == null) return null;
            const BindingFlags any = BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly;
            FieldInfo f = main.GetField("Settings", any);
            if (f != null) return f.GetValue(null);
            PropertyInfo p = main.GetProperty("Settings", any);
            return p == null ? null : p.GetValue(null, null);
        }

        private static string ReadSetting(object settings, string name)
        {
            PropertyInfo pi = settings.GetType().GetProperty(name);
            object v = pi == null ? null : pi.GetValue(settings, null);
            return v == null ? string.Empty : v.ToString();
        }

        private static void WriteSetting(object settings, string name, string value)
        {
            PropertyInfo pi = settings.GetType().GetProperty(name);
            if (pi == null || !pi.CanWrite) return;
            object v = pi.PropertyType.IsEnum ? Enum.Parse(pi.PropertyType, value) : (object)value;
            pi.SetValue(settings, v, null);
        }

        public void ForceAnonymousName()
        {
            try
            {
                object settings = RageCoopSettingsObject();
                if (settings != null) WriteSetting(settings, "Username", RageCoopClientSettings.LockedUsername);
            }
            catch { }
        }

        // Locks RageCoop's own F9 menu live (the settings file carries the same values for the next start).
        public bool SetRageCoopUiLocked(bool locked, out string error)
        {
            error = string.Empty;
            try
            {
                object settings = RageCoopSettingsObject();
                if (settings == null)
                {
                    error = "RageCoop's settings object was not found.";
                    CoopTrace.Write("client", "ui lock: settings object not found");
                    return false;
                }
                string[] keys = RageCoopClientSettings.Keys;
                lock (AppDomain.CurrentDomain)
                {
                    string[] backup = AppDomain.CurrentDomain.GetData(UiBackupSlot) as string[];
                    if (locked)
                    {
                        if (backup == null)
                        {
                            backup = new string[keys.Length];
                            for (int i = 0; i < keys.Length; i++) backup[i] = ReadSetting(settings, keys[i]);
                            AppDomain.CurrentDomain.SetData(UiBackupSlot, backup);
                        }
                        WriteSetting(settings, "MenuKey", RageCoopClientSettings.LockedMenuKey);
                        WriteSetting(settings, "MasterServer", RageCoopClientSettings.LockedMasterServer);
                        WriteSetting(settings, "LastServerAddress", RageCoopClientSettings.LockedServerAddress);
                        WriteSetting(settings, "Username", RageCoopClientSettings.LockedUsername);
                    }
                    else if (backup != null)
                    {
                        for (int i = 0; i < keys.Length; i++)
                            if (!string.IsNullOrEmpty(backup[i]) || keys[i] == "MasterServer") WriteSetting(settings, keys[i], backup[i]);
                        AppDomain.CurrentDomain.SetData(UiBackupSlot, null);
                    }
                }
                // The menu items cache the values they were built with: hide an open menu (it may show the real name or
                // address) and refresh the cached items from the settings we just wrote.
                string menuError;
                if (!RefreshRageCoopMenus(settings, locked, out menuError))
                {
                    error = menuError;
                    return false;
                }
                return true;
            }
            catch (Exception e)
            {
                error = "RageCoop's menu could not be " + (locked ? "locked." : "restored.");
                CoopTrace.Write("client", "ui lock failed: " + CoopTrace.Describe(e));
                return false;
            }
        }

        private static object StaticMember(Type t, string name)
        {
            if (t == null) return null;
            const BindingFlags any = BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly;
            FieldInfo f = t.GetField(name, any);
            if (f != null) return f.GetValue(null);
            PropertyInfo p = t.GetProperty(name, any);
            return p == null ? null : p.GetValue(null, null);
        }

        private static bool SetProperty(object target, string name, object value)
        {
            if (target == null) return false;
            PropertyInfo pi = target.GetType().GetProperty(name);
            if (pi == null || !pi.CanWrite) return false;
            pi.SetValue(target, value, null);
            return true;
        }

        // Reflection into RageCoop's internal menu classes (RageCoop.Client.dll is a hash-pinned stock binary).
        private static bool RefreshRageCoopMenus(object settings, bool locked, out string error)
        {
            error = string.Empty;
            int problems = 0;
            try
            {
                Assembly asm = typeof(API).Assembly;
                Type coop = asm.GetType("RageCoop.Client.Menus.CoopMenu", false);
                Type servers = asm.GetType("RageCoop.Client.Menus.ServersMenu", false);
                Type sets = asm.GetType("RageCoop.Client.Menus.SettingsMenu", false);
                if (coop == null || sets == null) problems++;

                // 1) hide whatever is open (the whole pool, then each menu by itself).
                object pool = StaticMember(coop, "MenuPool");
                MethodInfo hide = pool == null ? null : pool.GetType().GetMethod("HideAll", Type.EmptyTypes);
                if (hide != null) hide.Invoke(pool, null);   // optional: every menu is also hidden by itself below
                object[] menus = { StaticMember(coop, "Menu"), StaticMember(servers, "Menu"), StaticMember(sets, "Menu") };
                for (int i = 0; i < menus.Length; i++)
                    if (menus[i] != null && !SetProperty(menus[i], "Visible", false)) problems++;

                // 2) cached AltTitle texts follow the current settings (locked or restored values).
                if (!SetProperty(StaticMember(coop, "_usernameItem"), "AltTitle", ReadSetting(settings, "Username"))) problems++;
                if (!SetProperty(StaticMember(coop, "ServerIpItem"), "AltTitle", ReadSetting(settings, "LastServerAddress"))) problems++;
                if (!SetProperty(StaticMember(sets, "_menuKey"), "AltTitle", ReadSetting(settings, "MenuKey"))) problems++;
            }
            catch (Exception e)
            {
                problems++;
                CoopTrace.Write("client", "ui lock menu refresh failed: " + CoopTrace.Describe(e));
            }
            if (problems == 0) return true;
            CoopTrace.Write("client", "ui lock: " + problems + " RageCoop menu member(s) not reachable");
            error = "RageCoop's own menu could not be " + (locked ? "locked completely." : "refreshed.");
            return false;
        }

        public bool IsLocalServerRunning()
        {
            try
            {
                lock (hostServerGate)
                {
                    Process p = hostServerProcess;
                    return p != null && !p.HasExited;
                }
            }
            catch { return false; }
        }

        public void StopLocalHostServerIfStarted()
        {
            ReleaseOnlineMapping(0);
            lock (hostServerGate)
            {
                try
                {
                    if (hostServerProcess != null)
                    {
                        if (!hostServerProcess.HasExited)
                            hostServerProcess.Kill();
                        hostServerProcess.Dispose();
                        hostServerProcess = null;
                    }
                }
                catch { }
            }
        }

        private static bool IsUdpPortBound(int port)
        {
            try
            {
                IPEndPoint[] endpoints = IPGlobalProperties.GetIPGlobalProperties().GetActiveUdpListeners();
                for (int i = 0; i < endpoints.Length; i++)
                {
                    if (endpoints[i].Port == port)
                        return true;
                }
            }
            catch { }
            return false;
        }

        public void Notify(string message)
        {
            try
            {
                // Everything on screen is sanitized (design 2026-11-17-streamer-safe-coop): no address, code, path or user name.
                string safe = SafeText.ForScreen(message);
                if (!string.IsNullOrEmpty(safe))
                {
                    Function.Call(Hash.BEGIN_TEXT_COMMAND_THEFEED_POST, "STRING");
                    Function.Call(Hash.ADD_TEXT_COMPONENT_SUBSTRING_PLAYER_NAME, safe);
                    Function.Call(Hash.END_TEXT_COMMAND_THEFEED_POST_TICKER, false, false);
                }
            }
            catch { }
        }

        // While an F11 panel is open its keys (arrows, Enter, Backspace, Esc) belong to the panel:
        // GTA must not open the phone, move in the phone, or close/pause on them (Manatros 2026-11-17 22:30).
        // GTA input ids: 27 PHONE, 172-178 CELLPHONE_UP/DOWN/LEFT/RIGHT/SELECT/CANCEL/OPTION,
        // 188/187 FRONTEND_UP/DOWN, 201 FRONTEND_ACCEPT, 202 FRONTEND_CANCEL, 199/200 FRONTEND_PAUSE(_ALTERNATE), 37 SELECT_WEAPON.
        private static readonly int[] PanelKeys = { 27, 172, 173, 174, 175, 176, 177, 178, 188, 187, 201, 202, 199, 200, 37 };

        public static void BlockGameMenuKeys()
        {
            try
            {
                foreach (int c in PanelKeys) Function.Call(Hash.DISABLE_CONTROL_ACTION, 0, c, true);
            }
            catch { }
        }

        public void RenderOverlay(CoopOverlayView overlay)
        {
            if (overlay == null || IsGameLoading()) return;
            try
            {
                // Top-left lightweight connection banner (always lets player know solo vs connected session state).
                if (!string.IsNullOrEmpty(overlay.StatusBanner))
                {
                    var pill = new ContainerElement(new PointF(14f, 10f), new SizeF(480f, 24f), Color.FromArgb(180, 14, 18, 24));
                    pill.Items.Add(new ContainerElement(new PointF(0f, 0f), new SizeF(3f, 24f), Color.FromArgb(245, 242, 184, 48)));
                    pill.Items.Add(new TextElement(
                        "[F11 Coop] " + SafeText.ForScreen(overlay.StatusBanner),
                        new PointF(10f, 4f), 0.27f, Color.White, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    pill.Draw();
                }

                if (!overlay.MenuOpen) return;
                BlockGameMenuKeys();

                float height = 68f + (overlay.Items.Length * 28f) + (overlay.Warnings.Length > 0 ? 28f : 0f);
                var panel = new ContainerElement(new PointF(14f, 40f), new SizeF(480f, height), Color.FromArgb(225, 14, 18, 24));
                panel.Items.Add(new ContainerElement(new PointF(0f, 0f), new SizeF(480f, 3f), Color.FromArgb(245, 242, 184, 48)));
                panel.Items.Add(new TextElement(
                    string.IsNullOrEmpty(overlay.SubtitleLine) ? "Friends-Only Story + Skate Coop" : SafeText.ForScreen(overlay.SubtitleLine),
                    new PointF(12f, 9f), 0.34f, Color.Gold, GTA.UI.Font.ChaletLondon, Alignment.Left));

                float y = 36f;
                for (int i = 0; i < overlay.Items.Length; i++)
                {
                    bool sel = (i == overlay.SelectedIndex);
                    if (sel)
                    {
                        panel.Items.Add(new ContainerElement(
                            new PointF(6f, y - 2f), new SizeF(468f, 25f), Color.FromArgb(135, 42, 118, 210)));
                    }
                    Color c = !overlay.Items[i].Enabled ? Color.Silver : Color.White;
                    string prefix = sel ? "> " : "  ";
                    panel.Items.Add(new TextElement(
                        prefix + SafeText.ForScreen(overlay.Items[i].Label),
                        new PointF(12f, y), 0.30f, c, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    y += 28f;
                }

                if (overlay.Warnings.Length > 0)
                {
                    panel.Items.Add(new TextElement(
                        "Warning: " + SafeText.ForScreen(overlay.Warnings[0]),
                        new PointF(12f, y + 2f), 0.25f, Color.Orange, GTA.UI.Font.ChaletLondon, Alignment.Left));
                    y += 26f;
                }

                panel.Items.Add(new TextElement(
                    "Up/Down: Select  |  Enter: Activate  |  F11/Esc: Close",
                    new PointF(12f, y + 4f), 0.24f, Color.LightGray, GTA.UI.Font.ChaletLondon, Alignment.Left));
                panel.Draw();
            }
            catch { }
        }

        private void UpdateObjectiveBlip(HostWorldSyncState state)
        {
            if (state.MissionActive && state.HasObjective)
            {
                Vector3 pos = new Vector3(state.ObjectiveX, state.ObjectiveY, state.ObjectiveZ);
                if (hostObjectiveBlip == null || !hostObjectiveBlip.Exists())
                {
                    hostObjectiveBlip = Blip.Create(pos);
                    hostObjectiveBlip.ShowRoute = true;
                }
                else
                {
                    hostObjectiveBlip.Position = pos;
                }
                if (Enum.IsDefined(typeof(BlipSprite), state.ObjectiveSprite))
                    hostObjectiveBlip.Sprite = (BlipSprite)state.ObjectiveSprite;
                if (Enum.IsDefined(typeof(BlipColor), state.ObjectiveColor))
                    hostObjectiveBlip.Color = (BlipColor)state.ObjectiveColor;
            }
            else if (hostObjectiveBlip != null && hostObjectiveBlip.Exists())
            {
                hostObjectiveBlip.Delete();
                hostObjectiveBlip = null;
            }
        }

        private static float ClampCoord(float v)
        {
            if (float.IsNaN(v) || float.IsInfinity(v)) return 0.0f;
            return Math.Max(-HostWorldSyncState.MaxCoordAbs, Math.Min(HostWorldSyncState.MaxCoordAbs, v));
        }

        private static float ClampSpeed(float v)
        {
            if (float.IsNaN(v) || float.IsInfinity(v)) return 0.0f;
            return Math.Max(-RagdollSyncState.MaxSpeedAbs, Math.Min(RagdollSyncState.MaxSpeedAbs, v));
        }
    }
}
