// Fail-closed IT security policy for private Friend Coop (GTA Story + Skate together).
// Blocks random online lobbies, unpinned public internet endpoints, and spoofed hosts,
// while providing continuous verified-session status awareness for the UI.
using System;
using System.Collections.Generic;
using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Text;

namespace GTAxSkate.Coop
{
    public enum LobbyDiscoverySource
    {
        LocalHost = 0,
        DirectFriendInvite = 1,
        PinnedFriendList = 2,
        PublicServerBrowser = 3,
        RandomMatchmaking = 4,
        QuickJoinAny = 5
    }

    public enum EndpointTrustClass
    {
        Invalid = 0,
        Loopback = 1,
        PrivateLan = 2,
        PrivateVpnOverlay = 3, // e.g. Tailscale/CGNAT 100.64.0.0/10, IPv6 ULA fc00::/7
        PublicInternet = 4
    }

    public enum ConnectionSecurityStatus
    {
        OfflineSolo = 0,
        VerifyingFriendSession = 1,
        ConnectedTrustedFriend = 2,
        BlockedInsecure = 3
    }

    public sealed class ServerSecurityConfig
    {
        public readonly bool AnnounceSelfToPublicList;
        public readonly bool AllowRandomPublicJoin;
        public readonly bool RequireFriendSharedKey;
        public readonly string FriendSharedKey;
        public readonly bool BindLoopbackOrPrivateOnly;

        public ServerSecurityConfig(
            bool announceSelfToPublicList,
            bool allowRandomPublicJoin,
            bool requireFriendSharedKey,
            string friendSharedKey,
            bool bindLoopbackOrPrivateOnly)
        {
            AnnounceSelfToPublicList = announceSelfToPublicList;
            AllowRandomPublicJoin = allowRandomPublicJoin;
            RequireFriendSharedKey = requireFriendSharedKey;
            FriendSharedKey = friendSharedKey ?? string.Empty;
            BindLoopbackOrPrivateOnly = bindLoopbackOrPrivateOnly;
        }

        public static ServerSecurityConfig DefaultPrivateFriends(string friendSharedKey)
        {
            return new ServerSecurityConfig(false, false, true, friendSharedKey, true);
        }
    }

    public sealed class ClientConnectRequest
    {
        public readonly LobbyDiscoverySource Source;
        public readonly string EndpointAddress;
        public readonly int Port;
        public readonly bool ExplicitPinnedFriendHost;
        public readonly string HostLabel;
        public readonly string FriendSharedKey;

        public ClientConnectRequest(
            LobbyDiscoverySource source,
            string endpointAddress,
            int port,
            bool explicitPinnedFriendHost,
            string hostLabel,
            string friendSharedKey)
        {
            Source = source;
            EndpointAddress = endpointAddress ?? string.Empty;
            Port = port;
            ExplicitPinnedFriendHost = explicitPinnedFriendHost;
            HostLabel = hostLabel ?? string.Empty;
            FriendSharedKey = friendSharedKey ?? string.Empty;
        }
    }

    public sealed class PolicyDecision
    {
        public readonly bool Allowed;
        public readonly EndpointTrustClass TrustClass;
        public readonly string Reason;

        internal PolicyDecision(bool allowed, EndpointTrustClass trustClass, string reason)
        {
            Allowed = allowed;
            TrustClass = trustClass;
            Reason = reason;
        }
    }

    public static class CoopSecurityPolicy
    {
        public const int MinFriendKeyLength = 12;
        public const int ProofBytes = 32;

        public static PolicyDecision ValidateServerConfig(ServerSecurityConfig config, string bindAddress)
        {
            if (config == null)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Missing server security configuration.");
            if (config.AnnounceSelfToPublicList)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Public lobby listing (AnnounceSelf) must be disabled.");
            if (config.AllowRandomPublicJoin)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Random public lobby joins must be disabled.");
            if (!config.RequireFriendSharedKey)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Pre-shared friend authentication key is required.");
            if (string.IsNullOrEmpty(config.FriendSharedKey) || config.FriendSharedKey.Trim().Length < MinFriendKeyLength)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Friend shared key is missing or shorter than 12 characters.");

            EndpointTrustClass trust = ClassifyEndpoint(bindAddress);
            if (trust == EndpointTrustClass.Invalid)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Invalid server bind address.");
            if (config.BindLoopbackOrPrivateOnly && trust == EndpointTrustClass.PublicInternet)
                return new PolicyDecision(false, EndpointTrustClass.PublicInternet, "Server bind on raw public IP blocked; use Localhost/LAN/VPN.");

            return new PolicyDecision(true, trust, "Private friend server policy valid.");
        }

