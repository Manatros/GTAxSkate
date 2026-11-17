// Shared host-authoritative world, Story mission, cutscene, shop/board, ragdoll,
// and in-game Coop session UI models (C# 5 compatible; zero game/UI binaries referenced).
using System;
using System.Collections.Generic;
using System.Text;

namespace GTAxSkate.Coop
{
    public sealed class HostWorldSyncState
    {
        public const float MaxCoordAbs = 32768.0f;
        public const int MaxWeatherId = 14;
        public const int MaxWantedLevel = 5;

        public readonly int Epoch;
        public readonly int ClockHour;
        public readonly int ClockMinute;
        public readonly bool ClockPaused;
        public readonly int WeatherId;
        public readonly bool MissionActive;
        public readonly bool CutsceneActive;
        public readonly bool HostInVehicle;
        public readonly int WantedLevel;
        public readonly float WantedCoordX;
        public readonly float WantedCoordY;
        public readonly float WantedCoordZ;
        public readonly float HostPosX;
        public readonly float HostPosY;
        public readonly float HostPosZ;
        public readonly bool HasObjective;
        public readonly float ObjectiveX;
        public readonly float ObjectiveY;
        public readonly float ObjectiveZ;
        public readonly int ObjectiveSprite;
        public readonly int ObjectiveColor;
        public readonly bool AllowShopPurchase;

        public HostWorldSyncState(
            int epoch,
            int clockHour,
            int clockMinute,
            bool clockPaused,
            int weatherId,
            bool missionActive,
            bool cutsceneActive,
            bool hostInVehicle,
            int wantedLevel,
            float wantedCoordX,
            float wantedCoordY,
            float wantedCoordZ,
            float hostPosX,
            float hostPosY,
            float hostPosZ,
            bool hasObjective,
            float objectiveX,
            float objectiveY,
            float objectiveZ,
            int objectiveSprite,
            int objectiveColor,
            bool allowShopPurchase)
        {
            Epoch = epoch;
            ClockHour = clockHour;
            ClockMinute = clockMinute;
            ClockPaused = clockPaused;
            WeatherId = weatherId;
            MissionActive = missionActive;
            CutsceneActive = cutsceneActive;
            HostInVehicle = hostInVehicle;
            WantedLevel = wantedLevel;
            WantedCoordX = wantedCoordX;
            WantedCoordY = wantedCoordY;
            WantedCoordZ = wantedCoordZ;
            HostPosX = hostPosX;
            HostPosY = hostPosY;
            HostPosZ = hostPosZ;
            HasObjective = hasObjective;
            ObjectiveX = objectiveX;
            ObjectiveY = objectiveY;
            ObjectiveZ = objectiveZ;
            ObjectiveSprite = objectiveSprite;
            ObjectiveColor = objectiveColor;
            AllowShopPurchase = allowShopPurchase;
        }

        public static bool IsFiniteBoundedCoord(float v)
        {
            return !float.IsNaN(v) && !float.IsInfinity(v) && Math.Abs(v) <= MaxCoordAbs;
        }

        public bool IsValid()
        {
            if (Epoch < 0 || ClockHour < 0 || ClockHour > 23 || ClockMinute < 0 || ClockMinute > 59)
                return false;
            if (WeatherId < 0 || WeatherId > MaxWeatherId || WantedLevel < 0 || WantedLevel > MaxWantedLevel)
                return false;
            if (!IsFiniteBoundedCoord(HostPosX) || !IsFiniteBoundedCoord(HostPosY) || !IsFiniteBoundedCoord(HostPosZ))
                return false;
            if (!IsFiniteBoundedCoord(WantedCoordX) || !IsFiniteBoundedCoord(WantedCoordY) || !IsFiniteBoundedCoord(WantedCoordZ))
                return false;
            if (!IsFiniteBoundedCoord(ObjectiveX) || !IsFiniteBoundedCoord(ObjectiveY) || !IsFiniteBoundedCoord(ObjectiveZ))
                return false;
            if (ObjectiveSprite < 0 || ObjectiveSprite > 2048 || ObjectiveColor < 0 || ObjectiveColor > 255)
                return false;
            return true;
        }
    }

