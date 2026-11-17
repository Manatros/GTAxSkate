// UPnP port opening for "Host online (internet)" (design 2026-11-16-coop-internet-lobby, item 3). Pure logic
// behind IUpnpGateway; the IGD protocol client is shared/UpnpIgd.cs, its sockets live in client/NatUpnpGateway.cs.
//
//  - Every gateway call runs on a background MTA worker thread; the game thread only polls (never blocks).
//  - 8 s timeout: a late result is abandoned and the worker removes a mapping it created meanwhile.
//  - Only UDP <port> is mapped. Only mappings carrying OUR description are ever removed.
//  - A stale own mapping (left by a crash) is removed before adding (recovery at the next host).
//  - A non-global external address (CGNAT / double NAT / private) fails the attempt and removes our mapping.
//  - Any failure after the router accepted AddPortMapping removes our mapping again (own description only).
//  - Single flight across ALL services in the process (static counter): a new hosting is refused while any earlier
//    worker (add, timed-out add or removal) is still inside a router call, so a late cleanup can never delete the
//    mapping of a newer hosting (Copilot round 2).
//  - A removal first READS the entry (GetSpecificPortMappingEntry) and deletes only when its description is ours and its
//    internal client is this PC (Copilot round 3). A removal that fails or times out stays "pending" (AppDomain data, so a
//    successor copy sees it) and is retried at the next host attempt, at stop and at adapter start: bounded to 8 entries
//    and 5 retries each, own description only.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;

namespace GTAxSkate.Coop
{
    public sealed class UpnpMappingInfo
    {
        public int ExternalPort;
        public string Protocol;        // "UDP" / "TCP"
        public string InternalClient;
        public string Description;
        public string ExternalIp;
    }

    // Synchronous and possibly slow: only ever called from the worker thread.
    public interface IUpnpGateway
    {
        // False when no UPnP device answered. May throw UpnpUserException (player-facing) or network errors.
        bool IsAvailable();
        UpnpMappingInfo[] List();
        // Returns the new mapping (with its ExternalIp as reported by the router).
        UpnpMappingInfo Add(int port, string protocol, string internalClient, string description);
        // GetSpecificPortMappingEntry: the entry for this port/protocol, null when there is none. Throws on other faults.
        UpnpMappingInfo Get(int port, string protocol);
        void Remove(int port, string protocol);
    }

    public enum OnlineHostState { Idle = 0, Opening = 1, Open = 2, Failed = 3 }

    public sealed class OnlineHostStatus
    {
        public readonly OnlineHostState State;
        public readonly string ExternalIp;   // only when Open
        public readonly string Message;      // player-facing
        public readonly string TraceNote;    // for the trace (no address)

        public OnlineHostStatus(OnlineHostState state, string externalIp, string message, string traceNote)
        {
            State = state; ExternalIp = externalIp ?? string.Empty;
            Message = message ?? string.Empty; TraceNote = traceNote ?? string.Empty;
        }
        public static readonly OnlineHostStatus Idle = new OnlineHostStatus(OnlineHostState.Idle, null, null, null);
    }

    public sealed class UpnpHostPortService
    {
        public const int DefaultTimeoutMs = 8000;
        private readonly Func<IUpnpGateway> gatewayFactory;
        private readonly int timeoutMs;
        private readonly object gate = new object();
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private int generation;
        private OnlineHostStatus status = OnlineHostStatus.Idle;
        private long startedMs;
        private int port;
        private string description;
        private string lanClient;      // internal client the mapping was added with (this PC's LAN address)
        private bool mappingCreated;   // we added a mapping that has not been removed yet
        // Single flight (Copilot reviews 2026-11-17): a timed-out or removing worker may still be inside a slow router
        // call. The counter is static, so it holds across services (the bridge creates a new service per hosting):
        // no new attempt starts until every earlier worker has returned, and its late cleanup can never hit a newer mapping.
        private static readonly object FlightGate = new object();
        private static int flights;
        private static bool TryBeginFlight(bool exclusive)
        {
            lock (FlightGate)
            {
                if (exclusive && flights > 0) return false;
                flights++;
                return true;
            }
        }
        private static void EndFlight() { lock (FlightGate) { if (flights > 0) flights--; } }

