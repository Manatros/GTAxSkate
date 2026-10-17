using System;
using System.Collections.Generic;
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
    public sealed class ServerEvents
    {
        public event EventHandler<RageCoop.Server.Client> OnPlayerDisconnected;
        public void Disconnect(RageCoop.Server.Client client) { if(OnPlayerDisconnected!=null) OnPlayerDisconnected(this,client); }
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
        public readonly TestLogger Logger = new TestLogger();
        public abstract void OnStart(); public abstract void OnStop();
    }
    public static class API
    {
        public static bool IsOnServer = true;
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
    public static int Main()
    {
        try { Run();Console.WriteLine("PASS: "+checks+" actual adapter event checks; fixture transport only, no live server/gameplay.");return 0; }
        catch(Exception ex){Console.Error.WriteLine(ex);return 1;}
    }
    private static void Run()
    {
        var api=new RageCoop.Server.Scripting.API();
        var server=new ServerAdapter{API=api};server.OnStart();
        var owner=new RageCoop.Server.Client{Player=new RageCoop.Server.Player{ID=-11}};
        var other=new RageCoop.Server.Client{Player=new RageCoop.Server.Player{ID=22}};
        api.Clients[-11]=owner;api.Clients[22]=other;
        RageCoop.Client.Scripting.API.Reset();
        var client=new ClientAdapter();client.OnStart();
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
    }
}