    public sealed class HostShopSyncState
    {
        public const int MaxPlateChars = 8;
        public const int MaxNoticeChars = 96;

        public readonly int ShopEpoch;
        public readonly int HostCash;
        public readonly string EquippedBoardProfile;
        public readonly uint CrewVehicleModelHash;
        public readonly int CrewVehiclePrimaryColor;
        public readonly int CrewVehicleSecondaryColor;
        public readonly string CrewVehiclePlate;
        public readonly uint SharedWeaponHash;
        public readonly string StoreSyncNotice;

        public HostShopSyncState(
            int shopEpoch,
            int hostCash,
            string equippedBoardProfile,
            uint crewVehicleModelHash,
            int crewVehiclePrimaryColor,
            int crewVehicleSecondaryColor,
            string crewVehiclePlate,
            uint sharedWeaponHash,
            string storeSyncNotice)
        {
            ShopEpoch = shopEpoch;
            HostCash = hostCash;
            EquippedBoardProfile = SanitizeSafeAscii(equippedBoardProfile, SkateSessionCoordinator.MaxBoardProfileChars);
            CrewVehicleModelHash = crewVehicleModelHash;
            CrewVehiclePrimaryColor = crewVehiclePrimaryColor;
            CrewVehicleSecondaryColor = crewVehicleSecondaryColor;
            CrewVehiclePlate = SanitizeSafeAscii(crewVehiclePlate, MaxPlateChars);
            SharedWeaponHash = sharedWeaponHash;
            StoreSyncNotice = SanitizeSafeAscii(storeSyncNotice, MaxNoticeChars);
        }

        public bool IsValid()
        {
            return ShopEpoch >= 0 && HostCash >= 0 &&
                CrewVehiclePrimaryColor >= 0 && CrewVehiclePrimaryColor <= 255 &&
                CrewVehicleSecondaryColor >= 0 && CrewVehicleSecondaryColor <= 255;
        }

        public static string SanitizeSafeAscii(string raw, int maxLen)
        {
            if (string.IsNullOrEmpty(raw) || maxLen <= 0) return string.Empty;
            var sb = new StringBuilder(Math.Min(raw.Length, maxLen));
            for (int i = 0; i < raw.Length && sb.Length < maxLen; i++)
            {
                char c = raw[i];
                // Strip GTA HUD formatting tokens ('~') and non-printable chars.
                if (c >= 0x20 && c <= 0x7E && c != '~') sb.Append(c);
            }
            return sb.ToString().Trim();
        }
    }

    public sealed class RagdollSyncState
    {
        public const float MaxSpeedAbs = 120.0f;
        public const int MaxDurationMs = 5000;

        public readonly int SourceSlot;
        public readonly bool IsRagdoll;
        public readonly float PosX;
        public readonly float PosY;
        public readonly float PosZ;
        public readonly float VelX;
        public readonly float VelY;
        public readonly float VelZ;
        public readonly int DurationMs;

        public RagdollSyncState(
            int sourceSlot,
            bool isRagdoll,
            float posX,
            float posY,
            float posZ,
            float velX,
            float velY,
            float velZ,
            int durationMs)
        {
            SourceSlot = sourceSlot;
            IsRagdoll = isRagdoll;
            PosX = posX;
            PosY = posY;
            PosZ = posZ;
            VelX = velX;
            VelY = velY;
            VelZ = velZ;
            DurationMs = durationMs;
        }

        public static bool IsFiniteBoundedSpeed(float v)
        {
            return !float.IsNaN(v) && !float.IsInfinity(v) && Math.Abs(v) <= MaxSpeedAbs;
        }

        public bool IsValid()
        {
            if (SourceSlot < 0 || DurationMs < 0 || DurationMs > MaxDurationMs) return false;
            if (!HostWorldSyncState.IsFiniteBoundedCoord(PosX) ||
                !HostWorldSyncState.IsFiniteBoundedCoord(PosY) ||
                !HostWorldSyncState.IsFiniteBoundedCoord(PosZ))
                return false;
            if (!IsFiniteBoundedSpeed(VelX) || !IsFiniteBoundedSpeed(VelY) || !IsFiniteBoundedSpeed(VelZ))
                return false;
            return true;
        }
    }

