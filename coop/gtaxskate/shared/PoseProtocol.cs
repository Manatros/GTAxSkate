// Standalone numeric pose contract. No dependency on SK8V or a game/mod loader.
using System;
using System.Collections.Generic;
using System.IO;

namespace GTAxSkate.Coop
{
    // Row-vector rigid transform: three XYZ axes followed by translation.
    // Homogeneous fourth components are implicit 0,0,0,1. Units are metres.
    public struct Matrix12
    {
        public readonly float XX, XY, XZ, YX, YY, YZ, ZX, ZY, ZZ, X, Y, Z;
        public Matrix12(float xx, float xy, float xz, float yx, float yy, float yz,
            float zx, float zy, float zz, float x, float y, float z)
        { XX=xx; XY=xy; XZ=xz; YX=yx; YY=yy; YZ=yz; ZX=zx; ZY=zy; ZZ=zz; X=x; Y=y; Z=z; }
        public static Matrix12 Identity { get { return new Matrix12(1,0,0,0,1,0,0,0,1,0,0,0); } }
        internal void Write(BinaryWriter w)
        { w.Write(XX);w.Write(XY);w.Write(XZ);w.Write(YX);w.Write(YY);w.Write(YZ);w.Write(ZX);w.Write(ZY);w.Write(ZZ);w.Write(X);w.Write(Y);w.Write(Z); }
        internal static Matrix12 Read(BinaryReader r)
        { return new Matrix12(r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle(),r.ReadSingle()); }
        private static bool Finite(float f) { return !Single.IsNaN(f) && !Single.IsInfinity(f); }
        public bool IsRigid(float translationLimit)
        {
            if (!Finite(XX)||!Finite(XY)||!Finite(XZ)||!Finite(YX)||!Finite(YY)||!Finite(YZ)
                ||!Finite(ZX)||!Finite(ZY)||!Finite(ZZ)||!Finite(X)||!Finite(Y)||!Finite(Z)) return false;
            if (Math.Abs(X)>translationLimit || Math.Abs(Y)>translationLimit || Math.Abs(Z)>translationLimit) return false;
            // Proper rotations only: unit axes, perpendicular, positive determinant.
            float a=XX*XX+XY*XY+XZ*XZ, b=YX*YX+YY*YY+YZ*YZ, c=ZX*ZX+ZY*ZY+ZZ*ZZ;
            if (Math.Abs(a-1)>0.02f || Math.Abs(b-1)>0.02f || Math.Abs(c-1)>0.02f) return false;
            if (Math.Abs(XX*YX+XY*YY+XZ*YZ)>0.02f || Math.Abs(XX*ZX+XY*ZY+XZ*ZZ)>0.02f
                || Math.Abs(YX*ZX+YY*ZY+YZ*ZZ)>0.02f) return false;
            float det=XX*(YY*ZZ-YZ*ZY)-XY*(YX*ZZ-YZ*ZX)+XZ*(YX*ZY-YY*ZX);
            return Math.Abs(det-1)<0.05f;
        }
    }
    public struct BonePose
    {
        public readonly ushort Tag;
        public readonly short ParentIndex;
        public readonly Matrix12 Local;
        public BonePose(ushort tag, short parentIndex, Matrix12 local)
        { Tag=tag; ParentIndex=parentIndex; Local=local; }
    }
    public sealed class PoseFrame
    {
        public readonly Guid Session;
        public readonly ulong Sequence;
        public readonly uint PedModel;
        public readonly bool Riding;
        public readonly Matrix12 PedWorld, BoardWorld;
        private readonly BonePose[] bones;
        private readonly Matrix12[] boardLocals;
        public PoseFrame(Guid session, ulong sequence, uint pedModel, bool riding,
            Matrix12 pedWorld, Matrix12 boardWorld, BonePose[] poses, Matrix12[] boardPoses)
        {
            if (poses==null || poses.Length>PoseCodec.MaxBones || boardPoses==null || boardPoses.Length!=7) throw new ArgumentException("Bone count");
            Session=session;Sequence=sequence;PedModel=pedModel;Riding=riding;PedWorld=pedWorld;BoardWorld=boardWorld;
            bones=(BonePose[])poses.Clone();
            boardLocals=(Matrix12[])boardPoses.Clone();
        }
        public int BoneCount { get { return bones.Length; } }
        public BonePose BoneAt(int index) { return bones[index]; }
        // Fixed GTA board tags 0..6; parents -1,0,0,1,2,1,2.
        public Matrix12 BoardBoneAt(int index) { return boardLocals[index]; }
    }
    public static class PoseCodec
    {
        public const ushort Version=1;
        public const int MaxBones=512, HeaderBytes=472, BoneBytes=52;
        public const int MaxPacketBytes=HeaderBytes+MaxBones*BoneBytes;
        private const uint Magic=0x31535847; // bytes G X S 1
        public static bool Valid(PoseFrame p)
        {
            if (p==null || p.Session==Guid.Empty || p.Sequence==0 || p.PedModel==0 || p.BoneCount<1 || p.BoneCount>MaxBones
                || !p.PedWorld.IsRigid(32768) || !p.BoardWorld.IsRigid(32768)) return false;
            for(int i=0;i<7;i++)if(!p.BoardBoneAt(i).IsRigid(5))return false;
            var tags=new HashSet<ushort>();int roots=0;
            for(int i=0;i<p.BoneCount;i++)
            {
                BonePose b=p.BoneAt(i);
                if(!tags.Add(b.Tag)||b.ParentIndex < -1||b.ParentIndex>=p.BoneCount||b.ParentIndex==i||!b.Local.IsRigid(5))return false;
                if(b.ParentIndex==-1)roots++;
                // Hard bound also rejects cycles without recursive stack growth.
                int current=i, depth=0;
                while(current>=0)
                { if(++depth>64)return false;current=p.BoneAt(current).ParentIndex;
                  if(current < -1 || current>=p.BoneCount)return false; }
            }
            return roots==1;
        }
        public static byte[] Encode(PoseFrame pose)
        {
            if(!Valid(pose))throw new ArgumentException("Invalid pose");
            using(var stream=new MemoryStream(HeaderBytes+pose.BoneCount*BoneBytes))
            using(var w=new BinaryWriter(stream))
            {
                w.Write(Magic);w.Write(Version);w.Write((ushort)(pose.Riding?1:0));
                w.Write(pose.Session.ToByteArray());w.Write(pose.Sequence);w.Write(pose.PedModel);
                w.Write((ushort)pose.BoneCount);w.Write((ushort)0);
                pose.PedWorld.Write(w);pose.BoardWorld.Write(w);
                for(int i=0;i<7;i++)pose.BoardBoneAt(i).Write(w);
                for(int i=0;i<pose.BoneCount;i++){var b=pose.BoneAt(i);w.Write(b.Tag);w.Write(b.ParentIndex);b.Local.Write(w);}
                return stream.ToArray();
            }
        }
        public static bool TryDecode(byte[] bytes,out PoseFrame pose)
        {
            pose=null;
            if(bytes==null||bytes.Length<HeaderBytes+BoneBytes||bytes.Length>MaxPacketBytes)return false;
            try
            {
                using(var r=new BinaryReader(new MemoryStream(bytes,false)))
                {
                    if(r.ReadUInt32()!=Magic||r.ReadUInt16()!=Version)return false;
                    ushort flags=r.ReadUInt16();if(flags>1)return false;
                    Guid nonce=new Guid(r.ReadBytes(16));ulong sequence=r.ReadUInt64();uint model=r.ReadUInt32();
                    ushort count=r.ReadUInt16();if(r.ReadUInt16()!=0||count<1||count>MaxBones||bytes.Length!=HeaderBytes+count*BoneBytes)return false;
                    Matrix12 ped=Matrix12.Read(r),board=Matrix12.Read(r);var boardBones=new Matrix12[7];
                    for(int i=0;i<7;i++)boardBones[i]=Matrix12.Read(r);
                    var bones=new BonePose[count];
                    for(int i=0;i<count;i++)bones[i]=new BonePose(r.ReadUInt16(),r.ReadInt16(),Matrix12.Read(r));
                    var decoded=new PoseFrame(nonce,sequence,model,flags==1,ped,board,bones,boardBones);
                    if(!Valid(decoded))return false;pose=decoded;return true;
                }
            }
            catch(EndOfStreamException){return false;}
            catch(IOException){return false;}
            catch(ArgumentException){return false;}
        }
    }
}