        // The factory runs on the worker thread so COM objects live in the worker's (MTA) apartment.
        public UpnpHostPortService(Func<IUpnpGateway> gatewayFactory, int timeoutMs)
        {
            if (gatewayFactory == null) throw new ArgumentNullException("gatewayFactory");
            this.gatewayFactory = gatewayFactory;
            this.timeoutMs = timeoutMs > 0 ? timeoutMs : DefaultTimeoutMs;
        }

        public string Description { get { lock (gate) { return description; } } }
        public int Port { get { lock (gate) { return port; } } }

        public static string DescriptionFor(string installId)
        {
            return "GTAxSkate Coop " + (string.IsNullOrEmpty(installId) ? "install" : installId);
        }

        // Never blocks. A second Begin while Opening/Open is ignored.
        public void Begin(int udpPort, string mappingDescription, string lanIpv4)
        {
            int myGen;
            lock (gate)
            {
                if (status.State == OnlineHostState.Opening || status.State == OnlineHostState.Open) return;
                if (!TryBeginFlight(true))
                {
                    status = new OnlineHostStatus(OnlineHostState.Failed, null,
                        "The router is still answering the last request. Try again in a few seconds.", "upnp busy (previous worker alive)");
                    return;
                }
                myGen = ++generation;
                port = udpPort; description = mappingDescription; lanClient = lanIpv4;
                startedMs = clock.ElapsedMilliseconds;
                status = new OnlineHostStatus(OnlineHostState.Opening, null, "Opening the router port (UPnP)...", "upnp opening");
            }
            var worker = new Thread(delegate() { Work(myGen, udpPort, mappingDescription, lanIpv4); });
            worker.IsBackground = true;
            worker.Name = "GTAxSkate-UPnP";
            worker.SetApartmentState(ApartmentState.MTA);
            try { worker.Start(); }
            catch { EndFlight(); lock (gate) { status = Fail(udpPort, "upnp worker could not start"); } }
        }

        // Never blocks; call every tick.
        public OnlineHostStatus Poll()
        {
            lock (gate)
            {
                if (status.State == OnlineHostState.Opening && clock.ElapsedMilliseconds - startedMs >= timeoutMs)
                {
                    generation++;   // the worker's late result is now abandoned (it removes what it added)
                    status = Fail(port, "upnp timeout after " + timeoutMs + " ms");
                }
                return status;
            }
        }

        // Removes our own mapping, if any (also when a result is still pending). waitMs 0 = fire and forget
        // (game thread); tests wait. Returns true when the removal finished within waitMs (or nothing to do).
        public bool Release(int waitMs)
        {
            int removePort; string removeDescription, removeLan; bool needed;
            lock (gate)
            {
                generation++;   // abandon any worker still running: it cleans up after itself
                removePort = port; removeDescription = description; removeLan = lanClient;
                needed = mappingCreated;
                mappingCreated = false;
                status = OnlineHostStatus.Idle;
            }
            if (!needed)
            {
                RetryPendingAsync(gatewayFactory, 0);   // an earlier failed removal gets another chance at every stop
                return true;
            }
            return RemoveOwnAsync(gatewayFactory, removePort, removeDescription, removeLan, waitMs);
        }

        // ---- pending cleanups ---------------------------------------------------------------------------------
        // A removal that did not finish (error, timeout, router busy) is remembered and retried. The list lives in
        // AppDomain data as one BCL string (a successor copy of this integration reads it too).
        public const int MaxPendingCleanups = 8;
        public const int MaxCleanupRetries = 5;
        private const string PendingSlot = "GTAxSkate.Coop.UpnpPendingCleanup.v1";

        private sealed class PendingCleanup
        {
            public int Port; public string Lan = string.Empty; public int Retries; public string Description = string.Empty;
        }

