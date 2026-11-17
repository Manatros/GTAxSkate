// Opt-in switch for GTAxSkate multiplayer: default OFF, persisted in
// scripts\RageCoop\FriendsCoopPrefs.json, advanced by a generation counter on
// every change so queued/stale callbacks fail closed.
//
// Lives in GTAxSkate.Coop.Client.dll (no GTA.Script type, no GTA/RageCoop
// references, no side effects in static initialization). The bootstrap DLL reads
// it through its normal reference to the client DLL.
// State is kept in AppDomain data slots built from mscorlib types only, so every
// loaded copy of this assembly (bootstrap-resolved and RageCoop-resource-loaded)
// sees ONE state even if the loader produced two type identities.
using System;
using System.IO;
using GTAxSkate.Coop;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public static class CoopRuntimeControl
    {
        private const string StateSlot = "GTAxSkate.Coop.RuntimeControl.State.v1";
        private const string PathSlot = "GTAxSkate.Coop.RuntimeControl.PrefsPath.v1";
        private const string OwnerSlot = "GTAxSkate.Coop.RuntimeControl.Owner.v1";
        private const int Initialized = 0, EnabledFlag = 1, GenerationIdx = 2;

        private static int[] State()
        {
            lock (AppDomain.CurrentDomain)
            {
                int[] s = AppDomain.CurrentDomain.GetData(StateSlot) as int[];
                if (s == null)
                {
                    s = new int[3];
                    AppDomain.CurrentDomain.SetData(StateSlot, s);
                }
                return s;
            }
        }

        public static bool IsInitialized { get { int[] s = State(); lock (s) { return s[Initialized] != 0; } } }
        public static bool Enabled { get { int[] s = State(); lock (s) { return s[Initialized] != 0 && s[EnabledFlag] != 0; } } }
        public static int Generation { get { int[] s = State(); lock (s) { return s[GenerationIdx]; } } }
        public static string PrefsPath { get { return AppDomain.CurrentDomain.GetData(PathSlot) as string; } }

        // True only while multiplayer is on AND no disable/enable has happened since `generation` was captured.
        public static bool IsCurrent(int generation)
        {
            int[] s = State();
            lock (s) { return s[Initialized] != 0 && s[EnabledFlag] != 0 && s[GenerationIdx] == generation; }
        }

        // Reads the persisted choice ONCE per script domain; later calls return the current state
        // (so handoff or a re-created bootstrap never rereads the file or undoes a runtime choice).
        public static bool Initialize(string prefsPath)
        {
            int[] s = State();
            lock (s)
            {
                if (s[Initialized] != 0) return s[EnabledFlag] != 0;
                AppDomain.CurrentDomain.SetData(PathSlot, prefsPath);
                s[EnabledFlag] = CoopPrefs.Read(prefsPath) ? 1 : 0;
                s[Initialized] = 1;
                s[GenerationIdx]++;
                // Multiplayer was left on: RageCoop's own client menu stays locked from the start of this session.
                if (s[EnabledFlag] != 0) SyncRageCoopUiLock(prefsPath, true);
                return s[EnabledFlag] != 0;
            }
        }

        // Explicit user choice. Advances the generation on every real change (fail-closed for stale work),
        // then persists when a prefs path is known. persistError is empty on success or when nothing was saved.
        public static bool SetEnabled(bool enabled, out string persistError)
        {
            persistError = string.Empty;
            string path;
            lock (AppDomain.CurrentDomain)
            {
                int[] s = State();
                lock (s)
                {
                    bool was = s[Initialized] != 0 && s[EnabledFlag] != 0;
                    s[Initialized] = 1;
                    s[EnabledFlag] = enabled ? 1 : 0;
                    if (was != enabled) s[GenerationIdx]++;
                }
                path = PrefsPath;
            }
            if (string.IsNullOrEmpty(path)) return true;
            bool written = CoopPrefs.Write(path, enabled, out persistError);
            // Design 2026-11-17-streamer-safe-coop: while multiplayer is on, RageCoop's own client UI (F9 menu with server
            // address and name, server browser) is locked in its settings file; switching off puts the user's values back.
            SyncRageCoopUiLock(path, enabled);
            return written;
        }

        // scripts\RageCoop\FriendsCoopPrefs.json -> scripts\RageCoop\Data\RageCoop.Client.Settings.xml. Best effort, never throws.
        public static string RageCoopClientSettingsPath(string prefsPath)
        {
            if (string.IsNullOrEmpty(prefsPath)) return null;
            string dir = Path.GetDirectoryName(prefsPath);
            return string.IsNullOrEmpty(dir) ? null : Path.Combine(Path.Combine(dir, "Data"), "RageCoop.Client.Settings.xml");
        }

        private const string UiLockErrorSlot = "GTAxSkate.Coop.UiLockError.v1";

        // A failed lock/restore of RageCoop's settings file is reported to the player (never swallowed); the text is fixed
        // (the system error can carry a path). The next reader takes it once.
        public static string TakeUiLockError()
        {
            lock (AppDomain.CurrentDomain)
            {
                string e = AppDomain.CurrentDomain.GetData(UiLockErrorSlot) as string;
                AppDomain.CurrentDomain.SetData(UiLockErrorSlot, null);
                return e ?? string.Empty;
            }
        }

        private static void SetUiLockError(string message)
        {
            lock (AppDomain.CurrentDomain) AppDomain.CurrentDomain.SetData(UiLockErrorSlot, message);
        }

        private static void SyncRageCoopUiLock(string prefsPath, bool locked)
        {
            try
            {
                string settingsPath = RageCoopClientSettingsPath(prefsPath);
                if (string.IsNullOrEmpty(settingsPath)) return;
                string error;
                bool ok = locked ? RageCoopClientSettings.Apply(settingsPath, out error) : RageCoopClientSettings.Restore(settingsPath, out error);
                if (!ok)
                {
                    CoopTrace.Write("client", "RageCoop settings file " + (locked ? "lock" : "restore") + " failed");
                    SetUiLockError(locked
                        ? "RageCoop's settings file could not be locked: its own F9 menu may show your name or server address."
                        : "RageCoop's settings file could not be restored: check scripts\\RageCoop\\Data\\RageCoop.Client.Settings.xml.");
                }
            }
            catch (Exception e)
            {
                CoopTrace.Write("client", "RageCoop settings file sync failed: " + CoopTrace.Describe(e));
                SetUiLockError("RageCoop's settings file could not be updated: its own F9 menu may show your name or server address.");
            }
        }

        // Owner arbitration shared by EVERY loaded copy of this assembly: one token (a Guid, an mscorlib
        // type) lives in an AppDomain data slot, so exactly one client adapter can be live even if the
        // loader produced two type identities (bootstrap-resolved and RageCoop-resource-loaded).
        // The newest start claims (same rule as the in-copy handoff); every compare is done under one lock.
        private static object[] OwnerBox()
        {
            lock (AppDomain.CurrentDomain)
            {
                object[] box = AppDomain.CurrentDomain.GetData(OwnerSlot) as object[];
                if (box == null)
                {
                    // [0] owner token (Guid), [1] last heartbeat ms (long), [2] first-observed ms (long)
                    box = new object[3];
                    AppDomain.CurrentDomain.SetData(OwnerSlot, box);
                }
                return box;
            }
        }

        // Unconditionally becomes the owner; returns the previous token (Guid.Empty if none).
        public static Guid ClaimOwner(Guid token)
        {
            object[] box = OwnerBox();
            lock (box)
            {
                Guid previous = box[0] is Guid ? (Guid)box[0] : Guid.Empty;
                box[0] = token;
                box[1] = null;
                box[2] = null;
                return previous;
            }
        }

        public static bool IsOwner(Guid token)
        {
            if (token == Guid.Empty) return false;
            object[] box = OwnerBox();
            lock (box) { return box[0] is Guid && (Guid)box[0] == token; }
        }

        // Compare-exchange to "no owner": only the current owner can release. True when it did.
        public static bool ReleaseOwner(Guid token)
        {
            if (token == Guid.Empty) return false;
            object[] box = OwnerBox();
            lock (box)
            {
                if (!(box[0] is Guid) || (Guid)box[0] != token) return false;
                box[0] = null;
                box[1] = null;
                box[2] = null;
                return true;
            }
        }

        // Owner heartbeat: only the current owner writes. nowMs is a wall-clock ms value
        // (DateTime.UtcNow ticks / 10000), comparable across assembly copies.
        public static void Heartbeat(Guid token, long nowMs)
        {
            if (token == Guid.Empty) return;
            object[] box = OwnerBox();
            lock (box)
            {
                if (box[0] is Guid && (Guid)box[0] == token) box[1] = nowMs;
            }
        }

        public static long UtcNowMs() { return DateTime.UtcNow.Ticks / TimeSpan.TicksPerMillisecond; }

        // True when ANOTHER token owns AND it beat within staleMs. An owner that never beat yet counts as
        // fresh for staleMs after it was first observed here (a crashed copy cannot block forever).
        public static bool HasLiveOtherOwner(Guid mine, long nowMs, long staleMs)
        {
            object[] box = OwnerBox();
            lock (box)
            {
                if (!(box[0] is Guid) || (Guid)box[0] == Guid.Empty || (Guid)box[0] == mine) return false;
                long baseline;
                if (box[1] is long) baseline = (long)box[1];
                else
                {
                    if (!(box[2] is long)) box[2] = nowMs;
                    baseline = (long)box[2];
                }
                return nowMs - baseline < staleMs;
            }
        }

        // True when some other token currently owns (a displaced copy must not stop the owner's server).
        public static bool HasOtherOwner(Guid token)
        {
            object[] box = OwnerBox();
            lock (box) { return box[0] is Guid && (Guid)box[0] != Guid.Empty && (Guid)box[0] != token; }
        }

        // Test-only hook: forget everything (fixtures must opt in explicitly afterwards).
        internal static void ResetForTests()
        {
            lock (AppDomain.CurrentDomain)
            {
                AppDomain.CurrentDomain.SetData(StateSlot, null);
                AppDomain.CurrentDomain.SetData(PathSlot, null);
                AppDomain.CurrentDomain.SetData(OwnerSlot, null);
            }
        }
    }
}
