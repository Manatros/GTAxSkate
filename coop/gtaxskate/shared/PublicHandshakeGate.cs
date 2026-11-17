// Throttle for password guesses from PUBLIC internet endpoints (design 2026-11-16-coop-internet-lobby, item 5).
// This only slows guessing; it is no security boundary (source addresses rotate or are spoofed, and it runs
// after RageCoop's own RSA step). Loopback, LAN and VPN endpoints never pass through it.
//   - per IP: 5 failures within 10 minutes -> that IP is refused for 10 minutes;
//   - bounded table: at most 256 IPs, the least recently seen is evicted first;
//   - global: more than 30 failed public handshakes within 1 minute -> every new public handshake is
//     refused for 1 minute.
// The caller passes a monotonic clock (milliseconds); not thread safe (the server adapter holds its lock).
using System;
using System.Collections.Generic;

namespace GTAxSkate.Coop
{
    public sealed class PublicHandshakeGate
    {
        public const int MaxTrackedAddresses = 256;
        public const int PerAddressFailures = 5;
        public const long PerAddressWindowMs = 10L * 60 * 1000;
        public const long PerAddressLockMs = 10L * 60 * 1000;
        public const int GlobalFailures = 30;          // more than this many ...
        public const long GlobalWindowMs = 60L * 1000;  // ... within this window ...
        public const long GlobalLockMs = 60L * 1000;    // ... locks all public handshakes this long.

        private sealed class Entry
        {
            public readonly List<long> Failures = new List<long>();
            public long LockedUntil;
            public long LastSeen;
        }

        private readonly Dictionary<string, Entry> table = new Dictionary<string, Entry>(StringComparer.Ordinal);
        private readonly Queue<long> globalFailures = new Queue<long>();
        private long globalLockedUntil = -1;
        private bool globalLocked;

        public int TrackedAddresses { get { return table.Count; } }
        public bool GlobalLockActive(long nowMs) { return globalLocked && nowMs < globalLockedUntil; }
        // Set by the last call that started a lock; the adapter traces it (never with the address).
        public string LastEvent { get; private set; }

        // False: refuse this handshake without even checking the password.
        public bool TryBegin(string address, long nowMs, out string reason)
        {
            reason = null;
            if (GlobalLockActive(nowMs)) { reason = "global public-handshake lockout"; return false; }
            Entry e;
            if (address != null && table.TryGetValue(address, out e))
            {
                e.LastSeen = nowMs;
                if (nowMs < e.LockedUntil) { reason = "address locked out"; return false; }
            }
            return true;
        }

        public void RecordFailure(string address, long nowMs)
        {
            LastEvent = null;
            // Global counter.
            globalFailures.Enqueue(nowMs);
            while (globalFailures.Count > 0 && nowMs - globalFailures.Peek() > GlobalWindowMs) globalFailures.Dequeue();
            if (globalFailures.Count > GlobalFailures)
            {
                globalLocked = true;
                globalLockedUntil = nowMs + GlobalLockMs;
                globalFailures.Clear();
                LastEvent = "global lockout started";
            }

            if (address == null) return;
            Entry e;
            if (!table.TryGetValue(address, out e))
            {
                if (table.Count >= MaxTrackedAddresses) EvictOldest();
                e = new Entry();
                table[address] = e;
            }
            e.LastSeen = nowMs;
            for (int i = e.Failures.Count - 1; i >= 0; i--)
                if (nowMs - e.Failures[i] > PerAddressWindowMs) e.Failures.RemoveAt(i);
            e.Failures.Add(nowMs);
            if (e.Failures.Count >= PerAddressFailures)
            {
                e.LockedUntil = nowMs + PerAddressLockMs;
                e.Failures.Clear();
                LastEvent = "address lockout started";
            }
        }

        // A correct password clears that address's failures (a typo then a right entry is not a guess run).
        public void RecordSuccess(string address)
        {
            if (address != null) table.Remove(address);
        }

        private void EvictOldest()
        {
            string oldest = null; long oldestSeen = long.MaxValue;
            foreach (var pair in table)
                if (pair.Value.LastSeen < oldestSeen) { oldestSeen = pair.Value.LastSeen; oldest = pair.Key; }
            if (oldest != null) table.Remove(oldest);
        }
    }
}