        private static List<PendingCleanup> LoadPending()
        {
            var list = new List<PendingCleanup>();
            string raw = AppDomain.CurrentDomain.GetData(PendingSlot) as string;
            if (string.IsNullOrEmpty(raw)) return list;
            foreach (string line in raw.Split('\n'))
            {
                string[] f = line.Split(new[] { '|' }, 4);
                int port, retries;
                if (f.Length != 4 || !int.TryParse(f[0], out port) || !int.TryParse(f[2], out retries)) continue;
                list.Add(new PendingCleanup { Port = port, Lan = f[1], Retries = retries, Description = f[3] });
            }
            return list;
        }

        private static void SavePending(List<PendingCleanup> list)
        {
            var sb = new System.Text.StringBuilder();
            foreach (PendingCleanup p in list)
            {
                if (sb.Length > 0) sb.Append('\n');
                sb.Append(p.Port).Append('|').Append(p.Lan).Append('|').Append(p.Retries).Append('|').Append(p.Description);
            }
            AppDomain.CurrentDomain.SetData(PendingSlot, sb.Length == 0 ? null : sb.ToString());
        }

        private static void MarkPending(int port, string description, string lan)
        {
            if (string.IsNullOrEmpty(description)) return;
            lock (AppDomain.CurrentDomain)
            {
                List<PendingCleanup> l = LoadPending();
                PendingCleanup hit = l.Find(delegate(PendingCleanup p) { return p.Port == port && p.Description == description; });
                if (hit != null) { hit.Lan = lan ?? string.Empty; hit.Retries = 0; }
                else
                {
                    if (l.Count >= MaxPendingCleanups) l.RemoveAt(0);
                    l.Add(new PendingCleanup { Port = port, Lan = lan ?? string.Empty, Description = description });
                }
                SavePending(l);
            }
        }

        private static void ClearPending(int port, string description)
        {
            lock (AppDomain.CurrentDomain)
            {
                List<PendingCleanup> l = LoadPending();
                if (l.RemoveAll(delegate(PendingCleanup p) { return p.Port == port && p.Description == description; }) > 0) SavePending(l);
            }
        }

        internal static int PendingCleanupCount { get { lock (AppDomain.CurrentDomain) { return LoadPending().Count; } } }
        internal static void ClearPendingForTests() { lock (AppDomain.CurrentDomain) { SavePending(new List<PendingCleanup>()); } }

        // Retries every pending cleanup through gateway g (worker thread). No device: nothing to retry yet, nothing counted.
        internal static void RetryPendingWith(IUpnpGateway g)
        {
            if (g == null) return;
            List<PendingCleanup> snapshot;
            lock (AppDomain.CurrentDomain) { snapshot = LoadPending(); }
            if (snapshot.Count == 0) return;
            try { if (!g.IsAvailable()) return; }
            catch (Exception e) { CoopTrace.Write("client", "upnp pending cleanup: " + CoopTrace.Describe(e)); return; }
            foreach (PendingCleanup p in snapshot)
            {
                bool drop = false;
                lock (AppDomain.CurrentDomain)
                {
                    List<PendingCleanup> l = LoadPending();
                    PendingCleanup cur = l.Find(delegate(PendingCleanup x) { return x.Port == p.Port && x.Description == p.Description; });
                    if (cur == null) continue;
                    cur.Retries++;
                    if (cur.Retries > MaxCleanupRetries) { l.Remove(cur); drop = true; }
                    SavePending(l);
                }
                if (drop) { CoopTrace.Write("client", "upnp pending cleanup given up after " + MaxCleanupRetries + " retries"); continue; }
                try { RemoveOwn(g, p.Port, p.Description, string.IsNullOrEmpty(p.Lan) ? null : p.Lan); ClearPending(p.Port, p.Description); }
                catch (Exception e) { CoopTrace.Write("client", "upnp pending cleanup retry failed: " + CoopTrace.Describe(e)); }
            }
        }

