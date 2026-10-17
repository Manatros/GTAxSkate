using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using RageCoop.Client.Scripting;
using RageCoop.Core.Scripting;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class ClientAdapter : ClientScript
    {
        private readonly object gate = new object();
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private bool active, responseLogged;
        private int lifetime, attempts;
        private long nextHello;
        private byte[] challenge, session;
        private object[] pendingChallenge, pendingResult;
        // This remains false until transport origin and skeleton registration
        // have a separately reviewed implementation. Never enabled by a packet.
        public bool RemotePoseEnabled { get { return false; } }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        private static extern IntPtr GetModuleHandleW(string name);

        public override void OnStart()
        {
            int epoch;
            lock (gate)
            {
                active = true; epoch = ++lifetime;
                challenge = Guid.NewGuid().ToByteArray(); session = null;
                pendingChallenge = pendingResult = null;
                attempts = 0; nextHello = 0; responseLogged = false;
            }
            API.RegisterCustomEventHandler(CustomEvents.Hash(DiagnosticProtocol.Challenge), e =>
            {
                // Copy at the callback boundary; consume at most one reply on
                // the script thread, never queue arbitrary native work.
                lock (gate)
                {
                    if (!active || epoch != lifetime || !DiagnosticProtocol.Header(e.Args, 5) ||
                        !DiagnosticProtocol.Nonce(e.Args[1]) || !DiagnosticProtocol.Nonce(e.Args[2]) ||
                        !(e.Args[3] is int) || (int)e.Args[3] == 0 || !(e.Args[4] is bool) ||
                        (bool)e.Args[4] || !DiagnosticProtocol.Equal(challenge, (byte[])e.Args[1])) return;
                    if (session != null || pendingChallenge != null) return;
                    pendingChallenge = new object[] { 1, ((byte[])e.Args[1]).Clone(),
                        ((byte[])e.Args[2]).Clone(), e.Args[3], false };
                }
            });
            API.RegisterCustomEventHandler(CustomEvents.Hash(DiagnosticProtocol.Result), e =>
            {
                lock (gate)
                {
                    if (!active || epoch != lifetime || responseLogged || !DiagnosticProtocol.Header(e.Args, 4) ||
                        !DiagnosticProtocol.Nonce(e.Args[1]) || !DiagnosticProtocol.Nonce(e.Args[2]) ||
                        !(e.Args[3] is bool) || !(bool)e.Args[3] ||
                        !DiagnosticProtocol.Equal(challenge, (byte[])e.Args[1]) ||
                        !DiagnosticProtocol.Equal(session, (byte[])e.Args[2])) return;
                    pendingResult = new object[] { 1, ((byte[])e.Args[1]).Clone(), ((byte[])e.Args[2]).Clone(), true };
                }
            });
            API.QueueAction(new Action(() =>
            {
                lock (gate)
                {
                    if (!active || epoch != lifetime) return;
                    API.Events.OnTick -= Tick;
                    API.Events.OnTick += Tick;
                    Logger.Info("GTAxSkate Coop diagnostic adapter started; remote pose disabled; no trusted skeleton/origin capability.");
                }
            }));
        }

        private void Tick()
        {
            lock (gate)
            {
                if (!active || !API.IsOnServer) return;
                if (pendingChallenge != null)
                {
                    var response = pendingChallenge; pendingChallenge = null;
                    if ((int)response[3] != API.LocalPlayerID) return;
                    session = (byte[])response[2];
                    API.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Ack), 1, challenge, session);
                    Logger.Info("GTAxSkate Coop challenge received; diagnostic ACK sent. Server origin unavailable in public API; pose stays disabled.");
                }
                if (pendingResult != null)
                {
                    pendingResult = null; responseLogged = true;
                    Logger.Info("GTAxSkate Coop roundtrip response received (diagnostic only); remote pose disabled.");
                }
                if (session != null || attempts >= 3 || clock.ElapsedMilliseconds < nextHello) return;
                var version = API.CurrentVersion;
                if (version == null || version.Major != 1 || version.Minor != 5 || version.Build != 12)
                {
                    attempts = 3;
                    Logger.Warning("GTAxSkate Coop unsupported RAGECOOP API; handshake disabled."); return;
                }
                attempts++; nextHello = clock.ElapsedMilliseconds + 5000;
                API.SendCustomEvent(CustomEvents.Hash(DiagnosticProtocol.Hello), 1, challenge,
                    GetModuleHandleW("SkateVLegacy.asi") != IntPtr.Zero,
                    GetModuleHandleW("SkateVRuntime.dll") != IntPtr.Zero, false);
            }
        }

        public override void OnStop()
        {
            lock (gate)
            {
                active = false; lifetime++;
                challenge = session = null; pendingChallenge = pendingResult = null;
                API.Events.OnTick -= Tick;
            }
            // No workers, memory hooks, capture handles or game entities exist.
        }
    }
}
