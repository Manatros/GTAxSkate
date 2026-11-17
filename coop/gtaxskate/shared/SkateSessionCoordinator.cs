// Transport-neutral Skate 3-style session & Online Freeskate Activity coordinator.
// Shared across RAGECOOP-V CustomEvent relays and future skate_net direct host sessions.
using System;
using System.Collections.Generic;

namespace GTAxSkate.Coop
{
    public enum SkateMode
    {
        FreeSkate = 0,
        SpotBattle = 1,
        OneUp = 2,
        SKATE = 3,
        Deathrace = 4,
        Domination = 5,
        FreeskateAccumulation = 6,
        FreeskateGapTag = 7,
        FreeskateTrickList = 8,
        FreeskateSurvival = 9
    }

    public enum SkateWorldZone
    {
        LosSantos = 0,
        SkateSchoolIsland = 1,
        StartParkIsland = 2
    }

    public enum ActivityPhase
    {
        Lobby = 0,
        Active = 1,
        Finished = 2
    }

    public sealed class SessionPeerState
    {
        public readonly int Slot;
        public string BoardProfile { get; internal set; }
        public int RoundScore { get; internal set; }
        public int BestLineScore { get; internal set; }
        public bool Ready { get; internal set; }
        internal long JoinedAtMs;

        internal SessionPeerState(int slot, long joinedAtMs)
        {
            Slot = slot;
            JoinedAtMs = joinedAtMs;
            BoardProfile = string.Empty;
        }
    }

    public sealed class SkateSessionSnapshot
    {
        public readonly string JoinCode;
        public readonly string Title;
        public readonly int HostSlot;
        public readonly SkateWorldZone Zone;
        public readonly SkateMode Mode;
        public readonly string SpotId;
        public readonly ActivityPhase Phase;
        public readonly long PhaseEndsAtMs;
        public readonly SessionPeerState[] Peers;

        internal SkateSessionSnapshot(
            string joinCode, string title, int hostSlot, SkateWorldZone zone,
            SkateMode mode, string spotId, ActivityPhase phase, long phaseEndsAtMs,
            SessionPeerState[] peers)
        {
            JoinCode = joinCode;
            Title = title;
            HostSlot = hostSlot;
            Zone = zone;
            Mode = mode;
            SpotId = spotId;
            Phase = phase;
            PhaseEndsAtMs = phaseEndsAtMs;
            Peers = peers;
        }
    }

    public sealed class SkateSessionCoordinator
    {
        public const int MaxSessions = 16;
        public const int MaxPlayersPerSession = 10; // Matches Skate 3 / donor multiplayer.rs
        public const int MaxTitleChars = 48;
        public const int MaxSpotIdChars = 48;
        public const int MaxBoardProfileChars = 255;
        public const long MaxActivityDurationMs = 15 * 60 * 1000L;
        private const string CodeAlphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

        private sealed class Session
        {
            internal readonly string JoinCode;
            internal string Title;
            internal int HostSlot;
            internal SkateWorldZone Zone;
            internal SkateMode Mode;
            internal string SpotId;
            internal ActivityPhase Phase;
            internal long PhaseDurationMs;
            internal long PhaseStartedAtMs;
            internal readonly List<int> MemberOrder = new List<int>();
            internal readonly Dictionary<int, SessionPeerState> Peers = new Dictionary<int, SessionPeerState>();

            internal Session(string code, string title, int hostSlot, SkateWorldZone zone, SkateMode mode)
            {
                JoinCode = code;
                Title = title;
                HostSlot = hostSlot;
                Zone = zone;
                Mode = mode;
                SpotId = string.Empty;
                Phase = ActivityPhase.Lobby;
            }
        }

        private readonly Dictionary<string, Session> sessionsByCode = new Dictionary<string, Session>(StringComparer.Ordinal);
        private readonly Dictionary<int, string> peerToCode = new Dictionary<int, string>();
        private readonly object gate = new object();
        private uint codeSeed = 0x534B3833u; // "SK83"

        public int SessionCount
        {
            get { lock (gate) { return sessionsByCode.Count; } }
        }

        public bool TryCreateSession(
            int hostSlot, SkateWorldZone zone, SkateMode mode, string title, long nowMs, out string joinCode)
        {
            joinCode = null;
            if (hostSlot <= 0 || nowMs < 0) return false;
            string cleanTitle = SanitizeAscii(title, MaxTitleChars, "Skate Session");

            lock (gate)
            {
                if (peerToCode.ContainsKey(hostSlot)) return false;
                if (sessionsByCode.Count >= MaxSessions) return false;

                string code = AllocateJoinCode();
                if (code == null) return false;

                var session = new Session(code, cleanTitle, hostSlot, zone, mode);
                session.MemberOrder.Add(hostSlot);
                session.Peers[hostSlot] = new SessionPeerState(hostSlot, nowMs);

                sessionsByCode[code] = session;
                peerToCode[hostSlot] = code;
                joinCode = code;
                return true;
            }
        }