        public static PolicyDecision EvaluateOutboundConnect(ClientConnectRequest req)
        {
            if (req == null)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Missing connection request.");

            if (req.Source == LobbyDiscoverySource.PublicServerBrowser ||
                req.Source == LobbyDiscoverySource.RandomMatchmaking ||
                req.Source == LobbyDiscoverySource.QuickJoinAny)
            {
                return new PolicyDecision(false, EndpointTrustClass.Invalid,
                    "Random online lobbies and public server browsers are blocked by security policy.");
            }

            if (req.Port <= 0 || req.Port > 65535)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Invalid port.");

            if (string.IsNullOrEmpty(req.FriendSharedKey) || req.FriendSharedKey.Trim().Length < MinFriendKeyLength)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Friend shared key (>= 12 chars) is required.");

            EndpointTrustClass trust = ClassifyEndpoint(req.EndpointAddress);
            if (trust == EndpointTrustClass.Invalid)
                return new PolicyDecision(false, EndpointTrustClass.Invalid, "Unknown or unparseable host address.");

            if (trust == EndpointTrustClass.PublicInternet && !req.ExplicitPinnedFriendHost)
            {
                return new PolicyDecision(false, trust,
                    "Unpinned public internet host blocked; use Tailscale/WireGuard VPN, LAN, or explicitly pin this friend host.");
            }

            // A pinned public host must be a real global address: documentation, benchmarking, multicast and
            // other reserved ranges (anything a typo or a hostile code could point at) are never dialled.
            byte[] v4;
            if (trust == EndpointTrustClass.PublicInternet && LobbyCode.TryParseIPv4(req.EndpointAddress, out v4) &&
                !LobbyCode.IsGlobalIPv4(v4))
            {
                return new PolicyDecision(false, trust, "Reserved or non-routable address range blocked.");
            }

            return new PolicyDecision(true, trust, "Outbound target permitted pending mutual friend authentication.");
        }

        public const string DefaultScreenLabel = "Friend";
        private static readonly string[] FixedLabels = { "Friend", "LocalHost", "Online host", "LAN host", "VPN host" };

        // Host labels drawn on screen come ONLY from the fixed set above (design 2026-11-17-streamer-safe-coop, round 2):
        // a configured free-text label can be a name or a host name, so anything else is shown as "Friend".
        public static string ScreenLabel(string label)
        {
            if (string.IsNullOrEmpty(label)) return DefaultScreenLabel;
            string trimmed = label.Trim();
            for (int i = 0; i < FixedLabels.Length; i++)
                if (string.Equals(FixedLabels[i], trimmed, StringComparison.Ordinal)) return FixedLabels[i];
            return DefaultScreenLabel;
        }

        public static EndpointTrustClass ClassifyEndpoint(string addressText)
        {
            if (string.IsNullOrEmpty(addressText)) return EndpointTrustClass.Invalid;
            string clean = addressText.Trim();
            if (clean.Equals("localhost", StringComparison.OrdinalIgnoreCase))
                return EndpointTrustClass.Loopback;

            IPAddress ip;
            if (!IPAddress.TryParse(clean, out ip)) return EndpointTrustClass.Invalid;

            if (ip.IsIPv4MappedToIPv6) ip = ip.MapToIPv4();

            if (IPAddress.IsLoopback(ip)) return EndpointTrustClass.Loopback;

            if (ip.AddressFamily == AddressFamily.InterNetwork)
            {
                byte[] b = ip.GetAddressBytes();
                if (b[0] == 10) return EndpointTrustClass.PrivateLan;
                if (b[0] == 172 && b[1] >= 16 && b[1] <= 31) return EndpointTrustClass.PrivateLan;
                if (b[0] == 192 && b[1] == 168) return EndpointTrustClass.PrivateLan;
                if (b[0] == 100 && b[1] >= 64 && b[1] <= 127) return EndpointTrustClass.PrivateVpnOverlay; // RFC 6598 / Tailscale
                if (b[0] == 169 && b[1] == 254) return EndpointTrustClass.PrivateLan;
                if (b[0] == 0) return EndpointTrustClass.Invalid;
                return EndpointTrustClass.PublicInternet;
            }

            if (ip.AddressFamily == AddressFamily.InterNetworkV6)
            {
                if (ip.IsIPv6LinkLocal) return EndpointTrustClass.PrivateLan;
                byte[] b = ip.GetAddressBytes();
                if ((b[0] & 0xFE) == 0xFC) return EndpointTrustClass.PrivateVpnOverlay; // ULA fc00::/7
                return EndpointTrustClass.PublicInternet;
            }

            return EndpointTrustClass.Invalid;
        }

