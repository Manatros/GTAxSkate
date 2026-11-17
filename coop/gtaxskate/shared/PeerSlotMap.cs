// Explicit bidirectional map between signed nonzero transport IDs (e.g. RAGECOOP)
// and positive peer slots [1..] required by ServerPoseSessions and session state.
using System;
using System.Collections.Generic;

namespace GTAxSkate.Coop
{
    public sealed class PeerSlotMap
    {
        public const int MaxPeers = ServerPoseSessions.MaxPeers;
        private readonly Dictionary<int, int> rawToSlot = new Dictionary<int, int>();
        private readonly Dictionary<int, int> slotToRaw = new Dictionary<int, int>();
        private readonly object gate = new object();
        private int nextSlot = 1;

        public int Count
        {
            get { lock (gate) { return rawToSlot.Count; } }
        }

        // Allocates a fresh strictly positive slot for any nonzero signed raw client ID.
        // Never casts unchecked or uses Math.Abs (so -5 and +5 remain distinct peers).
        public bool TryAssignOrGetSlot(int rawClientId, out int slot)
        {
            slot = 0;
            if (rawClientId == 0) return false;
            lock (gate)
            {
                if (rawToSlot.TryGetValue(rawClientId, out slot)) return true;
                if (rawToSlot.Count >= MaxPeers) { slot = 0; return false; }

                int candidate = FindFreePositiveSlot();
                if (candidate <= 0) { slot = 0; return false; }

                rawToSlot[rawClientId] = candidate;
                slotToRaw[candidate] = rawClientId;
                slot = candidate;
                return true;
            }
        }

        public bool TryGetSlot(int rawClientId, out int slot)
        {
            slot = 0;
            if (rawClientId == 0) return false;
            lock (gate) { return rawToSlot.TryGetValue(rawClientId, out slot); }
        }

        public bool TryGetRawId(int slot, out int rawClientId)
        {
            rawClientId = 0;
            if (slot <= 0) return false;
            lock (gate) { return slotToRaw.TryGetValue(slot, out rawClientId); }
        }

        public bool TryReleaseByRawId(int rawClientId, out int releasedSlot)
        {
            releasedSlot = 0;
            if (rawClientId == 0) return false;
            lock (gate)
            {
                if (!rawToSlot.TryGetValue(rawClientId, out releasedSlot)) return false;
                rawToSlot.Remove(rawClientId);
                slotToRaw.Remove(releasedSlot);
                return true;
            }
        }

        public void Clear()
        {
            lock (gate)
            {
                rawToSlot.Clear();
                slotToRaw.Clear();
                nextSlot = 1;
            }
        }

        private int FindFreePositiveSlot()
        {
            // Monotonic allocation avoids accidental reuse across rapid reconnects.
            int guard = 0;
            while (guard < MaxPeers + 2)
            {
                int candidate = nextSlot;
                nextSlot = (nextSlot == int.MaxValue) ? 1 : nextSlot + 1;
                if (candidate > 0 && !slotToRaw.ContainsKey(candidate)) return candidate;
                guard++;
            }
            return 0;
        }
    }
}
