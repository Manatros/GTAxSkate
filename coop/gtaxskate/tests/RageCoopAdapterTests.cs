using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using GTAxSkate.Coop;
using GTAxSkate.Coop.RageCoopAdapter;

// Compile the ACTUAL adapters against a deterministic API fixture for event
// integration tests; build.ps1 separately compiles them against genuine DLLs.
public sealed class TestLogger
{
    public readonly List<string> Lines = new List<string>();
    public void Info(string value) { Lines.Add(value); }
    public void Warning(string value) { Lines.Add(value); }
}
public sealed class Message
{
    public int Hash;
    public object[] Args;
    public Message(int hash, object[] args)
    {
        Hash = hash; Args = (object[])args.Clone();
        for (int i = 0; i < Args.Length; i++) if (Args[i] is byte[]) Args[i] = ((byte[])Args[i]).Clone();
    }
}
namespace RageCoop.Core.Scripting
{
    public static class CustomEvents
    {
        public static int Hash(string value) { unchecked { int result=17; foreach(char c in value) result=result*31+c; return result; } }
    }
}
namespace RageCoop.Server
{
    public sealed class Player { public int ID; }
    public sealed class Client
    {
        public bool IsReady = true;
        public Player Player;
        public readonly Queue<Message> Sent = new Queue<Message>();
        public void SendCustomEvent(int hash, params object[] args) { Sent.Enqueue(new Message(hash,args)); }
    }
}
namespace RageCoop.Server.Scripting
{
    public sealed class CustomEventReceivedArgs : EventArgs { public RageCoop.Server.Client Client; public object[] Args; }
    public sealed class HandshakeEventArgs : EventArgs
    {
        public IPEndPoint EndPoint;
        public string PasswordHash;
        public bool Denied;
        public void Deny(string reason) { Denied = true; }
    }
    public sealed class ServerEvents
    {
        public event EventHandler<RageCoop.Server.Client> OnPlayerDisconnected;
        public event EventHandler<HandshakeEventArgs> OnPlayerHandshake;
        public void Disconnect(RageCoop.Server.Client client) { if(OnPlayerDisconnected!=null) OnPlayerDisconnected(this,client); }
        public bool Admit(string ip, string password)
        {
            string hash;
            using (var sha = System.Security.Cryptography.SHA256.Create())
                hash = BitConverter.ToString(sha.ComputeHash(System.Text.Encoding.UTF8.GetBytes(password))).Replace("-", "");
            var args = new HandshakeEventArgs { EndPoint = new IPEndPoint(IPAddress.Parse(ip), 49152), PasswordHash = hash };
            if (OnPlayerHandshake != null) OnPlayerHandshake(this, args);
            return !args.Denied;
        }
    }
    public abstract class ServerScript
    {
        public API API;
        public readonly TestLogger Logger = new TestLogger();
        public abstract void OnStart(); public abstract void OnStop();
    }
    public sealed class API
    {
        public readonly ServerEvents Events = new ServerEvents();
        public readonly Dictionary<int,RageCoop.Server.Client> Clients = new Dictionary<int,RageCoop.Server.Client>();
        private readonly Dictionary<int,List<Action<CustomEventReceivedArgs>>> handlers = new Dictionary<int,List<Action<CustomEventReceivedArgs>>>();
        public Dictionary<int,RageCoop.Server.Client> GetAllClients() { return new Dictionary<int,RageCoop.Server.Client>(Clients); }
        public void RegisterCustomEventHandler(string name, Action<CustomEventReceivedArgs> handler)
        {
            int hash=RageCoop.Core.Scripting.CustomEvents.Hash(name);
            if(!handlers.ContainsKey(hash))handlers[hash]=new List<Action<CustomEventReceivedArgs>>();
            handlers[hash].Add(handler);
        }
        public void Deliver(RageCoop.Server.Client source, Message message)
        {
            foreach(var handler in handlers[message.Hash])handler(new CustomEventReceivedArgs{Client=source,Args=message.Args});
        }
    }
}
namespace RageCoop.Client.Scripting
{
    public sealed class CustomEventReceivedArgs : EventArgs { public object[] Args; }
    public abstract class ClientScript
    {
        private readonly TestLogger backingLogger = new TestLogger();
        public bool SimulateNullResourceLogger;
        public TestLogger Logger
        {
            get
            {
                if (SimulateNullResourceLogger) throw new NullReferenceException("Simulated CurrentResource == null");
                return backingLogger;
            }
        }
        public abstract void OnStart(); public abstract void OnStop();
    }
    public static class API
    {
        public static bool IsOnServer = true;
        public static IPEndPoint ServerEndPoint = new IPEndPoint(IPAddress.Parse("100.88.12.34"), 4499);
        public static int LocalPlayerID = -11;
        public static Version CurrentVersion = new Version(1,5,12,15);
        public static readonly Queue<Message> Sent = new Queue<Message>();
        public static readonly Queue<Action> Actions = new Queue<Action>();
        private static readonly Dictionary<int,List<Action<CustomEventReceivedArgs>>> handlers = new Dictionary<int,List<Action<CustomEventReceivedArgs>>>();
        public static class Events
        {
            public static event Action OnTick;
            public static void Tick() { if(OnTick!=null)OnTick(); }
            public static void Clear() { OnTick=null; }
        }
        public static void QueueAction(Action action) { Actions.Enqueue(action); }
        public static void SendCustomEvent(int hash,params object[] args) { Sent.Enqueue(new Message(hash,args)); }
        public static void RegisterCustomEventHandler(int hash,Action<CustomEventReceivedArgs> handler)
        {
            if(!handlers.ContainsKey(hash))handlers[hash]=new List<Action<CustomEventReceivedArgs>>();
            handlers[hash].Add(handler);
        }
        public static void Deliver(Message message)
        { foreach(var handler in handlers[message.Hash])handler(new CustomEventReceivedArgs{Args=message.Args}); }
        public static void Reset()
        { handlers.Clear();Sent.Clear();Actions.Clear();Events.Clear();IsOnServer=true;LocalPlayerID=-11;CurrentVersion=new Version(1,5,12,15); }
    }
}
internal static class RageCoopAdapterTests
{
    private static int checks;
    private static void Check(bool value,string text) { checks++;if(!value)throw new Exception(text); }
    // A well-formed invite for the join flows (the password is a generated-style Crockford string).
    private const string TestPassword = "7H2KQ9XD4MZP0VBN3TRE";
    private const string TestLanInvite = "GTAxSkate coop invite lan:192.168.1.40:4499 " + TestPassword;
    public static int Main()
    {
        try { Run();Console.WriteLine("PASS: "+checks+" actual adapter event checks; fixture transport only, no live server/gameplay.");return 0; }
        catch(Exception ex){Console.Error.WriteLine(ex);return 1;}
    }
    private static void EnableFixture()
    {
        CoopRuntimeControl.ResetForTests();
        string error;
        Check(CoopRuntimeControl.SetEnabled(true, out error) && CoopRuntimeControl.Enabled, "fixture opts in explicitly");
    }
    private static void Run()
    {
        RunDefaultOffTests();
        RunOptInBootstrapTests();
        RunDisableTests();
        RunPeerReviewRegressionTests();
        RunDisplacedCopyTests();
        RunRuntimeControlIdentityTests();
        RunRageCoopUiLockTests();
        EnableFixture();
        var api=new RageCoop.Server.Scripting.API();
        var server=new ServerAdapter{API=api};server.OnStart();
        var owner=new RageCoop.Server.Client{Player=new RageCoop.Server.Player{ID=-11}};
        var other=new RageCoop.Server.Client{Player=new RageCoop.Server.Player{ID=22}};
        api.Clients[-11]=owner;api.Clients[22]=other;
        RageCoop.Client.Scripting.API.Reset();
        var client=new ClientAdapter{DiagnosticFixtureOnly=true};client.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count==1,"client sends bounded first hello");
        var hello=RageCoop.Client.Scripting.API.Sent.Dequeue();
        Check(hello.Args.Length==5 && hello.Args[4] is bool && !(bool)hello.Args[4],"no skeleton capability advertised");
        var impostor=new RageCoop.Server.Client{Player=new RageCoop.Server.Player{ID=-11}};
        api.Deliver(impostor,hello);
        Check(impostor.Sent.Count==0,"same claimed source without authenticated object rejected");
        owner.IsReady=false;api.Deliver(owner,hello);Check(owner.Sent.Count==0,"not-ready source rejected");owner.IsReady=true;
        var malformed=new Message(hello.Hash,new object[]{2,Guid.NewGuid().ToByteArray(),true,true,false});
        api.Deliver(owner,malformed);Check(owner.Sent.Count==0,"unknown protocol denied");
        malformed=new Message(hello.Hash,new object[]{1,Guid.NewGuid().ToByteArray(),true,true,true});
        api.Deliver(owner,malformed);Check(owner.Sent.Count==0,"unverified skeleton claim denied");
        api.Deliver(owner,hello);Check(owner.Sent.Count==1,"valid negative source gets diagnostic challenge");
        var challenge=owner.Sent.Dequeue();Check((int)challenge.Args[3]==-11,"source comes from authenticated connection");
        api.Deliver(owner,hello);Check(owner.Sent.Count==0,"immediate repeated hello rate limited");
        var wrong=new Message(challenge.Hash,challenge.Args);wrong.Args[1]=Guid.NewGuid().ToByteArray();
        RageCoop.Client.Scripting.API.Deliver(wrong);
        RageCoop.Client.Scripting.API.Events.Tick();Check(RageCoop.Client.Scripting.API.Sent.Count==0,"wrong client challenge ignored");
        RageCoop.Client.Scripting.API.Deliver(challenge);
        // The callback copies mutable byte arrays before its later script tick.
        ((byte[])challenge.Args[2])[0]^=1;
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count==1,"client ACK dispatched on main tick");
        var ack=RageCoop.Client.Scripting.API.Sent.Dequeue();
        api.Deliver(other,ack);Check(other.Sent.Count==0,"cross-source ACK cannot complete handshake");
        var badAck=new Message(ack.Hash,ack.Args);badAck.Args[2]=Guid.NewGuid().ToByteArray();
        api.Deliver(owner,badAck);Check(owner.Sent.Count==0,"wrong session ACK rejected");
        api.Deliver(owner,ack);Check(owner.Sent.Count==1,"authentic sender ACK completes diagnostic roundtrip");
        var result=owner.Sent.Dequeue();api.Deliver(owner,ack);Check(owner.Sent.Count==0,"ACK replay rejected");
        RageCoop.Client.Scripting.API.Deliver(result);RageCoop.Client.Scripting.API.Events.Tick();
        Check(!client.RemotePoseEnabled,"network response cannot enable native pose writes");
        Check(client.Logger.Lines.Exists(x=>x.Contains("roundtrip response")),"client diagnostic logged");
        Check(server.Logger.Lines.Exists(x=>x.Contains("handshake accepted source=-11")),"server authenticated acceptance logged");
        api.Events.Disconnect(owner);api.Deliver(owner,ack);Check(owner.Sent.Count==0,"disconnect clears pending source");
        server.OnStop();api.Deliver(owner,hello);Check(owner.Sent.Count==0,"stopped server ignores retained callbacks");
        server.OnStart();api.Deliver(owner,hello);Check(owner.Sent.Count==1,"server restart gets fresh challenge despite retained handlers");
        owner.Sent.Dequeue();api.Deliver(owner,ack);Check(owner.Sent.Count==0,"old ACK rejected after new server lifetime");
        client.OnStop();RageCoop.Client.Scripting.API.Deliver(result);RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count==0,"stopped client does not send events");
        RageCoop.Client.Scripting.API.Reset();
        client=new ClientAdapter();client.OnStart();client.OnStop();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count==0,"queued old startup cannot resubscribe after stop");
        RageCoop.Client.Scripting.API.Reset();RageCoop.Client.Scripting.API.CurrentVersion=new Version(1,6,0,0);
        client=new ClientAdapter();client.OnStart();RageCoop.Client.Scripting.API.Actions.Dequeue()();RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count==0,"unreviewed API version does not handshake");client.OnStop();

        // Private Friend-Coop wire integration tests (RAGECOOP ServerAdapter <-> ClientAdapter)
        RunTransportGuardTests();
        RunBootstrapSafetyTests();
        RunFriendSessionWireTests();
        RunInternetLobbyServerTests();
        RunInternetLobbyMenuTests();
        RunStopLocalServerTests();
        RunStreamerSafeLeakTests();
        RunRound2AdapterTests();
        RunRound3AdapterTests();
    }

    private sealed class LoadingMockWorldBridge : IGtaWorldBridge, IDisposable
    {
        public bool Loading = true;
        public int OverlayDraws;
        public int DisposedCount;
        public int StopServerCalls, DisconnectCalls, EnsureHostCalls;
        public readonly List<string> Notes = new List<string>();
        // Everything this bridge was asked to show: every notice and every string of every overlay frame (leak scan).
        public readonly List<string> AllShown = new List<string>();
        public Action EnsureHook;
        public CoopOverlayView LastOverlay;
        public void Dispose() { DisposedCount++; }
        public bool IsGameLoading() { return Loading; }
        public GTAxSkate.Coop.HostWorldSyncState CaptureHostWorldState(int epoch) { return null; }
        public LocalRagdollSample CaptureLocalRagdoll() { return null; }
        public void ApplyPeerWorldState(GTAxSkate.Coop.HostWorldSyncState state) { }
        public void ApplyPeerShopState(GTAxSkate.Coop.HostShopSyncState state) { }
        public void ApplyRemotePlayerRagdoll(int sourceSlot, string peerUsername, GTAxSkate.Coop.RagdollSyncState state) { }
        public void ExitPeerCutsceneStandbyIfNeeded(GTAxSkate.Coop.HostWorldSyncState lastWorldState, bool allowGroundRegroup) { }
        public void RestoreSoloDefaults() { }
        public int ConnectCalls; public string LastConnectKey; public bool ConnectResult = true;
        public string LastConnectAddress; public int LastConnectPort;
        public int ForceNameCalls; public bool NameForcedBeforeEveryConnect = true;
        public bool ConnectTransport(string endpointAddress, int port, string friendKey)
        {
            ConnectCalls++;
            if (ForceNameCalls < ConnectCalls) NameForcedBeforeEveryConnect = false;
            LastConnectKey = friendKey; LastConnectAddress = endpointAddress; LastConnectPort = port; return ConnectResult;
        }
        public void DisconnectTransport() { DisconnectCalls++; }
        public int FlushCalls;
        public void FlushTransport() { FlushCalls++; }
        public bool TransportConnecting = true;
        public bool IsTransportConnecting() { return TransportConnecting; }
        public bool EnsureLocalHostServerRunning(int port, string friendKey, out string statusMessage)
        {
            EnsureHostCalls++;
            if (EnsureHook != null) EnsureHook();
            statusMessage = EnsureMessage;
            return EnsureResult;
        }
        public bool EnsureResult = true; public string EnsureMessage = string.Empty;
        public bool LocalServerRunning;
        public bool IsLocalServerRunning() { return LocalServerRunning; }
        public void StopLocalHostServerIfStarted() { StopServerCalls++; LocalServerRunning = false; }
        public int BeginOnlineCalls, BeginOnlinePort;
        public GTAxSkate.Coop.OnlineHostStatus OnlineStatus = GTAxSkate.Coop.OnlineHostStatus.Idle;
        public void BeginOnlineHosting(int udpPort) { BeginOnlineCalls++; BeginOnlinePort = udpPort; }
        public GTAxSkate.Coop.OnlineHostStatus PollOnlineHosting() { return OnlineStatus; }

        // Clipboard fixture: a plain string plus tickets that stay Pending for ClipboardPendingPolls polls.
        public string ClipboardText = string.Empty;
        public int ClipboardCalls;            // CopyHidden operations begun
        public string LastClipboard;          // the last text copied
        public bool ClipboardCopyFails, ClipboardClearFails;
        public int ClipboardPendingPolls;
        public int ClipboardReads, ClipboardClears, ClearAttempts;
        private int nextTicket;
        private readonly Dictionary<int, KeyValuePair<int, ClipboardOpResult>> tickets = new Dictionary<int, KeyValuePair<int, ClipboardOpResult>>();
        public int BeginClipboard(ClipboardOpKind kind, string text, bool track)
        {
            ClipboardOpResult r;
            if (kind == ClipboardOpKind.CopyHidden)
            {
                ClipboardCalls++;
                if (ClipboardCopyFails) r = ClipboardOpResult.FailedResult;
                else { ClipboardText = text; LastClipboard = text; r = new ClipboardOpResult(ClipboardOpState.Done, null, true); }
            }
            else if (kind == ClipboardOpKind.Read)
            {
                ClipboardReads++;
                r = new ClipboardOpResult(ClipboardOpState.Done, ClipboardText, false);
            }
            else
            {
                ClearAttempts++;
                if (ClipboardClearFails) { int failTicket = ++nextTicket; if (track) tickets[failTicket] = new KeyValuePair<int, ClipboardOpResult>(ClipboardPendingPolls, ClipboardOpResult.FailedResult); return failTicket; }
                bool same = !string.IsNullOrEmpty(ClipboardText) && !string.IsNullOrEmpty(text) && ClipboardText == text;   // exact, like the real worker
                if (same) { ClipboardText = string.Empty; ClipboardClears++; }
                r = new ClipboardOpResult(ClipboardOpState.Done, null, same);
            }
            int ticket = ++nextTicket;
            if (track) tickets[ticket] = new KeyValuePair<int, ClipboardOpResult>(ClipboardPendingPolls, r);
            return ticket;
        }
        public ClipboardOpResult PollClipboard(int ticket)
        {
            KeyValuePair<int, ClipboardOpResult> t;
            if (!tickets.TryGetValue(ticket, out t)) return ClipboardOpResult.FailedResult;
            if (t.Key > 0) { tickets[ticket] = new KeyValuePair<int, ClipboardOpResult>(t.Key - 1, t.Value); return ClipboardOpResult.PendingResult; }
            tickets.Remove(ticket);
            return t.Value;
        }
        public string Lan = "192.168.1.20", Vpn = null;
        public void GetLocalAddresses(out string lanIPv4, out string vpnIPv4) { lanIPv4 = Lan; vpnIPv4 = Vpn; }
        public void ForceAnonymousName() { ForceNameCalls++; }
        public int UiLockCalls; public bool UiLocked;
        public bool UiLockFails; public string UiLockFailMessage = "RageCoop menu member not reachable.";
        public bool SetRageCoopUiLocked(bool locked, out string error) { UiLockCalls++; UiLocked = locked; error = UiLockFails ? UiLockFailMessage : string.Empty; return !UiLockFails; }
        public void Notify(string message) { Notes.Add(message); AllShown.Add(message); }
        public void RenderOverlay(CoopOverlayView overlay)
        {
            OverlayDraws++; LastOverlay = overlay;
            AllShown.Add(overlay.StatusBanner); AllShown.Add(overlay.SubtitleLine);
            foreach (string w in overlay.Warnings) AllShown.Add(w);
            foreach (CoopMenuItem item in overlay.Items) AllShown.Add(item.Label);
        }
    }


    private sealed class MockHost : ICoopBootstrapHost
    {
        public bool WorldReady = true, CoopReady = true, Connected;
        public long Now = 10000;
        public int Draws, CreateCount, Disconnects;
        public CoopPanelView LastPanel;
        public LoadingMockWorldBridge LastBridge;
        public readonly List<string> Notifications = new List<string>();
        public readonly List<string> Warnings = new List<string>();
        public bool IsWorldReady() { return WorldReady; }
        public bool IsWorldAndRageCoopReady() { return WorldReady && CoopReady; }
        public long NowMs() { return Now; }
        public ClientAdapter CreateAdapter()
        {
            CreateCount++;
            var adapter = new ClientAdapter();
            LastBridge = new LoadingMockWorldBridge { Loading = false };
            adapter.SetWorldBridge(LastBridge);
            return adapter;
        }
        public void DrawPanel(CoopPanelView view) { Draws++; LastPanel = view; }
        public void Notify(string message) { Notifications.Add(message); }
        public void LogWarning(string message) { Warnings.Add(message); }
        public bool IsTransportConnected() { return Connected; }
        public void DisconnectTransport() { Disconnects++; Connected = false; }
    }

    private static void Ticks(CoopBootstrapLogic logic, MockHost host, int count)
    {
        for (int i = 0; i < count; i++) { host.Now += 600; logic.Tick(); }
    }

    private static string NewTempDir()
    {
        string root = Environment.GetEnvironmentVariable("GTAXSKATE_COOP_TEST_TMP");
        if (string.IsNullOrEmpty(root)) root = Path.Combine(Path.GetTempPath(), "gtaxskate-coop-adapter-tests");
        string dir = Path.Combine(root, Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        return dir;
    }

    // Default OFF: nothing runs, nothing draws, nothing is claimed.
    private static void RunDefaultOffTests()
    {
        CoopRuntimeControl.ResetForTests();
        RageCoop.Client.Scripting.API.Reset();
        Check(!CoopRuntimeControl.Enabled && !CoopRuntimeControl.IsInitialized, "fresh state is off and uninitialised");

        // Unsolicited resource-loader OnStart/OnStop while off.
        var unsolicited = new ClientAdapter();
        unsolicited.OnStart();
        Check(!unsolicited.IsActive && ClientAdapter.ActiveOwner == null, "unsolicited OnStart claims no owner and stays inactive");
        Check(RageCoop.Client.Scripting.API.Actions.Count == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0,
            "unsolicited OnStart registers no ticks and sends nothing");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count == 0, "no tick handler exists after a refused OnStart");
        unsolicited.OnStop();
        Check(ClientAdapter.ActiveOwner == null && RageCoop.Client.Scripting.API.Actions.Count == 0, "inactive OnStop is harmless");
        Check(!unsolicited.HandleMenuKeyCode(0x7A) && !unsolicited.MenuState.IsOpen, "refused adapter ignores F11");

        // Bootstrap logic ticking on a ready world with RageCoop ready.
        var host = new MockHost();
        var logic = new CoopBootstrapLogic(host);
        Ticks(logic, host, 30);
        Check(host.CreateCount == 0 && ClientAdapter.ActiveOwner == null && !logic.AdapterStarted,
            "bootstrap never builds an adapter while off");
        Check(host.Draws == 0 && host.Notifications.Count == 0 && host.Warnings.Count == 0,
            "off + panel closed: zero draws and zero notices");
        Check(RageCoop.Client.Scripting.API.Actions.Count == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0,
            "off: no RageCoop actions queued, no custom events");

        // Gameplay keys are not consumed while the panel is closed.
        logic.EnqueueKey(CoopBootstrapLogic.VkUp);
        logic.EnqueueKey(CoopBootstrapLogic.VkDown);
        logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
        logic.EnqueueKey(CoopBootstrapLogic.VkEscape);
        Ticks(logic, host, 3);
        Check(!logic.PanelOpen && host.Draws == 0 && host.CreateCount == 0, "closed panel ignores Up/Down/Enter/Esc");

        // Raw F9/external transport observed while off is dropped silently; no adapter, no events.
        host.Connected = true;
        Ticks(logic, host, 2);
        Check(host.Disconnects == 1 && host.CreateCount == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0 && host.Notifications.Count == 0,
            "raw transport while off is silently disconnected without starting the adapter");

        // Not-ready world never draws or starts either.
        host.WorldReady = false;
        logic.EnqueueKey(CoopBootstrapLogic.VkF11);
        Ticks(logic, host, 3);
        Check(host.Draws == 0 && !logic.PanelOpen, "loading screen: no panel, no draw");
    }

    // Opt-in through the dormant F11 panel, persisted in a prefs file.
    private static void RunOptInBootstrapTests()
    {
        string dir = NewTempDir();
        string prefs = Path.Combine(dir, "FriendsCoopPrefs.json");
        try
        {
            CoopRuntimeControl.ResetForTests();
            RageCoop.Client.Scripting.API.Reset();
            File.WriteAllText(prefs, "{\"MultiplayerEnabled\": \"true\"}");
            Check(!CoopRuntimeControl.Initialize(prefs) && !CoopRuntimeControl.Enabled, "quoted/invalid persisted value means off");
            File.WriteAllText(prefs, "{\"MultiplayerEnabled\": true}");
            Check(!CoopRuntimeControl.Initialize(prefs), "prefs are read once per script domain, not on later calls");

            var host = new MockHost();
            var logic = new CoopBootstrapLogic(host);
            Ticks(logic, host, 3);
            logic.EnqueueKey(CoopBootstrapLogic.VkF11);
            Ticks(logic, host, 1);
            Check(logic.PanelOpen && host.Draws == 1 && host.LastPanel.Title == "Multiplayer: Off" &&
                host.LastPanel.Items[0] == "Enable Multiplayer" && host.CreateCount == 0, "F11 opens the Off panel without any adapter");
            host.CoopReady = false;
            Ticks(logic, host, 1);
            Check(host.Draws == 2 && logic.PanelOpen, "panel works even if RageCoop never initialises");
            host.CoopReady = true;

            logic.EnqueueKey(CoopBootstrapLogic.VkEscape);
            Ticks(logic, host, 1);
            Check(!logic.PanelOpen && !CoopRuntimeControl.Enabled && host.Draws == 2, "Esc closes without enabling");
            logic.EnqueueKey(CoopBootstrapLogic.VkF11);
            logic.EnqueueKey(CoopBootstrapLogic.VkDown);
            logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
            Ticks(logic, host, 1);
            Check(!logic.PanelOpen && !CoopRuntimeControl.Enabled && host.CreateCount == 0, "Close item leaves multiplayer off");

            int genOff = CoopRuntimeControl.Generation;
            logic.EnqueueKey(CoopBootstrapLogic.VkF11);
            logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
            Ticks(logic, host, 1);
            Check(CoopRuntimeControl.Enabled && CoopRuntimeControl.Generation != genOff, "Enable turns multiplayer on with a new generation");
            Check(GTAxSkate.Coop.CoopPrefs.Read(prefs), "Enable is persisted to the prefs file");
            Check(host.CreateCount == 0, "enable alone does not build the adapter before the next tick");

            // Startup stays interruptible while RageCoop is not ready.
            host.CoopReady = false;
            Ticks(logic, host, 2);
            Check(host.CreateCount == 0 && !logic.AdapterStarted, "enabled but not ready: waits without starting");
            logic.EnqueueKey(CoopBootstrapLogic.VkF11);
            Ticks(logic, host, 1);
            Check(logic.PanelOpen && host.LastPanel.Items[0] == GTAxSkate.Coop.CoopInGameMenuState.DisableLabel &&
                host.LastPanel.Status.Contains("Waiting"), "initialising panel keeps Disable and Close available");
            logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
            Ticks(logic, host, 1);
            Check(!CoopRuntimeControl.Enabled && !GTAxSkate.Coop.CoopPrefs.Read(prefs) && host.CreateCount == 0, "Disable from the initialising panel persists off");

            // Enable again -> adapter starts exactly once, adapter menu opens, Host is NOT triggered.
            host.CoopReady = true;
            logic.EnqueueKey(CoopBootstrapLogic.VkF11);
            logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
            Ticks(logic, host, 3);
            Check(host.CreateCount == 1 && logic.AdapterStarted && ClientAdapter.ActiveOwner != null, "re-enable starts exactly one adapter");
            RageCoop.Client.Scripting.API.Actions.Dequeue()();
            ClientAdapter owner = ClientAdapter.ActiveOwner;
            Check(owner.MenuState.IsOpen, "adapter menu opens after the user enabled it");
            Check(host.LastBridge.EnsureHostCalls == 0 && host.LastBridge.StopServerCalls == 0, "enabling never hosts automatically");
            Ticks(logic, host, 3);
            Check(host.CreateCount == 1, "startup happens once");
            var items = owner.MenuState.BuildVisibleItems(false, false, false, string.Empty, false, false, null);
            Check(items[items.Length - 1].Action == GTAxSkate.Coop.CoopMenuAction.DisableMultiplayer, "disable item present in the enabled menu");
            owner.OnStop();
        }
        finally
        {
            CoopRuntimeControl.ResetForTests();
            try { Directory.Delete(dir, true); } catch { }
        }
    }

    // Round 4 (Copilot round-3 review): a host action never kills a running local server with another key; the player
    // stops it with the "Stop local server" row, which shows only while this client is not connected or connecting.
    private static void RunStopLocalServerTests()
    {
        const CoopMenuAction Stop = CoopMenuAction.StopLocalServer;
        var s = new GTAxSkate.Coop.CoopInGameMenuState();
        Check(Array.FindIndex(s.BuildVisibleItems(false, false, false, string.Empty, false, false, null, null, false, false, false), i => i.Action == Stop) < 0, "no Stop row without a running local server");
        var rows = s.BuildVisibleItems(false, false, false, string.Empty, false, false, null, null, false, false, true);
        int at = Array.FindIndex(rows, i => i.Action == Stop);
        Check(at >= 0 && rows[at].Label == "Stop local server" && rows[at].Enabled, "a running local server while disconnected shows the Stop row");
        Check(Array.FindIndex(s.BuildVisibleItems(false, false, false, string.Empty, false, false, null, null, false, true, true), i => i.Action == Stop) < 0, "no Stop row while a connect is pending");
        Check(Array.FindIndex(s.BuildVisibleItems(true, true, false, string.Empty, false, false, null, null, false, false, true), i => i.Action == Stop) < 0, "no Stop row while connected");
        Check(Array.FindIndex(s.BuildVisibleItems(true, false, false, string.Empty, false, false, null, null, false, true, true), i => i.Action == Stop) < 0, "no Stop row while on the server but not yet authenticated");

        // Host with a server of another key still running: the notice is shown as is, nothing was started or killed.
        var b = new LoadingMockWorldBridge { Loading = false, LocalServerRunning = true, EnsureResult = false, EnsureMessage = LocalServerKey.StillRunningNotice };
        var a = StartAdapter(b);
        SelectMenu(a, 1);
        a.HandleMenuKeyCode(0x0D);
        Check(b.EnsureHostCalls == 1 && b.ConnectCalls == 0 && b.BeginOnlineCalls == 0 && b.StopServerCalls == 0 && b.LocalServerRunning, "the refused host action starts nothing and stops nothing");
        Check(b.Notes.Contains("A coop server from your last session is still running. Choose 'Stop local server' first."), "the sanitized notice is shown without a prefix");
        Check(b.LastOverlay != null && Array.Exists(b.LastOverlay.Items, i => i.Action == Stop), "the menu offers Stop local server");

        // Choosing it stops only our server (and its router mapping) once, then the row is gone.
        int idx = Array.FindIndex(b.LastOverlay.Items, i => i.Action == Stop);
        for (int i = 1; i < idx; i++) a.HandleMenuKeyCode(0x28);   // the menu is still open with "Host private session" (row 1) selected
        a.HandleMenuKeyCode(0x0D);
        Check(b.StopServerCalls == 1 && !b.LocalServerRunning && b.Notes.Contains("Local server stopped."), "Stop local server stops the server once");
        Check(b.DisconnectCalls == 0, "it does not touch a transport connection");
        Check(b.LastOverlay == null || !Array.Exists(b.LastOverlay.Items, i => i.Action == Stop), "the row disappears after the stop");
        a.StopForDisable();
    }

    private static void SelectMenu(ClientAdapter owner, int index)
    {
        owner.HandleMenuKeyCode(0x7A);
        for (int i = 0; i < index; i++) owner.HandleMenuKeyCode(0x28);
    }

    // Disable: stale work fails closed, owned server stops once, handoff is untouched.
    private static void RunDisableTests()
    {
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        string err;

        // Disable through the adapter menu stops the owned server exactly once and goes silent.
        var bridge = new LoadingMockWorldBridge { Loading = false };
        var adapter = new ClientAdapter();
        adapter.SetWorldBridge(bridge);
        adapter.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        int gen1 = CoopRuntimeControl.Generation;
        SelectMenu(adapter, 4); // Connect, Host private, Host online, Join online, Disable
        adapter.HandleMenuKeyCode(0x0D);
        Check(!CoopRuntimeControl.Enabled && !adapter.IsActive && ClientAdapter.ActiveOwner == null, "menu Disable switches off and releases the owner");
        Check(bridge.StopServerCalls == 1 && bridge.DisposedCount == 1 && bridge.DisconnectCalls == 1, "Disable stops the owned server once, disposes and disconnects");
        int draws = bridge.OverlayDraws;
        RageCoop.Client.Scripting.API.Events.Tick();
        adapter.StepEveryFrame();
        Check(!adapter.HandleMenuKeyCode(0x7A) && bridge.OverlayDraws == draws && RageCoop.Client.Scripting.API.Sent.Count == 0,
            "after Disable: no ticks, menu, draws or events");
        Check(CoopRuntimeControl.Generation != gen1, "Disable advanced the generation");

        // Re-enable gets a fresh generation; the old adapter stays dead.
        CoopRuntimeControl.SetEnabled(true, out err);
        Check(CoopRuntimeControl.Generation != gen1 && !adapter.IsLive, "re-enable gets a new generation, the old adapter stays dead");
        var adapter2 = new ClientAdapter();
        var bridge2 = new LoadingMockWorldBridge { Loading = false };
        adapter2.SetWorldBridge(bridge2);
        adapter2.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        Check(adapter2.IsLive && !adapter.IsLive, "only the new start is live");

        // Disabled behind the adapter's back (external switch) -> every entry point fails closed before cleanup.
        CoopRuntimeControl.SetEnabled(false, out err);
        draws = bridge2.OverlayDraws;
        Check(adapter2.IsActive && !adapter2.IsLive, "stale adapter is detected as not live");
        adapter2.StepEveryFrame();
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(!adapter2.HandleMenuKeyCode(0x7A) && bridge2.OverlayDraws == draws && RageCoop.Client.Scripting.API.Sent.Count == 0 &&
            bridge2.Notes.Count == 0, "queued work after an external disable fails closed (no draw/notice/event)");
        adapter2.StopForDisable();
        Check(bridge2.StopServerCalls == 1 && bridge2.DisposedCount == 1, "StopForDisable stops the owned server once");

        // Disable while the invite is still being read from the clipboard: no connect, no notice, no clipboard clear.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var bridge3 = new LoadingMockWorldBridge { Loading = false, ClipboardText = TestLanInvite, ClipboardPendingPolls = 2 };
        var adapter3 = new ClientAdapter();
        adapter3.SetWorldBridge(bridge3);
        adapter3.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        SelectMenu(adapter3, 3);
        adapter3.HandleMenuKeyCode(0x0D);
        Check(bridge3.ClipboardReads == 1 && bridge3.ConnectCalls == 0, "join starts a clipboard read and connects nothing yet");
        CoopRuntimeControl.SetEnabled(false, out err);
        adapter3.StepEveryFrame();
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(bridge3.EnsureHostCalls == 0 && bridge3.ConnectCalls == 0 && bridge3.Notes.Count == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0 &&
            bridge3.ClipboardClears == 0, "disable during a clipboard read: no connect, no notice, no clear");
        adapter3.StopForDisable();

        // Disable while the server is starting: the owned server is stopped again, no connect follows.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var bridge4 = new LoadingMockWorldBridge { Loading = false };
        var adapter4 = new ClientAdapter();
        adapter4.SetWorldBridge(bridge4);
        adapter4.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        bridge4.EnsureHook = delegate { CoopRuntimeControl.SetEnabled(false, out err); };
        SelectMenu(adapter4, 1);
        adapter4.HandleMenuKeyCode(0x0D);
        Check(bridge4.EnsureHostCalls == 1 && bridge4.StopServerCalls == 1 && RageCoop.Client.Scripting.API.Sent.Count == 0 && bridge4.Notes.Count == 0,
            "disable while hosting: owned server stopped, no connect, no notice");
        adapter4.StopForDisable();
        Check(bridge4.StopServerCalls == 2, "Disable stops only via StopLocalHostServerIfStarted (idempotent in the real bridge)");

        // Enabled-owner handoff keeps the server, then Disable stops it exactly once.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var bootBridge = new LoadingMockWorldBridge { Loading = false };
        var boot = new ClientAdapter();
        boot.SetWorldBridge(bootBridge);
        boot.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        var resBridge = new LoadingMockWorldBridge { Loading = false };
        var res = new ClientAdapter();
        res.SetWorldBridge(resBridge);
        res.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        Check(ClientAdapter.ActiveOwner == res && bootBridge.StopServerCalls == 0 && bootBridge.DisposedCount == 0 && resBridge.StopServerCalls == 0,
            "handoff while enabled keeps the local server (B152)");
        res.DisableMultiplayer();
        Check(!CoopRuntimeControl.Enabled && resBridge.StopServerCalls == 1 && bootBridge.StopServerCalls == 0,
            "disable after handoff stops the owned server once; the superseded bridge is untouched");
        boot.OnStop();
        Check(bootBridge.StopServerCalls == 0, "late OnStop of the superseded owner never stops the server");
        CoopRuntimeControl.ResetForTests();
    }

    // Peer review 2026-11-16 (Codex gpt-6.1-sol): each block reproduced a defect before its fix.
    private static void RunPeerReviewRegressionTests()
    {
        string err;
        // P1: two loaded copies of the client assembly arbitrate ONE owner through the shared AppDomain slot.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var bridgeA = new LoadingMockWorldBridge { Loading = false };
        var a = new ClientAdapter();
        a.SetWorldBridge(bridgeA);
        a.OnStart();
        Check(a.IsLive, "P1: first copy is live");
        var dupAsm = System.Reflection.Assembly.Load(File.ReadAllBytes(typeof(ClientAdapter).Assembly.Location));
        Type dupType = dupAsm.GetType("GTAxSkate.Coop.RageCoopAdapter.ClientAdapter", true);
        Check(dupType != typeof(ClientAdapter), "P1: the duplicate is a different type identity");
        object b = Activator.CreateInstance(dupType);
        dupType.GetMethod("OnStart").Invoke(b, null);
        bool bLive = (bool)dupType.GetProperty("IsLive").GetValue(b, null);
        Check(bLive != a.IsLive, "P1: exactly one of two assembly copies is live (a=" + a.IsLive + ", b=" + bLive + ")");
        Check(bLive && !a.IsLive, "P1: the newest start owns (same rule as same-copy handoff)");
        a.OnStop();
        Check(bridgeA.StopServerCalls == 0, "P1: a displaced copy's late OnStop never stops the server");
        Check((bool)dupType.GetProperty("IsLive").GetValue(b, null), "P1: a displaced copy's OnStop does not release the live owner");
        dupType.GetMethod("OnStop").Invoke(b, null);
        var after = new ClientAdapter();
        after.OnStart();
        Check(after.IsLive, "P1: ownership is claimable again after the owner stopped");
        after.OnStop();

        // P2 (handoff during the clipboard read): the old owner must not connect or notify through its captured bridge.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var oldBridge = new LoadingMockWorldBridge { Loading = false, ClipboardText = TestLanInvite, ClipboardPendingPolls = 1 };
        var oldOwner = new ClientAdapter();
        oldOwner.SetWorldBridge(oldBridge);
        oldOwner.OnStart();
        SelectMenu(oldOwner, 3);
        oldOwner.HandleMenuKeyCode(0x0D);
        ClientAdapter replacement = new ClientAdapter();
        replacement.OnStart();
        Check(!oldOwner.IsLive && replacement.IsLive, "P2: handoff happened during the clipboard read");
        oldOwner.StepEveryFrame();
        oldOwner.StepEveryFrame();
        Check(oldBridge.ConnectCalls == 0 && oldBridge.EnsureHostCalls == 0, "P2: superseded owner connected nothing after the read");
        Check(oldBridge.StopServerCalls == 0 && oldBridge.DisposedCount == 0, "P2: handoff never stops the server (B152)");
        Check(oldBridge.Notes.Count == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0 && oldBridge.ClipboardClears == 0, "P2: superseded owner is silent");
        replacement.OnStop();
        oldOwner.OnStop();

        // P2 (handoff while the server is starting): no connect, server stays up (handoff never stops it).
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var hb = new LoadingMockWorldBridge { Loading = false };
        ClientAdapter hrep = null;
        hb.EnsureHook = delegate { hrep = new ClientAdapter(); hrep.OnStart(); };
        var ho = new ClientAdapter();
        ho.SetWorldBridge(hb);
        ho.OnStart();
        SelectMenu(ho, 1);
        ho.HandleMenuKeyCode(0x0D);
        Check(hb.EnsureHostCalls == 1 && hb.StopServerCalls == 0 && hb.Notes.Count == 0 && RageCoop.Client.Scripting.API.Sent.Count == 0,
            "P2: handoff while hosting keeps the server, no connect, no notice");
        hrep.OnStop();
        ho.OnStop();

        // 2026-11-16 evening test: the host's own connect must carry the friend key as the transport
        // password, or its server refuses it ("limited to verified private friends").
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var kb = new LoadingMockWorldBridge { Loading = false };
        var ko = new ClientAdapter();
        ko.SetWorldBridge(kb);
        ko.OnStart();
        SelectMenu(ko, 1);
        ko.HandleMenuKeyCode(0x0D);
        Check(kb.EnsureHostCalls == 1 && kb.ConnectCalls == 1 && CoopInvite.IsValidPassword(kb.LastConnectKey) && kb.LastConnectKey.Length == CoopInvite.PasswordLength,
            "host connect passes the generated password to the transport as the friend key");
        Check(kb.ClipboardCalls == 1 && kb.LastClipboard == CoopInvite.BuildLocal(InviteLocatorKind.Lan, "192.168.1.20", 4499, kb.LastConnectKey),
            "the host's invite (LAN locator + the same password) is copied to the clipboard");
        Check(kb.NameForcedBeforeEveryConnect && kb.ForceNameCalls == 1, "the RageCoop name is forced before the connect");
        ko.OnStop();

        // Copilot review 2026-11-16: cancelling RageCoop's spoof warning clears the pending connect
        // and stops the server this host attempt started.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var cancelBridge = new LoadingMockWorldBridge { Loading = false, ConnectResult = false };
        var co = new ClientAdapter();
        co.SetWorldBridge(cancelBridge);
        co.OnStart();
        SelectMenu(co, 1);
        co.HandleMenuKeyCode(0x0D);
        Check(cancelBridge.ConnectCalls == 1 && cancelBridge.StopServerCalls == 1, "cancelled host connect stops the local server");
        Check(cancelBridge.ClipboardText == string.Empty && cancelBridge.ClipboardClears == 1, "a cancelled host takes its invite off the clipboard again");
        cancelBridge.ConnectResult = true;
        co.HandleMenuKeyCode(0x1B);
        SelectMenu(co, 0); // the menu keeps row 1 selected
        co.HandleMenuKeyCode(0x0D);
        Check(cancelBridge.ConnectCalls == 2, "after a cancel the host can try again (no stale pending connect) calls=" + cancelBridge.ConnectCalls + " ensure=" + cancelBridge.EnsureHostCalls + " notes=" + string.Join("|", cancelBridge.Notes));
        // Test 22:31: RageCoop gave up connecting -> pending connect ends at once (no 15 s "already pending").
        // Debounced (test 2026-11-17 10:14): a gave-up transport counts only after >= 5 s since the connect start AND
        // >= 1.5 s of continuous idle; an idle tick right before the connection lands must not fail the connect.
        cancelBridge.TransportConnecting = false;
        co.TestClockOffsetMs = 1000;
        co.StepEveryFrame();
        co.TestClockOffsetMs = 4000;
        co.StepEveryFrame();
        Check(!cancelBridge.Notes.Exists(n => n.Contains("could not reach the server")), "no fail-fast inside the first 5 s");
        cancelBridge.TransportConnecting = true;
        co.TestClockOffsetMs = 6000;
        co.StepEveryFrame();   // connecting again: the idle timer resets
        cancelBridge.TransportConnecting = false;
        co.TestClockOffsetMs = 6100;
        co.StepEveryFrame();   // idle counted from here
        co.TestClockOffsetMs = 6900;
        co.StepEveryFrame();
        Check(!cancelBridge.Notes.Exists(n => n.Contains("could not reach the server")), "no fail-fast before 1.5 s of continuous idle (even after 5 s)");
        cancelBridge.TransportConnecting = true;
        co.TestClockOffsetMs = 7000;
        co.StepEveryFrame();   // flips back: the timer resets again
        cancelBridge.TransportConnecting = false;
        co.TestClockOffsetMs = 7100;
        co.StepEveryFrame();
        co.TestClockOffsetMs = 8000;
        co.StepEveryFrame();   // only 0.9 s idle since the reset
        Check(!cancelBridge.Notes.Exists(n => n.Contains("could not reach the server")), "the idle timer resets whenever RageCoop is connecting again");
        co.TestClockOffsetMs = 8800;
        co.StepEveryFrame();   // 1.7 s idle, > 5 s since the start
        Check(cancelBridge.Notes.Exists(n => n.Contains("could not reach the server")), "failed transport connect ends the pending connect");
        co.OnStop();

        // The 10:14:50 false positive: IsConnecting turns false one tick before IsOnServer turns true.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var flipBridge = new LoadingMockWorldBridge { Loading = false };
        var fo = new ClientAdapter();
        fo.SetWorldBridge(flipBridge);
        fo.OnStart();
        SelectMenu(fo, 1);
        fo.HandleMenuKeyCode(0x0D);
        flipBridge.TransportConnecting = false;
        fo.TestClockOffsetMs = 20000;
        fo.StepEveryFrame();   // IsConnecting already false, IsOnServer not yet true
        RageCoop.Client.Scripting.API.IsOnServer = true;
        fo.TestClockOffsetMs = 20050;
        fo.StepEveryFrame();
        Check(!flipBridge.Notes.Exists(n => n.Contains("could not reach the server")), "IsConnecting false one tick before IsOnServer true is not a failed connect");
        fo.OnStop();

        // P3: an empty clipboard after a disable stays silent (no "copy the invite" notice).
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var pb = new LoadingMockWorldBridge { Loading = false, ClipboardText = string.Empty, ClipboardPendingPolls = 1 };
        var pa = new ClientAdapter();
        pa.SetWorldBridge(pb);
        pa.OnStart();
        SelectMenu(pa, 3);
        pa.HandleMenuKeyCode(0x0D);
        CoopRuntimeControl.SetEnabled(false, out err);
        pa.StepEveryFrame(); pa.StepEveryFrame();
        Check(pb.Notes.Count == 0 && pb.EnsureHostCalls == 0 && pb.ConnectCalls == 0, "P3: empty clipboard after a disable stays silent");
        pa.StopForDisable();

        // P3: handoff then an empty clipboard is silent too.
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var xb = new LoadingMockWorldBridge { Loading = false, ClipboardText = string.Empty, ClipboardPendingPolls = 1 };
        var xo = new ClientAdapter();
        xo.SetWorldBridge(xb);
        xo.OnStart();
        SelectMenu(xo, 3);
        xo.HandleMenuKeyCode(0x0D);
        ClientAdapter xrep = new ClientAdapter();
        xrep.OnStart();
        xo.StepEveryFrame(); xo.StepEveryFrame();
        Check(xb.Notes.Count == 0, "P3: empty clipboard after a handoff stays silent");
        xrep.OnStop();
        xo.OnStop();
        CoopRuntimeControl.ResetForTests();
    }


    // A second assembly copy (what RageCoop's resource loader creates): different type identity, shared AppDomain state.
    private sealed class DupCopy
    {
        public readonly Type AdapterType;
        public readonly Type ApiType;
        public readonly object Adapter;
        public DupCopy()
        {
            var asm = System.Reflection.Assembly.Load(File.ReadAllBytes(typeof(ClientAdapter).Assembly.Location));
            AdapterType = asm.GetType("GTAxSkate.Coop.RageCoopAdapter.ClientAdapter", true);
            ApiType = asm.GetType("RageCoop.Client.Scripting.API", true);
            Adapter = Activator.CreateInstance(AdapterType);
            ApiType.GetField("IsOnServer").SetValue(null, false);
        }
        public void Start()
        {
            AdapterType.GetMethod("OnStart").Invoke(Adapter, null);
            var actions = (Queue<Action>)ApiType.GetField("Actions").GetValue(null);
            while (actions.Count > 0) actions.Dequeue()();
        }
        public void Stop() { AdapterType.GetMethod("OnStop").Invoke(Adapter, null); }
        public void RageCoopTick() { var ev = ApiType.GetNestedType("Events"); ev.GetMethod("Tick").Invoke(null, null); }
        public bool IsLive { get { return (bool)AdapterType.GetProperty("IsLive").GetValue(Adapter, null); } }
        public bool IsActive { get { return (bool)AdapterType.GetProperty("IsActive").GetValue(Adapter, null); } }
        public Guid Token { get { return (Guid)AdapterType.GetField("ownerToken", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance).GetValue(Adapter); } }
    }

    // One tick of the fake clock; the live copy B beats first (as its own RageCoop tick would).
    private static void BeatTick(CoopBootstrapLogic logic, MockHost host, DupCopy b, bool beat)
    {
        host.Now += 600;
        if (beat) CoopRuntimeControl.Heartbeat(b.Token, host.Now);
        logic.Tick();
    }

    private static void BootstrapWithDisplacingCopy(out MockHost host, out CoopBootstrapLogic logic, out ClientAdapter a, out LoadingMockWorldBridge bridgeA, out DupCopy b)
    {
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        host = new MockHost();
        logic = new CoopBootstrapLogic(host);
        Ticks(logic, host, 1);
        Check(logic.AdapterStarted && host.CreateCount == 1, "D: bootstrap started owner A");
        a = ClientAdapter.ActiveOwner;
        bridgeA = host.LastBridge;
        b = new DupCopy();
        b.Start();
        Check(a.IsActive && !a.IsLive && a.IsDisplaced && b.IsLive, "D: B displaced A (A active, not live, displaced)");
    }

    // Re-check 2026-11-16: the bootstrap's own adapter (A) is displaced by a resource-loaded copy (B).
    // The bootstrap must step A down WITHOUT touching the shared transport that B uses.
    private static void RunDisplacedCopyTests()
    {
        MockHost host; CoopBootstrapLogic logic; ClientAdapter a; LoadingMockWorldBridge bridgeA; DupCopy b;

        // D1: displaced A + live B: transport untouched, B stays live, no restart over it.
        BootstrapWithDisplacingCopy(out host, out logic, out a, out bridgeA, out b);
        for (int i = 0; i < 30; i++) BeatTick(logic, host, b, true);   // 18 s
        Check(bridgeA.DisconnectCalls == 0 && host.Disconnects == 0, "D1: stepping A down never disconnects the shared transport, disconnects=" + bridgeA.DisconnectCalls);
        Check(b.IsLive && !a.IsActive, "D1: B stays live, A stepped down");
        Check(host.CreateCount == 1 && host.Notifications.Count == 0, "D1: no restart and no notice while B is live");
        Check(logic.DeferredToOtherCopy && !logic.AdapterStarted, "D1: bootstrap is in the deferred state");
        Check(bridgeA.StopServerCalls == 0 && bridgeA.DisposedCount == 0, "D1: displaced A never stops the server or disposes the shared bridge");

        // D2: B stops (token released): bootstrap restarts within one tick.
        b.Stop();
        BeatTick(logic, host, b, false);
        Check(host.CreateCount == 2 && logic.AdapterStarted && !logic.DeferredToOtherCopy, "D2: restart within one tick after B released the owner");
        Check(ClientAdapter.ActiveOwner != null && ClientAdapter.ActiveOwner.IsLive, "D2: the restarted adapter is live");
        logic.Abort();

        // D3: B stops heartbeating (crashed copy, token never released): restart after 5 s, not before.
        BootstrapWithDisplacingCopy(out host, out logic, out a, out bridgeA, out b);
        for (int i = 0; i < 5; i++) BeatTick(logic, host, b, true);
        long lastBeat = host.Now;
        Check(logic.DeferredToOtherCopy && host.CreateCount == 1, "D3: deferred while B beats");
        int ticksToRestart = 0;
        while (host.CreateCount == 1 && ticksToRestart < 30)
        {
            BeatTick(logic, host, b, false);
            ticksToRestart++;
            if (host.CreateCount == 1) Check(host.Now - lastBeat < 5000, "D3: no restart while the heartbeat is fresher than 5 s");
        }
        Check(host.CreateCount == 2 && host.Now - lastBeat >= 5000 && host.Now - lastBeat < 5000 + 600 + 1,
            "D3: restart on the first tick after the heartbeat went stale 5 s, waited " + (host.Now - lastBeat) + " ms");
        Check(logic.AdapterStarted && ClientAdapter.ActiveOwner.IsLive, "D3: crashed token never blocks the new owner");
        Check(bridgeA.DisconnectCalls == 0, "D3: no transport disconnect on the way");
        b.Stop();
        Check(ClientAdapter.ActiveOwner.IsLive, "D3: the crashed copy's late OnStop does not release the new owner");
        logic.Abort();

        // D4: Disable while deferred: disabled, nothing restarts, the owner tears itself down (server stop only by the owner).
        BootstrapWithDisplacingCopy(out host, out logic, out a, out bridgeA, out b);
        for (int i = 0; i < 3; i++) BeatTick(logic, host, b, true);
        Check(logic.DeferredToOtherCopy, "D4: deferred");
        logic.EnqueueKey(CoopBootstrapLogic.VkF11);
        BeatTick(logic, host, b, true);
        Check(logic.PanelOpen && host.LastPanel != null && host.LastPanel.Items[0] == GTAxSkate.Coop.CoopInGameMenuState.DisableLabel, "D4: panel opens while deferred");
        logic.EnqueueKey(CoopBootstrapLogic.VkReturn);
        BeatTick(logic, host, b, true);
        Check(!CoopRuntimeControl.Enabled, "D4: Disable from the panel works while deferred");
        BeatTick(logic, host, b, true);
        Check(!logic.DeferredToOtherCopy && host.CreateCount == 1 && !logic.AdapterStarted, "D4: deferred state cleared, nothing restarted");
        Check(b.IsActive, "D4: B is still running until its own tick notices");
        b.RageCoopTick();
        Check(!b.IsActive, "D4: B tears itself down after the disable");
        Check(bridgeA.StopServerCalls == 0 && bridgeA.DisconnectCalls == 0, "D4: displaced A never stops the server or the transport");
        CoopRuntimeControl.ResetForTests();

        // D5: the live adapter beats from its own RageCoop tick, with the shared wall clock.
        EnableFixture();
        var b5 = new DupCopy();
        b5.Start();
        Guid stranger = Guid.NewGuid();
        long now = CoopRuntimeControl.UtcNowMs();
        Check(!CoopRuntimeControl.HasLiveOtherOwner(b5.Token, now, 5000), "D5: the owner itself is never 'another owner'");
        b5.RageCoopTick();
        Check(CoopRuntimeControl.HasLiveOtherOwner(stranger, CoopRuntimeControl.UtcNowMs(), 5000), "D5: tick beat is fresh for others");
        Check(CoopRuntimeControl.HasLiveOtherOwner(stranger, CoopRuntimeControl.UtcNowMs() + 4000, 5000), "D5: still live after 4 s");
        Check(!CoopRuntimeControl.HasLiveOtherOwner(stranger, CoopRuntimeControl.UtcNowMs() + 6000, 5000), "D5: stale after 6 s");
        CoopRuntimeControl.Heartbeat(stranger, CoopRuntimeControl.UtcNowMs() + 6000);
        Check(!CoopRuntimeControl.HasLiveOtherOwner(stranger, CoopRuntimeControl.UtcNowMs() + 6000, 5000), "D5: only the current owner writes the heartbeat");
        b5.Stop();
        Check(!CoopRuntimeControl.HasLiveOtherOwner(stranger, CoopRuntimeControl.UtcNowMs(), 5000), "D5: released owner is not live");
        // A claim that never beat is fresh from first observation, then stale.
        Guid claimant = Guid.NewGuid();
        CoopRuntimeControl.ClaimOwner(claimant);
        Check(CoopRuntimeControl.HasLiveOtherOwner(stranger, 100000, 5000) && CoopRuntimeControl.HasLiveOtherOwner(stranger, 104000, 5000)
            && !CoopRuntimeControl.HasLiveOtherOwner(stranger, 105001, 5000), "D5: never-beating claimant is stale 5 s after first observation");
        CoopRuntimeControl.ResetForTests();
    }

    // Controller state is shared even when a second copy of the assembly is loaded.
    private static void RunRuntimeControlIdentityTests()
    {
        string srcRoot = Environment.GetEnvironmentVariable("GTAXSKATE_COOP_SRC");
        Check(!string.IsNullOrEmpty(srcRoot) && Directory.Exists(srcRoot), "test runner provides GTAXSKATE_COOP_SRC");
        string dir = NewTempDir();
        try
        {
            var provider = new Microsoft.CSharp.CSharpCodeProvider();
            var options = new System.CodeDom.Compiler.CompilerParameters { GenerateInMemory = false, OutputAssembly = Path.Combine(dir, "dup-control.dll") };
            options.TreatWarningsAsErrors = true;
            options.ReferencedAssemblies.Add("System.dll"); options.ReferencedAssemblies.Add("System.Xml.dll");   // RageCoopClientSettings (the UI lock written when multiplayer is switched on)
            var result = provider.CompileAssemblyFromFile(options,
                Path.Combine(Path.Combine(srcRoot, "ragecoop"), "CoopRuntimeControl.cs"),
                Path.Combine(Path.Combine(srcRoot, "shared"), "CoopPrefs.cs"),
                Path.Combine(Path.Combine(srcRoot, "shared"), "RageCoopClientSettings.cs"),
                Path.Combine(Path.Combine(srcRoot, "shared"), "CoopTrace.cs"));
            Check(!result.Errors.HasErrors, "duplicate controller copy compiles" + (result.Errors.HasErrors ? ": " + result.Errors[0] : string.Empty));
            var dupAssembly = System.Reflection.Assembly.LoadFrom(options.OutputAssembly);
            Type dup = dupAssembly.GetType("GTAxSkate.Coop.RageCoopAdapter.CoopRuntimeControl", true);
            Check(dup != typeof(CoopRuntimeControl), "a second assembly copy is a different type identity");

            CoopRuntimeControl.ResetForTests();
            Check(!(bool)dup.GetProperty("Enabled").GetValue(null, null), "both copies start off");
            string error;
            CoopRuntimeControl.SetEnabled(true, out error);
            Check((bool)dup.GetProperty("Enabled").GetValue(null, null), "state set through one copy is visible through the other");
            int gen = CoopRuntimeControl.Generation;
            Check((int)dup.GetProperty("Generation").GetValue(null, null) == gen, "generation is shared across copies");
            Check((bool)dup.GetMethod("IsCurrent").Invoke(null, new object[] { gen }), "IsCurrent agrees across copies");
            dup.GetMethod("SetEnabled").Invoke(null, new object[] { false, string.Empty });
            Check(!CoopRuntimeControl.Enabled && CoopRuntimeControl.Generation != gen, "a disable through the other copy fails this copy closed");
            Check(!CoopRuntimeControl.IsCurrent(gen), "stale generation is rejected across copies");
        }
        finally
        {
            CoopRuntimeControl.ResetForTests();
            try { Directory.Delete(dir, true); } catch { }
        }
    }

    // Switching multiplayer on locks RageCoop's own client UI in its settings file (F9 menu with address and name, server
    // browser); switching it off, or uninstalling, puts the user's values back (design 2026-11-17-streamer-safe-coop).
    private static void RunRageCoopUiLockTests()
    {
        string dir = NewTempDir();
        try
        {
            string prefs = Path.Combine(Path.Combine(Path.Combine(dir, "scripts"), "RageCoop"), "FriendsCoopPrefs.json");
            string settings = CoopRuntimeControl.RageCoopClientSettingsPath(prefs);
            Check(settings == Path.Combine(Path.Combine(Path.Combine(Path.Combine(dir, "scripts"), "RageCoop"), "Data"), "RageCoop.Client.Settings.xml"), "settings path sits in RageCoop's Data folder");
            Check(CoopRuntimeControl.RageCoopClientSettingsPath(null) == null && CoopRuntimeControl.RageCoopClientSettingsPath("") == null, "no prefs path -> no settings path");
            Directory.CreateDirectory(Path.GetDirectoryName(settings));
            File.WriteAllText(settings, "<Settings><Username>Real Name</Username><MenuKey>F9</MenuKey><LastServerAddress>203.0.113.9:4499</LastServerAddress></Settings>");

            CoopRuntimeControl.ResetForTests();
            Check(!CoopRuntimeControl.Initialize(prefs) && !RageCoopClientSettings.IsLocked(settings), "multiplayer off: RageCoop's file is untouched");
            string err;
            Check(CoopRuntimeControl.SetEnabled(true, out err) && RageCoopClientSettings.IsLocked(settings), "switching on locks RageCoop's client UI");
            Check(CoopRuntimeControl.SetEnabled(false, out err) && !RageCoopClientSettings.IsLocked(settings) &&
                File.ReadAllText(settings).Contains("Real Name") && File.ReadAllText(settings).Contains("<MenuKey>F9</MenuKey>"), "switching off restores the user's values");

            // Left on across a restart: RageCoop may have rewritten its file; the lock is re-applied at start and the backup is kept.
            CoopRuntimeControl.SetEnabled(true, out err);
            File.WriteAllText(settings, "<Settings><Username>Rewritten</Username><MenuKey>F9</MenuKey></Settings>");
            CoopRuntimeControl.ResetForTests();
            Check(CoopRuntimeControl.Initialize(prefs) && RageCoopClientSettings.IsLocked(settings), "a persisted on-state re-applies the lock at game start");
            CoopRuntimeControl.SetEnabled(false, out err);
            Check(!RageCoopClientSettings.IsLocked(settings) && File.ReadAllText(settings).Contains("Real Name"), "the original backup survived the restart");
        }
        finally
        {
            CoopRuntimeControl.ResetForTests();
            try { Directory.Delete(dir, true); } catch { }
        }
    }

    private static void RunBootstrapSafetyTests()
    {
        // Null-resource Logger throwing NullReferenceException + Game.IsLoading guard + single active owner handoff.
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var bridge = new LoadingMockWorldBridge { Loading = true };
        var boot = new ClientAdapter { SimulateNullResourceLogger = true };
        boot.SetWorldBridge(bridge);
        boot.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        boot.StepEveryFrame();
        Check(bridge.OverlayDraws == 0, "loading state suppresses all UI overlay draws");
        bridge.Loading = false;
        boot.StepEveryFrame();
        Check(bridge.OverlayDraws == 1, "post-load offline step draws status banner cleanly with null resource logger");
        Check(boot.HandleMenuKeyCode(0x7a) && boot.MenuState.IsOpen, "F11 works offline after loading");
        Check(bridge.OverlayDraws == 2, "F11 toggle renders updated menu immediately");

        // Second ClientAdapter cleanly stops the first active owner and inherits single ownership.
        var resOwner = new ClientAdapter { SimulateNullResourceLogger = true };
        var bridge2 = new LoadingMockWorldBridge { Loading = false };
        resOwner.SetWorldBridge(bridge2);
        resOwner.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        Check(!boot.IsActive && ClientAdapter.ActiveOwner == resOwner, "single active owner transfers cleanly");
        Check(bridge.DisposedCount == 0, "handoff preserves local host server without disposing prior bridge");
        Check(bridge.StopServerCalls == 0 && bridge2.StopServerCalls == 0, "handoff never stops the local host server (B152)");
        boot.StepEveryFrame();
        Check(bridge.OverlayDraws == 2, "superseded bootstrap owner emits zero extra draws or ticks");
        resOwner.StepEveryFrame();
        Check(bridge2.OverlayDraws == 1, "new active owner renders cleanly");
        resOwner.OnStop();
        Check(ClientAdapter.ActiveOwner == null && bridge2.DisposedCount == 1, "OnStop clears ActiveOwner and disposes bridge");
        Check(bridge2.StopServerCalls == 1, "OnStop stops the owned local server exactly once");
    }

    private static void RunTransportGuardTests()
    {
        RageCoop.Client.Scripting.API.Reset();
        var client = new ClientAdapter();
        client.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        var request = new GTAxSkate.Coop.ClientConnectRequest(
            GTAxSkate.Coop.LobbyDiscoverySource.DirectFriendInvite, "100.88.12.35", 4499, false,
            "Other friend", "Fixture-Shared-Key");
        GTAxSkate.Coop.PolicyDecision decision;
        Check(!client.BeginTrustedSessionConnect(request, out decision), "cannot authenticate against a different endpoint");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(client.ConnectionBanner.Contains("Unverified lobby") && RageCoop.Client.Scripting.API.Sent.Count == 0,
            "unarmed connection is blocked without sending events");
        client.OnStop();

        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        client = new ClientAdapter();
        client.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        Check(client.HandleMenuKeyCode(0x7a) && client.MenuState.IsOpen, "F11 opens the fixture menu");
        Check(client.HandleMenuKeyCode(0x1b) && !client.MenuState.IsOpen, "Escape closes the fixture menu");
        client.OnStop();
    }

    private static ServerAdapter NewFriendServer(string friendKey, out RageCoop.Server.Scripting.API api)
    {
        api = new RageCoop.Server.Scripting.API();
        var server = new ServerAdapter { API = api };
        server.ConfigureFriendSecurity(GTAxSkate.Coop.ServerSecurityConfig.DefaultPrivateFriends(friendKey), "127.0.0.1");
        server.OnStart();
        return server;
    }

    // Internet lobby (design 2026-11-16-coop-internet-lobby): public endpoints need the password; failures are throttled.
    private static void RunInternetLobbyServerTests()
    {
        string key = "SkateFriends-Secret-99";
        RageCoop.Server.Scripting.API api;
        ServerAdapter server = NewFriendServer(key, out api);
        Check(api.Events.Admit("8.8.4.4", key), "public endpoint + right password is admitted");
        Check(!api.Events.Admit("8.8.4.4", "wrong password"), "public endpoint + wrong password is denied");
        Check(api.Events.Admit("8.8.4.4", key), "one failure does not lock an address out");
        for (int i = 0; i < 5; i++) Check(!api.Events.Admit("9.9.9.9", "guess number " + i), "wrong guess " + i + " denied");
        Check(!api.Events.Admit("9.9.9.9", key), "5 failures lock that address out even for the right password");
        Check(api.Events.Admit("8.8.4.4", key) && api.Events.Admit("1.1.1.1", key), "other public addresses are unaffected by that lockout");
        Check(api.Events.Admit("127.0.0.1", key) && api.Events.Admit("192.168.1.9", key) && api.Events.Admit("100.88.12.34", key),
            "loopback, LAN and VPN are never throttled");
        Check(!api.Events.Admit("127.0.0.1", "wrong password") && !api.Events.Admit("192.168.1.9", "wrong password"), "LAN/loopback still need the password");
        for (int i = 0; i < 20; i++) api.Events.Admit("127.0.0.1", "wrong password");
        Check(api.Events.Admit("127.0.0.1", key), "LAN/loopback failures never lock them out");
        server.OnStop();

        // Global cap: 31 failures from many addresses lock all public handshakes, LAN and loopback stay admitted.
        ServerAdapter server2 = NewFriendServer(key, out api);
        for (int i = 0; i < 31; i++) Check(!api.Events.Admit("30.0." + (i / 100) + "." + (i % 100 + 1), "wrong"), "spread failure " + i);
        Check(!api.Events.Admit("77.77.77.77", key), "global lockout refuses a new public address even with the right password");
        Check(api.Events.Admit("127.0.0.1", key) && api.Events.Admit("10.0.0.5", key), "global lockout never affects loopback or LAN");
        server2.OnStop();

        // Without the friend key configured nothing is admitted, public or not.
        var api3 = new RageCoop.Server.Scripting.API();
        var open = new ServerAdapter { API = api3 };
        open.OnStart();
        Check(!api3.Events.Admit("8.8.4.4", key) && !api3.Events.Admit("127.0.0.1", key), "an unconfigured server admits nobody");
        open.OnStop();
    }

    // ---- Streamer-safe leak scan (design 2026-11-17-streamer-safe-coop) -----------------------------------------------
    // Everything a bridge was asked to show (every notice, every overlay frame: banner, subtitle, warnings, menu rows)
    // must be free of addresses, ports, lobby codes, passwords, join codes, Windows paths and user names.
    private static readonly System.Text.RegularExpressions.Regex LeakIpv4 = new System.Text.RegularExpressions.Regex(@"\d{1,3}(\.\d{1,3}){3}");
    private static readonly System.Text.RegularExpressions.Regex LeakIpv6 = new System.Text.RegularExpressions.Regex(@"::|([0-9A-Fa-f]{1,4}:){7}");
    private static readonly System.Text.RegularExpressions.Regex LeakHostPort = new System.Text.RegularExpressions.Regex(@"[A-Za-z0-9_.-]+:\d{4,5}");
    private static readonly System.Text.RegularExpressions.Regex LeakPath = new System.Text.RegularExpressions.Regex(@"[A-Za-z]:[\\/]|\\\\[A-Za-z0-9]");
    private static readonly System.Text.RegularExpressions.Regex LeakLobbyCode = new System.Text.RegularExpressions.Regex(@"SK8-?[0-9A-Za-z]{4}");

    private static void AssertNoLeak(LoadingMockWorldBridge bridge, string what, params string[] secrets)
    {
        foreach (string text in bridge.AllShown)
        {
            if (string.IsNullOrEmpty(text)) continue;
            Check(!LeakIpv4.IsMatch(text), what + ": no IPv4 address on screen: " + text);
            Check(!LeakIpv6.IsMatch(text), what + ": no IPv6 address on screen: " + text);
            Check(!LeakHostPort.IsMatch(text), what + ": no host:port on screen: " + text);
            Check(!LeakPath.IsMatch(text), what + ": no Windows path on screen: " + text);
            Check(!LeakLobbyCode.IsMatch(text), what + ": no lobby code on screen: " + text);
            foreach (string secret in secrets)
                if (!string.IsNullOrEmpty(secret))
                    Check(text.IndexOf(secret, StringComparison.OrdinalIgnoreCase) < 0, what + ": a secret is on screen: " + text);
        }
    }

    private static ClientAdapter StartAdapter(LoadingMockWorldBridge bridge)
    {
        EnableFixture();
        RageCoop.Client.Scripting.API.Reset();
        RageCoop.Client.Scripting.API.IsOnServer = false;
        var adapter = new ClientAdapter();
        adapter.SetWorldBridge(bridge);
        adapter.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        return adapter;
    }

    private static int CountNotes(LoadingMockWorldBridge bridge, string prefix)
    {
        int n = 0;
        foreach (string note in bridge.Notes) if (note.StartsWith(prefix, StringComparison.Ordinal)) n++;
        return n;
    }

    // Menu rows "Host online (internet)" and "Join friend (paste invite)" with the mock bridge: nothing is typed or drawn.
    private static void RunInternetLobbyMenuTests()
    {
        string code = null;
        Check(GTAxSkate.Coop.LobbyCode.TryEncode("8.8.4.4", 4499, out code), "test code encodes");
        string onlineInvite = CoopInvite.BuildOnline(code, TestPassword);
        Check(onlineInvite != null, "test online invite builds");

        // Row order of the disconnected menu.
        var items = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(false, false, false, string.Empty, false, false, null);
        Check(items.Length == 5 && items[0].Action == GTAxSkate.Coop.CoopMenuAction.ConnectConfiguredHost && !items[0].Enabled &&
            items[1].Action == GTAxSkate.Coop.CoopMenuAction.HostPrivateSession && items[1].Label == "Host private session (LAN / VPN)" &&
            items[2].Action == GTAxSkate.Coop.CoopMenuAction.HostOnline && items[2].Label == "Host online (internet)" &&
            items[3].Action == GTAxSkate.Coop.CoopMenuAction.JoinOnline && items[3].Label == "Join friend (paste invite)" &&
            items[4].Action == GTAxSkate.Coop.CoopMenuAction.DisableMultiplayer, "disconnected menu lists the rows without any address or typed entry");
        var withInvite = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(false, false, false, string.Empty, false, false, null, null, true);
        Check(withInvite.Length == 6 && withInvite[4].Action == GTAxSkate.Coop.CoopMenuAction.CopyInvite, "a hosting PC gets a Copy invite row");
        var cfgItems = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(false, false, true, "Alex-VPN", false, false, null);
        Check(cfgItems[0].Enabled && cfgItems[0].Label == "Connect to Friend: Friend", "a configured friend host keeps its row (free-text labels are never drawn)");
        var badLabel = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(false, false, true, "alex@192.168.1.5", false, false, null);
        Check(badLabel[0].Label == "Connect to Friend: Friend", "a configured label with digits, dots or @ is shown as Friend");
        var connected = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(true, true, false, string.Empty, false, false, null);
        foreach (var item in connected)
            Check(item.Action != GTAxSkate.Coop.CoopMenuAction.HostOnline && item.Action != GTAxSkate.Coop.CoopMenuAction.JoinOnline, "online rows only appear while disconnected");
        Check(connected[0].Label == "Create Friend Lobby (10 Slots)" && connected[1].Label == "Join Friend Lobby (paste code)",
            "the in-game join row now pastes the session code");
        var inSession = new GTAxSkate.Coop.CoopInGameMenuState().BuildVisibleItems(true, true, false, string.Empty, true, true, "ABC234");
        Check(inSession[0].Label == "Sync World / Mission / Shop Now" && inSession[1].Action == GTAxSkate.Coop.CoopMenuAction.CopyJoinCode, "the host row copies the join code");
        foreach (var item in inSession) Check(item.Label.IndexOf("ABC234", StringComparison.Ordinal) < 0, "no menu row shows the session join code");

        // Host online: the app generates the password, the server and port start, the local connect carries it; the invite arrives later.
        var hb = new LoadingMockWorldBridge { Loading = false };
        var ha = StartAdapter(hb);
        Check(hb.UiLockCalls == 1 && hb.UiLocked, "starting the adapter locks RageCoop's own client UI");
        SelectMenu(ha, 2);
        ha.HandleMenuKeyCode(0x0D);
        string password = hb.LastConnectKey;
        Check(hb.EnsureHostCalls == 1 && hb.BeginOnlineCalls == 1 && hb.BeginOnlinePort == 4499 && hb.ConnectCalls == 1 &&
            hb.LastConnectAddress == "127.0.0.1" && CoopInvite.IsValidPassword(password) && password.Length == CoopInvite.PasswordLength,
            "Host online starts the server, opens the port and connects locally with a generated 20-symbol password");
        Check(hb.ClipboardCalls == 0 && hb.NameForcedBeforeEveryConnect, "nothing is copied before the lobby opens; the name was forced before the connect");
        Check(hb.LastOverlay != null && hb.LastOverlay.Items.Length == 6 && hb.LastOverlay.Items[5].Label.StartsWith("Online lobby: ") &&
            !hb.LastOverlay.Items[5].Enabled, "menu shows a status row while the port opens");
        hb.OnlineStatus = new GTAxSkate.Coop.OnlineHostStatus(GTAxSkate.Coop.OnlineHostState.Opening, null, "Opening", "");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardCalls == 0, "no invite while the port is still opening");
        hb.OnlineStatus = new GTAxSkate.Coop.OnlineHostStatus(GTAxSkate.Coop.OnlineHostState.Open, "8.8.4.4", "Port open. Your own PC cannot test the public route; ask your friend to join.", "");
        RageCoop.Client.Scripting.API.Events.Tick();
        RageCoop.Client.Scripting.API.Events.Tick();
        string expectedInvite = CoopInvite.BuildOnline(code, password);
        Check(hb.ClipboardCalls == 1 && hb.LastClipboard == expectedInvite, "the invite (lobby code + password) is copied to the clipboard");
        Check(hb.Notes.Exists(n => n.StartsWith("Invite copied to the clipboard (hidden for streaming). Send it to your friend in a private message.") && n.Contains("cannot test")),
            "the notice says the invite is copied and hidden, and keeps the self-test limit");
        Check(hb.LastOverlay.Items.Length == 7 && hb.LastOverlay.Items[4].Action == GTAxSkate.Coop.CoopMenuAction.CopyInvite &&
            hb.LastOverlay.Items[6].Label.StartsWith("Online lobby: open"), "menu shows Copy invite and an open status, never the code");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardCalls == 1 && CountNotes(hb, "Invite copied") == 1, "the invite is announced once");
        AssertNoLeak(hb, "host online", password, code, expectedInvite, "8.8.4.4");
        // Copy it again from the menu.
        ha.HandleMenuKeyCode(0x28); ha.HandleMenuKeyCode(0x28);
        ha.HandleMenuKeyCode(0x0D);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardCalls == 2 && CountNotes(hb, "Invite copied") == 2, "Copy invite copies it again");
        // The host's clipboard copy is cleared after 10 minutes, but only if it still holds the invite.
        hb.ClipboardText = "something the host copied meanwhile";
        ha.TestClockOffsetMs = 11 * 60 * 1000;
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardClears == 0 && hb.ClipboardText == "something the host copied meanwhile", "a changed clipboard is never cleared");
        hb.ClipboardText = expectedInvite;
        ha.HandleMenuKeyCode(0x0D);   // copy again (re-arms the expiry)
        RageCoop.Client.Scripting.API.Events.Tick();
        RageCoop.Client.Scripting.API.Events.Tick();
        ha.TestClockOffsetMs += 11 * 60 * 1000;
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardClears == 1 && hb.ClipboardText == string.Empty, "after 10 minutes the invite leaves the clipboard");
        ha.HandleMenuKeyCode(0x0D);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(hb.ClipboardText == expectedInvite, "copied once more for the stop test");
        ha.StopForDisable();
        Check(hb.StopServerCalls >= 1 && hb.ClipboardText == string.Empty && !hb.UiLocked, "disabling stops the server, takes the invite off the clipboard and unlocks RageCoop's UI");
        AssertNoLeak(hb, "host online (whole session)", password, code, expectedInvite, "8.8.4.4");

        // Host online, router refused: message shown, no invite, no clipboard.
        var fb = new LoadingMockWorldBridge { Loading = false };
        var fa = StartAdapter(fb);
        SelectMenu(fa, 2);
        fa.HandleMenuKeyCode(0x0D);
        string refused = "Your router did not open the port (or your line has no public IPv4). Forward UDP 4499 to this PC, or use Tailscale.";
        fb.OnlineStatus = new GTAxSkate.Coop.OnlineHostStatus(GTAxSkate.Coop.OnlineHostState.Failed, null, refused, "upnp no device");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(fb.Notes.Contains(refused) && fb.ClipboardCalls == 0 && fb.LastOverlay.Items[5].Label.Contains("not available"), "UPnP failure shows the manual-forward / Tailscale message and no invite");
        AssertNoLeak(fb, "host online refused", fb.LastConnectKey);
        fa.StopForDisable();

        // Host online, the clipboard copy fails: the notice says so and never falls back to showing the invite.
        var xb = new LoadingMockWorldBridge { Loading = false, ClipboardCopyFails = true };
        var xa = StartAdapter(xb);
        SelectMenu(xa, 2);
        xa.HandleMenuKeyCode(0x0D);
        xb.OnlineStatus = new GTAxSkate.Coop.OnlineHostStatus(GTAxSkate.Coop.OnlineHostState.Open, "8.8.4.4", "Port open.", "");
        RageCoop.Client.Scripting.API.Events.Tick();
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(xb.Notes.Contains("Could not copy the invite. Try again.") && CountNotes(xb, "Invite copied") == 0, "a failed copy says so");
        AssertNoLeak(xb, "host online copy failed", xb.LastConnectKey, code);
        xa.StopForDisable();

        // Private host: LAN address by default, the VPN address when there is one, and a clear refusal without any network.
        var pb = new LoadingMockWorldBridge { Loading = false };
        var pa = StartAdapter(pb);
        SelectMenu(pa, 1);
        pa.HandleMenuKeyCode(0x0D);
        Check(pb.ClipboardCalls == 1 && pb.LastClipboard == CoopInvite.BuildLocal(InviteLocatorKind.Lan, "192.168.1.20", 4499, pb.LastConnectKey), "private host copies a LAN invite");
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(pb.Notes.Exists(n => n.StartsWith("Invite copied to the clipboard (hidden")), "private host announces the hidden copy");
        AssertNoLeak(pb, "host private", pb.LastConnectKey, "192.168.1.20");
        pa.StopForDisable();

        var vb = new LoadingMockWorldBridge { Loading = false, Vpn = "100.101.102.103" };
        var va = StartAdapter(vb);
        SelectMenu(va, 1);
        va.HandleMenuKeyCode(0x0D);
        Check(vb.LastClipboard == CoopInvite.BuildLocal(InviteLocatorKind.Vpn, "100.101.102.103", 4499, vb.LastConnectKey), "with a VPN address the invite carries it");
        va.StopForDisable();

        var nb = new LoadingMockWorldBridge { Loading = false, Lan = null };
        var na = StartAdapter(nb);
        SelectMenu(na, 1);
        na.HandleMenuKeyCode(0x0D);
        Check(nb.EnsureHostCalls == 0 && nb.ConnectCalls == 0 && nb.Notes.Exists(n => n.Contains("No LAN or VPN network address")), "no network address: nothing starts and the player is told");
        na.StopForDisable();

        // Host online, local connect cancelled: server stopped, invite gone and the online row disappears.
        var cb = new LoadingMockWorldBridge { Loading = false, ConnectResult = false };
        var ca = StartAdapter(cb);
        SelectMenu(ca, 2);
        ca.HandleMenuKeyCode(0x0D);
        Check(cb.StopServerCalls == 1 && cb.LastOverlay.Items.Length == 5, "cancelled host stops the server and clears the online status row");
        ca.StopForDisable();

        // Join friend (paste invite): online code. The invite is read from the clipboard, never typed; the address is not shown.
        // Pasted from a chat: odd case in the prefix, a line break before the password, spaces and quotes around it.
        string pasted = "\r\n  \"gtaxskate COOP invite " + code + "\r\n" + TestPassword + "\" \r\n";
        var jb = new LoadingMockWorldBridge { Loading = false, ClipboardText = pasted, ClipboardPendingPolls = 2 };
        var ja = StartAdapter(jb);
        SelectMenu(ja, 3);
        ja.HandleMenuKeyCode(0x0D);
        Check(jb.ClipboardReads == 1 && jb.ConnectCalls == 0, "Join starts a clipboard read");
        ja.StepEveryFrame();
        Check(jb.ConnectCalls == 0, "the connect waits for the finished read (nothing blocks the game thread)");
        ja.StepEveryFrame();
        Check(jb.ConnectCalls == 1 && jb.LastConnectAddress == "8.8.4.4" && jb.LastConnectPort == 4499 && jb.LastConnectKey == TestPassword,
            "Join connects to the decoded host with the password from the invite");
        Check(jb.ClipboardText == string.Empty && jb.ClipboardClears == 1, "a parsed invite leaves the clipboard");
        Check(jb.NameForcedBeforeEveryConnect, "the name is forced before the join");
        Check(jb.Notes.Exists(n => n == "Connecting to Online host..."), "the notice shows a label only");
        AssertNoLeak(jb, "join online", TestPassword, code, "8.8.4.4");
        ja.StopForDisable();

        // LAN and VPN invites.
        foreach (string locator in new[] { "lan:192.168.1.40:4499", "vpn:100.64.5.6:4500" })
        {
            var lb = new LoadingMockWorldBridge { Loading = false, ClipboardText = "GTAxSkate coop invite " + locator + " " + TestPassword };
            var la = StartAdapter(lb);
            SelectMenu(la, 3);
            la.HandleMenuKeyCode(0x0D);
            la.StepEveryFrame();
            string wantHost = locator.StartsWith("lan:") ? "192.168.1.40" : "100.64.5.6";
            int wantPort = locator.StartsWith("lan:") ? 4499 : 4500;
            Check(lb.ConnectCalls == 1 && lb.LastConnectAddress == wantHost && lb.LastConnectPort == wantPort && lb.LastConnectKey == TestPassword,
                "Join connects through the " + locator.Substring(0, 3) + " locator");
            Check(lb.Notes.Exists(n => n == (locator.StartsWith("lan:") ? "Connecting to LAN host..." : "Connecting to VPN host...")), "label only for " + locator.Substring(0, 3));
            AssertNoLeak(lb, "join " + locator.Substring(0, 3), TestPassword, wantHost);
            la.StopForDisable();
        }

        // Refused invites: empty clipboard, garbage, mistyped code, reserved code, short password, a public IP in a lan: locator.
        string typo = onlineInvite.Replace(code, code.Substring(0, 5) + (code[5] == '2' ? '3' : '2') + code.Substring(6));
        string reservedCode = GTAxSkate.Coop.LobbyCode.EncodeRaw(new byte[] { 192, 168, 1, 5 }, 4499);
        string[] bad =
        {
            string.Empty, "hello there", typo,
            "GTAxSkate coop invite " + reservedCode + " " + TestPassword,
            "GTAxSkate coop invite " + code + " short",
            "GTAxSkate coop invite lan:8.8.4.4:4499 " + TestPassword
        };
        string[] expect = { "Copy the invite your friend sent you", "not a valid invite", "not a valid invite", "not a valid invite", "not a valid invite", "not a valid invite" };
        for (int i = 0; i < bad.Length; i++)
        {
            var bb = new LoadingMockWorldBridge { Loading = false, ClipboardText = bad[i] };
            var ba = StartAdapter(bb);
            SelectMenu(ba, 3);
            ba.HandleMenuKeyCode(0x0D);
            ba.StepEveryFrame();
            Check(bb.ConnectCalls == 0 && bb.Notes.Exists(n => n.Contains(expect[i])), "Join refuses case " + i + " (" + expect[i] + ")");
            Check(bb.ClipboardText == bad[i], "a refused invite is left on the clipboard");
            AssertNoLeak(bb, "join refused " + i, TestPassword, code);
            ba.StopForDisable();
        }

        // A clipboard read that never finishes ends with a notice, never a hang or a connect.
        var tb = new LoadingMockWorldBridge { Loading = false, ClipboardText = TestLanInvite, ClipboardPendingPolls = 100000 };
        var ta = StartAdapter(tb);
        SelectMenu(ta, 3);
        ta.HandleMenuKeyCode(0x0D);
        ta.StepEveryFrame();
        Check(tb.ConnectCalls == 0 && tb.Notes.Count == 0, "a pending read is silent at first");
        ta.TestClockOffsetMs = 6000;
        ta.StepEveryFrame();
        Check(tb.ConnectCalls == 0 && tb.Notes.Contains("Could not read the clipboard. Try again."), "a stuck read ends with a notice");
        ta.StopForDisable();

        // The old typed pinned-friend entry is gone: row 0 is disabled without a saved friend and never prompts.
        var ub = new LoadingMockWorldBridge { Loading = false };
        var ua = StartAdapter(ub);
        SelectMenu(ua, 0);
        ua.HandleMenuKeyCode(0x0D);
        Check(ub.ConnectCalls == 0 && ub.ClipboardReads == 0 && ub.Notes.Count == 0, "no saved friend: the first row does nothing and asks for nothing");
        ua.StopForDisable();
        CoopRuntimeControl.ResetForTests();
    }

    // Leak test over relayed error texts and the remaining flows: nothing a stream could show leaks through Notify or the overlay.
    private static void RunStreamerSafeLeakTests()
    {
        string windowsUser = Environment.UserName;
        string[] errors =
        {
            "connect to 203.0.113.9:4499 refused",
            "[fe80::1]:4499 unreachable and 2001:db8::7 too",
            @"could not open C:\Users\" + windowsUser + @"\Documents\GTA\log.txt",
            "welcome back, " + windowsUser + " (%USERNAME%)",
            "host my-pc.local:4499 said no",
            "bad invite SK8-ABCD-EFGH-JKMN",
            "password " + TestPassword + " is wrong"
        };
        var bridge = new LoadingMockWorldBridge { Loading = false };
        var adapter = StartAdapter(bridge);
        adapter.DiagnosticFixtureOnly = true;   // the unverified-lobby guard would drop the connection before the reasons are shown
        RageCoop.Client.Scripting.API.IsOnServer = true;
        SafeText.AddSecret(SafeText.GroupJoin, TestPassword);
        foreach (string error in errors)
        {
            RageCoop.Client.Scripting.API.Deliver(new Message(RageCoop.Core.Scripting.CustomEvents.Hash(SessionProtocol.SessionRejected), new object[] { 1, error }));
            RageCoop.Client.Scripting.API.Events.Tick();
        }
        Check(CountNotes(bridge, "Coop: ") == errors.Length, "every relayed rejection reason was shown (sanitized), got " + CountNotes(bridge, "Coop: "));
        AssertNoLeak(bridge, "relayed server texts", TestPassword, windowsUser);
        foreach (string note in bridge.Notes)
            if (note.StartsWith("Coop: ")) Check(note.Contains(SafeText.Hidden), "the relayed text was masked: " + note);
        adapter.StopForDisable();
        SafeText.ClearSecrets();

        // The bootstrap panel and its notices (RageCoop / bootstrap / preference errors) are sanitized as well.
        SafeText.AddSecret(SafeText.GroupHost, TestPassword);
        var panel = new CoopPanelView("Multiplayer", new[] { "Enable at 10.0.0.5:4499" }, 0,
            @"Start failed: C:\Users\" + windowsUser + @"\x.dll (192.168.1.9)", "Remembered in " + TestPassword);
        Check(panel.Items[0].IndexOf("10.0.0.5", StringComparison.Ordinal) < 0 && panel.Status.IndexOf("192.168.1.9", StringComparison.Ordinal) < 0 &&
            panel.Status.IndexOf(windowsUser, StringComparison.OrdinalIgnoreCase) < 0 && panel.Help.IndexOf(TestPassword, StringComparison.Ordinal) < 0,
            "the bootstrap panel masks addresses, paths, user names and secrets");
        SafeText.ClearSecrets();
        CoopRuntimeControl.ResetForTests();
    }

    // Round 2 (live test 2026-11-17 10:14 and the Copilot/Gemini peer reviews): UI-lock failures are told to the player,
    // a copy whose read-back failed is still tracked for clearing, the expiry survives a crash (hash + time only).
    private static void RunRound2AdapterTests()
    {
        string dir = NewTempDir();
        InviteExpiryStore.DirectoryOverride = dir;
        string storePath = Path.Combine(dir, InviteExpiryStore.FileName);
        try
        {
            // 1. RageCoop's live menu could not be locked: a sanitized notice, not silence.
            var fb = new LoadingMockWorldBridge { Loading = false, UiLockFails = true };
            var fa = StartAdapter(fb);
            Check(fb.Notes.Exists(n => n.Contains("RageCoop menu member not reachable.") && n.Contains("Do not open the F9 menu while streaming.")), "a failed live lock is reported to the player: " + string.Join(" | ", fb.Notes));
            int before = fb.Notes.Count;
            fa.StopForDisable();
            Check(fb.UiLockCalls == 2 && fb.Notes.Count > before && fb.Notes.Exists(n => n.Contains("RageCoop menu member not reachable.") && !n.Contains("Do not open")), "a failed restore is reported too");
            AssertNoLeak(fb, "ui lock notices", TestPassword, Environment.UserName);

            // 2. The settings file could not be locked/restored: reported once, with a fixed text (no path).
            string prefs = Path.Combine(Path.Combine(Path.Combine(dir, "scripts"), "RageCoop"), "FriendsCoopPrefs.json");
            string settings = CoopRuntimeControl.RageCoopClientSettingsPath(prefs);
            Directory.CreateDirectory(Path.GetDirectoryName(settings));
            File.WriteAllText(settings, "<<< not xml");
            CoopRuntimeControl.ResetForTests();
            CoopRuntimeControl.Initialize(prefs);
            string err;
            CoopRuntimeControl.SetEnabled(true, out err);
            var sb = new LoadingMockWorldBridge { Loading = false };
            var sa = new ClientAdapter();
            sa.SetWorldBridge(sb);
            RageCoop.Client.Scripting.API.Reset();
            RageCoop.Client.Scripting.API.IsOnServer = false;
            sa.OnStart();
            Check(sb.Notes.Exists(n => n.Contains("settings file could not be locked")) && CoopRuntimeControl.TakeUiLockError().Length == 0, "a failed settings-file lock is reported once");
            AssertNoLeak(sb, "settings-file notices", TestPassword, Environment.UserName);
            CoopRuntimeControl.SetEnabled(false, out err);
            string restoreError = CoopRuntimeControl.TakeUiLockError();
            Check(restoreError.Length == 0 || restoreError.IndexOf(dir, StringComparison.OrdinalIgnoreCase) < 0, "no path in the settings-file notice");
            sa.StopForDisable();
            CoopRuntimeControl.ResetForTests();

            // 3. The copy failed (read-back timed out or failed) but the text may be on the clipboard: still cleared at expiry.
            var cb = new LoadingMockWorldBridge { Loading = false, ClipboardCopyFails = true };
            var ca = StartAdapter(cb);
            SelectMenu(ca, 1);
            ca.HandleMenuKeyCode(0x0D);
            ca.StepEveryFrame(); ca.StepEveryFrame();
            Check(cb.Notes.Exists(n => n.StartsWith("Could not copy the invite")), "the failed copy is reported");
            Check(File.Exists(storePath), "the expiry record was written although the copy was reported failed");
            string raw = File.ReadAllText(storePath);
            Check(raw.IndexOf(cb.LastConnectKey, StringComparison.Ordinal) < 0 && raw.IndexOf("192.168", StringComparison.Ordinal) < 0 && raw.IndexOf("invite", StringComparison.Ordinal) < 0, "the record holds only a hash and a time");
            Check(cb.ClearAttempts == 0, "not cleared before the expiry");
            ca.TestClockOffsetMs = 11 * 60 * 1000;
            ca.StepEveryFrame();
            Check(cb.ClearAttempts == 1 && !File.Exists(storePath), "a failed-but-tracked copy is cleared at expiry and the record removed");
            ca.StopForDisable();

            // 4. A successful copy writes the record; stopping hosting clears the clipboard and removes it.
            var hb = new LoadingMockWorldBridge { Loading = false };
            var ha = StartAdapter(hb);
            SelectMenu(ha, 1);
            ha.HandleMenuKeyCode(0x0D);
            ha.StepEveryFrame(); ha.StepEveryFrame();
            Check(hb.ClipboardCalls == 1 && hb.ClipboardText.Length > 0 && File.Exists(storePath), "a successful copy records its expiry");
            ha.StopForDisable();
            Check(hb.ClipboardText == string.Empty && !File.Exists(storePath), "stopping hosting clears the invite and removes the record");

            // 5. Crash recovery: the previous session left a record; the next start clears the clipboard only if it still
            //    holds exactly that text (hash match) and only after the expiry time.
            string leftover = "GTAxSkate coop invite lan:192.168.1.20:4499 " + CoopInvite.GeneratePassword();
            InviteExpiryStore.Write(leftover, DateTime.UtcNow.AddMinutes(-1));
            var sb2 = new LoadingMockWorldBridge { Loading = false, ClipboardText = leftover };
            var sa2 = StartAdapter(sb2);
            sa2.StepEveryFrame(); sa2.StepEveryFrame(); sa2.StepEveryFrame();
            Check(sb2.ClipboardText == string.Empty && !File.Exists(storePath), "an expired leftover invite is cleared after a crash");
            sa2.StopForDisable();

            InviteExpiryStore.Write(leftover, DateTime.UtcNow.AddMinutes(-1));
            var sb3 = new LoadingMockWorldBridge { Loading = false, ClipboardText = "something the user copied since" };
            var sa3 = StartAdapter(sb3);
            sa3.StepEveryFrame(); sa3.StepEveryFrame(); sa3.StepEveryFrame();
            Check(sb3.ClipboardText == "something the user copied since" && sb3.ClearAttempts == 0 && !File.Exists(storePath), "a changed clipboard is never touched; the stale record is dropped");
            sa3.StopForDisable();

            InviteExpiryStore.Write(leftover, DateTime.UtcNow.AddMilliseconds(500));
            var sb4 = new LoadingMockWorldBridge { Loading = false, ClipboardText = leftover };
            var sa4 = StartAdapter(sb4);
            sa4.StepEveryFrame(); sa4.StepEveryFrame();
            Check(sb4.ClipboardText == leftover && File.Exists(storePath), "before the expiry time nothing is cleared");
            System.Threading.Thread.Sleep(700);
            sa4.StepEveryFrame(); sa4.StepEveryFrame(); sa4.StepEveryFrame();
            Check(sb4.ClipboardText == string.Empty && !File.Exists(storePath), "once the time has passed the matching clipboard is cleared");
            sa4.StopForDisable();

            InviteExpiryStore.Write(leftover, DateTime.UtcNow.AddMinutes(-1));
            var sb5 = new LoadingMockWorldBridge { Loading = false, ClipboardText = leftover + " " };
            var sa5 = StartAdapter(sb5);
            sa5.StepEveryFrame(); sa5.StepEveryFrame(); sa5.StepEveryFrame();
            Check(sb5.ClipboardText == leftover + " ", "a clipboard that differs only by trailing whitespace is a different clipboard: left alone");
            sa5.StopForDisable();
        }
        finally
        {
            InviteExpiryStore.DirectoryOverride = null;
            CoopRuntimeControl.ResetForTests();
            try { Directory.Delete(dir, true); } catch { }
        }
    }

    // Round 3 (Copilot round-2 review): host actions never touch a server that is connecting or connected, and the
    // persisted clipboard-expiry record is deleted only after the clear really finished.
    private static void RunRound3AdapterTests()
    {
        string dir = NewTempDir();
        InviteExpiryStore.DirectoryOverride = dir;
        string storePath = Path.Combine(dir, InviteExpiryStore.FileName);
        try
        {
            // 1. While a connect is pending, both host rows are disabled and nothing restarts the server.
            var pb = new LoadingMockWorldBridge { Loading = false };
            var pa = StartAdapter(pb);
            SelectMenu(pa, 2);
            pa.HandleMenuKeyCode(0x0D);                      // Host online: server started, local connect pending
            Check(pb.EnsureHostCalls == 1 && pb.ConnectCalls == 1, "round 3 setup: one server start and one connect");
            Check(HostRowsDisabled(pb), "host rows are disabled while a connect is pending: " + RowDump(pb));
            pa.HandleMenuKeyCode(0x26); pa.HandleMenuKeyCode(0x26); pa.HandleMenuKeyCode(0x26); pa.HandleMenuKeyCode(0x26); pa.HandleMenuKeyCode(0x26);
            pa.HandleMenuKeyCode(0x28);                      // Host private session row
            pa.HandleMenuKeyCode(0x0D);
            pa.HandleMenuKeyCode(0x28);                      // Host online row
            pa.HandleMenuKeyCode(0x0D);
            Check(pb.EnsureHostCalls == 1 && pb.BeginOnlineCalls == 1 && pb.ConnectCalls == 1, "no second host action runs while the first connect is pending");
            // connected to the server but not yet authenticated
            RageCoop.Client.Scripting.API.IsOnServer = true;
            pa.StepEveryFrame();
            Check(HostRowsDisabled(pb), "host rows stay disabled while connected but not yet authenticated: " + RowDump(pb));
            Check(pb.EnsureHostCalls == 1, "still only one server start");
            pa.StopForDisable();

            // 2. The expiry record outlives the clear until the clear has reported completion.
            var cb = new LoadingMockWorldBridge { Loading = false, ClipboardPendingPolls = 12 };   // a tick polls a few times (StepEveryFrame also runs the event tick)
            var ca = StartAdapter(cb);
            SelectMenu(ca, 1);
            ca.HandleMenuKeyCode(0x0D);
            for (int i = 0; i < 10; i++) ca.StepEveryFrame();
            Check(cb.ClipboardText.Length > 0 && File.Exists(storePath), "round 3 setup: invite copied, record written");
            ca.TestClockOffsetMs = 11 * 60 * 1000;
            ca.StepEveryFrame();
            Check(cb.ClearAttempts == 1 && File.Exists(storePath), "the record is still there right after the clear was started (not yet confirmed): attempts=" + cb.ClearAttempts + " exists=" + File.Exists(storePath));
            ca.StepEveryFrame();
            Check(File.Exists(storePath), "and while the clear is still pending");
            for (int i = 0; i < 8; i++) ca.StepEveryFrame();
            Check(!File.Exists(storePath) && cb.ClipboardText == string.Empty, "the record is deleted once the clear reported completion");
            ca.StopForDisable();

            // 3. A clear that fails keeps the record for the next start.
            var fb = new LoadingMockWorldBridge { Loading = false, ClipboardClearFails = true };
            var fa = StartAdapter(fb);
            SelectMenu(fa, 1);
            fa.HandleMenuKeyCode(0x0D);
            for (int i = 0; i < 4; i++) fa.StepEveryFrame();
            Check(File.Exists(storePath), "round 3 setup: record written");
            fa.TestClockOffsetMs = 11 * 60 * 1000;
            for (int i = 0; i < 4; i++) fa.StepEveryFrame();
            Check(fb.ClearAttempts >= 1 && File.Exists(storePath), "a failed clear keeps the record so the next start retries");
            fa.StopForDisable();
            InviteExpiryStore.Delete();

            // 4. Stopping hosting with a clear still in flight leaves the record (the next start finishes the job).
            var sb = new LoadingMockWorldBridge { Loading = false, ClipboardPendingPolls = 6 };
            var sa = StartAdapter(sb);
            SelectMenu(sa, 1);
            sa.HandleMenuKeyCode(0x0D);
            for (int i = 0; i < 8; i++) sa.StepEveryFrame();
            Check(File.Exists(storePath), "round 3 setup: record written (stop)");
            sa.StopForDisable();
            Check(sb.ClipboardText == string.Empty && File.Exists(storePath), "stop: clipboard cleared, record kept until the clear is confirmed");
        }
        finally
        {
            InviteExpiryStore.DirectoryOverride = null;
            CoopRuntimeControl.ResetForTests();
            try { Directory.Delete(dir, true); } catch { }
        }
    }

    private static bool HostRowsDisabled(LoadingMockWorldBridge b)
    {
        if (b.LastOverlay == null) return false;
        bool any = false;
        foreach (var item in b.LastOverlay.Items)
            if (item.Action == GTAxSkate.Coop.CoopMenuAction.HostPrivateSession || item.Action == GTAxSkate.Coop.CoopMenuAction.HostOnline)
            { any = true; if (item.Enabled) return false; }
        return any;
    }

    private static string RowDump(LoadingMockWorldBridge b)
    {
        if (b.LastOverlay == null) return "(no overlay)";
        var sb = new System.Text.StringBuilder();
        foreach (var item in b.LastOverlay.Items) sb.Append(item.Action).Append(item.Enabled ? "+ " : "- ");
        return sb.ToString();
    }

    private static void RunFriendSessionWireTests()
    {
        string friendKey = "SkateFriends-Secret-99";
        var api = new RageCoop.Server.Scripting.API();
        var server = new ServerAdapter { API = api };
        server.ConfigureFriendSecurity(
            GTAxSkate.Coop.ServerSecurityConfig.DefaultPrivateFriends(friendKey), "100.88.12.34");
        server.OnStart();
        Check(!api.Events.Admit("203.0.113.44", "wrong password"), "stock transport denies a public source with the wrong password");
        Check(api.Events.Admit("203.0.113.44", friendKey), "stock transport admits a public source with the matching password (internet lobby)");
        Check(!api.Events.Admit("100.88.12.34", "wrong password"), "stock transport denies wrong password");
        Check(api.Events.Admit("100.88.12.34", friendKey), "stock transport admits private friend with matching password");

        var hostConn = new RageCoop.Server.Client { Player = new RageCoop.Server.Player { ID = -11 } };
        var peerConn = new RageCoop.Server.Client { Player = new RageCoop.Server.Player { ID = 22 } };
        api.Clients[-11] = hostConn;
        api.Clients[22] = peerConn;

        RageCoop.Client.Scripting.API.Reset();
        var client = new ClientAdapter { DiagnosticFixtureOnly = true };
        var wireBridge = new LoadingMockWorldBridge { Loading = false };   // records everything the player could see
        client.SetWorldBridge(wireBridge);
        client.OnStart();
        RageCoop.Client.Scripting.API.Actions.Dequeue()();
        RageCoop.Client.Scripting.API.Events.Tick();
        RageCoop.Client.Scripting.API.Sent.Dequeue(); // consume initial startup diagnostic Hello
        Check(client.ConnectionBanner.Contains("[SOLO]"), "starts cleanly in Solo status");

        // 1. Public server browser & unpinned public IP blocked before sending any wire messages.
        GTAxSkate.Coop.PolicyDecision dec;
        Check(!client.BeginTrustedSessionConnect(
            new GTAxSkate.Coop.ClientConnectRequest(
                GTAxSkate.Coop.LobbyDiscoverySource.PublicServerBrowser, "192.168.1.5", 4499, true, "RandomLobby", friendKey),
            out dec), "adapter blocks random server browser");
        Check(RageCoop.Client.Scripting.API.Sent.Count == 0 && client.ConnectionBanner.Contains("BLOCKED"),
            "blocked lobby emits explicit BLOCKED status and zero traffic");

        Check(!client.BeginTrustedSessionConnect(
            new GTAxSkate.Coop.ClientConnectRequest(
                GTAxSkate.Coop.LobbyDiscoverySource.DirectFriendInvite, "203.0.113.44", 4499, false, "PublicHost", friendKey),
            out dec), "adapter blocks unpinned public IP");

        // 2. Host cannot call HostFriendSession before mutual auth completes.
        Check(!client.HostFriendSession(GTAxSkate.Coop.SkateWorldZone.SkateSchoolIsland, GTAxSkate.Coop.SkateMode.FreeSkate, "PreAuth"),
            "fail closed before mutual friend auth");

        // 3. Valid VPN friend invite completes mutual HMAC challenge-response.
        Check(client.BeginTrustedSessionConnect(
            new GTAxSkate.Coop.ClientConnectRequest(
                GTAxSkate.Coop.LobbyDiscoverySource.DirectFriendInvite, "100.88.12.34", 4499, false, "VPN host", friendKey),
            out dec), "start mutual friend connect over Tailscale VPN");
        Check(RageCoop.Client.Scripting.API.Sent.Count == 1, "AuthInit sent");
        var authInit = RageCoop.Client.Scripting.API.Sent.Dequeue();

        api.Deliver(hostConn, authInit);
        Check(hostConn.Sent.Count == 1, "server sends mutual AuthChallenge");
        var authChallenge = hostConn.Sent.Dequeue();
        // 2026-11-16 evening: a resent AuthInit gets the SAME challenge, and a second challenge is
        // ignored, so a late resend can no longer invalidate the proof being answered.
        api.Deliver(hostConn, authInit);
        Check(hostConn.Sent.Count == 1, "server answers a resent AuthInit");
        var dupChallenge = hostConn.Sent.Dequeue();
        Check(((byte[])dupChallenge.Args[2]).SequenceEqual((byte[])authChallenge.Args[2]),
            "resent AuthInit gets the same server nonce");
        RageCoop.Client.Scripting.API.Deliver(authChallenge);
        RageCoop.Client.Scripting.API.Deliver(dupChallenge);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count == 1, "client sends mutual AuthProof after verifying server proof");
        var authProof = RageCoop.Client.Scripting.API.Sent.Dequeue();
        api.Deliver(hostConn, authProof);
        Check(hostConn.Sent.Count == 1, "server sends FriendAuthAccepted");
        var authAccepted = hostConn.Sent.Dequeue();
        RageCoop.Client.Scripting.API.Deliver(authAccepted);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(client.SecurityTracker.IsConnectedToPlayerSession &&
            client.ConnectionBanner.Contains("[CONNECTED - PRIVATE COOP]") &&
            client.ConnectionBanner.Contains("VPN host"),
            "client reports explicit verified friend connection");
        Check(!client.ConnectionBanner.Contains("100.88.12.34") && wireBridge.Notes.Exists(n => n.StartsWith("Friends Coop Verified!") && n.Contains("VPN host") && n.Contains("[VPN]")),
            "the verified notice names the host by label and trust tag only");

        // Join Friend Lobby (paste code): the session code is read from the clipboard, never typed or shown; it is cleared after use.
        wireBridge.ClipboardText = " abc234 ";
        client.HandleMenuKeyCode(0x7A);
        client.HandleMenuKeyCode(0x28);
        client.HandleMenuKeyCode(0x0D);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(wireBridge.ClipboardReads == 1 && RageCoop.Client.Scripting.API.Sent.Count == 1, "the pasted session code is sent as a Join request");
        var pasteJoin = RageCoop.Client.Scripting.API.Sent.Dequeue();
        Check(pasteJoin.Hash == RageCoop.Core.Scripting.CustomEvents.Hash(SessionProtocol.SessionJoin) && (string)pasteJoin.Args[1] == "ABC234" &&
            wireBridge.ClipboardText == string.Empty, "the code is normalised and the clipboard cleared");
        wireBridge.ClipboardText = "not a code!";
        client.HandleMenuKeyCode(0x0D);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(RageCoop.Client.Scripting.API.Sent.Count == 0 && wireBridge.Notes.Exists(n => n.StartsWith("Copy the join code your friend sent you")),
            "an invalid clipboard asks for the join code again");
        client.HandleMenuKeyCode(0x26);
        client.HandleMenuKeyCode(0x1B);

        // 4. Host creates a Skate School Island session -> receives live snapshot with JoinCode & board/score sync.
        Check(client.HostFriendSession(GTAxSkate.Coop.SkateWorldZone.SkateSchoolIsland,
            GTAxSkate.Coop.SkateMode.SpotBattle, "Coach Frank Co-op"), "send HostFriendSession");
        var hostMsg = RageCoop.Client.Scripting.API.Sent.Dequeue();
        api.Deliver(hostConn, hostMsg);
        Check(hostConn.Sent.Count == 1, "server broadcasts SessionSnapshot to host");
        var snapMsg1 = hostConn.Sent.Dequeue();
        RageCoop.Client.Scripting.API.Deliver(snapMsg1);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(client.LatestSnapshot != null && client.LatestSnapshot.JoinCode.Length == 6 &&
            client.LatestSnapshot.Zone == GTAxSkate.Coop.SkateWorldZone.SkateSchoolIsland,
            "client receives decoded session snapshot + JoinCode");
        string sessionCode = client.LatestSnapshot.JoinCode;

        // The host copies the join code (hidden); no row, banner or overlay line shows it.
        client.HandleMenuKeyCode(0x7A);
        client.HandleMenuKeyCode(0x28);
        client.HandleMenuKeyCode(0x0D);
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(wireBridge.LastClipboard == sessionCode && wireBridge.Notes.Contains("Join code copied to the clipboard (hidden)."), "the host's join code goes to the clipboard, hidden");
        client.HandleMenuKeyCode(0x26);
        client.HandleMenuKeyCode(0x1B);

        // Peer with wrong friend key fails server auth cleanly and cannot join.
        var badNonce = Guid.NewGuid().ToByteArray();
        api.Deliver(peerConn, new Message(authInit.Hash, new object[] { 1, badNonce }));
        var peerChal = peerConn.Sent.Dequeue();
        byte[] badProof = GTAxSkate.Coop.CoopSecurityPolicy.ComputeMutualProof("WrongKey-123456", badNonce, (byte[])peerChal.Args[2], "CLI");
        api.Deliver(peerConn, new Message(authProof.Hash, new object[] { 1, badNonce, peerChal.Args[2], badProof }));
        var peerReject = peerConn.Sent.Dequeue();
        Check(((string)peerReject.Args[1]).Contains("verification failed"), "server rejects wrong friend key");

        // Unauthenticated peer cannot join session.
        api.Deliver(peerConn, new Message(RageCoop.Core.Scripting.CustomEvents.Hash("GTAxSkate.Coop.Session.v1.Join"),
            new object[] { 1, client.LatestSnapshot.JoinCode }));
        Check(hostConn.Sent.Count == 0, "unauthenticated peer join blocked cleanly");

        // Valid peer joins -> host snapshot reflects Skaters: 2 cleanly.
        var goodNonce = Guid.NewGuid().ToByteArray();
        api.Deliver(peerConn, new Message(authInit.Hash, new object[] { 1, goodNonce }));
        var peerChal2 = peerConn.Sent.Dequeue();
        byte[] goodProof = GTAxSkate.Coop.CoopSecurityPolicy.ComputeMutualProof(friendKey, goodNonce, (byte[])peerChal2.Args[2], "CLI");
        api.Deliver(peerConn, new Message(authProof.Hash, new object[] { 1, goodNonce, peerChal2.Args[2], goodProof }));
        peerConn.Sent.Dequeue(); // AuthAccepted
        api.Deliver(peerConn, new Message(RageCoop.Core.Scripting.CustomEvents.Hash("GTAxSkate.Coop.Session.v1.Join"),
            new object[] { 1, client.LatestSnapshot.JoinCode }));
        Check(hostConn.Sent.Count == 1 && peerConn.Sent.Count == 1, "both friends receive updated SessionSnapshot");
        RageCoop.Client.Scripting.API.Deliver(hostConn.Sent.Dequeue());
        peerConn.Sent.Dequeue();
        RageCoop.Client.Scripting.API.Events.Tick();
        Check(client.ConnectionBanner.Contains("Skaters: 2") && client.LatestSnapshot.Peers.Length == 2,
            "banner and snapshot show 2 friends connected");

        var world = new GTAxSkate.Coop.HostWorldSyncState(1, 12, 30, false, 1,
            true, false, false, 0, 0, 0, 0, 10, 20, 30, true, 11, 22, 33, 1, 5, true);
        byte[] worldBytes = SessionProtocol.EncodeWorldSync(world);
        int worldHash = RageCoop.Core.Scripting.CustomEvents.Hash(SessionProtocol.SessionHostWorldSync);
        api.Deliver(peerConn, new Message(worldHash, new object[] { 1, worldBytes }));
        Check(hostConn.Sent.Count == 0 && peerConn.Sent.Count == 0, "non-host world state is rejected");
        Check(client.SubmitHostWorldState(world), "host submits bounded world state");
        api.Deliver(hostConn, RageCoop.Client.Scripting.API.Sent.Dequeue());
        Check(hostConn.Sent.Count == 0 && peerConn.Sent.Count == 1, "host world state is relayed only to session peers");
        GTAxSkate.Coop.HostWorldSyncState decodedWorld;
        Check(SessionProtocol.TryDecodeWorldSync((byte[])peerConn.Sent.Dequeue().Args[1], out decodedWorld) &&
            decodedWorld.MissionActive && decodedWorld.ObjectiveY == 22, "relayed world packet preserves mission objective");
        Array.Resize(ref worldBytes, worldBytes.Length + 1);
        Check(!SessionProtocol.TryDecodeWorldSync(worldBytes, out decodedWorld), "world codec rejects trailing data");

        // Mod-parity check over RAGECOOP wire: matching interfering + cosmetic mods -> clean; differing mods -> warnings surfaced cleanly.
        int modsReportHash = RageCoop.Core.Scripting.CustomEvents.Hash(SessionProtocol.SessionModsReport);
        Check(client.SubmitLocalModManifest(new[]
        {
            new GTAxSkate.Coop.ModEntry("dlcpacks/skate_school/dlc.rpf", "v01", GTAxSkate.Coop.ModImpactClass.CriticalMapOrPhysics),
            new GTAxSkate.Coop.ModEntry("scripts/RealismTraffic.dll", "rt-v1", GTAxSkate.Coop.ModImpactClass.GameplayInterference),
            new GTAxSkate.Coop.ModEntry("SkateVRuntime.dll", "aa11", GTAxSkate.Coop.ModImpactClass.CriticalMapOrPhysics),
            new GTAxSkate.Coop.ModEntry("ReShade.ini", "preset1", GTAxSkate.Coop.ModImpactClass.CosmeticLocalOnly)
        }), "authenticated client sends local mod manifest");
        api.Deliver(hostConn, RageCoop.Client.Scripting.API.Sent.Dequeue());

        byte[] matchingPeerMods = SessionProtocol.EncodeModsManifest(new[]
        {
            new GTAxSkate.Coop.ModEntry("dlcpacks/skate_school/dlc.rpf", "v01", GTAxSkate.Coop.ModImpactClass.CriticalMapOrPhysics),
            new GTAxSkate.Coop.ModEntry("scripts/RealismTraffic.dll", "rt-v1", GTAxSkate.Coop.ModImpactClass.GameplayInterference),
            new GTAxSkate.Coop.ModEntry("SkateVRuntime.dll", "aa11", GTAxSkate.Coop.ModImpactClass.CriticalMapOrPhysics),
            new GTAxSkate.Coop.ModEntry("dxgi.dll", "enb-diff", GTAxSkate.Coop.ModImpactClass.CosmeticLocalOnly)
        });
        api.Deliver(peerConn, new Message(modsReportHash, new object[] { 1, matchingPeerMods }));
        Check(hostConn.Sent.Count == 0 && peerConn.Sent.Count == 0, "matching interfering + different cosmetic mods emit no warnings");

        byte[] conflictingPeerMods = SessionProtocol.EncodeModsManifest(new[]
        {
            new GTAxSkate.Coop.ModEntry("SkateVRuntime.dll", "bb22", GTAxSkate.Coop.ModImpactClass.CriticalMapOrPhysics),
            new GTAxSkate.Coop.ModEntry("scripts/CrazyTraffic.dll", "10", GTAxSkate.Coop.ModImpactClass.GameplayInterference)
        });
        api.Deliver(peerConn, new Message(modsReportHash, new object[] { 1, conflictingPeerMods }));
        Check(hostConn.Sent.Count == 1 && peerConn.Sent.Count == 1, "conflicting mods send warning to both friends");
        RageCoop.Client.Scripting.API.Deliver(hostConn.Sent.Dequeue());
        RageCoop.Client.Scripting.API.Events.Tick();
        string[] warnings = client.LatestModWarnings;
        Check(warnings.Length == 1 && warnings[0].Contains("HIGH DESYNC RISK") &&
            warnings[0].Contains("SkateVRuntime.dll") && warnings[0].Contains("dlcpacks/skate_school/dlc.rpf") &&
            warnings[0].Contains("CrazyTraffic.dll") && !warnings[0].Contains("ReShade.ini"),
            "client surfaces actionable mod interference warnings without cosmetic noise");

        // AutoLoadAndScanFromGtaFolder loads FriendsCoopClient.json and scans mods from mock GTA folder.
        string mockGta = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "sk8_mock_gta_" + Guid.NewGuid().ToString("N"));
        try
        {
            string coopCfgDir = System.IO.Path.Combine(System.IO.Path.Combine(mockGta, "scripts"), "RageCoop");
            System.IO.Directory.CreateDirectory(coopCfgDir);
            System.IO.File.WriteAllText(System.IO.Path.Combine(mockGta, "SkateVRuntime.dll"), "runtime");
            System.IO.File.WriteAllText(System.IO.Path.Combine(coopCfgDir, "FriendsCoopClient.json"),
                "{\"BlockPublicServerBrowser\":true,\"TrustedHostLabel\":\"VPN-Host\",\"TrustedHostIp\":\"100.88.12.34\",\"Port\":4499,\"FriendKey\":\"" + friendKey + "\"}");
            Check(client.AutoLoadAndScanFromGtaFolder(mockGta) &&
                client.ConfiguredConnectRequest != null &&
                client.ConfiguredConnectRequest.EndpointAddress == "100.88.12.34",
                "adapter loads hardened client JSON & auto-scans GTA mods cleanly");
        }
        finally
        {
            try { if (System.IO.Directory.Exists(mockGta)) System.IO.Directory.Delete(mockGta, true); } catch { }
        }

        // HostShopSync: non-host rejected; host relayed to peer and decoded cleanly.
        peerConn.Sent.Clear();
        RageCoop.Client.Scripting.API.Sent.Clear();
        var shop = new GTAxSkate.Coop.HostShopSyncState(
            1, 2500, "Sprunk-Pro", 0x5208A519u, 27, 0, "SK8COOP", 0x1B06D571u, "Crew bought Sprunk deck");
        byte[] shopBytes = SessionProtocol.EncodeShopSync(shop);
        int shopHash = RageCoop.Core.Scripting.CustomEvents.Hash(SessionProtocol.SessionHostShopSync);
        api.Deliver(peerConn, new Message(shopHash, new object[] { 1, shopBytes }));
        Check(hostConn.Sent.Count == 0 && peerConn.Sent.Count == 0, "non-host shop state is rejected");
        Check(client.SubmitHostShopState(shop), "host submits bounded shop state");
        api.Deliver(hostConn, RageCoop.Client.Scripting.API.Sent.Dequeue());
        Check(hostConn.Sent.Count == 0 && peerConn.Sent.Count == 1, "host shop state is relayed only to session peers");
        GTAxSkate.Coop.HostShopSyncState decodedShop;
        Check(SessionProtocol.TryDecodeShopSync((byte[])peerConn.Sent.Dequeue().Args[1], out decodedShop) &&
            decodedShop.EquippedBoardProfile == "Sprunk-Pro" && decodedShop.HostCash == 2500,
            "relayed shop packet preserves board profile and cash");

        // RagdollSync: sender slot is enforced by server/decoder; invalid/spoofed slot ignored.
        var rag = new GTAxSkate.Coop.RagdollSyncState(999, true, 12.5f, -34.0f, 5.2f, 3.1f, -1.2f, 4.0f, 1200);
        Check(client.SubmitRagdollState(rag), "client submits bounded ragdoll state");
        api.Deliver(hostConn, RageCoop.Client.Scripting.API.Sent.Dequeue());
        Check(peerConn.Sent.Count == 1, "ragdoll event relayed to peer");
        Message ragMsg = peerConn.Sent.Dequeue();
        GTAxSkate.Coop.RagdollSyncState decodedRag;
        Check(SessionProtocol.TryDecodeRagdollSync((byte[])ragMsg.Args[2], (int)ragMsg.Args[1], out decodedRag) &&
            decodedRag.SourceSlot == 1 && decodedRag.IsRagdoll,
            "ragdoll packet enforces authenticated sender slot (1) instead of spoofed 999");

        // F11 in-game menu: toggle open, navigate, and close cleanly.
        Check(!client.MenuState.IsOpen, "F11 menu starts closed");
        Check(client.HandleMenuKeyCode(0x7A) && client.MenuState.IsOpen, "F11 opens in-game coop menu");
        Check(client.HandleMenuKeyCode(0x28) && client.MenuState.SelectedIndex == 1, "Down arrow moves menu selection");
        Check(client.HandleMenuKeyCode(0x1B) && !client.MenuState.IsOpen, "Escape closes in-game coop menu");

        AssertNoLeak(wireBridge, "friend session over the wire", friendKey, sessionCode, "100.88.12.34");
        Check(wireBridge.AllShown.Count > 20, "the leak scan saw many frames (" + wireBridge.AllShown.Count + ")");
        client.OnStop();
        server.OnStop();
    }
}