    public enum CoopMenuAction
    {
        None = 0,
        ConnectConfiguredHost = 1,
        HostPrivateSession = 2,
        JoinSessionByCode = 3,
        PushWorldSyncNow = 4,
        DisconnectToSolo = 5,
        DisableMultiplayer = 6,
        HostOnline = 7,     // "Host online (internet)": local server + UPnP + invite on the clipboard
        JoinOnline = 8,     // "Join friend (paste invite)": reads the clipboard, nothing is typed
        CopyInvite = 9,     // host: copy the invite to the clipboard again (hidden)
        CopyJoinCode = 10,  // host in a session: copy the session join code to the clipboard (hidden)
        StopLocalServer = 11 // our own local server runs while this client is not connected: stop it (and its UPnP mapping)
    }

    public sealed class CoopMenuItem
    {
        public readonly CoopMenuAction Action;
        public readonly string Label;
        public readonly bool Enabled;

        public CoopMenuItem(CoopMenuAction action, string label, bool enabled)
        {
            Action = action;
            Label = label ?? string.Empty;
            Enabled = enabled;
        }
    }

    public sealed class CoopInGameMenuState
    {
        public const string DisableLabel = "Disable Multiplayer & Return to Solo";
        public bool IsOpen { get; private set; }
        public int SelectedIndex { get; private set; }

        public void Toggle()
        {
            IsOpen = !IsOpen;
            if (IsOpen && SelectedIndex < 0) SelectedIndex = 0;
        }

        public void Close()
        {
            IsOpen = false;
        }

        public void MoveSelection(int delta, int itemCount)
        {
            if (itemCount <= 0) { SelectedIndex = 0; return; }
            int next = (SelectedIndex + delta) % itemCount;
            if (next < 0) next += itemCount;
            SelectedIndex = next;
        }

        public CoopMenuItem[] BuildVisibleItems(
            bool connectedToServer,
            bool authenticated,
            bool hasConfiguredHost,
            string configuredHostLabel,
            bool inSession,
            bool isHost,
            string joinCode)
        {
            return BuildVisibleItems(connectedToServer, authenticated, hasConfiguredHost, configuredHostLabel,
                inSession, isHost, joinCode, null, false);
        }

        public CoopMenuItem[] BuildVisibleItems(
            bool connectedToServer,
            bool authenticated,
            bool hasConfiguredHost,
            string configuredHostLabel,
            bool inSession,
            bool isHost,
            string joinCode,
            string onlineInfo)
        {
            return BuildVisibleItems(connectedToServer, authenticated, hasConfiguredHost, configuredHostLabel,
                inSession, isHost, joinCode, onlineInfo, false);
        }

        // Streamer-safe (design 2026-11-17-streamer-safe-coop): no row ever shows an address, an invite, a lobby
        // code or a session join code (joinCode is accepted for old callers and never drawn). onlineInfo: non-empty
        // while an internet lobby is being opened or is open (a disabled status row). inviteAvailable: this PC is
        // hosting and has an invite to copy again.
        public CoopMenuItem[] BuildVisibleItems(
            bool connectedToServer,
            bool authenticated,
            bool hasConfiguredHost,
            string configuredHostLabel,
            bool inSession,
            bool isHost,
            string joinCode,
            string onlineInfo,
            bool inviteAvailable)
        {
            return BuildVisibleItems(connectedToServer, authenticated, hasConfiguredHost, configuredHostLabel,
                inSession, isHost, joinCode, onlineInfo, inviteAvailable, false);
        }

        // connectBusy: a connect is pending, or the client is on the server but not yet authenticated. The two host rows
        // are disabled then: a host action restarts or reuses the local server and must never run under a live connect.
        public CoopMenuItem[] BuildVisibleItems(
            bool connectedToServer,
            bool authenticated,
            bool hasConfiguredHost,
            string configuredHostLabel,
            bool inSession,
            bool isHost,
            string joinCode,
            string onlineInfo,
            bool inviteAvailable,
            bool connectBusy)
        {
            return BuildVisibleItems(connectedToServer, authenticated, hasConfiguredHost, configuredHostLabel,
                inSession, isHost, joinCode, onlineInfo, inviteAvailable, connectBusy, false);
        }

