using System;
using GTAxSkate.Coop;

internal static class PoseProtocolTests
{
    private static int checks;
    private static void Check(bool value,string text){checks++;if(!value)throw new Exception(text);}
    private static Matrix12 Rotate(float a,float x){return new Matrix12((float)Math.Cos(a),(float)Math.Sin(a),0,-(float)Math.Sin(a),(float)Math.Cos(a),0,0,0,1,x,0,0);}
    private static Matrix12[] Board(){var b=new Matrix12[7];for(int i=0;i<7;i++)b[i]=Matrix12.Identity;return b;}
    private static BonePose[] Bones(int n){var b=new BonePose[n];for(int i=0;i<n;i++)b[i]=new BonePose((ushort)i,(short)(i==0?-1:0),Matrix12.Identity);return b;}
    private static PoseFrame Frame(Guid nonce,ulong seq,int count=3,uint model=123)
    {return new PoseFrame(nonce,seq,model,true,Rotate(0.2f,100),Rotate(0.3f,100),Bones(count),Board());}
    private static bool Rejected(byte[] b){PoseFrame p;return !PoseCodec.TryDecode(b,out p);}
    public static int Main()
    {
        try{Run();Console.WriteLine("PASS: "+checks+" checks; exact roundtrip/bounds/layout/source/session/replay/rate/latest-state tests. No live integration tested.");return 0;}
        catch(Exception e){Console.Error.WriteLine(e.Message);return 1;}
    }
    private static void Run()
    {
        Guid session=Guid.NewGuid();PoseFrame decoded;
        var original=Frame(session,1);byte[] bytes=PoseCodec.Encode(original);
        Check(bytes.Length==PoseCodec.HeaderBytes+3*52,"wire size");
        Check(PoseCodec.TryDecode(bytes,out decoded),"roundtrip decode");
        Check(Convert.ToBase64String(bytes)==Convert.ToBase64String(PoseCodec.Encode(decoded)),"bit-exact float roundtrip");
        Check(decoded.Session==session&&decoded.Sequence==1&&decoded.PedModel==123,"header");
        var max=PoseCodec.Encode(Frame(session,1,512));Check(max.Length==PoseCodec.MaxPacketBytes&&max.Length==27096,"512 bone bound");
        Check(!Rejected(max),"maximum valid frame");
        Check(Rejected(null)&&Rejected(new byte[1])&&Rejected(new byte[PoseCodec.MaxPacketBytes+1]),"size rejection");
        var shortBytes=new byte[bytes.Length-1];Array.Copy(bytes,shortBytes,shortBytes.Length);Check(Rejected(shortBytes),"truncated");
        var trailing=new byte[bytes.Length+1];Array.Copy(bytes,trailing,bytes.Length);Check(Rejected(trailing),"trailing");
        var version=(byte[])bytes.Clone();version[4]=99;Check(Rejected(version),"unknown version");
        var flags=(byte[])bytes.Clone();flags[6]=128;Check(Rejected(flags),"unknown flags");
        var reserved=(byte[])bytes.Clone();reserved[38]=1;Check(Rejected(reserved),"reserved field");
        var nan=(byte[])bytes.Clone();Array.Copy(BitConverter.GetBytes(float.NaN),0,nan,40,4);Check(Rejected(nan),"NaN");
        var infinity=(byte[])bytes.Clone();Array.Copy(BitConverter.GetBytes(float.PositiveInfinity),0,infinity,40,4);Check(Rejected(infinity),"infinity");
        var duplicate=(byte[])bytes.Clone();duplicate[PoseCodec.HeaderBytes+52]=0;duplicate[PoseCodec.HeaderBytes+53]=0;Check(Rejected(duplicate),"duplicate tags");
        Check(!new Matrix12(-1,0,0,0,1,0,0,0,1,0,0,0).IsRigid(5),"reflection");
        Check(!new Matrix12(1,0,0,0,2,0,0,0,1,0,0,0).IsRigid(5),"scale");
        Check(!new Matrix12(1,0,0,0,1,0,0,0,1,6,0,0).IsRigid(5),"local displacement");
        Check(!Rotate(0,32769).IsRigid(32768),"world displacement");
        var cyclic=Bones(3);cyclic[1]=new BonePose(1,2,Matrix12.Identity);cyclic[2]=new BonePose(2,1,Matrix12.Identity);
        Check(!PoseCodec.Valid(new PoseFrame(session,1,123,true,Matrix12.Identity,Matrix12.Identity,cyclic,Board())),"cycle");
        var deep=Bones(65);for(int i=1;i<65;i++)deep[i]=new BonePose((ushort)i,(short)(i-1),Matrix12.Identity);
        Check(!PoseCodec.Valid(new PoseFrame(session,1,123,true,Matrix12.Identity,Matrix12.Identity,deep,Board())),"depth");
        var sourceBones=Bones(3);var sourceBoard=Board();sourceBoard[3]=Rotate(0.6f,0);
        var independent=new PoseFrame(session,7,123,true,Rotate(1,300),Rotate(1,300),sourceBones,sourceBoard);
        sourceBones[1]=new BonePose(77,0,Matrix12.Identity);sourceBoard[3]=Matrix12.Identity;
        Check(independent.BoneAt(1).Tag==1&&independent.BoardBoneAt(3).XY!=0,"constructor copies caller arrays");
        Check(PoseCodec.TryDecode(PoseCodec.Encode(independent),out decoded),"local poses");
        Check(decoded.BoneAt(1).Local.XX==1&&decoded.BoneAt(1).Local.X==0,"root motion leaves parent local matrix unchanged");
        Check(decoded.BoardBoneAt(3).XY==independent.BoardBoneAt(3).XY,"wheel bone survives, no approximation");
        var layout=new SkeletonLayout(original);
        Check(layout.Matches(independent)&&layout.Fingerprint.Length==64,"layout independent of pose/world/nonce");
        Check(!layout.Matches(Frame(session,2,3,124)),"model layout mismatch");
        var changed=Bones(3);changed[2]=new BonePose(2,1,Matrix12.Identity);
        var changedFrame=new PoseFrame(session,3,123,true,Matrix12.Identity,Matrix12.Identity,changed,Board());
        Check(!layout.Matches(changedFrame)&&layout.Fingerprint!=new SkeletonLayout(changedFrame).Fingerprint,"parent layout mismatch");
        var guard=new ServerPoseSessions();Guid a=guard.BeginAuthenticatedSession(1,layout,100),b=guard.BeginAuthenticatedSession(2,layout,100);AcceptedPose accepted;
        Check(!guard.TryAccept(2,PoseCodec.Encode(Frame(a,1)),100,out accepted),"cross-source forgery");
        Check(!guard.TryAccept(999,PoseCodec.Encode(Frame(a,1)),100,out accepted),"unregistered source");
        Check(guard.TryAccept(1,PoseCodec.Encode(Frame(a,1)),100,out accepted)&&accepted.Source==1,"source bound by server argument");
        Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,1)),150,out accepted),"replay");
        Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,2,3,124)),200,out accepted),"server rejects changed model");
        Check(!guard.TryAccept(1,PoseCodec.Encode(new PoseFrame(a,2,123,true,Matrix12.Identity,Matrix12.Identity,changed,Board())),250,out accepted),"server rejects changed layout");
        Check(guard.TryAccept(1,PoseCodec.Encode(Frame(a,2)),300,out accepted),"rejection does not consume sequence");
        Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,3)),299,out accepted),"clock regression");
        Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,3)),30301,out accepted),"idle expiry");
        guard.Disconnect(1);Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,3)),30400,out accepted),"disconnect");
        Guid next=guard.BeginAuthenticatedSession(1,layout,30400);
        Check(!guard.TryAccept(1,PoseCodec.Encode(Frame(a,99)),30400,out accepted),"old session cannot reuse source ID");
        Check(guard.TryAccept(1,PoseCodec.Encode(Frame(next,1)),30450,out accepted),"fresh session resets sequence");
        var burst=new ServerPoseSessions();Guid burstNonce=burst.BeginAuthenticatedSession(1,layout,0);
        for(ulong i=1;i<=4;i++)Check(burst.TryAccept(1,PoseCodec.Encode(Frame(burstNonce,i)),0,out accepted),"bounded burst");
        Check(!burst.TryAccept(1,PoseCodec.Encode(Frame(burstNonce,5)),0,out accepted),"rate limit");
        Check(burst.TryAccept(1,PoseCodec.Encode(Frame(burstNonce,5)),50,out accepted),"20Hz refill");
        burst.Clear();Check(!burst.TryAccept(1,PoseCodec.Encode(Frame(burstNonce,6)),100,out accepted),"resource clear");
        var latest=new LatestRemotePoses();latest.BeginTrustedServerSession(1,next,layout);
        Check(latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,1)),0),"remote first");
        Check(latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,3)),50),"latest replaces skipping sequence");
        Check(latest.TryLatest(1,60,out accepted)&&accepted.Frame.Sequence==3,"one latest frame");
        Check(!latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,2)),70),"late/replay rejected");
        Check(!latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,4,3,124)),80),"remote layout mismatch");
        Check(!latest.TryLatest(1,551,out accepted),"render expiry");
        Check(!latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,2)),552),"expiry keeps replay barrier");
        latest.Disconnect(1);Check(!latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,4)),600),"remote disconnect");
        latest.BeginTrustedServerSession(1,Guid.NewGuid(),layout);Check(!latest.TryApplyTrustedServerFrame(1,PoseCodec.Encode(Frame(next,100)),650),"remote source reuse epoch");
        latest.Clear();Check(!latest.TryLatest(1,700,out accepted),"client resource clear");
        var capped=new ServerPoseSessions();
        for(int i=1;i<=ServerPoseSessions.MaxPeers;i++)capped.BeginAuthenticatedSession(i,layout,0);
        bool refused=false;try{capped.BeginAuthenticatedSession(65,layout,0);}catch(InvalidOperationException){refused=true;}
        Check(refused,"server source cap");capped.Expire(30001);
        Check(capped.BeginAuthenticatedSession(65,layout,30001)!=Guid.Empty,"expiry releases source capacity");
        var cappedRemote=new LatestRemotePoses();
        for(int i=1;i<=ServerPoseSessions.MaxPeers;i++)cappedRemote.BeginTrustedServerSession(i,next,layout);
        refused=false;try{cappedRemote.BeginTrustedServerSession(65,next,layout);}catch(InvalidOperationException){refused=true;}
        Check(refused,"remote source cap");cappedRemote.Disconnect(1);
        cappedRemote.BeginTrustedServerSession(65,next,layout);
        Check(!cappedRemote.TryLatest(65,0,out accepted),"new remote has no invented pose");
    }
}