        public bool TryJoinByCode(int peerSlot, string joinCode, long nowMs)
        {
            if (peerSlot <= 0 || nowMs < 0 || string.IsNullOrEmpty(joinCode)) return false;
            string normalized = joinCode.Trim().ToUpperInvariant();

            lock (gate)
            {
                if (peerToCode.ContainsKey(peerSlot)) return false;
                Session session;
                if (!sessionsByCode.TryGetValue(normalized, out session)) return false;
                if (session.Peers.Count >= MaxPlayersPerSession) return false;

                session.MemberOrder.Add(peerSlot);
                session.Peers[peerSlot] = new SessionPeerState(peerSlot, nowMs);
                peerToCode[peerSlot] = normalized;
                return true;
            }
        }

        public bool TryLeaveOrDisconnect(int peerSlot, out bool sessionClosed, out int newHostSlot)
        {
            sessionClosed = false;
            newHostSlot = 0;
            if (peerSlot <= 0) return false;

            lock (gate)
            {
                string code;
                if (!peerToCode.TryGetValue(peerSlot, out code)) return false;
                peerToCode.Remove(peerSlot);

                Session session;
                if (!sessionsByCode.TryGetValue(code, out session)) return false;
                session.Peers.Remove(peerSlot);
                session.MemberOrder.Remove(peerSlot);

                if (session.MemberOrder.Count == 0)
                {
                    sessionsByCode.Remove(code);
                    sessionClosed = true;
                    return true;
                }

                if (session.HostSlot == peerSlot)
                {
                    // Migrate host ownership deterministically to longest-present remaining peer.
                    session.HostSlot = session.MemberOrder[0];
                    newHostSlot = session.HostSlot;
                }
                else
                {
                    newHostSlot = session.HostSlot;
                }
                return true;
            }
        }

        public bool TryConfigureActivity(
            int requesterSlot, SkateMode mode, SkateWorldZone zone, string spotId, long durationMs)
        {
            if (requesterSlot <= 0 || durationMs < 0 || durationMs > MaxActivityDurationMs) return false;
            string cleanSpot = SanitizeAscii(spotId, MaxSpotIdChars, string.Empty);

            lock (gate)
            {
                Session session;
                if (!TryHostSessionLocked(requesterSlot, out session)) return false;
                if (session.Phase == ActivityPhase.Active) return false;

                session.Mode = mode;
                session.Zone = zone;
                session.SpotId = cleanSpot;
                session.PhaseDurationMs = durationMs;
                session.Phase = ActivityPhase.Lobby;
                ResetScoresLocked(session);
                return true;
            }
        }

        public bool TryStartActivity(int requesterSlot, long nowMs)
        {
            if (requesterSlot <= 0 || nowMs < 0) return false;
            lock (gate)
            {
                Session session;
                if (!TryHostSessionLocked(requesterSlot, out session)) return false;
                if (session.Phase == ActivityPhase.Active) return false;

                ResetScoresLocked(session);
                session.Phase = ActivityPhase.Active;
                session.PhaseStartedAtMs = nowMs;
                return true;
            }
        }

        public bool TryUpdateBoardProfile(int peerSlot, string boardProfile)
        {
            if (peerSlot <= 0) return false;
            string clean = SanitizeAscii(boardProfile, MaxBoardProfileChars, string.Empty);

            lock (gate)
            {
                Session session;
                SessionPeerState peer;
                if (!TryPeerLocked(peerSlot, out session, out peer)) return false;
                peer.BoardProfile = clean;
                return true;
            }
        }

        public bool TrySetReady(int peerSlot, bool ready)
        {
            if (peerSlot <= 0) return false;
            lock (gate)
            {
                Session session;
                SessionPeerState peer;
                if (!TryPeerLocked(peerSlot, out session, out peer)) return false;
                peer.Ready = ready;
                return true;
            }
        }

