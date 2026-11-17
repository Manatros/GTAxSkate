using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Xml;
using RageCoop.Core.Scripting;
using RageCoop.Server;
using RageCoop.Server.Scripting;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class ServerAdapter : ServerScript
    {
        private sealed class Pending
        {
            internal Client Owner;
            internal byte[] Challenge, Session;
            internal long Started, LastReply;
            internal bool Complete;
        }
        private sealed class FriendPeerAuth
        {
            internal Client Owner;
            internal int Slot;
            internal byte[] ClientNonce;
            internal byte[] ServerNonce;
            internal long StartedMs;
            internal bool Authenticated;
            internal ModEntry[] ModManifest;
        }
        private readonly object gate = new object();
        private readonly Dictionary<int, Pending> peers = new Dictionary<int, Pending>();
        private readonly Dictionary<int, FriendPeerAuth> friendAuth = new Dictionary<int, FriendPeerAuth>();
        private readonly PeerSlotMap slotMap = new PeerSlotMap();
        private readonly SkateSessionCoordinator sessions = new SkateSessionCoordinator();
        private readonly Dictionary<string, byte[]> cachedWorldByCode = new Dictionary<string, byte[]>(StringComparer.Ordinal);
        private readonly Dictionary<string, byte[]> cachedShopByCode = new Dictionary<string, byte[]>(StringComparer.Ordinal);
        private readonly Dictionary<int, long> lastWorldSyncMs = new Dictionary<int, long>();
        private readonly Dictionary<int, long> lastShopSyncMs = new Dictionary<int, long>();
        private readonly Dictionary<int, long> lastRagdollSyncMs = new Dictionary<int, long>();
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private ServerSecurityConfig securityConfig = ServerSecurityConfig.DefaultPrivateFriends(string.Empty);
        private string bindAddress = "127.0.0.1";
        private bool securityValid;
        private byte[] transportPasswordHash;
        private readonly PublicHandshakeGate publicGate = new PublicHandshakeGate();
        private bool active;
        private int lifetime;

        public void ConfigureFriendSecurity(ServerSecurityConfig config, string serverBindAddress)
        {
            lock (gate)
            {
                securityConfig = config ?? ServerSecurityConfig.DefaultPrivateFriends(string.Empty);
                bindAddress = serverBindAddress ?? "127.0.0.1";
                PolicyDecision check = CoopSecurityPolicy.ValidateServerConfig(securityConfig, bindAddress);
                securityValid = check.Allowed;
                using (var sha = SHA256.Create())
                    transportPasswordHash = securityValid
                        ? sha.ComputeHash(Encoding.UTF8.GetBytes(securityConfig.FriendSharedKey))
                        : null;
                if (!securityValid)
                    Logger.Warning("GTAxSkate Coop friend session server locked down: " + check.Reason);
                else
                    Logger.Info("GTAxSkate Coop private friend server security configured (" + check.TrustClass + ").");
                CoopTrace.Write("server", securityValid ? "friend security configured (" + check.TrustClass + ")" : "locked down: " + check.Reason);
            }
        }

        public override void OnStart()
        {
            LoadLocalServerSecurity();
            int epoch;
            lock (gate)
            {
                active = true; epoch = ++lifetime;
                peers.Clear(); friendAuth.Clear(); slotMap.Clear();
                cachedWorldByCode.Clear(); cachedShopByCode.Clear();
                lastWorldSyncMs.Clear(); lastShopSyncMs.Clear(); lastRagdollSyncMs.Clear();
                PolicyDecision check = CoopSecurityPolicy.ValidateServerConfig(securityConfig, bindAddress);
                securityValid = check.Allowed;
            }
            API.RegisterCustomEventHandler(DiagnosticProtocol.Hello, e => Hello(e, epoch));
            API.RegisterCustomEventHandler(DiagnosticProtocol.Ack, e => Ack(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.FriendAuthInit, e => OnFriendAuthInit(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.FriendAuthProof, e => OnFriendAuthProof(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionHost, e => OnSessionHost(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionJoin, e => OnSessionJoin(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionLeave, e => OnSessionLeave(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionConfigure, e => OnSessionConfigure(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionStart, e => OnSessionStart(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionBoard, e => OnSessionBoard(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionScore, e => OnSessionScore(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionModsReport, e => OnSessionModsReport(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionHostWorldSync, e => OnHostWorldSync(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionHostShopSync, e => OnHostShopSync(e, epoch));
            API.RegisterCustomEventHandler(SessionProtocol.SessionRagdollSync, e => OnRagdollSync(e, epoch));
            API.Events.OnPlayerDisconnected += Disconnected;
            API.Events.OnPlayerHandshake += AdmitPrivateTransport;
            Logger.Info("GTAxSkate Coop server adapter started; diagnostic + fail-closed friend-session coordinator ready.");
        }

        private void LoadLocalServerSecurity()
        {
            string key = Environment.GetEnvironmentVariable("GTAXSKATE_COOP_FRIEND_KEY");
            if (string.IsNullOrWhiteSpace(key)) return;
            try
            {
                var settings = new XmlDocument { XmlResolver = null };
                using (var input = XmlReader.Create(Path.Combine(Environment.CurrentDirectory, "Settings.xml"),
                    new XmlReaderSettings { DtdProcessing = DtdProcessing.Prohibit, MaxCharactersInDocument = 65536 }))
                    settings.Load(input);
                foreach (string flag in new[] { "AnnounceSelf", "UseP2P", "UseZeroTier", "AutoUpdate" })
                {
                    XmlNode node = settings.SelectSingleNode("/Settings/" + flag);
                    bool enabled;
                    if (node == null || !bool.TryParse(node.InnerText, out enabled) || enabled)
                        throw new InvalidDataException("Settings.xml must explicitly disable " + flag + ".");
                }

                // The stock server listens on all interfaces. The admission
                // hook below restricts remote addresses before entity sync.
                ConfigureFriendSecurity(ServerSecurityConfig.DefaultPrivateFriends(key), "127.0.0.1");
            }
            catch (Exception e)
            {
                ConfigureFriendSecurity(null, null);
                Logger.Warning("GTAxSkate Coop host configuration rejected: " + e.Message);
            }
        }

        // Loopback/LAN/VPN endpoints: the transport password hash must match. Public internet endpoints (the
        // "Host online" lobby, design 2026-11-16-coop-internet-lobby) are admitted by the same password check but
        // pass the failure throttle first. The address itself is never traced, only its class.
        private void AdmitPrivateTransport(object sender, HandshakeEventArgs handshake)
        {
            lock (gate)
            {
                var trust = handshake.EndPoint == null ? EndpointTrustClass.Invalid
                    : CoopSecurityPolicy.ClassifyEndpoint(handshake.EndPoint.Address.ToString());
                if (!active || !securityValid || transportPasswordHash == null || trust == EndpointTrustClass.Invalid)
                {
                    handshake.Deny("This server is limited to verified private friends.");
                    CoopTrace.Write("server", "handshake denied: active=" + active + " securityValid=" + securityValid +
                        " key=" + (transportPasswordHash != null) + " trust=" + trust);
                    return;
                }
                bool isPublic = trust == EndpointTrustClass.PublicInternet;
                string address = isPublic ? handshake.EndPoint.Address.ToString() : null;
                long now = clock.ElapsedMilliseconds;
                string lockReason;
                if (isPublic && !publicGate.TryBegin(address, now, out lockReason))
                {
                    handshake.Deny("This server is limited to verified private friends.");
                    CoopTrace.Write("server", "public handshake refused: " + lockReason);
                    return;
                }
                bool matches = CoopSecurityPolicy.FixedTimeEquals(transportPasswordHash, DecodePasswordHash(handshake.PasswordHash));
                if (!matches)
                {
                    handshake.Deny("This server is limited to verified private friends.");
                    CoopTrace.Write("server", "handshake denied: wrong transport password trust=" + trust);
                    if (isPublic)
                    {
                        publicGate.RecordFailure(address, now);
                        if (publicGate.LastEvent != null) CoopTrace.Write("server", "public handshake " + publicGate.LastEvent);
                    }
                    return;
                }
                if (isPublic)
                {
                    publicGate.RecordSuccess(address);
                    CoopTrace.Write("server", "public handshake admitted (transport password matched)");
                }
            }
        }

        private static byte[] DecodePasswordHash(string hex)
        {
            if (hex == null || hex.Length != 64) return null;
            var hash = new byte[32];
            for (int i = 0; i < hash.Length; i++)
                if (!byte.TryParse(hex.Substring(i * 2, 2), System.Globalization.NumberStyles.AllowHexSpecifier,
                    System.Globalization.CultureInfo.InvariantCulture, out hash[i])) return null;
            return hash;
        }
        private int Source(Client client)
        {
            if (client == null || !client.IsReady || client.Player == null || client.Player.ID == 0) return 0;
            Client actual;
            return API.GetAllClients().TryGetValue(client.Player.ID, out actual) &&
                Object.ReferenceEquals(actual, client) ? client.Player.ID : 0;
        }
        private void Hello(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int id = Source(e.Client);
                if (!active || epoch != lifetime || id == 0 || !DiagnosticProtocol.Header(e.Args, 5) ||
                    !DiagnosticProtocol.Nonce(e.Args[1]) || !(e.Args[2] is bool) || !(e.Args[3] is bool) ||
                    !(e.Args[4] is bool) || (bool)e.Args[4]) return;
                long now = clock.ElapsedMilliseconds;
                var expired = new List<int>();
                foreach (var peer in peers)
                    if (now - peer.Value.Started > 15000) expired.Add(peer.Key);
                foreach (var source in expired) peers.Remove(source);
                Pending pending;
                if (peers.TryGetValue(id, out pending))
                {
                    if (!Object.ReferenceEquals(pending.Owner, e.Client)) { peers.Remove(id); pending = null; }
                    else if (!DiagnosticProtocol.Equal(pending.Challenge, (byte[])e.Args[1]) ||
                        now - pending.LastReply < 1000) return;
                }
                if (pending == null)
                {
                    if (peers.Count >= 64) return;
                    pending = new Pending { Owner = e.Client, Challenge = (byte[])((byte[])e.Args[1]).Clone(),
                        Session = Guid.NewGuid().ToByteArray(), Started = now };
                    peers[id] = pending;
                    Logger.Info("GTAxSkate Coop hello source=" + id + " hostLoaded=" + e.Args[2] +
                        " runtimeLoaded=" + e.Args[3] + "; reported presence only, pose disabled.");
                }
                pending.LastReply = now;
                e.Client.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Challenge), 1,
                    pending.Challenge, pending.Session, id, false);
            }
        }
        private void Ack(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int id = Source(e.Client); Pending pending;
                if (!active || epoch != lifetime || id == 0 || !DiagnosticProtocol.Header(e.Args, 3) ||
                    !DiagnosticProtocol.Nonce(e.Args[1]) || !DiagnosticProtocol.Nonce(e.Args[2]) ||
                    !peers.TryGetValue(id, out pending) || pending.Complete ||
                    !Object.ReferenceEquals(pending.Owner, e.Client) ||
                    clock.ElapsedMilliseconds - pending.Started > 15000 ||
                    !DiagnosticProtocol.Equal(pending.Challenge, (byte[])e.Args[1]) ||
                    !DiagnosticProtocol.Equal(pending.Session, (byte[])e.Args[2])) return;
                pending.Complete = true;
                Logger.Info("GTAxSkate Coop diagnostic handshake accepted source=" + id + "; remote pose disabled.");
                e.Client.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Result), 1,
                    pending.Challenge, pending.Session, true);
            }
        }
        private void ReleaseSlotStateLocked(int slot)
        {
            if (slot <= 0) return;
            lastWorldSyncMs.Remove(slot);
            lastShopSyncMs.Remove(slot);
            lastRagdollSyncMs.Remove(slot);

            SkateSessionSnapshot beforeLeave;
            bool hadSession = sessions.TryGetSnapshotForPeer(slot, clock.ElapsedMilliseconds, out beforeLeave);
            bool closed;
            int newHost;
            if (sessions.TryLeaveOrDisconnect(slot, out closed, out newHost))
            {
                if (hadSession && beforeLeave != null && (closed || beforeLeave.HostSlot == slot))
                {
                    cachedWorldByCode.Remove(beforeLeave.JoinCode);
                    cachedShopByCode.Remove(beforeLeave.JoinCode);
                }
                if (!closed) BroadcastSnapshotToSessionMembersLocked(newHost);
            }
        }

        private void Disconnected(object sender, Client client)
        {
            lock (gate)
            {
                var ids = new List<int>();
                foreach (var peer in peers) if (Object.ReferenceEquals(peer.Value.Owner, client)) ids.Add(peer.Key);
                foreach (int id in ids) peers.Remove(id);

                var authIds = new List<int>();
                foreach (var pair in friendAuth) if (Object.ReferenceEquals(pair.Value.Owner, client)) authIds.Add(pair.Key);
                foreach (int rawId in authIds)
                {
                    FriendPeerAuth auth = friendAuth[rawId];
                    friendAuth.Remove(rawId);
                    int releasedSlot;
                    if (slotMap.TryReleaseByRawId(rawId, out releasedSlot))
                        ReleaseSlotStateLocked(releasedSlot);
                    else if (auth.Slot > 0)
                        ReleaseSlotStateLocked(auth.Slot);
                }
            }
        }

        private void OnFriendAuthInit(CustomEventReceivedArgs e, int epoch)
        {
            CoopTrace.Write("server", "FriendAuthInit received (job start)");
            try { OnFriendAuthInitCore(e, epoch); }
            catch (Exception ex) { CoopTrace.Write("server", "FriendAuthInit failed: " + ex); throw; }
        }

        private void OnFriendAuthInitCore(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int rawId = Source(e.Client);
                if (!active || epoch != lifetime || rawId == 0 || !SessionProtocol.Header(e.Args, 2) ||
                    !DiagnosticProtocol.Nonce(e.Args[1]))
                {
                    CoopTrace.Write("server", "FriendAuthInit ignored: active=" + active + " epoch=" + (epoch == lifetime) +
                        " source=" + rawId + " (0 = client not ready)");
                    return;
                }

                FriendPeerAuth existing;
                if (friendAuth.TryGetValue(rawId, out existing) && existing.Authenticated)
                {
                    CoopTrace.Write("server", "FriendAuthInit ignored: source=" + rawId + " already authenticated");
                    return; // Already authenticated; ignore duplicate AuthInit replay.
                }
                // A resend of the same attempt gets the same challenge again (a fresh server nonce here
                // invalidated the proof the client was already computing; 2026-11-16 evening test).
                if (existing != null && securityValid && Object.ReferenceEquals(existing.Owner, e.Client) &&
                    DiagnosticProtocol.Equal(existing.ClientNonce, (byte[])e.Args[1]) &&
                    clock.ElapsedMilliseconds - existing.StartedMs <= 15000)
                {
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthChallenge),
                        SessionProtocol.Version, existing.ClientNonce, existing.ServerNonce, existing.Slot,
                        CoopSecurityPolicy.ComputeMutualProof(securityConfig.FriendSharedKey, existing.ClientNonce, existing.ServerNonce, "SRV"));
                    CoopTrace.Write("server", "FriendAuthInit resend answered with the same challenge source=" + rawId);
                    return;
                }

                if (!securityValid)
                {
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRejected),
                        SessionProtocol.Version, "Server friend-security key not configured or insecure bind.");
                    return;
                }

                int slot;
                if (!slotMap.TryAssignOrGetSlot(rawId, out slot))
                {
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRejected),
                        SessionProtocol.Version, "Server peer slot limit reached.");
                    return;
                }

                byte[] clientNonce = (byte[])((byte[])e.Args[1]).Clone();
                byte[] serverNonce = Guid.NewGuid().ToByteArray();
                byte[] srvProof = CoopSecurityPolicy.ComputeMutualProof(
                    securityConfig.FriendSharedKey, clientNonce, serverNonce, "SRV");

                friendAuth[rawId] = new FriendPeerAuth
                {
                    Owner = e.Client,
                    Slot = slot,
                    ClientNonce = clientNonce,
                    ServerNonce = serverNonce,
                    StartedMs = clock.ElapsedMilliseconds,
                    Authenticated = false
                };

                e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthChallenge),
                    SessionProtocol.Version, clientNonce, serverNonce, slot, srvProof);
                CoopTrace.Write("server", "FriendAuthChallenge sent source=" + rawId + " slot=" + slot);
            }
        }

        private void OnFriendAuthProof(CustomEventReceivedArgs e, int epoch)
        {
            CoopTrace.Write("server", "FriendAuthProof received (job start)");
            try { OnFriendAuthProofCore(e, epoch); }
            catch (Exception ex) { CoopTrace.Write("server", "FriendAuthProof failed: " + ex); throw; }
        }

        private void OnFriendAuthProofCore(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int rawId = Source(e.Client);
                FriendPeerAuth auth;
                if (!active || epoch != lifetime || rawId == 0 || !securityValid ||
                    !SessionProtocol.Header(e.Args, 4) || !DiagnosticProtocol.Nonce(e.Args[1]) ||
                    !DiagnosticProtocol.Nonce(e.Args[2]) || !SessionProtocol.Proof(e.Args[3]) ||
                    !friendAuth.TryGetValue(rawId, out auth) || auth.Authenticated ||
                    !Object.ReferenceEquals(auth.Owner, e.Client) ||
                    clock.ElapsedMilliseconds - auth.StartedMs > 15000 ||
                    !DiagnosticProtocol.Equal(auth.ClientNonce, (byte[])e.Args[1]) ||
                    !DiagnosticProtocol.Equal(auth.ServerNonce, (byte[])e.Args[2])) return;

                if (!CoopSecurityPolicy.VerifyMutualProof(
                    securityConfig.FriendSharedKey, auth.ClientNonce, auth.ServerNonce, "CLI", (byte[])e.Args[3]))
                {
                    friendAuth.Remove(rawId);
                    int released;
                    slotMap.TryReleaseByRawId(rawId, out released);
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRejected),
                        SessionProtocol.Version, "Friend key verification failed.");
                    Logger.Warning("GTAxSkate Coop rejected unauthenticated peer source=" + rawId);
                    CoopTrace.Write("server", "friend proof wrong source=" + rawId);
                    return;
                }

                auth.Authenticated = true;
                CoopTrace.Write("server", "friend auth verified source=" + rawId + " slot=" + auth.Slot);
                Logger.Info("GTAxSkate Coop mutual friend auth verified source=" + rawId + " slot=" + auth.Slot);
                e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.FriendAuthAccepted),
                    SessionProtocol.Version, auth.ClientNonce, auth.Slot);
            }
        }

        private bool TryAuthenticatedSlotLocked(CustomEventReceivedArgs e, int epoch, out int slot)
        {
            slot = 0;
            int rawId = Source(e.Client);
            FriendPeerAuth auth;
            if (!active || epoch != lifetime || rawId == 0 || !securityValid ||
                !friendAuth.TryGetValue(rawId, out auth) || !auth.Authenticated ||
                !Object.ReferenceEquals(auth.Owner, e.Client))
                return false;
            slot = auth.Slot;
            return slot > 0;
        }

        private void OnSessionHost(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 4) ||
                    !(e.Args[1] is int) || !(e.Args[2] is int) || !SessionProtocol.BoundedText(e.Args[3], SkateSessionCoordinator.MaxTitleChars))
                    return;

                int zoneRaw = (int)e.Args[1], modeRaw = (int)e.Args[2];
                if (!Enum.IsDefined(typeof(SkateWorldZone), zoneRaw) || !Enum.IsDefined(typeof(SkateMode), modeRaw)) return;

                string code;
                if (!sessions.TryCreateSession(slot, (SkateWorldZone)zoneRaw, (SkateMode)modeRaw,
                    (string)e.Args[3], clock.ElapsedMilliseconds, out code))
                {
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRejected),
                        SessionProtocol.Version, "Already in a session or max sessions reached.");
                    return;
                }
                BroadcastSnapshotToSessionMembersLocked(slot);
            }
        }

        private void OnSessionJoin(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !SessionProtocol.BoundedText(e.Args[1], 16)) return;

                if (!sessions.TryJoinByCode(slot, (string)e.Args[1], clock.ElapsedMilliseconds))
                {
                    e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionRejected),
                        SessionProtocol.Version, "Invalid join code, session full, or already in a session.");
                    return;
                }
                BroadcastSnapshotToSessionMembersLocked(slot);
                SkateSessionSnapshot snap;
                if (sessions.TryGetSnapshotForPeer(slot, clock.ElapsedMilliseconds, out snap))
                {
                    byte[] cachedWorld;
                    if (cachedWorldByCode.TryGetValue(snap.JoinCode, out cachedWorld) && cachedWorld != null)
                        e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostWorldSync), SessionProtocol.Version, cachedWorld);
                    byte[] cachedShop;
                    if (cachedShopByCode.TryGetValue(snap.JoinCode, out cachedShop) && cachedShop != null)
                        e.Client.SendCustomEvent(CustomEvents.Hash(SessionProtocol.SessionHostShopSync), SessionProtocol.Version, cachedShop);
                }
                int rawId;
                FriendPeerAuth joiningAuth;
                if (slotMap.TryGetRawId(slot, out rawId) && friendAuth.TryGetValue(rawId, out joiningAuth))
                    EvaluateModParityLocked(joiningAuth);
            }
        }

        private void OnSessionLeave(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 1)) return;
                ReleaseSlotStateLocked(slot);
            }
        }

        private void OnSessionConfigure(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 5) ||
                    !(e.Args[1] is int) || !(e.Args[2] is int) ||
                    !SessionProtocol.BoundedText(e.Args[3], SkateSessionCoordinator.MaxSpotIdChars) ||
                    !(e.Args[4] is long)) return;

                int modeRaw = (int)e.Args[1], zoneRaw = (int)e.Args[2];
                if (!Enum.IsDefined(typeof(SkateMode), modeRaw) || !Enum.IsDefined(typeof(SkateWorldZone), zoneRaw)) return;

                if (sessions.TryConfigureActivity(slot, (SkateMode)modeRaw, (SkateWorldZone)zoneRaw,
                    (string)e.Args[3], (long)e.Args[4]))
                    BroadcastSnapshotToSessionMembersLocked(slot);
            }
        }

        private void OnSessionStart(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 1)) return;
                if (sessions.TryStartActivity(slot, clock.ElapsedMilliseconds))
                    BroadcastSnapshotToSessionMembersLocked(slot);
            }
        }

        private void OnSessionBoard(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !SessionProtocol.BoundedText(e.Args[1], SkateSessionCoordinator.MaxBoardProfileChars)) return;
                if (sessions.TryUpdateBoardProfile(slot, (string)e.Args[1]))
                    BroadcastSnapshotToSessionMembersLocked(slot);
            }
        }

        private void OnSessionScore(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 4) ||
                    !(e.Args[1] is int) || !(e.Args[2] is int) || !(e.Args[3] is bool)) return;
                if (sessions.TryReportScore(slot, (int)e.Args[1], (int)e.Args[2], (bool)e.Args[3], clock.ElapsedMilliseconds))
                    BroadcastSnapshotToSessionMembersLocked(slot);
            }
        }

        private void OnSessionModsReport(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !(e.Args[1] is byte[])) return;

                ModEntry[] decoded;
                if (!SessionProtocol.TryDecodeModsManifest((byte[])e.Args[1], out decoded)) return;

                FriendPeerAuth senderAuth = null;
                int senderRawId;
                if (!slotMap.TryGetRawId(slot, out senderRawId) || !friendAuth.TryGetValue(senderRawId, out senderAuth))
                    return;

                senderAuth.ModManifest = decoded;
                EvaluateModParityLocked(senderAuth);
            }
        }

        private void EvaluateModParityLocked(FriendPeerAuth reportingAuth)
        {
            if (reportingAuth == null || reportingAuth.ModManifest == null) return;
            long now = clock.ElapsedMilliseconds;
            SkateSessionSnapshot snap;
            if (!sessions.TryGetSnapshotForPeer(reportingAuth.Slot, now, out snap)) return;

            int warnHash = CustomEvents.Hash(SessionProtocol.SessionModWarning);
            for (int i = 0; i < snap.Peers.Length; i++)
            {
                int otherSlot = snap.Peers[i].Slot;
                if (otherSlot == reportingAuth.Slot) continue;
                int otherRawId;
                FriendPeerAuth otherAuth;
                if (!slotMap.TryGetRawId(otherSlot, out otherRawId) ||
                    !friendAuth.TryGetValue(otherRawId, out otherAuth) ||
                    !otherAuth.Authenticated || otherAuth.ModManifest == null) continue;

                ModCompatibilityReport reportForSender = ModCompatibilityChecker.CompareManifests(
                    reportingAuth.ModManifest, otherAuth.ModManifest, "Slot#" + otherSlot);
                if (reportForSender.HasWarnings && reportingAuth.Owner != null)
                {
                    reportingAuth.Owner.SendCustomEvent(warnHash, SessionProtocol.Version, otherSlot, reportForSender.WarningSummary);
                    Logger.Warning("GTAxSkate Coop mod mismatch (slot " + reportingAuth.Slot + " vs " + otherSlot + "): " + reportForSender.WarningSummary);
                }

                ModCompatibilityReport reportForOther = ModCompatibilityChecker.CompareManifests(
                    otherAuth.ModManifest, reportingAuth.ModManifest, "Slot#" + reportingAuth.Slot);
                if (reportForOther.HasWarnings && otherAuth.Owner != null)
                {
                    otherAuth.Owner.SendCustomEvent(warnHash, SessionProtocol.Version, reportingAuth.Slot, reportForOther.WarningSummary);
                }
            }
        }

        private void BroadcastSnapshotToSessionMembersLocked(int memberSlot)
        {
            if (memberSlot <= 0) return;
            long now = clock.ElapsedMilliseconds;
            SkateSessionSnapshot snap;
            if (!sessions.TryGetSnapshotForPeer(memberSlot, now, out snap)) return;
            byte[] encoded = SessionProtocol.EncodeSnapshot(snap, now);
            if (encoded == null) return;

            int hash = CustomEvents.Hash(SessionProtocol.SessionSnapshot);
            for (int i = 0; i < snap.Peers.Length; i++)
            {
                int rawId;
                FriendPeerAuth auth;
                if (slotMap.TryGetRawId(snap.Peers[i].Slot, out rawId) &&
                    friendAuth.TryGetValue(rawId, out auth) && auth.Authenticated && auth.Owner != null)
                {
                    auth.Owner.SendCustomEvent(hash, SessionProtocol.Version, encoded);
                }
            }
        }

        private static bool IsRateLimited(Dictionary<int, long> map, int slot, long nowMs, long minIntervalMs)
        {
            long prev;
            if (map.TryGetValue(slot, out prev) && nowMs - prev < minIntervalMs) return true;
            map[slot] = nowMs;
            return false;
        }

        private void BroadcastToOtherSessionMembersLocked(
            SkateSessionSnapshot snap, int senderSlot, string eventName, object[] payloadArgs)
        {
            int hash = CustomEvents.Hash(eventName);
            for (int i = 0; i < snap.Peers.Length; i++)
            {
                int peerSlot = snap.Peers[i].Slot;
                if (peerSlot == senderSlot) continue;
                int rawId;
                FriendPeerAuth auth;
                if (slotMap.TryGetRawId(peerSlot, out rawId) &&
                    friendAuth.TryGetValue(rawId, out auth) && auth.Authenticated && auth.Owner != null)
                {
                    auth.Owner.SendCustomEvent(hash, payloadArgs);
                }
            }
        }

        private void OnHostWorldSync(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !(e.Args[1] is byte[])) return;

                long now = clock.ElapsedMilliseconds;
                SkateSessionSnapshot snap;
                if (!sessions.TryGetSnapshotForPeer(slot, now, out snap) || snap.HostSlot != slot)
                    return; // Fail closed: non-host peers cannot overwrite session world/mission state.

                if (IsRateLimited(lastWorldSyncMs, slot, now, 180)) return;

                HostWorldSyncState decoded;
                if (!SessionProtocol.TryDecodeWorldSync((byte[])e.Args[1], out decoded)) return;
                byte[] normalized = SessionProtocol.EncodeWorldSync(decoded);
                if (normalized == null) return;

                cachedWorldByCode[snap.JoinCode] = normalized;
                BroadcastToOtherSessionMembersLocked(
                    snap, slot, SessionProtocol.SessionHostWorldSync, new object[] { SessionProtocol.Version, normalized });
            }
        }

        private void OnHostShopSync(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !(e.Args[1] is byte[])) return;

                long now = clock.ElapsedMilliseconds;
                SkateSessionSnapshot snap;
                if (!sessions.TryGetSnapshotForPeer(slot, now, out snap) || snap.HostSlot != slot)
                    return; // Only session Host dictates shop/gear/crew vehicle state.

                if (IsRateLimited(lastShopSyncMs, slot, now, 250)) return;

                HostShopSyncState decoded;
                if (!SessionProtocol.TryDecodeShopSync((byte[])e.Args[1], out decoded)) return;
                byte[] normalized = SessionProtocol.EncodeShopSync(decoded);
                if (normalized == null) return;

                cachedShopByCode[snap.JoinCode] = normalized;
                BroadcastToOtherSessionMembersLocked(
                    snap, slot, SessionProtocol.SessionHostShopSync, new object[] { SessionProtocol.Version, normalized });
            }
        }

        private void OnRagdollSync(CustomEventReceivedArgs e, int epoch)
        {
            lock (gate)
            {
                int slot;
                if (!TryAuthenticatedSlotLocked(e, epoch, out slot) || !SessionProtocol.Header(e.Args, 2) ||
                    !(e.Args[1] is byte[])) return;

                long now = clock.ElapsedMilliseconds;
                SkateSessionSnapshot snap;
                if (!sessions.TryGetSnapshotForPeer(slot, now, out snap)) return;
                if (IsRateLimited(lastRagdollSyncMs, slot, now, 45)) return;

                RagdollSyncState decoded;
                if (!SessionProtocol.TryDecodeRagdollSync((byte[])e.Args[1], slot, out decoded)) return;
                byte[] normalized = SessionProtocol.EncodeRagdollSync(decoded);
                if (normalized == null) return;

                BroadcastToOtherSessionMembersLocked(
                    snap, slot, SessionProtocol.SessionRagdollSync, new object[] { SessionProtocol.Version, slot, normalized });
            }
        }

        public override void OnStop()
        {
            lock (gate)
            {
                active = false; lifetime++;
                peers.Clear(); friendAuth.Clear(); slotMap.Clear();
                cachedWorldByCode.Clear(); cachedShopByCode.Clear();
                lastWorldSyncMs.Clear(); lastShopSyncMs.Clear(); lastRagdollSyncMs.Clear();
            }
            API.Events.OnPlayerDisconnected -= Disconnected;
            API.Events.OnPlayerHandshake -= AdmitPrivateTransport;
        }
    }
}
