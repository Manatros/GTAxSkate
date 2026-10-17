using System;
using System.IO;
using System.Security.Cryptography;

namespace GTAxSkate.Coop
{
    // Exact ordered identity, not a guess based on bone count. Native target
    // code must independently match its ped's model/tags/parents before posing.
    public sealed class SkeletonLayout
    {
        public readonly uint PedModel;
        public readonly string Fingerprint;
        private readonly ushort[] tags;
        private readonly short[] parents;
        public SkeletonLayout(PoseFrame validatedTemplate)
        {
            if(!PoseCodec.Valid(validatedTemplate))throw new ArgumentException("Invalid template layout");
            PedModel=validatedTemplate.PedModel;tags=new ushort[validatedTemplate.BoneCount];parents=new short[tags.Length];
            using(var stream=new MemoryStream())using(var writer=new BinaryWriter(stream))
            {
                writer.Write(PoseCodec.Version);writer.Write(PedModel);writer.Write((ushort)tags.Length);
                for(int i=0;i<tags.Length;i++){var b=validatedTemplate.BoneAt(i);tags[i]=b.Tag;parents[i]=b.ParentIndex;writer.Write(b.Tag);writer.Write(b.ParentIndex);}
                using(var sha=SHA256.Create())Fingerprint=BitConverter.ToString(sha.ComputeHash(stream.ToArray())).Replace("-","").ToLowerInvariant();
            }
        }
        public bool Matches(PoseFrame frame)
        {
            if(frame==null||frame.PedModel!=PedModel||frame.BoneCount!=tags.Length)return false;
            for(int i=0;i<tags.Length;i++){var b=frame.BoneAt(i);if(b.Tag!=tags[i]||b.ParentIndex!=parents[i])return false;}
            return true;
        }
    }
}
