using System;
using System.Collections.Generic;
using System.Diagnostics;
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
        private readonly object gate = new object();
        private readonly Dictionary<int, Pending> peers = new Dictionary<int, Pending>();
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private bool active;
        private int lifetime;

        public override void OnStart()
        {
            int epoch;
            lock (gate) { active = true; epoch = ++lifetime; peers.Clear(); }
            API.RegisterCustomEventHandler(DiagnosticProtocol.Hello, e => Hello(e, epoch));
            API.RegisterCustomEventHandler(DiagnosticProtocol.Ack, e => Ack(e, epoch));
            API.Events.OnPlayerDisconnected += Disconnected;
            Logger.Info("GTAxSkate Coop server diagnostic adapter started; no pose relay/native authority enabled.");
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
        private void Disconnected(object sender, Client client)
        {
            lock (gate)
            {
                var ids = new List<int>();
                foreach (var peer in peers) if (Object.ReferenceEquals(peer.Value.Owner, client)) ids.Add(peer.Key);
                foreach (int id in ids) peers.Remove(id);
            }
        }
        public override void OnStop()
        {
            lock (gate) { active = false; lifetime++; peers.Clear(); }
            API.Events.OnPlayerDisconnected -= Disconnected;
        }
    }
}