        // Starts a retry worker when something is pending and no other router worker is alive (a new hosting's own worker
        // retries them itself). Never blocks; waitMs 0 = fire and forget.
        public static void RetryPendingAsync(Func<IUpnpGateway> factory, int waitMs)
        {
            if (PendingCleanupCount == 0) return;
            if (!TryBeginFlight(true)) return;
            var done = new ManualResetEvent(false);
            var t = new Thread(delegate()
            {
                try { RetryPendingWith(factory()); }
                catch (Exception e) { CoopTrace.Write("client", "upnp pending cleanup failed: " + CoopTrace.Describe(e)); }
                finally { EndFlight(); done.Set(); }
            });
            t.IsBackground = true; t.Name = "GTAxSkate-UPnP-retry";
            t.SetApartmentState(ApartmentState.MTA);
            try { t.Start(); }
            catch { EndFlight(); return; }
            if (waitMs > 0) done.WaitOne(waitMs);
        }

        public static bool RemoveOwnAsync(Func<IUpnpGateway> factory, int port, string description, int waitMs)
        {
            return RemoveOwnAsync(factory, port, description, null, waitMs);
        }

        // Stateless removal for crash recovery or another loaded copy of the integration: deletes the UDP mapping on
        // <port> only when its description is ours and (lan given) its internal client is this PC; nothing else. The
        // removal is marked pending first and cleared only when it finished, so an error or a timeout is retried later.
        public static bool RemoveOwnAsync(Func<IUpnpGateway> factory, int port, string description, string lan, int waitMs)
        {
            var done = new ManualResetEvent(false);
            MarkPending(port, description, lan);
            TryBeginFlight(false);   // counted before the thread exists: a Begin right after this call already sees it
            var t = new Thread(delegate()
            {
                try
                {
                    IUpnpGateway g = factory();
                    RemoveOwn(g, port, description, lan);
                    ClearPending(port, description);
                    RetryPendingWith(g);   // other cleanups that failed earlier
                }
                catch (Exception e) { CoopTrace.Write("client", "upnp remove failed (kept pending): " + CoopTrace.Describe(e)); }
                finally { EndFlight(); done.Set(); }
            });
            t.IsBackground = true; t.Name = "GTAxSkate-UPnP-remove";
            t.SetApartmentState(ApartmentState.MTA);
            try { t.Start(); }
            catch { EndFlight(); return false; }
            return waitMs > 0 && done.WaitOne(waitMs);
        }

        internal static int RemoveOwn(IUpnpGateway g, int port, string description)
        {
            return RemoveOwn(g, port, description, null);
        }

        // Reads the entry first: deleted only when the description is ours and, when lan is given, the internal client is
        // this PC (a replacement mapping of another device or app is never touched). 0 = nothing deleted (gone or not
        // ours: nothing more to retry). Throws when the router cannot be asked or the delete failed: the caller keeps it pending.
        internal static int RemoveOwn(IUpnpGateway g, int port, string description, string lan)
        {
            if (g == null || !g.IsAvailable()) throw new InvalidOperationException("no UPnP device");
            UpnpMappingInfo m = g.Get(port, "UDP");
            if (m == null) return 0;
            if (!string.Equals(m.Description, description, StringComparison.Ordinal)) return 0;
            if (lan != null && !string.Equals(m.InternalClient, lan, StringComparison.Ordinal)) return 0;
            g.Remove(port, "UDP");
            return 1;
        }

        private static OnlineHostStatus Fail(int port, string trace)
        {
            return new OnlineHostStatus(OnlineHostState.Failed, null,
                "Your router did not open the port (or your line has no public IPv4). Forward UDP " + port +
                " to this PC, or use Tailscale. LAN/VPN friends can still join.", trace);
        }

        private void Work(int myGen, int udpPort, string desc, string lanIpv4)
        {
            try { WorkCore(myGen, udpPort, desc, lanIpv4); }
            finally { EndFlight(); }   // only after the late cleanup: Begin stays refused until then
        }

