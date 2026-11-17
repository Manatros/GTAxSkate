// Abstraction over GTA V / SHVDN3 & RAGECOOP client UI/world operations (C# 5).
// Allows deterministic offline unit tests via mocks while compiling real SHVDN3
// natives cleanly inside client/ShvdnWorldBridge.cs.
using System;
using GTAxSkate.Coop;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class LocalRagdollSample
    {
        public readonly bool IsRagdoll;
        public readonly float PosX, PosY, PosZ;
        public readonly float VelX, VelY, VelZ;

        public LocalRagdollSample(bool isRagdoll, float px, float py, float pz, float vx, float vy, float vz)
        {
            IsRagdoll = isRagdoll;
            PosX = px; PosY = py; PosZ = pz;
            VelX = vx; VelY = vy; VelZ = vz;
        }
    }

    public sealed class CoopOverlayView
    {
        public readonly bool MenuOpen;
        public readonly string StatusBanner;
        public readonly string SubtitleLine;
        public readonly string[] Warnings;
        public readonly CoopMenuItem[] Items;
        public readonly int SelectedIndex;

        public CoopOverlayView(
            bool menuOpen,
            string statusBanner,
            string subtitleLine,
            string[] warnings,
            CoopMenuItem[] items,
            int selectedIndex)
        {
            MenuOpen = menuOpen;
            StatusBanner = statusBanner ?? string.Empty;
            SubtitleLine = subtitleLine ?? string.Empty;
            Warnings = warnings ?? new string[0];
            Items = items ?? new CoopMenuItem[0];
            SelectedIndex = selectedIndex;
        }
    }

    public interface IGtaWorldBridge
    {
        bool IsGameLoading();
        HostWorldSyncState CaptureHostWorldState(int epoch);
        LocalRagdollSample CaptureLocalRagdoll();
        void ApplyPeerWorldState(HostWorldSyncState state);
        void ApplyPeerShopState(HostShopSyncState state);
        void ApplyRemotePlayerRagdoll(int sourceSlot, string peerUsername, RagdollSyncState state);
        void ExitPeerCutsceneStandbyIfNeeded(HostWorldSyncState lastWorldState, bool allowGroundRegroup);
        void RestoreSoloDefaults();
        // friendKey becomes RageCoop's transport password: the host server admits only its SHA-256 (ServerAdapter.AdmitPrivateTransport).
        // Returns false when the attempt did not start (error, or the player cancelled RageCoop's warning).
        bool ConnectTransport(string endpointAddress, int port, string friendKey);
        void DisconnectTransport();
        // Sends RageCoop's queued messages now (its own flush runs only at the end of a sync that can fail).
        void FlushTransport();
        // True while RageCoop is still trying to connect (public-key fetch, handshake).
        bool IsTransportConnecting();
        bool EnsureLocalHostServerRunning(int port, string friendKey, out string statusMessage);
        void StopLocalHostServerIfStarted();
        // True while the server process this integration started is alive (drives the "Stop local server" row).
        bool IsLocalServerRunning();
        // Internet lobby (design 2026-11-16-coop-internet-lobby): UPnP runs on a background worker and never blocks the
        // game thread. StopLocalHostServerIfStarted also removes this install's own port mapping.
        void BeginOnlineHosting(int udpPort);
        OnlineHostStatus PollOnlineHosting();
        // Streamer-safe coop (design 2026-11-17-streamer-safe-coop): nothing secret is typed or drawn. Invites travel only
        // through the clipboard; the game thread never waits (Begin returns a ticket that later ticks Poll).
        // track=false: fire and forget (a clear). There is deliberately no on-screen-keyboard prompt on this interface.
        int BeginClipboard(ClipboardOpKind kind, string text, bool track);
        ClipboardOpResult PollClipboard(int ticket);
        // This PC's private LAN IPv4 and its VPN (100.64.0.0/10) IPv4, null when absent.
        void GetLocalAddresses(out string lanIPv4, out string vpnIPv4);
        // RageCoop username forced to the neutral name before every connect (name tags, player list, join notices).
        void ForceAnonymousName();
        // RageCoop's own F9 menu locked/unlocked (menu key, server browser, last address, name); restores the user's values.
        // False (with a sanitized reason) when the lock/restore did not fully apply; the caller tells the player.
        bool SetRageCoopUiLocked(bool locked, out string error);
        void Notify(string message);
        void RenderOverlay(CoopOverlayView overlay);
    }

    public sealed class NullGtaWorldBridge : IGtaWorldBridge
    {
        public bool IsGameLoading() { return false; }
        public HostWorldSyncState CaptureHostWorldState(int epoch) { return null; }
        public LocalRagdollSample CaptureLocalRagdoll() { return null; }
        public void ApplyPeerWorldState(HostWorldSyncState state) { }
        public void ApplyPeerShopState(HostShopSyncState state) { }
        public void ApplyRemotePlayerRagdoll(int sourceSlot, string peerUsername, RagdollSyncState state) { }
        public void ExitPeerCutsceneStandbyIfNeeded(HostWorldSyncState lastWorldState, bool allowGroundRegroup) { }
        public void RestoreSoloDefaults() { }
        public bool ConnectTransport(string endpointAddress, int port, string friendKey) { return true; }
        public void DisconnectTransport() { }
        public void FlushTransport() { }
        public bool IsTransportConnecting() { return false; }
        public bool EnsureLocalHostServerRunning(int port, string friendKey, out string statusMessage)
        {
            statusMessage = string.Empty;
            return true;
        }
        public void StopLocalHostServerIfStarted() { }
        public bool IsLocalServerRunning() { return false; }
        public void BeginOnlineHosting(int udpPort) { }
        public OnlineHostStatus PollOnlineHosting() { return OnlineHostStatus.Idle; }
        public int BeginClipboard(ClipboardOpKind kind, string text, bool track) { return 0; }
        public ClipboardOpResult PollClipboard(int ticket) { return ClipboardOpResult.FailedResult; }
        public void GetLocalAddresses(out string lanIPv4, out string vpnIPv4) { lanIPv4 = null; vpnIPv4 = null; }
        public void ForceAnonymousName() { }
        public bool SetRageCoopUiLocked(bool locked, out string error) { error = string.Empty; return true; }
        public void Notify(string message) { }
        public void RenderOverlay(CoopOverlayView overlay) { }
    }
}
