// Bounded wire constants and validators for RAGECOOP private Friend-Session events.
using System;
using System.IO;
using System.Text;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    internal static class SessionProtocol
    {
        internal const int Version = 1;
        internal const int MaxTextBytes = 255;
        internal const int MaxWarningBytes = 1024;

        internal const string FriendAuthInit = "GTAxSkate.Coop.Session.v1.AuthInit";
        internal const string FriendAuthChallenge = "GTAxSkate.Coop.Session.v1.AuthChallenge";
        internal const string FriendAuthProof = "GTAxSkate.Coop.Session.v1.AuthProof";
        internal const string FriendAuthAccepted = "GTAxSkate.Coop.Session.v1.AuthAccepted";
        internal const string SessionHost = "GTAxSkate.Coop.Session.v1.Host";
        internal const string SessionJoin = "GTAxSkate.Coop.Session.v1.Join";
        internal const string SessionLeave = "GTAxSkate.Coop.Session.v1.Leave";
        internal const string SessionConfigure = "GTAxSkate.Coop.Session.v1.Configure";
        internal const string SessionStart = "GTAxSkate.Coop.Session.v1.Start";
        internal const string SessionBoard = "GTAxSkate.Coop.Session.v1.Board";
        internal const string SessionScore = "GTAxSkate.Coop.Session.v1.Score";
        internal const string SessionSnapshot = "GTAxSkate.Coop.Session.v1.Snapshot";
        internal const string SessionRejected = "GTAxSkate.Coop.Session.v1.Rejected";
        internal const string SessionModsReport = "GTAxSkate.Coop.Session.v1.ModsReport";
        internal const string SessionModWarning = "GTAxSkate.Coop.Session.v1.ModWarning";
        internal const string SessionHostWorldSync = "GTAxSkate.Coop.Session.v1.HostWorldSync";
        internal const string SessionHostShopSync = "GTAxSkate.Coop.Session.v1.HostShopSync";
        internal const string SessionRagdollSync = "GTAxSkate.Coop.Session.v1.RagdollSync";
        internal const int MaxWorldSyncBytes = 256;
        internal const int MaxShopSyncBytes = 512;
        internal const int MaxRagdollSyncBytes = 96;

        internal static bool Header(object[] args, int exactCount)
        {
            return args != null && args.Length == exactCount && args[0] is int && (int)args[0] == Version;
        }

        internal static bool Proof(object value)
        {
            return value is byte[] && ((byte[])value).Length == CoopSecurityPolicy.ProofBytes;
        }

        internal static bool BoundedText(object value, int maxLen)
        {
            return value is string && ((string)value).Length <= maxLen;
        }

        internal static byte[] EncodeSnapshot(SkateSessionSnapshot snap, long serverNowMs)
        {
            if (snap == null) return null;
            using (var ms = new MemoryStream())
            using (var w = new BinaryWriter(ms, Encoding.UTF8))
            {
                w.Write(Version);
                WriteAscii(w, snap.JoinCode, 16);
                WriteAscii(w, snap.Title, SkateSessionCoordinator.MaxTitleChars);
                w.Write(snap.HostSlot);
                w.Write((int)snap.Zone);
                w.Write((int)snap.Mode);
                WriteAscii(w, snap.SpotId, SkateSessionCoordinator.MaxSpotIdChars);
                w.Write((int)snap.Phase);
                long remaining = (snap.Phase == ActivityPhase.Active && snap.PhaseEndsAtMs > serverNowMs)
                    ? (snap.PhaseEndsAtMs - serverNowMs)
                    : 0L;
                w.Write(remaining);
                int count = snap.Peers != null ? Math.Min(snap.Peers.Length, SkateSessionCoordinator.MaxPlayersPerSession) : 0;
                w.Write(count);
                for (int i = 0; i < count; i++)
                {
                    SessionPeerState p = snap.Peers[i];
                    w.Write(p.Slot);
                    w.Write(p.RoundScore);
                    w.Write(p.BestLineScore);
                    w.Write(p.Ready);
                    WriteAscii(w, p.BoardProfile, SkateSessionCoordinator.MaxBoardProfileChars);
                }
                w.Flush();
                return ms.ToArray();
            }
        }

        internal static bool TryDecodeSnapshot(byte[] data, long clientNowMs, out SkateSessionSnapshot snap)
        {
            snap = null;
            if (data == null || data.Length < 28 || data.Length > 4096) return false;
            try
            {
                using (var ms = new MemoryStream(data, false))
                using (var r = new BinaryReader(ms, Encoding.UTF8))
                {
                    int ver = r.ReadInt32();
                    if (ver != Version) return false;
                    string code = ReadAscii(r, 16);
                    string title = ReadAscii(r, SkateSessionCoordinator.MaxTitleChars);
                    int hostSlot = r.ReadInt32();
                    int zoneRaw = r.ReadInt32();
                    int modeRaw = r.ReadInt32();
                    string spotId = ReadAscii(r, SkateSessionCoordinator.MaxSpotIdChars);
                    int phaseRaw = r.ReadInt32();
                    long remaining = r.ReadInt64();
                    int count = r.ReadInt32();

                    if (hostSlot <= 0 || !Enum.IsDefined(typeof(SkateWorldZone), zoneRaw) ||
                        !Enum.IsDefined(typeof(SkateMode), modeRaw) || !Enum.IsDefined(typeof(ActivityPhase), phaseRaw) ||
                        remaining < 0 || remaining > SkateSessionCoordinator.MaxActivityDurationMs ||
                        count < 0 || count > SkateSessionCoordinator.MaxPlayersPerSession)
                        return false;

                    var peers = new SessionPeerState[count];
                    for (int i = 0; i < count; i++)
                    {
                        int slot = r.ReadInt32();
                        int roundScore = r.ReadInt32();
                        int bestScore = r.ReadInt32();
                        bool ready = r.ReadBoolean();
                        string board = ReadAscii(r, SkateSessionCoordinator.MaxBoardProfileChars);
                        if (slot <= 0 || roundScore < 0 || bestScore < 0) return false;
                        var p = new SessionPeerState(slot, clientNowMs);
                        p.RoundScore = roundScore;
                        p.BestLineScore = bestScore;
                        p.Ready = ready;
                        p.BoardProfile = board;
                        peers[i] = p;
                    }

                    if (ms.Position != data.Length) return false;
                    long endsAt = remaining > 0 ? clientNowMs + remaining : 0L;
                    snap = new SkateSessionSnapshot(
                        code, title, hostSlot, (SkateWorldZone)zoneRaw, (SkateMode)modeRaw,
                        spotId, (ActivityPhase)phaseRaw, endsAt, peers);
                    return true;
                }
            }
            catch
            {
                return false;
            }
        }

        private static void WriteAscii(BinaryWriter w, string value, int maxChars)
        {
            string s = value ?? string.Empty;
            int len = Math.Min(s.Length, maxChars);
            byte[] bytes = Encoding.ASCII.GetBytes(s.Substring(0, len));
            w.Write((ushort)bytes.Length);
            if (bytes.Length > 0) w.Write(bytes);
        }

        private static string ReadAscii(BinaryReader r, int maxChars)
        {
            ushort len = r.ReadUInt16();
            if (len > maxChars) throw new InvalidDataException("String length exceeds maximum.");
            if (len == 0) return string.Empty;
            byte[] bytes = r.ReadBytes(len);
            if (bytes.Length != len) throw new EndOfStreamException();
            return Encoding.ASCII.GetString(bytes);
        }

        internal static byte[] EncodeModsManifest(ModEntry[] entries)
        {
            ModEntry[] normalized = ModCompatibilityChecker.NormalizeManifest(entries);
            using (var ms = new MemoryStream())
            using (var w = new BinaryWriter(ms, Encoding.UTF8))
            {
                w.Write(Version);
                w.Write(normalized.Length);
                for (int i = 0; i < normalized.Length; i++)
                {
                    WriteAscii(w, normalized[i].RelativeId, 64);
                    WriteAscii(w, normalized[i].Digest, 16);
                    w.Write((byte)normalized[i].Impact);
                }
                w.Flush();
                return ms.ToArray();
            }
        }

        internal static bool TryDecodeModsManifest(byte[] data, out ModEntry[] entries)
        {
            entries = null;
            if (data == null || data.Length < 8 || data.Length > 4096) return false;
            try
            {
                using (var ms = new MemoryStream(data, false))
                using (var r = new BinaryReader(ms, Encoding.UTF8))
                {
                    int ver = r.ReadInt32();
                    if (ver != Version) return false;
                    int count = r.ReadInt32();
                    if (count < 0 || count > ModCompatibilityChecker.MaxModEntries) return false;

                    var list = new ModEntry[count];
                    for (int i = 0; i < count; i++)
                    {
                        string id = ReadAscii(r, 64);
                        string digest = ReadAscii(r, 16);
                        byte impactRaw = r.ReadByte();
                        if (!Enum.IsDefined(typeof(ModImpactClass), (int)impactRaw)) return false;
                        list[i] = new ModEntry(id, digest, (ModImpactClass)impactRaw);
                    }
                    if (ms.Position != data.Length) return false;
                    entries = list;
                    return true;
                }
            }
            catch
            {
                return false;
            }
        }

        internal static byte[] EncodeWorldSync(HostWorldSyncState state)
        {
            if (state == null || !state.IsValid()) return null;
            using (var ms = new MemoryStream())
            using (var w = new BinaryWriter(ms, Encoding.UTF8))
            {
                w.Write(Version);
                w.Write(state.Epoch);
                w.Write((byte)state.ClockHour);
                w.Write((byte)state.ClockMinute);
                w.Write(state.ClockPaused);
                w.Write((byte)state.WeatherId);
                w.Write(state.MissionActive);
                w.Write(state.CutsceneActive);
                w.Write(state.HostInVehicle);
                w.Write((byte)state.WantedLevel);
                w.Write(state.WantedCoordX); w.Write(state.WantedCoordY); w.Write(state.WantedCoordZ);
                w.Write(state.HostPosX); w.Write(state.HostPosY); w.Write(state.HostPosZ);
                w.Write(state.HasObjective);
                w.Write(state.ObjectiveX); w.Write(state.ObjectiveY); w.Write(state.ObjectiveZ);
                w.Write(state.ObjectiveSprite);
                w.Write((byte)state.ObjectiveColor);
                w.Write(state.AllowShopPurchase);
                w.Flush();
                byte[] bytes = ms.ToArray();
                return bytes.Length <= MaxWorldSyncBytes ? bytes : null;
            }
        }

        internal static bool TryDecodeWorldSync(byte[] data, out HostWorldSyncState state)
        {
            state = null;
            if (data == null || data.Length < 48 || data.Length > MaxWorldSyncBytes) return false;
            try
            {
                using (var ms = new MemoryStream(data, false))
                using (var r = new BinaryReader(ms, Encoding.UTF8))
                {
                    if (r.ReadInt32() != Version) return false;
                    int epoch = r.ReadInt32();
                    int hour = r.ReadByte();
                    int minute = r.ReadByte();
                    bool paused = r.ReadBoolean();
                    int weather = r.ReadByte();
                    bool mission = r.ReadBoolean();
                    bool cutscene = r.ReadBoolean();
                    bool inVeh = r.ReadBoolean();
                    int wanted = r.ReadByte();
                    float wx = r.ReadSingle(), wy = r.ReadSingle(), wz = r.ReadSingle();
                    float hx = r.ReadSingle(), hy = r.ReadSingle(), hz = r.ReadSingle();
                    bool hasObj = r.ReadBoolean();
                    float ox = r.ReadSingle(), oy = r.ReadSingle(), oz = r.ReadSingle();
                    int sprite = r.ReadInt32();
                    int color = r.ReadByte();
                    bool allowShop = r.ReadBoolean();
                    if (ms.Position != data.Length) return false;

                    var candidate = new HostWorldSyncState(
                        epoch, hour, minute, paused, weather, mission, cutscene, inVeh, wanted,
                        wx, wy, wz, hx, hy, hz, hasObj, ox, oy, oz, sprite, color, allowShop);
                    if (!candidate.IsValid()) return false;
                    state = candidate;
                    return true;
                }
            }
            catch { return false; }
        }

        internal static byte[] EncodeShopSync(HostShopSyncState state)
        {
            if (state == null || !state.IsValid()) return null;
            using (var ms = new MemoryStream())
            using (var w = new BinaryWriter(ms, Encoding.UTF8))
            {
                w.Write(Version);
                w.Write(state.ShopEpoch);
                w.Write(state.HostCash);
                WriteAscii(w, state.EquippedBoardProfile, SkateSessionCoordinator.MaxBoardProfileChars);
                w.Write(state.CrewVehicleModelHash);
                w.Write((byte)state.CrewVehiclePrimaryColor);
                w.Write((byte)state.CrewVehicleSecondaryColor);
                WriteAscii(w, state.CrewVehiclePlate, HostShopSyncState.MaxPlateChars);
                w.Write(state.SharedWeaponHash);
                WriteAscii(w, state.StoreSyncNotice, HostShopSyncState.MaxNoticeChars);
                w.Flush();
                byte[] bytes = ms.ToArray();
                return bytes.Length <= MaxShopSyncBytes ? bytes : null;
            }
        }

        internal static bool TryDecodeShopSync(byte[] data, out HostShopSyncState state)
        {
            state = null;
            if (data == null || data.Length < 22 || data.Length > MaxShopSyncBytes) return false;
            try
            {
                using (var ms = new MemoryStream(data, false))
                using (var r = new BinaryReader(ms, Encoding.UTF8))
                {
                    if (r.ReadInt32() != Version) return false;
                    int epoch = r.ReadInt32();
                    int cash = r.ReadInt32();
                    string board = ReadAscii(r, SkateSessionCoordinator.MaxBoardProfileChars);
                    uint vehModel = r.ReadUInt32();
                    int c1 = r.ReadByte();
                    int c2 = r.ReadByte();
                    string plate = ReadAscii(r, HostShopSyncState.MaxPlateChars);
                    uint weap = r.ReadUInt32();
                    string notice = ReadAscii(r, HostShopSyncState.MaxNoticeChars);
                    if (ms.Position != data.Length) return false;

                    var candidate = new HostShopSyncState(epoch, cash, board, vehModel, c1, c2, plate, weap, notice);
                    if (!candidate.IsValid()) return false;
                    state = candidate;
                    return true;
                }
            }
            catch { return false; }
        }

        internal static byte[] EncodeRagdollSync(RagdollSyncState state)
        {
            if (state == null || !state.IsValid()) return null;
            using (var ms = new MemoryStream())
            using (var w = new BinaryWriter(ms, Encoding.UTF8))
            {
                w.Write(Version);
                w.Write(state.SourceSlot);
                w.Write(state.IsRagdoll);
                w.Write(state.PosX); w.Write(state.PosY); w.Write(state.PosZ);
                w.Write(state.VelX); w.Write(state.VelY); w.Write(state.VelZ);
                w.Write(state.DurationMs);
                w.Flush();
                byte[] bytes = ms.ToArray();
                return bytes.Length <= MaxRagdollSyncBytes ? bytes : null;
            }
        }

        internal static bool TryDecodeRagdollSync(byte[] data, int enforcedSourceSlot, out RagdollSyncState state)
        {
            state = null;
            if (data == null || data.Length != 37 || enforcedSourceSlot <= 0) return false;
            try
            {
                using (var ms = new MemoryStream(data, false))
                using (var r = new BinaryReader(ms, Encoding.UTF8))
                {
                    if (r.ReadInt32() != Version) return false;
                    r.ReadInt32(); // Ignore client-claimed slot; enforce authenticated sender slot.
                    bool isRag = r.ReadBoolean();
                    float px = r.ReadSingle(), py = r.ReadSingle(), pz = r.ReadSingle();
                    float vx = r.ReadSingle(), vy = r.ReadSingle(), vz = r.ReadSingle();
                    int dur = r.ReadInt32();
                    if (ms.Position != data.Length) return false;

                    var candidate = new RagdollSyncState(enforcedSourceSlot, isRag, px, py, pz, vx, vy, vz, dur);
                    if (!candidate.IsValid()) return false;
                    state = candidate;
                    return true;
                }
            }
            catch { return false; }
        }
    }
}