        private void WorkCore(int myGen, int udpPort, string desc, string lanIpv4)
        {
            OnlineHostStatus result;
            bool addAttempted = false;   // the router may hold our mapping (also when Add threw halfway)
            IUpnpGateway g = null;
            try
            {
                if (string.IsNullOrEmpty(lanIpv4)) throw new InvalidOperationException("no LAN address");
                g = gatewayFactory();
                if (g == null || !g.IsAvailable())
                {
                    result = Fail(udpPort, "upnp no device");
                }
                else
                {
                    RetryPendingWith(g);   // removals that failed earlier (own description only)
                    bool foreign = false;
                    foreach (UpnpMappingInfo m in g.List())
                    {
                        if (m == null || m.ExternalPort != udpPort || !string.Equals(m.Protocol, "UDP", StringComparison.OrdinalIgnoreCase)) continue;
                        if (string.Equals(m.Description, desc, StringComparison.Ordinal)) RemoveOwn(g, udpPort, desc, null); // stale own
                        else foreign = true;
                    }
                    if (foreign)
                    {
                        result = new OnlineHostStatus(OnlineHostState.Failed, null,
                            "UDP port " + udpPort + " is already forwarded to another device or app on your router. Free it, or forward it to this PC yourself.",
                            "upnp port held by foreign mapping");
                    }
                    else
                    {
                        addAttempted = true;
                        MarkPending(udpPort, desc, lanIpv4);   // until the mapping is open and owned by the service, or removed
                        UpnpMappingInfo mapped = g.Add(udpPort, "UDP", lanIpv4, desc);
                        lock (gate) { if (myGen == generation) mappingCreated = true; }
                        string ext = mapped == null ? null : mapped.ExternalIp;
                        if (!LobbyCode.IsGlobalIPv4(ext))
                        {
                            // Empty / unspecified = DS-Lite (the router has only an IPv6 uplink address); private = double NAT / CGNAT.
                            bool noIPv4 = string.IsNullOrEmpty(ext) || ext == "0.0.0.0";
                            result = new OnlineHostStatus(OnlineHostState.Failed, null,
                                noIPv4 ? UpnpIgdGateway.NoPublicIPv4 :
                                "Your router reports a non-public address (double NAT / carrier-grade NAT), so friends cannot reach you directly. " +
                                "Use Tailscale instead. LAN/VPN friends can still join.",
                                noIPv4 ? "upnp external address empty (DS-Lite)" : "upnp external address not global");
                        }
                        else
                        {
                            result = new OnlineHostStatus(OnlineHostState.Open, ext,
                                "Port open. Your own PC cannot test the public route; ask your friend to join.", "upnp open (external address is global)");
                        }
                    }
                }
            }
            catch (UpnpUserException e)
            {
                result = e.UserMessage != null
                    ? new OnlineHostStatus(OnlineHostState.Failed, null, e.UserMessage, e.TraceNote)
                    : Fail(udpPort, e.TraceNote);
            }
            catch (Exception e)
            {
                result = Fail(udpPort, "upnp error " + CoopTrace.Describe(e));
            }

            // Anything but an open port takes back what the router may have accepted (own description only), BEFORE the
            // failure is shown, so a failed attempt never leaves the port open.
            if (addAttempted && result.State != OnlineHostState.Open)
            {
                TryRemoveOwn(g, udpPort, desc, lanIpv4, "upnp cleanup after a failed attempt failed (kept pending): ");
                addAttempted = false;
            }

            bool abandoned;
            lock (gate)
            {
                abandoned = myGen != generation;
                if (!abandoned)
                {
                    status = result;
                    if (result.State != OnlineHostState.Open) mappingCreated = false;
                }
            }
            if (!abandoned) CoopTrace.Write("client", result.TraceNote);
            // Open and still ours: the service owns it now (Release removes it). Otherwise it is removed or stays pending.
            if (!abandoned && addAttempted) ClearPending(udpPort, desc);
            // Timed out or released while we were working: take back what we added (own description only).
            if (abandoned && addAttempted) TryRemoveOwn(g, udpPort, desc, lanIpv4, "upnp late cleanup failed (kept pending): ");
        }

        // Own description and this PC's address only. Success clears the pending mark; a failure leaves it for the retry.
        private void TryRemoveOwn(IUpnpGateway g, int udpPort, string desc, string lan, string traceNote)
        {
            try
            {
                RemoveOwn(g ?? gatewayFactory(), udpPort, desc, lan);
                ClearPending(udpPort, desc);
            }
            catch (Exception e) { CoopTrace.Write("client", traceNote + CoopTrace.Describe(e)); }
        }
    }
}