        // localServerRunning: the local server process this integration started is alive. The "Stop local server" row
        // shows only while this client is neither connected nor connecting (a host action refuses a different key while
        // the server runs, and this row is the player's way out).
        public CoopMenuItem[] BuildVisibleItems(
            bool connectedToServer,
            bool authenticated,
            bool hasConfiguredHost,
            string configuredHostLabel,
            bool inSession,
            bool isHost,
            string joinCode,
            string onlineInfo,
            bool inviteAvailable,
            bool connectBusy,
            bool localServerRunning)
        {
            var items = new List<CoopMenuItem>();
            if (!connectedToServer || !authenticated)
            {
                if (hasConfiguredHost)
                    items.Add(new CoopMenuItem(CoopMenuAction.ConnectConfiguredHost,
                        "Connect to Friend: " + CoopSecurityPolicy.ScreenLabel(configuredHostLabel), true));
                else
                    items.Add(new CoopMenuItem(CoopMenuAction.ConnectConfiguredHost, "Connect to saved friend (none set up)", false));
                items.Add(new CoopMenuItem(CoopMenuAction.HostPrivateSession, "Host private session (LAN / VPN)", !connectBusy));
                items.Add(new CoopMenuItem(CoopMenuAction.HostOnline, "Host online (internet)", !connectBusy));
                items.Add(new CoopMenuItem(CoopMenuAction.JoinOnline, "Join friend (paste invite)", true));
                if (inviteAvailable)
                    items.Add(new CoopMenuItem(CoopMenuAction.CopyInvite, "Copy invite to clipboard", true));
                if (localServerRunning && !connectedToServer && !connectBusy)
                    items.Add(new CoopMenuItem(CoopMenuAction.StopLocalServer, "Stop local server", true));
                items.Add(new CoopMenuItem(CoopMenuAction.DisableMultiplayer, DisableLabel, true));
            }
            else if (!inSession)
            {
                items.Add(new CoopMenuItem(CoopMenuAction.HostPrivateSession, "Create Friend Lobby (10 Slots)", true));
                items.Add(new CoopMenuItem(CoopMenuAction.JoinSessionByCode, "Join Friend Lobby (paste code)", true));
                if (inviteAvailable)
                    items.Add(new CoopMenuItem(CoopMenuAction.CopyInvite, "Copy invite to clipboard", true));
                items.Add(new CoopMenuItem(CoopMenuAction.DisconnectToSolo, "Disconnect -> Solo Story Mode", true));
                items.Add(new CoopMenuItem(CoopMenuAction.DisableMultiplayer, DisableLabel, true));
            }
            else
            {
                if (isHost)
                {
                    items.Add(new CoopMenuItem(CoopMenuAction.PushWorldSyncNow, "Sync World / Mission / Shop Now", true));
                    items.Add(new CoopMenuItem(CoopMenuAction.CopyJoinCode, "Copy join code to clipboard", true));
                }
                else
                {
                    items.Add(new CoopMenuItem(CoopMenuAction.None, "Synced to Host", false));
                }
                if (inviteAvailable)
                    items.Add(new CoopMenuItem(CoopMenuAction.CopyInvite, "Copy invite to clipboard", true));
                items.Add(new CoopMenuItem(CoopMenuAction.DisconnectToSolo, "Leave Lobby & Return to Solo Story", true));
                items.Add(new CoopMenuItem(CoopMenuAction.DisableMultiplayer, DisableLabel, true));
            }

            if (!string.IsNullOrEmpty(onlineInfo))
                items.Add(new CoopMenuItem(CoopMenuAction.None, onlineInfo, false));

            if (SelectedIndex >= items.Count) SelectedIndex = Math.Max(0, items.Count - 1);
            return items.ToArray();
        }
    }
}
