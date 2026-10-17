using System;
using System.Collections.Generic;

namespace GTAxSkate.Coop
{
    public sealed class AcceptedPose
    {
        public readonly int Source;
        public readonly PoseFrame Frame;
        public readonly long ReceivedAtMs;
        internal AcceptedPose(int source,PoseFrame frame,long now){Source=source;Frame=frame;ReceivedAtMs=now;}
    }
    // The adapter must pass the source from the authenticated transport's
    // CustomEventReceivedArgs.Client, NEVER a value supplied in Args/payload.
    // No client-supplied source ID exists in the wire format.
    public sealed class ServerPoseSessions
    {
        public const int MaxPeers=64, SessionIdleMs=30000;
        private sealed class Peer
        { public Guid Nonce;public ulong Sequence;public long LastSeen,TokenAt;public double Tokens=4;public SkeletonLayout Layout; }
        private readonly Dictionary<int,Peer> peers=new Dictionary<int,Peer>();
        private readonly object gate=new object();
        public Guid BeginAuthenticatedSession(int authenticatedSource,SkeletonLayout expectedLayout,long now)
        {
            if(authenticatedSource<=0||now<0||expectedLayout==null)throw new ArgumentException("Authenticated source/layout/time");
            lock(gate)
            {
                if(!peers.ContainsKey(authenticatedSource)&&peers.Count>=MaxPeers)throw new InvalidOperationException("Pose peer limit");
                var nonce=Guid.NewGuid();peers[authenticatedSource]=new Peer{Nonce=nonce,LastSeen=now,TokenAt=now,Layout=expectedLayout};return nonce;
            }
        }
        public bool TryAccept(int authenticatedSource,byte[] bytes,long now,out AcceptedPose accepted)
        {
            accepted=null;
            lock(gate)
            {
                Peer p;
                if(!peers.TryGetValue(authenticatedSource,out p)||now<p.LastSeen||now<p.TokenAt||now-p.LastSeen>SessionIdleMs)return false;
                p.Tokens=Math.Min(4,p.Tokens+(now-p.TokenAt)*0.020);p.TokenAt=now;
                if(p.Tokens<1)return false;p.Tokens-=1;
                PoseFrame frame;
                if(!PoseCodec.TryDecode(bytes,out frame)||frame.Session!=p.Nonce||frame.Sequence<=p.Sequence||!p.Layout.Matches(frame))return false;
                p.Sequence=frame.Sequence;p.LastSeen=now;
                accepted=new AcceptedPose(authenticatedSource,frame,now);return true;
            }
        }
        public void Disconnect(int authenticatedSource){lock(gate){peers.Remove(authenticatedSource);}}
        public void Expire(long now)
        {
            lock(gate)
            {
                var old=new List<int>();foreach(var pair in peers)if(now<pair.Value.LastSeen||now-pair.Value.LastSeen>SessionIdleMs)old.Add(pair.Key);
                foreach(int id in old)peers.Remove(id);
            }
        }
        public void Clear(){lock(gate){peers.Clear();}}
    }
    // Latest-only receive state. Call session registration only for a trusted
    // server announcement, never a peer's event. It does not authenticate the
    // transport itself; that check belongs in the separately reviewed adapter.
    public sealed class LatestRemotePoses
    {
        public const int PoseTtlMs=500;
        private sealed class Remote { public Guid Nonce;public ulong Sequence;public AcceptedPose Latest;public SkeletonLayout Layout; }
        private readonly Dictionary<int,Remote> peers=new Dictionary<int,Remote>();
        private readonly object gate=new object();
        public void BeginTrustedServerSession(int source,Guid nonce,SkeletonLayout expectedLayout)
        {
            if(source<=0||nonce==Guid.Empty||expectedLayout==null)throw new ArgumentException("Source/session/layout");
            lock(gate)
            {
                if(!peers.ContainsKey(source)&&peers.Count>=ServerPoseSessions.MaxPeers)throw new InvalidOperationException("Pose peer limit");
                peers[source]=new Remote{Nonce=nonce,Layout=expectedLayout};
            }
        }
        public bool TryApplyTrustedServerFrame(int source,byte[] bytes,long now)
        {
            lock(gate)
            {
                Remote r;PoseFrame p;
                if(now<0||!peers.TryGetValue(source,out r)||!PoseCodec.TryDecode(bytes,out p)||p.Session!=r.Nonce||p.Sequence<=r.Sequence||!r.Layout.Matches(p))return false;
                if(r.Latest!=null&&now<r.Latest.ReceivedAtMs)return false;
                r.Sequence=p.Sequence;r.Latest=new AcceptedPose(source,p,now);return true;
            }
        }
        public bool TryLatest(int source,long now,out AcceptedPose pose)
        {
            lock(gate)
            {
                pose=null;Remote r;
                if(!peers.TryGetValue(source,out r)||r.Latest==null)return false;
                if(now<r.Latest.ReceivedAtMs||now-r.Latest.ReceivedAtMs>PoseTtlMs){r.Latest=null;return false;}
                pose=r.Latest;return true;
            }
        }
        public void Disconnect(int source){lock(gate){peers.Remove(source);}}
        public void Clear(){lock(gate){peers.Clear();}}
    }
}
