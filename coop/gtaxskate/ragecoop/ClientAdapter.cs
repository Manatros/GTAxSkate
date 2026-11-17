using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using RageCoop.Client.Scripting;
using RageCoop.Core.Scripting;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class ClientAdapter : ClientScript
    {
        private static readonly object activeOwnerGate = new object();
        private static ClientAdapter activeOwner;
        private readonly object gate = new object();
        private readonly Stopwatch clock = Stopwatch.StartNew();
#if COOP_FIXTURE
        // Tests advance time (the 10-minute clipboard expiry, the clipboard timeout) without sleeping.
        internal long TestClockOffsetMs;
        private long NowMs { get { return clock.ElapsedMilliseconds + TestClockOffsetMs; } }
#else
        private long NowMs { get { return clock.ElapsedMilliseconds; } }
#endif
        private const int ConnectTimeoutMs = 15000;
        // The pinned callback API does not prove that incoming CustomEvents
        // originated at the server. Production leaves game-state writes off.
#if COOP_FIXTURE
        private static readonly bool ApplyWorldEffects = true;
        internal bool DiagnosticFixtureOnly;
#else
        private static readonly bool ApplyWorldEffects = false;
#endif
        private bool active, responseLogged, handlersRegistered, everStarted;
        private int lifetime, attempts, runtimeGeneration;
        // Per-start owner token, mirrored in the AppDomain-wide slot of CoopRuntimeControl (cross-copy arbitration).
        private Guid ownerToken;
        private long nextHello, lastEventTickMs = -10000;
        private byte[] challenge, session;
        private object[] pendingChallenge, pendingResult;
        private readonly SessionConnectionTracker securityTracker = new SessionConnectionTracker();
        private byte[] pendingAuthNonce;
        private string pendingFriendKey = string.Empty;
        private object[] pendingFriendChallenge;
        private object[] pendingFriendAccepted;
        private byte[] pendingSnapshotBytes;
        private string pendingRejectedReason;
        private int assignedSlot;
        private bool friendSessionAuthenticated;
        private SkateSessionSnapshot latestSnapshot;
        private readonly List<string> pendingModWarnings = new List<string>();
        private readonly List<string> latestModWarnings = new List<string>();
        private ModEntry[] localModManifest;
        private ClientConnectRequest configuredConnectRequest;
        private ClientConnectRequest pendingConnectRequest;
        private long connectDeadlineMs;
        private long connectStartedMs;
        // Fail-fast debounce (test 2026-11-17 10:14): RageCoop clears IsConnecting a tick before IsOnServer turns true.
        private long transportIdleSinceMs;
        private const int FailFastMinConnectMs = 5000;
        private const int FailFastIdleMs = 1500;
        // Set once the server answered FriendAuthInit: no more resends (a late resend made the
        // server issue a new challenge while the first one was being answered).
        private bool friendChallengeSeen;
        private bool pendingAutoHost;
        private int pendingAuthRetryCount;
        private long nextAuthAttemptMs;
        private IGtaWorldBridge worldBridge = new NullGtaWorldBridge();
        private readonly CoopInGameMenuState menuState = new CoopInGameMenuState();
        private byte[] pendingWorldSyncBytes;
        private byte[] pendingShopSyncBytes;
        private readonly List<object[]> pendingRagdollEvents = new List<object[]>();
        private HostWorldSyncState latestWorldState;
        private HostShopSyncState latestShopState;
        private bool wasOnServer;
        private bool peerCutsceneStandby;
        private bool lastSentMissionActive;
        private bool lastSentCutsceneActive;
        private bool lastSentRagdoll;
        private int worldSyncEpoch;
        private long nextWorldBroadcastMs;
        // "Host online (internet)": port of the requested lobby (0 = none), its result and the code shown to the player.
        private int onlineHostPort;
        private bool onlineResultHandled;
        private bool onlineOpen;      // the lobby port is open (a status row only; the code is never drawn)
        private string onlineStatusText;
        // Streamer-safe invites (design 2026-11-17-streamer-safe-coop): nothing secret is typed or drawn.
        private const long InviteClipboardLifetimeMs = 10 * 60 * 1000;
        private const long ClipboardOpTimeoutMs = 5000;
        private const string InviteCopiedNotice = "Invite copied to the clipboard (hidden for streaming). Send it to your friend in a private message.";
        private const string InviteCopyFailedNotice = "Could not copy the invite. Try again.";
        private const string ClipboardReadFailedNotice = "Could not read the clipboard. Try again.";
        private string hostPassword;     // fresh per hosting; never drawn, never typed
        private string hostInvite;       // the host's invite text (null until it exists)
        private int hostCopyTicket;      // a clipboard copy in flight (0 = none)
        private long hostCopyStartedMs;
        private string hostCopyText, hostCopyOkNotice, hostCopyFailNotice;
        private string hostCopiedText;   // what this PC last put on the clipboard; cleared after 10 minutes or when hosting stops
        private long hostCopiedClearAtMs;
        // An invite left on the clipboard by a crashed session (hash + expiry from InviteExpiryStore); cleared once expired.
        private string staleInviteHash;
        private DateTime staleInviteExpiresUtc;
        private int staleReadTicket;
        private long staleReadStartedMs;
        private enum JoinReadKind { Invite, SessionCode }
        private int joinReadTicket;      // a clipboard read in flight (0 = none)
        private long joinReadStartedMs;
        private JoinReadKind joinReadKind;
        private int joinReadGeneration, joinReadLifetime;
        private Guid joinReadToken;
        private long nextRagdollBroadcastMs;
        private long lastHostWorldRxMs;
        // This remains false until transport origin and skeleton registration
        // have a separately reviewed implementation. Never enabled by a packet.
        public bool RemotePoseEnabled { get { return false; } }
        public static ClientAdapter ActiveOwner { get { lock (activeOwnerGate) { return activeOwner; } } }
        public bool IsActive { get { lock (gate) { return active; } } }
        // Active AND owner AND still the same multiplayer-enabled generation (see CoopRuntimeControl).
        public bool IsLive { get { lock (gate) { return IsLiveLocked(); } } }
        // Another loaded copy of this assembly owns the shared token while this one still thinks it is active.
        public bool IsDisplaced
        {
            get
            {
                lock (activeOwnerGate)
                {
                    return ownerToken != Guid.Empty && CoopRuntimeControl.HasOtherOwner(ownerToken);
                }
            }
        }
        public SessionConnectionTracker SecurityTracker { get { return securityTracker; } }
        public SkateSessionSnapshot LatestSnapshot { get { lock (gate) { return latestSnapshot; } } }
        public HostWorldSyncState LatestWorldState { get { lock (gate) { return latestWorldState; } } }
        public HostShopSyncState LatestShopState { get { lock (gate) { return latestShopState; } } }
        public bool IsWorldAuthority
        {
            get
            {
                lock (gate)
                {
                    return friendSessionAuthenticated && latestSnapshot != null &&
                        assignedSlot > 0 && latestSnapshot.HostSlot == assignedSlot;
                }
            }
        }
        public string[] LatestModWarnings { get { lock (gate) { return latestModWarnings.ToArray(); } } }
        public ClientConnectRequest ConfiguredConnectRequest { get { lock (gate) { return configuredConnectRequest; } } }
        public CoopInGameMenuState MenuState { get { return menuState; } }
        public string ConnectionBanner { get { return securityTracker.BuildStatusBanner(); } }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        private static extern IntPtr GetModuleHandleW(string name);

        private bool IsActiveOwner()
        {
            lock (activeOwnerGate)
            {
                return ReferenceEquals(activeOwner, this) && CoopRuntimeControl.IsOwner(ownerToken);
            }
        }

        // The menu action that captured (generation, lifetime, token) is still the live one: nothing was
        // disabled, restarted or handed off since. Caller holds `gate`.
        private bool IsStillValidLocked(int generation, int life, Guid token)
        {
            return IsLiveLocked() && runtimeGeneration == generation && lifetime == life && ownerToken == token;
        }

        private bool IsStillValid(int generation, int life, Guid token)
        {
            lock (gate) { return IsStillValidLocked(generation, life, token); }
        }

        // Single fail-closed gate for every callback, tick, menu action and outbound entry point:
        // started, still the owner, multiplayer still on, and no enable/disable since this start.
        // Caller holds `gate` (or accepts a racy read).
        private bool IsLiveLocked()
        {
            return active && IsActiveOwner() && CoopRuntimeControl.IsCurrent(runtimeGeneration);
        }

        private void SafeLogInfo(string message)
        {
            CoopTrace.Write("client", "INFO " + message);
#if COOP_FIXTURE
            try
            {
                if (Logger != null) Logger.Info(message);
            }
            catch { }
#else
            try
            {
                if (CurrentResource != null && Logger != null)
                {
                    Logger.Info(message);
                    return;
                }
            }
            catch { }
            try
            {
                var fallback = API.Logger;
                if (fallback != null) fallback.Info(message);
            }
            catch { }
#endif
        }

        private void SafeLogWarning(string message)
        {
            CoopTrace.Write("client", "WARN " + message);
#if COOP_FIXTURE
            try
            {
                if (Logger != null) Logger.Warning(message);
            }
            catch { }
#else
            try
            {
                if (CurrentResource != null && Logger != null)
                {
                    Logger.Warning(message);
                    return;
                }
            }
            catch { }
            try
            {
                var fallback = API.Logger;
                if (fallback != null) fallback.Warning(message);
            }
            catch { }
#endif
        }

        // Locks (or restores) RageCoop's own F9 menu through the bridge; a failure of the live lock or of the settings
        // file is told to the player with a fixed, sanitized notice (never swallowed).
        private static void ApplyRageCoopUiLock(IGtaWorldBridge bridge, bool locked)
        {
            string fileError = CoopRuntimeControl.TakeUiLockError();
            string error = string.Empty;
            bool ok;
            try { ok = bridge.SetRageCoopUiLocked(locked, out error); }
            catch (Exception e)
            {
                ok = false;
                error = "RageCoop's own menu could not be " + (locked ? "locked." : "restored.");
                CoopTrace.Write("client", "ui lock call failed: " + CoopTrace.Describe(e));
            }
            if (!string.IsNullOrEmpty(fileError)) Say(bridge, "Coop: " + fileError);
            if (!ok) Say(bridge, "Coop: " + (string.IsNullOrEmpty(error) ? "RageCoop's own menu was not locked." : error) +
                (locked ? " Do not open the F9 menu while streaming." : string.Empty));
        }

        // Every string we put on screen passes SafeText.ForScreen (addresses, codes, paths, user names, current secrets).
        private static void Say(IGtaWorldBridge bridge, string message)
        {
            if (bridge != null) bridge.Notify(SafeText.ForScreen(message));
        }

        private void Say(string message)
        {
            Say(worldBridge, message);
        }

        public void SetWorldBridge(IGtaWorldBridge bridge)
        {
            lock (gate)
            {
                worldBridge = bridge ?? new NullGtaWorldBridge();
            }
        }

        public override void OnStart()
        {
            // Default OFF: an unsolicited start (e.g. RageCoop's resource loader) claims no owner,
            // builds no bridge, scans no mods and registers no handlers or ticks. It cannot enable itself.
            if (!CoopRuntimeControl.Enabled) return;
            ClientAdapter priorOwner = null;
            ClientConnectRequest inheritedConfigured = null;
            ClientConnectRequest inheritedPending = null;
            bool inheritedAutoHost = false;
            ModEntry[] inheritedMods = null;
            lock (activeOwnerGate)
            {
                if (activeOwner != null && !ReferenceEquals(activeOwner, this) && activeOwner.IsActive)
                {
                    priorOwner = activeOwner;
                }
                activeOwner = this;
                // Newest start owns across ALL loaded copies of this assembly (shared AppDomain slot).
                ownerToken = Guid.NewGuid();
                CoopRuntimeControl.ClaimOwner(ownerToken);
            }
            if (priorOwner != null)
            {
                lock (priorOwner.gate)
                {
                    inheritedConfigured = priorOwner.configuredConnectRequest;
                    inheritedPending = priorOwner.pendingConnectRequest;
                    inheritedAutoHost = priorOwner.pendingAutoHost;
                    inheritedMods = priorOwner.localModManifest;
                }
                try { priorOwner.StopInternal(false); } catch { }
            }

            int epoch;
            lock (gate)
            {
                active = true; everStarted = true; epoch = ++lifetime;
                runtimeGeneration = CoopRuntimeControl.Generation;
                lastEventTickMs = -10000;
                challenge = Guid.NewGuid().ToByteArray(); session = null;
                pendingChallenge = pendingResult = null;
                pendingAuthNonce = null; pendingFriendKey = string.Empty;
                pendingFriendChallenge = pendingFriendAccepted = null;
                pendingSnapshotBytes = null; pendingRejectedReason = null;
                assignedSlot = 0; friendSessionAuthenticated = false; latestSnapshot = null;
                pendingModWarnings.Clear(); latestModWarnings.Clear();
                localModManifest = inheritedMods;
                configuredConnectRequest = inheritedConfigured;
                pendingConnectRequest = inheritedPending;
                pendingAutoHost = inheritedAutoHost;
                connectDeadlineMs = inheritedPending != null ? NowMs + ConnectTimeoutMs : 0;
                pendingAuthRetryCount = 0; nextAuthAttemptMs = 0; friendChallengeSeen = false;
                pendingWorldSyncBytes = null; pendingShopSyncBytes = null; pendingRagdollEvents.Clear();
                latestWorldState = null; latestShopState = null;
                wasOnServer = false; peerCutsceneStandby = false;
                lastSentMissionActive = false; lastSentCutsceneActive = false; lastSentRagdoll = false;
                worldSyncEpoch = 0; nextWorldBroadcastMs = 0; nextRagdollBroadcastMs = 0; lastHostWorldRxMs = 0;
                menuState.Close();
#if !COOP_FIXTURE
                if (worldBridge is NullGtaWorldBridge) worldBridge = new ShvdnWorldBridge(this);
#endif
                securityTracker.DisconnectToSolo("Solo Story Mode", NowMs);
                attempts = 0; nextHello = 0; responseLogged = false;
                if (localModManifest == null)
                {
                    try
                    {
                        string gameRoot = CoopInstallPaths.ResolveGameRoot();
                        if (!string.IsNullOrEmpty(gameRoot))
                            AutoLoadAndScanFromGtaFolderLocked(gameRoot);
                    }
                    catch { }
                }
            }
            try
            {
                IGtaWorldBridge startBridge;
                lock (gate) { startBridge = worldBridge; }
                ApplyRageCoopUiLock(startBridge, true);
                lock (gate) { BeginStaleInviteCheckLocked(); }
            }
            catch { }
            try
            {
                if (!handlersRegistered)
                {
                    handlersRegistered = true;
                    RegisterCustomHandlers();
                }
                API.QueueAction(new Action(() =>
                {
                    lock (gate)
                    {
                        if (!IsLiveLocked() || epoch != lifetime) return;
                        API.Events.OnTick -= OnRageCoopTick;
                        API.Events.OnTick += OnRageCoopTick;
                        SafeLogInfo("GTAxSkate Coop adapter started; " + securityTracker.BuildStatusBanner());
                    }
                }));
            }
            catch
            {
                try { OnStop(); } catch { }
                throw;
            }
        }

        private void RegisterCustomHandlers()
        {
            API.RegisterCustomEventHandler(CustomEvents.Hash(DiagnosticProtocol.Challenge), e =>
            {
                // Copy at the callback boundary; consume at most one reply on
                // the script thread, never queue arbitrary native work.
                lock (gate)
                {
                    if (!IsLiveLocked() || !DiagnosticProtocol.Header(e.Args, 5) ||
                        !DiagnosticProtocol.Nonce(e.Args[1]) || !DiagnosticProtocol.Nonce(e.Args[2]) ||
                        !(e.Args[3] is int) || (int)e.Args[3] == 0 || !(e.Args[4] is bool) ||
                        (bool)e.Args[4] || !DiagnosticProtocol.Equal(challenge, (byte[])e.Args[1])) return;
                    if (session != null || pendingChallenge != null) return;
                    pendingChallenge = new object[] { 1, ((byte[])e.Args[1]).Clone(),
                        ((byte[])e.Args[2]).Clone(), e.Args[3], false };
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(DiagnosticProtocol.Result), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || responseLogged || !DiagnosticProtocol.Header(e.Args, 4) ||
                        !DiagnosticProtocol.Nonce(e.Args[1]) || !DiagnosticProtocol.Nonce(e.Args[2]) ||
                        !(e.Args[3] is bool) || !(bool)e.Args[3] ||
                        !DiagnosticProtocol.Equal(challenge, (byte[])e.Args[1]) ||
                        !DiagnosticProtocol.Equal(session, (byte[])e.Args[2])) return;
                    pendingResult = new object[] { 1, ((byte[])e.Args[1]).Clone(), ((byte[])e.Args[2]).Clone(), true };
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.FriendAuthChallenge), e =>
            {
                lock (gate)
                {
                    CoopTrace.Write("client", "FriendAuthChallenge received: live=" + IsLiveLocked() + " nonceMatch=" +
                        (pendingAuthNonce != null && e.Args != null && e.Args.Length > 1 && e.Args[1] is byte[] &&
                         DiagnosticProtocol.Equal(pendingAuthNonce, (byte[])e.Args[1])));
                    if (!IsLiveLocked() || pendingAuthNonce == null || friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 5) || !DiagnosticProtocol.Nonce(e.Args[1]) ||
                        !DiagnosticProtocol.Nonce(e.Args[2]) || !(e.Args[3] is int) || (int)e.Args[3] <= 0 ||
                        !SessionProtocol.Proof(e.Args[4]) || !DiagnosticProtocol.Equal(pendingAuthNonce, (byte[])e.Args[1])) return;
                    if (friendChallengeSeen) { CoopTrace.Write("client", "FriendAuthChallenge ignored: already answering one"); return; }
                    friendChallengeSeen = true;
                    pendingFriendChallenge = new object[] {
                        SessionProtocol.Version, ((byte[])e.Args[1]).Clone(), ((byte[])e.Args[2]).Clone(),
                        e.Args[3], ((byte[])e.Args[4]).Clone()
                    };
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.FriendAuthAccepted), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || pendingAuthNonce == null ||
                        !SessionProtocol.Header(e.Args, 3) || !DiagnosticProtocol.Nonce(e.Args[1]) ||
                        !(e.Args[2] is int) || (int)e.Args[2] <= 0 ||
                        !DiagnosticProtocol.Equal(pendingAuthNonce, (byte[])e.Args[1])) return;
                    pendingFriendAccepted = new object[] { SessionProtocol.Version, ((byte[])e.Args[1]).Clone(), e.Args[2] };
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionSnapshot), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 2) || !(e.Args[1] is byte[])) return;
                    pendingSnapshotBytes = (byte[])((byte[])e.Args[1]).Clone();
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionRejected), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !SessionProtocol.Header(e.Args, 2) ||
                        !SessionProtocol.BoundedText(e.Args[1], SessionProtocol.MaxTextBytes)) return;
                    pendingRejectedReason = (string)e.Args[1];
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionModWarning), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 3) || !(e.Args[1] is int) ||
                        !SessionProtocol.BoundedText(e.Args[2], SessionProtocol.MaxWarningBytes)) return;
                    pendingModWarnings.Add((string)e.Args[2]);
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionHostWorldSync), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 2) || !(e.Args[1] is byte[])) return;
                    byte[] raw = (byte[])e.Args[1];
                    if (raw.Length <= SessionProtocol.MaxWorldSyncBytes)
                        pendingWorldSyncBytes = (byte[])raw.Clone();
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionHostShopSync), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 2) || !(e.Args[1] is byte[])) return;
                    byte[] raw = (byte[])e.Args[1];
                    if (raw.Length <= SessionProtocol.MaxShopSyncBytes)
                        pendingShopSyncBytes = (byte[])raw.Clone();
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(SessionProtocol.SessionRagdollSync), e =>
            {
                lock (gate)
                {
                    if (!IsLiveLocked() || !friendSessionAuthenticated ||
                        !SessionProtocol.Header(e.Args, 3) || !(e.Args[1] is int) || !(e.Args[2] is byte[])) return;
                    byte[] raw = (byte[])e.Args[2];
                    if (raw.Length == 37 && pendingRagdollEvents.Count < 16)
                        pendingRagdollEvents.Add(new object[] { e.Args[1], raw.Clone() });
                }
            });
        }

        public bool BeginTrustedSessionConnect(ClientConnectRequest request, out PolicyDecision decision)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer)
                {
                    friendSessionAuthenticated = false;
                    assignedSlot = 0;
                    latestSnapshot = null;
                    pendingAuthNonce = null;
                    pendingFriendKey = string.Empty;
                    decision = new PolicyDecision(false, EndpointTrustClass.Invalid, "Not active on a server.");
                    return false;
                }

                decision = CoopSecurityPolicy.EvaluateOutboundConnect(request);
                if (!decision.Allowed)
                    return securityTracker.BeginFriendConnect(request, Guid.NewGuid().ToByteArray(), NowMs, out decision);
                if (!IsExpectedTransport(request) || pendingAuthNonce != null || friendSessionAuthenticated)
                {
                    decision = new PolicyDecision(false, EndpointTrustClass.Invalid, "Friend connection does not match the active endpoint.");
                    return false;
                }
                pendingConnectRequest = request;
                connectDeadlineMs = NowMs + ConnectTimeoutMs;
                pendingAutoHost = false;
                return BeginFriendAuthLocked(request, out decision);
            }
        }

        public bool HostFriendSession(SkateWorldZone zone, SkateMode mode, string title)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHost),
                    SessionProtocol.Version, (int)zone, (int)mode, title ?? "Friend Skate Session");
                return true;
            }
        }

        public bool JoinFriendSession(string joinCode)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated || string.IsNullOrEmpty(joinCode)) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionJoin),
                    SessionProtocol.Version, joinCode);
                return true;
            }
        }

        public void LeaveFriendSession()
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return;
                latestSnapshot = null;
                latestWorldState = null;
                latestShopState = null;
                peerCutsceneStandby = false;
                worldBridge.RestoreSoloDefaults();
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionLeave), SessionProtocol.Version);
            }
        }

        public bool ConfigureFriendActivity(SkateMode mode, SkateWorldZone zone, string spotId, long durationMs)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionConfigure),
                    SessionProtocol.Version, (int)mode, (int)zone, spotId ?? string.Empty, durationMs);
                return true;
            }
        }

        public bool StartFriendActivity()
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionStart), SessionProtocol.Version);
                return true;
            }
        }

        public bool SendBoardProfile(string boardProfile)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionBoard),
                    SessionProtocol.Version, boardProfile ?? string.Empty);
                return true;
            }
        }

        public bool SubmitDryLandScore(int trickScore, int bestLineScore, bool dryLandVerified)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated || !dryLandVerified) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionScore),
                    SessionProtocol.Version, trickScore, bestLineScore, dryLandVerified);
                return true;
            }
        }

        public bool SubmitLocalModManifest(ModEntry[] entries)
        {
            lock (gate)
            {
                localModManifest = ModCompatibilityChecker.NormalizeManifest(entries);
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated) return false;
                byte[] encoded = SessionProtocol.EncodeModsManifest(localModManifest);
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionModsReport),
                    SessionProtocol.Version, encoded);
                return true;
            }
        }

        public bool ScanAndSubmitLocalMods(string gtaRoot)
        {
            ModEntry[] scanned = LocalModScanner.ScanGtaRoot(gtaRoot);
            return SubmitLocalModManifest(scanned);
        }

        public bool AutoLoadAndScanFromGtaFolder(string gtaRoot)
        {
            lock (gate)
            {
                return AutoLoadAndScanFromGtaFolderLocked(gtaRoot);
            }
        }

        private bool AutoLoadAndScanFromGtaFolderLocked(string gtaRoot)
        {
            localModManifest = LocalModScanner.ScanGtaRoot(gtaRoot);
            ClientConnectRequest loaded;
            string cfgError;
            if (ClientConfigLoader.TryLoadFromGtaRoot(gtaRoot, out loaded, out cfgError))
            {
                configuredConnectRequest = loaded;
                if (active && API.IsOnServer && friendSessionAuthenticated)
                {
                    byte[] encoded = SessionProtocol.EncodeModsManifest(localModManifest);
                    API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionModsReport),
                        SessionProtocol.Version, encoded);
                }
                return true;
            }
            configuredConnectRequest = null;
            return false;
        }

        public bool SubmitHostWorldState(HostWorldSyncState state)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated || latestSnapshot == null ||
                    latestSnapshot.HostSlot != assignedSlot || state == null)
                    return false;
                byte[] encoded = SessionProtocol.EncodeWorldSync(state);
                if (encoded == null) return false;
                latestWorldState = state;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostWorldSync), SessionProtocol.Version, encoded);
                return true;
            }
        }

        public bool SubmitHostShopState(HostShopSyncState state)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated || latestSnapshot == null ||
                    latestSnapshot.HostSlot != assignedSlot || state == null)
                    return false;
                byte[] encoded = SessionProtocol.EncodeShopSync(state);
                if (encoded == null) return false;
                latestShopState = state;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostShopSync), SessionProtocol.Version, encoded);
                return true;
            }
        }

        public bool SubmitRagdollState(RagdollSyncState state)
        {
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer || !friendSessionAuthenticated || assignedSlot <= 0 || state == null)
                    return false;
                var stamped = new RagdollSyncState(
                    assignedSlot, state.IsRagdoll, state.PosX, state.PosY, state.PosZ,
                    state.VelX, state.VelY, state.VelZ, state.DurationMs);
                byte[] encoded = SessionProtocol.EncodeRagdollSync(stamped);
                if (encoded == null) return false;
                API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRagdollSync), SessionProtocol.Version, encoded);
                return true;
            }
        }

        // Virtual key codes for in-game F11 menu navigation (keeps WinForms ref out of shared/fixture tests).
        public bool HandleMenuKeyCode(int virtualKey)
        {
            bool runActivate = false;
            lock (gate)
            {
                if (!IsLiveLocked()) return false;
                if (virtualKey == 0x7A) // VK_F11
                {
                    menuState.Toggle();
                    RenderOverlayLocked();
                    return true;
                }
                if (!menuState.IsOpen) return false;
                if (virtualKey == 0x26) // VK_UP
                {
                    CoopMenuItem[] items = BuildMenuItemsLocked();
                    menuState.MoveSelection(-1, items.Length);
                    RenderOverlayLocked();
                    return true;
                }
                if (virtualKey == 0x28) // VK_DOWN
                {
                    CoopMenuItem[] items = BuildMenuItemsLocked();
                    menuState.MoveSelection(1, items.Length);
                    RenderOverlayLocked();
                    return true;
                }
                if (virtualKey == 0x08 || virtualKey == 0x1B) // VK_BACK / VK_ESCAPE
                {
                    menuState.Close();
                    RenderOverlayLocked();
                    return true;
                }
                if (virtualKey == 0x0D) // VK_RETURN
                {
                    runActivate = true;
                }
            }
            if (runActivate)
            {
                ActivateSelectedMenuItemUnlocked();
                return true;
            }
            return false;
        }

        public bool ActivateSelectedMenuItem()
        {
            return ActivateSelectedMenuItemUnlocked();
        }

        private bool ActivateSelectedMenuItemUnlocked()
        {
            CoopMenuItem chosen;
            ClientConnectRequest cfgReq;
            bool onServer;
            bool authenticated;
            bool hasSnapshot;
            IGtaWorldBridge bridge;
            int startGeneration, startLifetime;
            Guid startToken;
            lock (gate)
            {
                if (!IsLiveLocked()) return false;
                // Captured together: any disable, re-enable, restart or owner handoff changes at least one.
                startGeneration = runtimeGeneration; startLifetime = lifetime; startToken = ownerToken;
                CoopMenuItem[] items = BuildMenuItemsLocked();
                if (items.Length == 0) return false;
                int idx = menuState.SelectedIndex >= 0 && menuState.SelectedIndex < items.Length ? menuState.SelectedIndex : 0;
                chosen = items[idx];
                if (!chosen.Enabled) return false;
                cfgReq = configuredConnectRequest;
                onServer = API.IsOnServer;
                authenticated = friendSessionAuthenticated;
                hasSnapshot = latestSnapshot != null;
                bridge = worldBridge;
            }

            if (chosen.Action == CoopMenuAction.DisableMultiplayer)
            {
                DisableMultiplayer();
                return true;
            }

            // EnsureLocalHostServerRunning may spawn a process and ConnectTransport shows a blocking RageCoop warning;
            // run them outside `lock (gate)` so background RageCoop callbacks cannot deadlock.
            // Every wait below (server start) is followed by IsStillValid BEFORE any notice or launch.
            // Nothing secret is ever typed: there is no on-screen-keyboard prompt on the bridge any more.
            if (chosen.Action == CoopMenuAction.ConnectConfiguredHost)
            {
                ClientConnectRequest req = cfgReq;
                if (req == null)
                {
                    Say(bridge, "No saved friend is set up. Use Join friend (paste invite).");
                    return false;
                }
                lock (gate)
                {
                    if (!IsStillValidLocked(startGeneration, startLifetime, startToken)) return false;
                    bool ok = StartValidatedInGameConnectLocked(req, false);
                    RenderOverlayLocked();
                    if (!ok) return false;
                }
                return ConnectPendingUnlocked(bridge, req, false, startGeneration, startLifetime, startToken);
            }
            if (chosen.Action == CoopMenuAction.HostPrivateSession || chosen.Action == CoopMenuAction.HostOnline)
            {
                bool online = chosen.Action == CoopMenuAction.HostOnline;
                if (!(onServer && authenticated))
                {
                    // A connect in flight owns the local server: never restart or re-key it from a second host action.
                    bool blocked;
                    lock (gate) { blocked = HostActionBlockedLocked(); }
                    if (blocked)
                    {
                        Say(bridge, "A connection is already starting. Wait for it, or disconnect first.");
                        return false;
                    }
                }
                if (!online && onServer && authenticated)
                {
                    lock (gate)
                    {
                        if (!IsLiveLocked()) return false;
                        if (latestSnapshot == null)
                            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHost),
                                SessionProtocol.Version, (int)SkateWorldZone.LosSantos, (int)SkateMode.FreeSkate, "Story+Skate Crew");
                        RenderOverlayLocked();
                        return true;
                    }
                }
                // A fresh random password per hosting (20 Crockford symbols, about 100 bits): generated here, copied
                // to the clipboard inside the invite, never typed and never drawn.
                string key = CoopInvite.GeneratePassword();
                SafeText.AddSecret(SafeText.GroupHost, key);
                int port = cfgReq != null ? cfgReq.Port : 4499;
                string localInvite = null;
                if (!online)
                {
                    string lan, vpn;
                    bridge.GetLocalAddresses(out lan, out vpn);
                    if (!string.IsNullOrEmpty(vpn)) localInvite = CoopInvite.BuildLocal(InviteLocatorKind.Vpn, vpn, port, key);
                    if (localInvite == null && !string.IsNullOrEmpty(lan)) localInvite = CoopInvite.BuildLocal(InviteLocatorKind.Lan, lan, port, key);
                    if (localInvite == null)
                    {
                        SafeText.ClearGroup(SafeText.GroupHost);
                        Say(bridge, "No LAN or VPN network address was found on this PC. Connect to your network, or use Host online.");
                        return false;
                    }
                    SafeText.AddSecret(SafeText.GroupHost, localInvite);
                }
                string hostErr;
                // Recheck owner, lifetime and generation before spawning anything.
                if (!IsStillValid(startGeneration, startLifetime, startToken)) { SafeText.ClearGroup(SafeText.GroupHost); return false; }
                bool hostStarted = bridge.EnsureLocalHostServerRunning(port, key, out hostErr);
                if (!IsStillValid(startGeneration, startLifetime, startToken))
                {
                    // Disabled (or disabled and re-enabled) while the server was starting: stop only the server this
                    // integration started, stay silent. An owner handoff alone keeps the server (B152): the new owner reuses it.
                    if (hostStarted && !CoopRuntimeControl.IsCurrent(startGeneration)) StopHostServer(bridge);
                    return false;
                }
                if (!hostStarted)
                {
                    SafeText.ClearGroup(SafeText.GroupHost);
                    // The still-running-server notice is already a complete, sanitized sentence.
                    Say(bridge, hostErr == LocalServerKey.StillRunningNotice ? hostErr
                        : "Local Host failed: " + (string.IsNullOrEmpty(hostErr) ? "server error" : hostErr));
                    return false;
                }
                var hostReq = new ClientConnectRequest(
                    LobbyDiscoverySource.LocalHost, "127.0.0.1", port, true, "LocalHost", key);
                lock (gate)
                {
                    if (!IsStillValidLocked(startGeneration, startLifetime, startToken)) return false;
                    hostPassword = key;
                    hostInvite = localInvite;
                    if (online)
                    {
                        onlineHostPort = port; onlineResultHandled = false; onlineOpen = false;
                        onlineStatusText = "opening the router port (UPnP)...";
                        CoopTrace.Write("client", "host online requested");
                        bridge.BeginOnlineHosting(port);
                    }
                    bool ok = StartValidatedInGameConnectLocked(hostReq, true);
                    if (ok && !online) BeginHostCopyLocked(localInvite, InviteCopiedNotice, InviteCopyFailedNotice);
                    RenderOverlayLocked();
                    if (!ok) return false;
                }
                return ConnectPendingUnlocked(bridge, hostReq, true, startGeneration, startLifetime, startToken);
            }
            if (chosen.Action == CoopMenuAction.JoinOnline || chosen.Action == CoopMenuAction.JoinSessionByCode)
            {
                // The invite (or the session join code) is read from the clipboard on a worker thread and handled on a
                // later tick; nothing is typed and nothing is drawn.
                lock (gate)
                {
                    if (!IsStillValidLocked(startGeneration, startLifetime, startToken)) return false;
                    if (joinReadTicket != 0) return false;   // a clipboard read is already running
                    joinReadKind = chosen.Action == CoopMenuAction.JoinOnline ? JoinReadKind.Invite : JoinReadKind.SessionCode;
                    joinReadGeneration = startGeneration; joinReadLifetime = startLifetime; joinReadToken = startToken;
                    joinReadStartedMs = NowMs;
                    joinReadTicket = bridge.BeginClipboard(ClipboardOpKind.Read, null, true);
                    if (joinReadTicket <= 0)
                    {
                        joinReadTicket = 0;
                        Say(bridge, ClipboardReadFailedNotice);
                        return false;
                    }
                    return true;
                }
            }

            lock (gate)
            {
                if (!IsLiveLocked()) return false;
                bool ok = ActivateSelectedMenuItemLocked(chosen.Action);
                RenderOverlayLocked();
                return ok;
            }
        }

        private void OnRageCoopTick()
        {
            bool stale;
            Guid token;
            lock (gate)
            {
                // Still the token owner but disabled (or disabled and re-enabled) behind our back and nobody
                // else tears us down (a resource-loaded copy is not driven by the bootstrap): tear down here.
                stale = active && IsActiveOwner() && !CoopRuntimeControl.IsCurrent(runtimeGeneration);
                token = ownerToken;
                if (!stale && IsLiveLocked()) CoopRuntimeControl.Heartbeat(token, CoopRuntimeControl.UtcNowMs());
            }
            if (stale) { StopForDisable(); return; }
            lastEventTickMs = NowMs;
            Tick();
            FlushIfOnServer();
        }

        public void StepEveryFrame()
        {
            PollInviteFlowsUnlocked();
            IGtaWorldBridge bridge;
            bool onServer;
            lock (gate)
            {
                if (!IsLiveLocked()) return;
                CoopRuntimeControl.Heartbeat(ownerToken, CoopRuntimeControl.UtcNowMs());
                bridge = worldBridge;
                if (bridge == null || bridge.IsGameLoading()) return;
                onServer = API.IsOnServer;
                if (onServer)
                {
                    if (NowMs - lastEventTickMs >= 100)
                        RenderOverlayLocked();
                    return;
                }
                if (NowMs - lastEventTickMs < 100) return;
            }
            Tick();
            FlushIfOnServer();
        }

        // 2026-11-16 22:19 test: only the first custom event ever reached the server. RAGECOOP 1.5 sends with
        // AutoFlushSendQueue = false and flushes at the end of EntityPool.DoSync, which sits in a silent
        // try/catch in Main.OnTick; when that sync throws, our events stay queued. Flush them ourselves.
        private void FlushIfOnServer()
        {
            IGtaWorldBridge bridge;
            lock (gate)
            {
                if (!IsLiveLocked() || !API.IsOnServer) return;
                bridge = worldBridge;
            }
            bridge.FlushTransport();
        }

        // True while a connect is pending or the client is on the server but not yet authenticated.
        private bool HostActionBlockedLocked()
        {
            return pendingConnectRequest != null || (API.IsOnServer && !friendSessionAuthenticated);
        }

        private CoopMenuItem[] BuildMenuItemsLocked()
        {
            bool hasCfg = configuredConnectRequest != null;
            string hostLabel = hasCfg ? configuredConnectRequest.HostLabel : string.Empty;
            bool inSession = latestSnapshot != null;
            bool isHost = inSession && assignedSlot > 0 && latestSnapshot.HostSlot == assignedSlot;
            string onlineInfo = null;
            if (onlineHostPort > 0)
                onlineInfo = onlineOpen
                    ? "Online lobby: open (use Copy invite to send it again)"
                    : "Online lobby: " + (string.IsNullOrEmpty(onlineStatusText) ? "opening..." : onlineStatusText);
            // The session join code and the invite are never drawn: rows only offer to copy them.
            return menuState.BuildVisibleItems(
                API.IsOnServer, friendSessionAuthenticated, hasCfg, hostLabel, inSession, isHost, string.Empty, onlineInfo, hostInvite != null,
                HostActionBlockedLocked(), worldBridge.IsLocalServerRunning());
        }

        private bool ActivateSelectedMenuItemLocked(CoopMenuAction expectedAction)
        {
            CoopMenuItem[] items = BuildMenuItemsLocked();
            if (items.Length == 0) return false;
            int idx = menuState.SelectedIndex >= 0 && menuState.SelectedIndex < items.Length ? menuState.SelectedIndex : 0;
            CoopMenuItem chosen = items[idx];
            if (!chosen.Enabled || chosen.Action != expectedAction) return false;
            if (chosen.Action == CoopMenuAction.ConnectConfiguredHost ||
                chosen.Action == CoopMenuAction.HostPrivateSession ||
                chosen.Action == CoopMenuAction.HostOnline ||
                chosen.Action == CoopMenuAction.JoinOnline ||
                chosen.Action == CoopMenuAction.JoinSessionByCode)
                return false;   // handled in ActivateSelectedMenuItemUnlocked (server start, transport and clipboard reads)

            if (chosen.Action == CoopMenuAction.CopyInvite)
            {
                if (hostInvite == null) return false;
                BeginHostCopyLocked(hostInvite, InviteCopiedNotice, InviteCopyFailedNotice);
                return true;
            }
            if (chosen.Action == CoopMenuAction.CopyJoinCode)
            {
                if (latestSnapshot == null || latestSnapshot.HostSlot != assignedSlot) return false;
                BeginHostCopyLocked(latestSnapshot.JoinCode, "Join code copied to the clipboard (hidden).", "Could not copy the join code. Try again.");
                return true;
            }
            if (chosen.Action == CoopMenuAction.PushWorldSyncNow)
            {
                HostWorldSyncState sample = worldBridge.CaptureHostWorldState(++worldSyncEpoch);
                if (sample != null)
                {
                    byte[] enc = SessionProtocol.EncodeWorldSync(sample);
                    if (enc != null)
                    {
                        latestWorldState = sample;
                        API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostWorldSync), SessionProtocol.Version, enc);
                    }
                }
                if (latestShopState != null)
                {
                    byte[] shopEnc = SessionProtocol.EncodeShopSync(latestShopState);
                    if (shopEnc != null)
                        API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostShopSync), SessionProtocol.Version, shopEnc);
                }
                Say("Synced Host World, Mission & Shop state to friends.");
                return true;
            }
            if (chosen.Action == CoopMenuAction.StopLocalServer)
            {
                if (API.IsOnServer || pendingConnectRequest != null) return false;   // never under a live or pending connect
                // Ends only our own server process and its router mapping; invite and online status go with it.
                StopHostServer(worldBridge);
                RenderOverlayLocked();
                Say("Local server stopped.");
                return true;
            }
            if (chosen.Action == CoopMenuAction.DisconnectToSolo)
            {
                menuState.Close();
                ResetSessionStateLocked("Solo Story Mode", NowMs, true);
                worldBridge.DisconnectTransport();
                StopHostServer(worldBridge);
                Say("Disconnected cleanly -> Solo Story Mode.");
                return true;
            }
            return false;
        }

        private bool StartValidatedInGameConnectLocked(ClientConnectRequest req, bool autoHost)
        {
            PolicyDecision decision = CoopSecurityPolicy.EvaluateOutboundConnect(req);
            if (!decision.Allowed)
            {
                securityTracker.BeginFriendConnect(req, Guid.NewGuid().ToByteArray(), NowMs, out decision);
                Say("BLOCKED: " + decision.Reason);
                SafeLogWarning("GTAxSkate Coop in-game connect blocked: " + decision.Reason);
                return false;
            }

            if (API.IsOnServer || pendingConnectRequest != null)
            {
                Say("A connection is already active or pending.");
                return false;
            }
            pendingConnectRequest = req;
            pendingAutoHost = autoHost;
            connectStartedMs = NowMs;
            transportIdleSinceMs = 0;
            connectDeadlineMs = NowMs + ConnectTimeoutMs;
            // Label only: never the address (a stream shows the screen).
            Say("Connecting to " + CoopSecurityPolicy.ScreenLabel(req.HostLabel) + "...");
            return true;
        }

        // The transport call runs outside `gate`: with a friend key RageCoop shows a blocking
        // "Server's IP can be spoofed" warning (Copilot review 2026-11-16).
        private bool ConnectPendingUnlocked(IGtaWorldBridge bridge, ClientConnectRequest req, bool autoHost,
            int startGeneration, int startLifetime, Guid startToken)
        {
            bridge.ForceAnonymousName();
            if (bridge.ConnectTransport(req.EndpointAddress, req.Port, req.FriendSharedKey)) return true;
            bool stillValid;
            lock (gate)
            {
                stillValid = IsStillValidLocked(startGeneration, startLifetime, startToken);
                if (stillValid && Object.ReferenceEquals(pendingConnectRequest, req))
                {
                    pendingConnectRequest = null;
                    pendingAutoHost = false;
                    connectDeadlineMs = 0;
                    RenderOverlayLocked();
                }
            }
            if (autoHost && stillValid)
            {
                StopHostServer(bridge);
                lock (gate) { RenderOverlayLocked(); }   // the online status row is gone with the server
            }
            if (stillValid) Say(bridge, "Connection cancelled.");
            return false;
        }

        // Every place that stops the local server also closes the internet lobby (the bridge removes the UPnP mapping).
        private void StopHostServer(IGtaWorldBridge bridge)
        {
            string toClear;
            lock (gate)
            {
                onlineHostPort = 0; onlineResultHandled = false; onlineOpen = false; onlineStatusText = null;
                // Hosting ends: the invite is not kept anywhere. Clear the clipboard if it still holds what we put there.
                toClear = hostCopiedText ?? (hostCopyTicket != 0 ? hostCopyText : null);
                hostCopiedText = null; hostCopyTicket = 0; hostCopyText = null;
                hostInvite = null; hostPassword = null;
            }
            SafeText.ClearGroup(SafeText.GroupHost);
            if (toClear != null) lock (gate) { BeginTrackedClearLocked(bridge, toClear); }
            bridge.StopLocalHostServerIfStarted();
        }

        // Copies host text (invite, join code) to the clipboard on the worker thread; PollInviteFlows reports the result.
        private void BeginHostCopyLocked(string text, string okNotice, string failNotice)
        {
            hostCopyText = text; hostCopyOkNotice = okNotice; hostCopyFailNotice = failNotice;
            hostCopyStartedMs = NowMs;
            hostCopyTicket = worldBridge.BeginClipboard(ClipboardOpKind.CopyHidden, text, true);
            if (hostCopyTicket <= 0)
            {
                hostCopyTicket = 0;
                Say(failNotice);
            }
        }

        private void PollHostClipboardLocked(long now)
        {
            if (hostCopyTicket != 0)
            {
                ClipboardOpResult r = worldBridge.PollClipboard(hostCopyTicket);
                if (r.State == ClipboardOpState.Pending && now - hostCopyStartedMs > ClipboardOpTimeoutMs)
                    r = ClipboardOpResult.FailedResult;
                if (r.State == ClipboardOpState.Done)
                {
                    hostCopyTicket = 0;
                    TrackHostCopiedLocked(now);
                    Say(hostCopyOkNotice);
                }
                else if (r.State == ClipboardOpState.Failed)
                {
                    hostCopyTicket = 0;
                    // The text may be on the clipboard anyway (the read-back failed or timed out after the set): keep
                    // tracking it so it is cleared at expiry or when hosting stops. A clear is compare-and-clear, so
                    // tracking text that never arrived is harmless.
                    TrackHostCopiedLocked(now);
                    Say(hostCopyFailNotice);
                }
            }
            if (hostCopiedText != null && now >= hostCopiedClearAtMs)
            {
                BeginTrackedClearLocked(worldBridge, hostCopiedText);
                hostCopiedText = null;
            }
            PollClearTicketsLocked(now);
            PollStaleInviteLocked(now);
        }

        // The persisted expiry record is the only protection against a crash between "clear requested" and "clear done":
        // it is deleted only after the compare-and-clear worker reported completion (cleared, or no longer matching).
        // A failed or timed-out clear keeps the record: the next start retries.
        private sealed class ClearWatch { public int Ticket; public long StartedMs; public string Hash; }
        private readonly List<ClearWatch> clearWatches = new List<ClearWatch>();

        private void BeginTrackedClearLocked(IGtaWorldBridge bridge, string text)
        {
            if (string.IsNullOrEmpty(text)) return;
            int ticket = bridge.BeginClipboard(ClipboardOpKind.ClearIfEquals, text, true);
            if (ticket <= 0) return;   // the worker is gone: the record stays for the next start
            var watch = new ClearWatch { Ticket = ticket, StartedMs = NowMs, Hash = InviteExpiryStore.Hash(text) };
            if (!FinishClearIfDoneLocked(bridge, watch, NowMs)) clearWatches.Add(watch);
        }

        // True when the watch is finished (successfully or not) and can be dropped.
        private bool FinishClearIfDoneLocked(IGtaWorldBridge bridge, ClearWatch w, long now)
        {
            ClipboardOpResult r = bridge.PollClipboard(w.Ticket);
            if (r.State == ClipboardOpState.Pending) return now - w.StartedMs > ClipboardOpTimeoutMs;   // timed out: keep the record
            if (r.State == ClipboardOpState.Done) InviteExpiryStore.DeleteIfHash(w.Hash);
            return true;
        }

        private void PollClearTicketsLocked(long now)
        {
            for (int i = clearWatches.Count - 1; i >= 0; i--)
                if (FinishClearIfDoneLocked(worldBridge, clearWatches[i], now)) clearWatches.RemoveAt(i);
        }

        private void TrackHostCopiedLocked(long now)
        {
            hostCopiedText = hostCopyText;
            hostCopiedClearAtMs = now + InviteClipboardLifetimeMs;
            // A crash stops the timer above: the hash and the expiry time survive on disk (never the text).
            if (!string.IsNullOrEmpty(hostCopiedText))
                InviteExpiryStore.Write(hostCopiedText, DateTime.UtcNow.AddMilliseconds(InviteClipboardLifetimeMs));
        }

        // At start: an expiry record left by a session that did not end cleanly. Once its time has passed the clipboard is
        // read; only text that hashes to the stored value is cleared (anything the user copied since stays untouched).
        private void BeginStaleInviteCheckLocked()
        {
            string hash; DateTime expires;
            if (!InviteExpiryStore.TryRead(out hash, out expires)) return;
            staleInviteHash = hash; staleInviteExpiresUtc = expires; staleReadTicket = 0;
        }

        private void PollStaleInviteLocked(long now)
        {
            if (staleInviteHash == null) return;
            if (staleReadTicket == 0)
            {
                if (DateTime.UtcNow < staleInviteExpiresUtc) return;
                staleReadTicket = worldBridge.BeginClipboard(ClipboardOpKind.Read, null, true);
                staleReadStartedMs = now;
                if (staleReadTicket <= 0) { staleReadTicket = 0; staleInviteHash = null; }   // record stays for the next start
                return;
            }
            ClipboardOpResult r = worldBridge.PollClipboard(staleReadTicket);
            if (r.State == ClipboardOpState.Pending && now - staleReadStartedMs > ClipboardOpTimeoutMs) r = ClipboardOpResult.FailedResult;
            if (r.State == ClipboardOpState.Pending) return;
            staleReadTicket = 0;
            if (r.State == ClipboardOpState.Done)
            {
                if (!string.IsNullOrEmpty(r.Text) && string.Equals(InviteExpiryStore.Hash(r.Text), staleInviteHash, StringComparison.Ordinal))
                    BeginTrackedClearLocked(worldBridge, r.Text);   // the record goes when the clear is confirmed
                else
                    InviteExpiryStore.Delete();   // no longer there: nothing left to clear
            }
            staleInviteHash = null;
        }

        // The join side: the clipboard read finished. Returns the request to connect (outside the lock) or null.
        private ClientConnectRequest HandleJoinClipboardLocked(string text, JoinReadKind kind)
        {
            if (kind == JoinReadKind.Invite)
            {
                CoopInviteData data;
                InviteStatus parsed = CoopInvite.Parse(text, out data);
                if (parsed != InviteStatus.Ok)
                {
                    Say(CoopInvite.Describe(parsed));
                    return null;
                }
                SafeText.AddSecret(SafeText.GroupJoin, data.Password);
                SafeText.AddSecret(SafeText.GroupJoin, text);
                // A successfully parsed invite leaves the clipboard (if it still holds it), so it is not pasted into a chat by accident.
                worldBridge.BeginClipboard(ClipboardOpKind.ClearIfEquals, text, false);
                ClientConnectRequest req = CoopInvite.ToConnectRequest(data);
                if (!StartValidatedInGameConnectLocked(req, false)) return null;
                RenderOverlayLocked();
                return req;
            }
            string code = (text ?? string.Empty).Trim().ToUpperInvariant();
            if (!IsPlausibleSessionCode(code))
            {
                Say("Copy the join code your friend sent you, then choose this again.");
                return null;
            }
            if (!friendSessionAuthenticated || !API.IsOnServer) return null;
            SafeText.AddSecret(SafeText.GroupSession, code);
            worldBridge.BeginClipboard(ClipboardOpKind.ClearIfEquals, text, false);
            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionJoin), SessionProtocol.Version, code);
            RenderOverlayLocked();
            return null;
        }

        private static bool IsPlausibleSessionCode(string code)
        {
            if (string.IsNullOrEmpty(code) || code.Length < 4 || code.Length > 12) return false;
            for (int i = 0; i < code.Length; i++)
            {
                char c = code[i];
                if (!((c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9'))) return false;
            }
            return true;
        }

        // Runs at the start of every tick, outside `gate` for the transport call (RageCoop may block on its warning).
        private void PollInviteFlowsUnlocked()
        {
            ClientConnectRequest connectReq = null;
            IGtaWorldBridge bridge;
            int generation, life; Guid token;
            lock (gate)
            {
                if (!IsLiveLocked()) return;
                bridge = worldBridge;
                long now = NowMs;
                PollHostClipboardLocked(now);
                generation = joinReadGeneration; life = joinReadLifetime; token = joinReadToken;
                if (joinReadTicket != 0)
                {
                    if (!IsStillValidLocked(generation, life, token))
                    {
                        joinReadTicket = 0;   // disabled, restarted or handed off while reading: drop it silently
                    }
                    else
                    {
                        ClipboardOpResult r = bridge.PollClipboard(joinReadTicket);
                        if (r.State == ClipboardOpState.Pending && now - joinReadStartedMs > ClipboardOpTimeoutMs)
                            r = ClipboardOpResult.FailedResult;
                        if (r.State != ClipboardOpState.Pending)
                        {
                            joinReadTicket = 0;
                            if (r.State == ClipboardOpState.Failed) Say(ClipboardReadFailedNotice);
                            else connectReq = HandleJoinClipboardLocked(r.Text, joinReadKind);
                        }
                    }
                }
            }
            if (connectReq != null) ConnectPendingUnlocked(bridge, connectReq, false, generation, life, token);
        }

        private void PollOnlineHostingLocked()
        {
            if (onlineHostPort <= 0 || onlineResultHandled) return;
            OnlineHostStatus st = worldBridge.PollOnlineHosting();
            if (st == null) return;
            if (st.State == OnlineHostState.Open)
            {
                string code;
                string invite = null;
                if (LobbyCode.TryEncode(st.ExternalIp, onlineHostPort, out code))
                    invite = CoopInvite.BuildOnline(code, hostPassword);
                if (invite != null)
                {
                    onlineResultHandled = true;
                    onlineOpen = true;
                    hostInvite = invite;
                    SafeText.AddSecret(SafeText.GroupHost, code);
                    SafeText.AddSecret(SafeText.GroupHost, invite);
                    CoopTrace.Write("client", "online invite ready");
                    // One invite (lobby code + password) goes to the clipboard; the notice never shows it.
                    BeginHostCopyLocked(invite, InviteCopiedNotice + " " + st.Message, InviteCopyFailedNotice);
                    RenderOverlayLocked();
                    return;
                }
                st = new OnlineHostStatus(OnlineHostState.Failed, null,
                    "Your router reported an address that cannot be used for a lobby code. Use Tailscale, or forward UDP " + onlineHostPort + " yourself.",
                    "lobby code not encodable");
            }
            if (st.State == OnlineHostState.Failed)
            {
                onlineResultHandled = true;
                onlineStatusText = "not available (see the notice)";
                CoopTrace.Write("client", "online lobby failed: " + st.TraceNote);
                Say(st.Message);
                RenderOverlayLocked();
            }
        }

        private bool BeginFriendAuthLocked(ClientConnectRequest request, out PolicyDecision decision)
        {
            byte[] nonce = Guid.NewGuid().ToByteArray();
            friendSessionAuthenticated = false;
            assignedSlot = 0;
            latestSnapshot = null;
            pendingFriendChallenge = pendingFriendAccepted = null;
            pendingSnapshotBytes = null;
            if (!securityTracker.BeginFriendConnect(request, nonce, NowMs, out decision))
            {
                pendingAuthNonce = null;
                pendingFriendKey = string.Empty;
                SafeLogWarning("GTAxSkate Coop " + securityTracker.BuildStatusBanner());
                return false;
            }
            pendingAuthNonce = nonce;
            pendingFriendKey = request.FriendSharedKey;
            friendChallengeSeen = false;
            // The verification window starts with the auth attempt, not with the transport connect
            // (RageCoop readiness can use most of the connect window; Copilot review 2026-11-16).
            connectDeadlineMs = NowMs + ConnectTimeoutMs;
            CoopTrace.Write("client", "FriendAuthInit sent to a host (port " + request.Port + ")");
            pendingAuthRetryCount = 1;
            nextAuthAttemptMs = NowMs + 1500;
            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthInit), SessionProtocol.Version, nonce);
            return true;
        }

        private static bool IsExpectedTransport(ClientConnectRequest request)
        {
            IPEndPoint connected = API.ServerEndPoint;
            if (request == null || connected == null || connected.Port != request.Port) return false;
            string endpoint = request.EndpointAddress.Trim();
            if (string.Equals(endpoint, "localhost", StringComparison.OrdinalIgnoreCase))
                return IPAddress.IsLoopback(connected.Address);
            IPAddress wanted;
            if (!IPAddress.TryParse(endpoint, out wanted)) return false;
            IPAddress actual = connected.Address;
            if (wanted.IsIPv4MappedToIPv6) wanted = wanted.MapToIPv4();
            if (actual.IsIPv4MappedToIPv6) actual = actual.MapToIPv4();
            return actual.Equals(wanted);
        }

        private void ResetSessionStateLocked(string soloReason, long nowMs, bool clearPendingConnect)
        {
            pendingAuthNonce = null;
            pendingFriendKey = string.Empty;
            pendingFriendChallenge = pendingFriendAccepted = null;
            pendingSnapshotBytes = null;
            pendingWorldSyncBytes = null;
            pendingShopSyncBytes = null;
            pendingRagdollEvents.Clear();
            assignedSlot = 0;
            friendSessionAuthenticated = false;
            latestSnapshot = null;
            latestWorldState = null;
            latestShopState = null;
            peerCutsceneStandby = false;
            lastSentMissionActive = false;
            lastSentCutsceneActive = false;
            lastSentRagdoll = false;
            pendingAuthRetryCount = 0;
            friendChallengeSeen = false;
            if (clearPendingConnect)
            {
                pendingConnectRequest = null;
                pendingAutoHost = false;
                connectDeadlineMs = 0;
            }
            worldBridge.RestoreSoloDefaults();
            securityTracker.DisconnectToSolo(soloReason, nowMs);
        }

        private void Tick()
        {
            PollInviteFlowsUnlocked();
            TickLocked();
        }

        private void TickLocked()
        {
            lock (gate)
            {
                if (!IsLiveLocked() || worldBridge.IsGameLoading()) return;
                long now = NowMs;
                PollOnlineHostingLocked();
                if (!friendSessionAuthenticated && connectDeadlineMs != 0 && now >= connectDeadlineMs)
                {
                    CoopTrace.Write("client", "verification timed out: onServer=" + API.IsOnServer + " nonce=" + (pendingAuthNonce != null) +
                        " challengeSeen=" + friendChallengeSeen + " slot=" + assignedSlot + " inits=" + pendingAuthRetryCount);
                    BlockTransportLocked("Friend connection verification timed out.", now);
                    return;
                }
                if (!API.IsOnServer)
                {
                    // RageCoop gave up (e.g. "Failed to retrive server's public key"): end our pending connect now
                    // instead of blocking retries with "already active or pending" until the deadline (test 22:31).
                    bool transportGaveUp = false;
                    if (pendingConnectRequest != null && !wasOnServer)
                    {
                        if (worldBridge.IsTransportConnecting()) transportIdleSinceMs = 0;
                        else
                        {
                            if (transportIdleSinceMs == 0) transportIdleSinceMs = now;
                            transportGaveUp = now - connectStartedMs >= FailFastMinConnectMs && now - transportIdleSinceMs >= FailFastIdleMs;
                        }
                    }
                    else transportIdleSinceMs = 0;
                    if (transportGaveUp)
                    {
                        transportIdleSinceMs = 0;
                        CoopTrace.Write("client", "transport connect failed (RageCoop no longer connecting)");
                        ResetSessionStateLocked("Connection failed", now, true);
                        Say("Coop: could not reach the server. Check that the host is running and the invite is current.");
                        RenderOverlayLocked();
                        return;
                    }
                    if (wasOnServer || peerCutsceneStandby || friendSessionAuthenticated || latestSnapshot != null)
                    {
                        wasOnServer = false;
                        ResetSessionStateLocked("Disconnected -> Solo Story Mode", now, true);
                    }
                    RenderOverlayLocked();
                    return;
                }

                wasOnServer = true;
                transportIdleSinceMs = 0;

                // Fail-closed guard: block direct F9/external connections that bypassed policy validation.
                bool guard = true;
#if COOP_FIXTURE
                guard = !DiagnosticFixtureOnly;
#endif
                if (guard && !friendSessionAuthenticated && pendingAuthNonce == null)
                {
                    ClientConnectRequest req = pendingConnectRequest;
                    if (!IsExpectedTransport(req))
                    {
                        BlockTransportLocked("Unverified lobby disconnected.", now);
                        return;
                    }
                    PolicyDecision dec;
                    if (!BeginFriendAuthLocked(req, out dec))
                    {
                        BlockTransportLocked(dec.Reason, now);
                        return;
                    }
                }

                // Async FriendAuthInit retry in case server wasn't Client.IsReady on tick 0 after API.Connect.
                // Resent until the server answers or the connect deadline ends (RageCoop marks the peer
                // ready only after its resource download, which can take longer than 5 x 1.5 s).
                if (!friendSessionAuthenticated && pendingAuthNonce != null && !friendChallengeSeen &&
                    pendingFriendChallenge == null && now >= nextAuthAttemptMs)
                {
                    pendingAuthRetryCount++;
                    nextAuthAttemptMs = now + 1500;
                    API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthInit), SessionProtocol.Version, pendingAuthNonce);
                    CoopTrace.Write("client", "FriendAuthInit resent #" + pendingAuthRetryCount);
                }

                if (pendingChallenge != null)
                {
                    var response = pendingChallenge; pendingChallenge = null;
                    if ((int)response[3] != API.LocalPlayerID) return;
                    session = (byte[])response[2];
                    API.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Ack), 1, challenge, session);
                }
                if (pendingResult != null)
                {
                    pendingResult = null; responseLogged = true;
                    SafeLogInfo("GTAxSkate Coop roundtrip response received (diagnostic only).");
                }
                if (pendingRejectedReason != null)
                {
                    string reason = pendingRejectedReason;
                    pendingRejectedReason = null;
                    Say("Coop: " + reason);
                    if (!friendSessionAuthenticated)
                    {
                        ResetSessionStateLocked("Rejected: " + reason, now, true);
                        worldBridge.DisconnectTransport();
                        RenderOverlayLocked();
                        return;
                    }
                    SafeLogWarning("GTAxSkate Coop server rejected: " + reason);
                }
                if (pendingFriendChallenge != null)
                {
                    var fch = pendingFriendChallenge; pendingFriendChallenge = null;
                    byte[] cNonce = (byte[])fch[1];
                    byte[] sNonce = (byte[])fch[2];
                    int slot = (int)fch[3];
                    byte[] srvProof = (byte[])fch[4];

                    if (securityTracker.CompleteMutualServerVerification(cNonce, sNonce, srvProof, string.Empty, 1, now))
                    {
                        byte[] cliProof = CoopSecurityPolicy.ComputeMutualProof(pendingFriendKey, cNonce, sNonce, "CLI");
                        pendingFriendKey = string.Empty;
                        assignedSlot = slot;
                        API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthProof),
                            SessionProtocol.Version, cNonce, sNonce, cliProof);
                        SafeLogInfo("GTAxSkate Coop mutual server verified; proof sent for slot " + slot);
                    }
                    else
                    {
                        ResetSessionStateLocked("Server auth mismatch", now, true);
                        worldBridge.DisconnectTransport();
                        RenderOverlayLocked();
                        return;
                    }
                }
                if (pendingFriendAccepted != null)
                {
                    var facc = pendingFriendAccepted; pendingFriendAccepted = null;
                    if ((int)facc[2] == assignedSlot && assignedSlot > 0)
                    {
                        friendSessionAuthenticated = true;
                        pendingConnectRequest = null;
                        connectDeadlineMs = 0;
                        pendingAuthNonce = null;
                        SafeLogInfo("GTAxSkate Coop " + securityTracker.BuildStatusBanner());
                        Say("Friends Coop Verified! " + securityTracker.BuildStatusBanner());
                        if (localModManifest != null)
                        {
                            byte[] encoded = SessionProtocol.EncodeModsManifest(localModManifest);
                            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionModsReport),
                                SessionProtocol.Version, encoded);
                        }
                        if (pendingAutoHost)
                        {
                            pendingAutoHost = false;
                            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHost),
                                SessionProtocol.Version, (int)SkateWorldZone.LosSantos, (int)SkateMode.FreeSkate, "Story+Skate Crew");
                        }
                    }
                }
                if (pendingModWarnings.Count > 0)
                {
                    for (int i = 0; i < pendingModWarnings.Count; i++)
                    {
                        string w = pendingModWarnings[i];
                        if (!latestModWarnings.Contains(w) && latestModWarnings.Count < 16) latestModWarnings.Add(w);
                        SafeLogWarning("GTAxSkate Coop " + w);
                        Say("Mod Parity Warning: " + w);
                    }
                    pendingModWarnings.Clear();
                }
                if (pendingSnapshotBytes != null)
                {
                    byte[] raw = pendingSnapshotBytes; pendingSnapshotBytes = null;
                    SkateSessionSnapshot decoded;
                    if (SessionProtocol.TryDecodeSnapshot(raw, now, out decoded))
                    {
                        if (latestSnapshot != null &&
                            (decoded.HostSlot != latestSnapshot.HostSlot || decoded.JoinCode != latestSnapshot.JoinCode))
                            ClearHostWorldLocked();
                        latestSnapshot = decoded;
                        SafeText.AddSecret(SafeText.GroupSession, decoded.JoinCode);   // never drawn; masked if it ever reaches a string
                        securityTracker.UpdateLivePeerCount(decoded.Peers.Length, now);
                    }
                }

                ProcessWorldAndGameplaySyncLocked(now);
                RenderOverlayLocked();

                if (session == null && attempts < 3 && now >= nextHello)
                {
                    var version = API.CurrentVersion;
                    if (version == null || version.Major != 1 || version.Minor != 5 || version.Build != 12)
                    {
                        attempts = 3;
                        SafeLogWarning("GTAxSkate Coop unsupported RAGECOOP API; handshake disabled."); return;
                    }
                    attempts++; nextHello = now + 5000;
                    API.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Hello), 1, challenge,
                        GetModuleHandleW("SkateVLegacy.asi") != IntPtr.Zero,
                        GetModuleHandleW("SkateVRuntime.dll") != IntPtr.Zero, false);
                }
            }
        }

        private void BlockTransportLocked(string reason, long now)
        {
            ResetSessionStateLocked(reason, now, true);
            securityTracker.FailBlocked(reason, now);
            Say("BLOCKED: " + reason);
            worldBridge.DisconnectTransport();
            RenderOverlayLocked();
        }

        private void ClearHostWorldLocked()
        {
            latestWorldState = null;
            latestShopState = null;
            pendingWorldSyncBytes = null;
            pendingShopSyncBytes = null;
            pendingRagdollEvents.Clear();
            peerCutsceneStandby = false;
            worldBridge.RestoreSoloDefaults();
        }

        private void ProcessWorldAndGameplaySyncLocked(long now)
        {
            if (!friendSessionAuthenticated || latestSnapshot == null || assignedSlot <= 0) return;

            bool isHost = latestSnapshot.HostSlot == assignedSlot;
            if (isHost)
            {
                if (peerCutsceneStandby)
                {
                    peerCutsceneStandby = false;
                    worldBridge.ExitPeerCutsceneStandbyIfNeeded(latestWorldState, false);
                }

                HostWorldSyncState sampled = worldBridge.CaptureHostWorldState(worldSyncEpoch + 1);
                if (sampled != null && sampled.IsValid())
                {
                    bool transition = (sampled.MissionActive != lastSentMissionActive) ||
                        (sampled.CutsceneActive != lastSentCutsceneActive);
                    if (transition || now >= nextWorldBroadcastMs)
                    {
                        worldSyncEpoch++;
                        var publish = new HostWorldSyncState(
                            worldSyncEpoch, sampled.ClockHour, sampled.ClockMinute, sampled.ClockPaused, sampled.WeatherId,
                            sampled.MissionActive, sampled.CutsceneActive, sampled.HostInVehicle, sampled.WantedLevel,
                            sampled.WantedCoordX, sampled.WantedCoordY, sampled.WantedCoordZ,
                            sampled.HostPosX, sampled.HostPosY, sampled.HostPosZ,
                            sampled.HasObjective, sampled.ObjectiveX, sampled.ObjectiveY, sampled.ObjectiveZ,
                            sampled.ObjectiveSprite, sampled.ObjectiveColor, sampled.AllowShopPurchase);
                        byte[] encoded = SessionProtocol.EncodeWorldSync(publish);
                        if (encoded != null)
                        {
                            latestWorldState = publish;
                            lastSentMissionActive = publish.MissionActive;
                            lastSentCutsceneActive = publish.CutsceneActive;
                            nextWorldBroadcastMs = now + 200;
                            API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostWorldSync), SessionProtocol.Version, encoded);
                        }
                    }
                }
            }
            else
            {
                if (pendingWorldSyncBytes != null)
                {
                    byte[] rawW = pendingWorldSyncBytes; pendingWorldSyncBytes = null;
                    HostWorldSyncState decodedW;
                    if (SessionProtocol.TryDecodeWorldSync(rawW, out decodedW))
                    {
                        latestWorldState = decodedW;
                        lastHostWorldRxMs = now;
                        if (decodedW.CutsceneActive)
                        {
                            peerCutsceneStandby = true;
                        }
                        else if (peerCutsceneStandby)
                        {
                            peerCutsceneStandby = false;
                            if (ApplyWorldEffects) worldBridge.ExitPeerCutsceneStandbyIfNeeded(decodedW, true);
                        }
                    }
                }

                if (latestWorldState != null && now - lastHostWorldRxMs > 4000)
                {
                    ClearHostWorldLocked();
                    Say("Host world sync paused.");
                }

                if (ApplyWorldEffects && latestWorldState != null)
                    worldBridge.ApplyPeerWorldState(latestWorldState);

                if (pendingShopSyncBytes != null)
                {
                    byte[] rawS = pendingShopSyncBytes; pendingShopSyncBytes = null;
                    HostShopSyncState decodedS;
                    if (SessionProtocol.TryDecodeShopSync(rawS, out decodedS))
                    {
                        latestShopState = decodedS;
                        if (ApplyWorldEffects) worldBridge.ApplyPeerShopState(decodedS);
                    }
                }
            }

            // Capture and relay local ragdoll bails/falls cleanly.
            LocalRagdollSample rag = worldBridge.CaptureLocalRagdoll();
            if (rag != null && (rag.IsRagdoll != lastSentRagdoll || (rag.IsRagdoll && now >= nextRagdollBroadcastMs)))
            {
                var ragState = new RagdollSyncState(
                    assignedSlot, rag.IsRagdoll, rag.PosX, rag.PosY, rag.PosZ,
                    rag.VelX, rag.VelY, rag.VelZ, rag.IsRagdoll ? 1200 : 0);
                byte[] ragBytes = SessionProtocol.EncodeRagdollSync(ragState);
                if (ragBytes != null)
                {
                    lastSentRagdoll = rag.IsRagdoll;
                    nextRagdollBroadcastMs = now + 80;
                    API.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRagdollSync), SessionProtocol.Version, ragBytes);
                }
            }

            if (pendingRagdollEvents.Count > 0)
            {
                for (int i = 0; i < pendingRagdollEvents.Count; i++)
                {
                    object[] item = pendingRagdollEvents[i];
                    int srcSlot = (int)item[0];
                    byte[] rawRag = (byte[])item[1];
                    RagdollSyncState decodedRag;
                    if (ApplyWorldEffects && srcSlot != assignedSlot && SessionProtocol.TryDecodeRagdollSync(rawRag, srcSlot, out decodedRag))
                        worldBridge.ApplyRemotePlayerRagdoll(srcSlot, "Slot#" + srcSlot, decodedRag);
                }
                pendingRagdollEvents.Clear();
            }
        }

        private void RenderOverlayLocked()
        {
            if (!IsLiveLocked() || worldBridge.IsGameLoading()) return;
            string subtitle = string.Empty;
            if (latestSnapshot != null)
            {
                string role = latestSnapshot.HostSlot == assignedSlot ? "HOST" : "CREW";
                string mission = latestWorldState != null && latestWorldState.CutsceneActive
                    ? " | CUTSCENE STANDBY"
                    : (latestWorldState != null && latestWorldState.MissionActive ? " | MISSION ACTIVE" : string.Empty);
                // The session join code is a secret for a stream: players, never the code.
                subtitle = role + " Lobby (" + latestSnapshot.Peers.Length +
                    "/" + SkateSessionCoordinator.MaxPlayersPerSession + ")" + mission;
            }
            CoopMenuItem[] items = menuState.IsOpen ? BuildMenuItemsLocked() : new CoopMenuItem[0];
            var safeItems = new CoopMenuItem[items.Length];
            for (int i = 0; i < items.Length; i++)
                safeItems[i] = new CoopMenuItem(items[i].Action, SafeText.ForScreen(items[i].Label), items[i].Enabled);
            string[] warnings = latestModWarnings.ToArray();
            for (int i = 0; i < warnings.Length; i++) warnings[i] = SafeText.ForScreen(warnings[i]);
            worldBridge.RenderOverlay(new CoopOverlayView(
                menuState.IsOpen,
                SafeText.ForScreen(securityTracker.BuildStatusBanner()),
                SafeText.ForScreen(subtitle),
                warnings,
                safeItems,
                menuState.SelectedIndex));
        }

        public override void OnStop()
        {
            StopInternal(true);
        }

        // Explicit "Disable Multiplayer": advance the runtime generation FIRST (queued callbacks now
        // fail closed), persist the choice, then tear down. Stops only the server this integration started.
        public void DisableMultiplayer()
        {
            string persistError;
            bool saved = CoopRuntimeControl.SetEnabled(false, out persistError);
            StopForDisable();
            if (!saved) SafeLogWarning("GTAxSkate Coop: could not save multiplayer preference: " + persistError);
        }

        // Teardown after the runtime control was already switched off (menu or bootstrap path).
        public void StopForDisable()
        {
            IGtaWorldBridge bridge;
            lock (gate) { bridge = worldBridge; }
            try { if (bridge != null) bridge.DisconnectTransport(); } catch { }
            StopInternal(true);
        }

        // Another loaded copy owns the shared token: stop this copy WITHOUT touching the transport or the server
        // (both belong to the live owner). Same teardown as an owner handoff.
        public void StepDownDisplaced()
        {
            StopInternal(false);
        }

        // stopServer: false = owner handoff (keeps the running local server and does not dispose the
        // bridge, whose static server handle the new owner's bridge reuses); true = Disable / OnStop
        // (stops the server this integration started, independent of loading state, then disposes).
        private void StopInternal(bool stopServer)
        {
            lock (activeOwnerGate)
            {
                if (ReferenceEquals(activeOwner, this))
                    activeOwner = null;
                // Only the current owner releases the shared token; a displaced copy leaves the owner alone
                // and never stops the server the owner uses (unless multiplayer was switched off).
                if (ownerToken != Guid.Empty)
                {
                    CoopRuntimeControl.ReleaseOwner(ownerToken);
                    if (stopServer && CoopRuntimeControl.Enabled && CoopRuntimeControl.HasOtherOwner(ownerToken))
                        stopServer = false;
                }
            }
            lock (gate)
            {
                // Never started (e.g. an unsolicited OnStart refused while multiplayer was off): nothing to undo.
                if (!everStarted) return;
                active = false; lifetime++;
                challenge = session = null; pendingChallenge = pendingResult = null;
                pendingModWarnings.Clear(); latestModWarnings.Clear(); localModManifest = null;
                ResetSessionStateLocked("Stopped", NowMs, stopServer);
                API.Events.OnTick -= OnRageCoopTick;
                API.Events.OnTick -= Tick;
                if (stopServer && worldBridge != null)
                {
                    try { ApplyRageCoopUiLock(worldBridge, false); } catch { }
                    // Masks for the join password and the session code stay until multiplayer stops (a rejected join must not unmask them).
                    SafeText.ClearGroup(SafeText.GroupJoin);
                    SafeText.ClearGroup(SafeText.GroupSession);
                    try { StopHostServer(worldBridge); } catch { }
                    if (worldBridge is IDisposable) ((IDisposable)worldBridge).Dispose();
                }
                worldBridge = new NullGtaWorldBridge();
            }
        }
    }
}