        // Mutual HMAC-SHA256 authentication proof so raw secrets never cross the wire.
        // roleTag must differ cleanly between server proof ("SRV") and client proof ("CLI").
        public static byte[] ComputeMutualProof(
            string friendSharedKey, byte[] clientNonce, byte[] serverNonce, string roleTag)
        {
            if (string.IsNullOrEmpty(friendSharedKey) || !IsValidNonce(clientNonce) ||
                !IsValidNonce(serverNonce) || string.IsNullOrEmpty(roleTag))
                throw new ArgumentException("Invalid mutual proof inputs.");

            byte[] keyBytes = Encoding.UTF8.GetBytes(friendSharedKey);
            byte[] domain = Encoding.ASCII.GetBytes("GTAxSkate.FriendAuth.v1|");
            byte[] roleBytes = Encoding.ASCII.GetBytes(roleTag);

            byte[] payload = new byte[domain.Length + roleBytes.Length + 1 + 16 + 16];
            int at = 0;
            Buffer.BlockCopy(domain, 0, payload, at, domain.Length); at += domain.Length;
            Buffer.BlockCopy(roleBytes, 0, payload, at, roleBytes.Length); at += roleBytes.Length;
            payload[at++] = (byte)'|';
            Buffer.BlockCopy(clientNonce, 0, payload, at, 16); at += 16;
            Buffer.BlockCopy(serverNonce, 0, payload, at, 16);

            using (var hmac = new HMACSHA256(keyBytes))
            {
                return hmac.ComputeHash(payload);
            }
        }

        public static bool VerifyMutualProof(
            string friendSharedKey, byte[] clientNonce, byte[] serverNonce, string roleTag, byte[] candidateProof)
        {
            if (candidateProof == null || candidateProof.Length != ProofBytes ||
                string.IsNullOrEmpty(friendSharedKey) || !IsValidNonce(clientNonce) ||
                !IsValidNonce(serverNonce) || string.IsNullOrEmpty(roleTag))
                return false;

            byte[] expected = ComputeMutualProof(friendSharedKey, clientNonce, serverNonce, roleTag);
            return FixedTimeEquals(expected, candidateProof);
        }

        public static bool FixedTimeEquals(byte[] a, byte[] b)
        {
            if (a == null || b == null || a.Length != b.Length) return false;
            int diff = 0;
            for (int i = 0; i < a.Length; i++) diff |= a[i] ^ b[i];
            return diff == 0;
        }

        private static bool IsValidNonce(byte[] nonce)
        {
            return nonce != null && nonce.Length == 16 && new Guid(nonce) != Guid.Empty;
        }
    }

    // Continuous connection awareness state machine so the player ALWAYS knows when
    // verifying, connected to a friend session, or blocked for security reasons.
    public sealed class SessionConnectionTracker
    {
        private readonly object gate = new object();
        private ConnectionSecurityStatus status = ConnectionSecurityStatus.OfflineSolo;
        private EndpointTrustClass trustClass = EndpointTrustClass.Invalid;
        private string hostLabel = string.Empty;
        private string statusReason = "Solo Story Mode";
        private int connectedPeerCount;
        private long lastCheckedAtMs;
        private byte[] pendingClientNonce;
        private byte[] pendingServerNonce;
        private string pendingKey = string.Empty;
        private string pendingHostLabel = string.Empty;
        private EndpointTrustClass pendingTrust = EndpointTrustClass.Invalid;

        public ConnectionSecurityStatus Status
        {
            get { lock (gate) { return status; } }
        }

        public string StatusReason
        {
            get { lock (gate) { return statusReason; } }
        }

        public bool IsConnectedToPlayerSession
        {
            get { lock (gate) { return status == ConnectionSecurityStatus.ConnectedTrustedFriend; } }
        }

        public bool BeginFriendConnect(ClientConnectRequest req, byte[] clientNonce, long nowMs, out PolicyDecision decision)
        {
            decision = CoopSecurityPolicy.EvaluateOutboundConnect(req);
            lock (gate)
            {
                lastCheckedAtMs = nowMs;
                if (!decision.Allowed)
                {
                    ResetPendingLocked();
                    status = ConnectionSecurityStatus.BlockedInsecure;
                    trustClass = decision.TrustClass;
                    hostLabel = string.Empty;
                    connectedPeerCount = 0;
                    statusReason = "BLOCKED: " + decision.Reason;
                    return false;
                }

                if (clientNonce == null || clientNonce.Length != 16 || new Guid(clientNonce) == Guid.Empty)
                {
                    ResetPendingLocked();
                    status = ConnectionSecurityStatus.BlockedInsecure;
                    statusReason = "BLOCKED: Invalid client nonce.";
                    decision = new PolicyDecision(false, EndpointTrustClass.Invalid, "Invalid client nonce.");
                    return false;
                }

                status = ConnectionSecurityStatus.VerifyingFriendSession;
                trustClass = decision.TrustClass;
                pendingTrust = decision.TrustClass;
                pendingClientNonce = (byte[])clientNonce.Clone();
                pendingServerNonce = null;
                pendingKey = req.FriendSharedKey;
                pendingHostLabel = CoopSecurityPolicy.ScreenLabel(req.HostLabel);
                hostLabel = pendingHostLabel;
                connectedPeerCount = 0;
                statusReason = "Verifying friend host (" + FormatTrustTag(trustClass) + ")...";
                return true;
            }
        }