        // Rejects negative scores, underwater positions, or out-of-window submissions.
        public bool TryReportScore(
            int peerSlot, int trickScore, int bestLineScore, bool dryLandVerified, long nowMs)
        {
            if (peerSlot <= 0 || trickScore < 0 || bestLineScore < 0 || !dryLandVerified || nowMs < 0)
                return false;

            lock (gate)
            {
                Session session;
                SessionPeerState peer;
                if (!TryPeerLocked(peerSlot, out session, out peer)) return false;
                if (session.Phase != ActivityPhase.Active) return false;
                if (session.PhaseDurationMs > 0 && nowMs > session.PhaseStartedAtMs + session.PhaseDurationMs)
                {
                    session.Phase = ActivityPhase.Finished;
                    return false;
                }

                peer.RoundScore = trickScore;
                if (bestLineScore > peer.BestLineScore) peer.BestLineScore = bestLineScore;
                return true;
            }
        }

        public bool TryGetSnapshotForPeer(int peerSlot, long nowMs, out SkateSessionSnapshot snapshot)
        {
            snapshot = null;
            if (peerSlot <= 0 || nowMs < 0) return false;
            lock (gate)
            {
                string code;
                Session session;
                if (!peerToCode.TryGetValue(peerSlot, out code) || !sessionsByCode.TryGetValue(code, out session))
                    return false;

                if (session.Phase == ActivityPhase.Active && session.PhaseDurationMs > 0 &&
                    nowMs >= session.PhaseStartedAtMs + session.PhaseDurationMs)
                {
                    session.Phase = ActivityPhase.Finished;
                }

                var peersCopy = new SessionPeerState[session.MemberOrder.Count];
                for (int i = 0; i < session.MemberOrder.Count; i++)
                {
                    SessionPeerState src = session.Peers[session.MemberOrder[i]];
                    var copy = new SessionPeerState(src.Slot, src.JoinedAtMs);
                    copy.BoardProfile = src.BoardProfile;
                    copy.RoundScore = src.RoundScore;
                    copy.BestLineScore = src.BestLineScore;
                    copy.Ready = src.Ready;
                    peersCopy[i] = copy;
                }

                // Deterministic ranking during active/finished scored modes: RoundScore desc, Slot asc.
                if (session.Mode != SkateMode.FreeSkate && session.Phase != ActivityPhase.Lobby)
                {
                    Array.Sort(peersCopy, delegate(SessionPeerState a, SessionPeerState b)
                    {
                        int cmp = b.RoundScore.CompareTo(a.RoundScore);
                        return cmp != 0 ? cmp : a.Slot.CompareTo(b.Slot);
                    });
                }

                long endsAt = (session.Phase == ActivityPhase.Active && session.PhaseDurationMs > 0)
                    ? session.PhaseStartedAtMs + session.PhaseDurationMs
                    : 0;

                snapshot = new SkateSessionSnapshot(
                    session.JoinCode, session.Title, session.HostSlot, session.Zone,
                    session.Mode, session.SpotId, session.Phase, endsAt, peersCopy);
                return true;
            }
        }

        private bool TryHostSessionLocked(int requesterSlot, out Session session)
        {
            session = null;
            string code;
            if (!peerToCode.TryGetValue(requesterSlot, out code)) return false;
            if (!sessionsByCode.TryGetValue(code, out session)) return false;
            return session.HostSlot == requesterSlot;
        }

        private bool TryPeerLocked(int peerSlot, out Session session, out SessionPeerState peer)
        {
            session = null;
            peer = null;
            string code;
            if (!peerToCode.TryGetValue(peerSlot, out code)) return false;
            if (!sessionsByCode.TryGetValue(code, out session)) return false;
            return session.Peers.TryGetValue(peerSlot, out peer);
        }

        private static void ResetScoresLocked(Session session)
        {
            foreach (var peer in session.Peers.Values)
            {
                peer.RoundScore = 0;
                peer.BestLineScore = 0;
            }
        }

        private string AllocateJoinCode()
        {
            for (int attempt = 0; attempt < 64; attempt++)
            {
                char[] chars = new char[6];
                for (int i = 0; i < 6; i++)
                {
                    codeSeed = codeSeed * 1664525u + 1013904223u;
                    chars[i] = CodeAlphabet[(int)((codeSeed >> 16) % (uint)CodeAlphabet.Length)];
                }
                string code = new string(chars);
                if (!sessionsByCode.ContainsKey(code)) return code;
            }
            return null;
        }

        private static string SanitizeAscii(string input, int maxChars, string fallback)
        {
            if (string.IsNullOrEmpty(input)) return fallback;
            var filtered = new List<char>(Math.Min(input.Length, maxChars));
            for (int i = 0; i < input.Length && filtered.Count < maxChars; i++)
            {
                char c = input[i];
                if (c >= 0x20 && c <= 0x7E) filtered.Add(c);
            }
            string result = new string(filtered.ToArray()).Trim();
            return result.Length == 0 ? fallback : result;
        }
    }
}