        public bool CompleteMutualServerVerification(
            byte[] clientNonce, byte[] serverNonce, byte[] serverProof, string joinCode, int initialPeers, long nowMs)
        {
            lock (gate)
            {
                lastCheckedAtMs = nowMs;
                if (status != ConnectionSecurityStatus.VerifyingFriendSession || pendingClientNonce == null)
                {
                    FailClosedLocked("No pending verification handshake.");
                    return false;
                }

                if (!CoopSecurityPolicy.FixedTimeEquals(pendingClientNonce, clientNonce))
                {
                    FailClosedLocked("Client nonce mismatch on server challenge.");
                    return false;
                }

                if (!CoopSecurityPolicy.VerifyMutualProof(pendingKey, pendingClientNonce, serverNonce, "SRV", serverProof))
                {
                    FailClosedLocked("Host failed mutual friend authentication proof.");
                    return false;
                }

                pendingServerNonce = (byte[])serverNonce.Clone();
                status = ConnectionSecurityStatus.ConnectedTrustedFriend;
                trustClass = pendingTrust;
                hostLabel = pendingHostLabel;
                connectedPeerCount = initialPeers < 1 ? 1 : initialPeers;
                statusReason = "Verified friend session with " + hostLabel;
                pendingKey = string.Empty;
                return true;
            }
        }

        public void UpdateLivePeerCount(int peerCount, long nowMs)
        {
            lock (gate)
            {
                lastCheckedAtMs = nowMs;
                if (status == ConnectionSecurityStatus.ConnectedTrustedFriend)
                    connectedPeerCount = peerCount < 1 ? 1 : peerCount;
            }
        }

        public void FailBlocked(string reason, long nowMs)
        {
            lock (gate)
            {
                lastCheckedAtMs = nowMs;
                trustClass = EndpointTrustClass.Invalid;
                FailClosedLocked(reason);
            }
        }

        public void DisconnectToSolo(string reason, long nowMs)
        {
            lock (gate)
            {
                lastCheckedAtMs = nowMs;
                ResetPendingLocked();
                status = ConnectionSecurityStatus.OfflineSolo;
                trustClass = EndpointTrustClass.Invalid;
                hostLabel = string.Empty;
                connectedPeerCount = 0;
                statusReason = string.IsNullOrEmpty(reason) ? "Solo Story Mode" : reason;
            }
        }

        // Always produces an explicit, human-readable status string for all UI presets (incl. RP).
        // Streamer-safe (design 2026-11-17-streamer-safe-coop): only a label and a trust tag, never an address
        // or a session join code; the whole text also passes SafeText.ForScreen.
        public string BuildStatusBanner()
        {
            lock (gate)
            {
                return SafeText.ForScreen(BuildStatusBannerLocked());
            }
        }

        private string BuildStatusBannerLocked()
        {
            switch (status)
            {
                case ConnectionSecurityStatus.OfflineSolo:
                    return "[SOLO] Offline Story Mode (no other player connected)";
                case ConnectionSecurityStatus.VerifyingFriendSession:
                    return "[CHECKING] Verifying private friend host: " + hostLabel + " [" + FormatTrustTag(trustClass) + "]";
                case ConnectionSecurityStatus.ConnectedTrustedFriend:
                    return "[CONNECTED - PRIVATE COOP] Host: " + hostLabel + " [" + FormatTrustTag(trustClass) +
                        "] | Skaters: " + connectedPeerCount;
                case ConnectionSecurityStatus.BlockedInsecure:
                default:
                    return "[" + statusReason + "]";
            }
        }

        private void FailClosedLocked(string reason)
        {
            ResetPendingLocked();
            status = ConnectionSecurityStatus.BlockedInsecure;
            hostLabel = string.Empty;
            connectedPeerCount = 0;
            statusReason = "BLOCKED: " + reason;
        }

        private void ResetPendingLocked()
        {
            pendingClientNonce = null;
            pendingServerNonce = null;
            pendingKey = string.Empty;
            pendingHostLabel = string.Empty;
            pendingTrust = EndpointTrustClass.Invalid;
        }

        private static string FormatTrustTag(EndpointTrustClass trust)
        {
            switch (trust)
            {
                case EndpointTrustClass.Loopback: return "LOCALHOST";
                case EndpointTrustClass.PrivateLan: return "LAN";
                case EndpointTrustClass.PrivateVpnOverlay: return "VPN";
                case EndpointTrustClass.PublicInternet: return "ONLINE";
                default: return "UNTRUSTED";
            }
        }
    }
}
