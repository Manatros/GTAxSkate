// Experimental ABI 8 raw probe, NOT approved for live integration.
// No explicit setters/hooks/rendering/transport, but sv_get_output MUTATES the
// runtime's shared presentation latch. Calling this alongside SK8V can replace
// the frame its character/board getters consume; it is not side-effect-free.
// Official SK8V 1.1.9 has no SkateModsRuntime handle export required here.
using System;
using System.Runtime.InteropServices;
using System.Threading;

namespace GTAxSkate.Coop
{
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct CaptureOutput
    {
        public uint Size, Status;
        public ulong Tick;
        public fixed float SkaterPosition[3];
        public fixed float SkaterRotation[4];
        public float SkaterHeadingDegrees;
        public fixed float BoardPosition[3];
        public fixed float BoardRotation[4];
        public fixed float Velocity[3];
        public uint CameraValid;
        public fixed float CameraPosition[3];
        public fixed float CameraRotation[3];
        public float CameraFov;
        public fixed byte StateUtf8[64];
    }
    [StructLayout(LayoutKind.Sequential)]
    internal unsafe struct CaptureBoard
    {
        public uint Size, BoneCount;
        public ulong Tick;
        public fixed float Entity[16];
        public fixed float World[112];
    }

    // Borrowed storage, valid ONLY after a successful TryCapture and until the
    // next call. A consumer must copy before queueing to any other thread.
    public sealed class RawNativeCapture
    {
        public bool Valid { get; internal set; }
        public CaptureOutput Output { get; internal set; }
        public int CharacterBoneCount { get; internal set; }
        public readonly float[] CharacterWorld = new float[NativeCapture.MaxCharacterBones * 16];
        public readonly float[] BoardEntity = new float[16];
        // SKATEBOARD_ROOT, TRUCK_FRONT, TRUCK_BACK, LEFT_WHEELFRONT,
        // LEFT_WHEELBACK, RIGHT_WHEELFRONT, RIGHT_WHEELBACK; 16 floats each.
        public readonly float[] BoardWorld = new float[7 * 16];
        // Character export supplies matrices but NO bone IDs, parents, model,
        // outfit or asset identity. Never infer a trusted layout from the count.
        public bool HasTrustedSkeletonLayout { get { return false; } }
        public string ReplicationBlocker { get { return "Missing independently trusted model/bone-ID/parent layout and peer asset/layout handshake; raw matrices cannot establish compatibility."; } }
    }

    // Experimental probe only; do not activate from the RAGECOOP adapter.
    // Main-thread checks do not establish a safe SK8V lifetime/scheduling seam.
    // The caller MUST keep the host/runtime loaded and prevent script restart,
    // disposal or any other sv_get_output caller during these bounded reads.
    // GetModuleHandle does not pin a DLL; tick checks cannot prove its lifetime.
    // A host-owned copy of its already-produced frame is the proposed safe
    // integration boundary; no such snapshot-copy interface is implemented here.
    public sealed unsafe class NativeCapture
    {
        public const int MaxCharacterBones = 512;
        private readonly int mainThread;
        private readonly string hostName, runtimeName;
        private readonly float[] scratch = new float[(MaxCharacterBones + 1) * 16];
        private IntPtr boundHost, boundRuntime;
        private RuntimeHandle getRuntime;
        private ApiVersion api;
        private GetOutput getOutput;
        private GetCharacter getCharacter;
        private GetBoard getBoard;
        private IntPtr previousHandle;
        private ulong previousTick;
        private bool hadCapture;
        public readonly RawNativeCapture Frame = new RawNativeCapture();

        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, ExactSpelling=true)]
        private static extern IntPtr GetModuleHandleW(string name);
        [DllImport("kernel32.dll", CharSet=CharSet.Ansi, ExactSpelling=true)]
        private static extern IntPtr GetProcAddress(IntPtr module, string name);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate IntPtr RuntimeHandle();
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint ApiVersion();
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint GetOutput(IntPtr rt, CaptureOutput* output);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint GetCharacter(IntPtr rt, float* matrices, uint capacityBones);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint GetBoard(IntPtr rt, CaptureBoard* board);

        public NativeCapture(int mainScriptThreadId, string hostModule = "SkateVLegacy.asi", string runtimeModule = "SkateVRuntime.dll")
        {
            if (mainScriptThreadId != Thread.CurrentThread.ManagedThreadId) throw new ArgumentException("Construct on the registered main script thread");
            mainThread=mainScriptThreadId; hostName=hostModule; runtimeName=runtimeModule;
        }
        private static T Export<T>(IntPtr module, string name) where T : class
        {
            IntPtr p=GetProcAddress(module,name);
            return p==IntPtr.Zero ? null : Marshal.GetDelegateForFunctionPointer(p,typeof(T)) as T;
        }
        private bool Bind(out string reason)
        {
            reason=null;
            IntPtr host=GetModuleHandleW(hostName), runtime=GetModuleHandleW(runtimeName);
            if(host==IntPtr.Zero || runtime==IntPtr.Zero) { reason="modules-not-loaded"; return false; }
            if(host!=boundHost || runtime!=boundRuntime)
            {
                getRuntime=Export<RuntimeHandle>(host,"SkateModsRuntime");
                api=Export<ApiVersion>(runtime,"sv_api_version");
                getOutput=Export<GetOutput>(runtime,"sv_get_output");
                getCharacter=Export<GetCharacter>(runtime,"sv_get_character_pose");
                getBoard=Export<GetBoard>(runtime,"sv_get_board_pose");
                boundHost=host; boundRuntime=runtime; hadCapture=false;
            }
            if(getRuntime==null || api==null || getOutput==null || getCharacter==null || getBoard==null) { reason="missing-export"; return false; }
            if(api()!=8) { reason="abi-mismatch"; return false; }
            return true;
        }
        private static bool Finite(float value) { return !float.IsNaN(value) && !float.IsInfinity(value); }
        private static bool FiniteOutput(CaptureOutput* output)
        {
            // All float regions, excluding CameraValid at byte 88.
            float* p=(float*)((byte*)output+16);
            for(int i=0;i<18;i++) if(!Finite(p[i]))return false;
            p=(float*)((byte*)output+92);
            for(int i=0;i<7;i++) if(!Finite(p[i]))return false;
            return true;
        }
        public bool TryCapture(out string reason)
        {
            if(Thread.CurrentThread.ManagedThreadId!=mainThread) { reason="main-thread-only"; return false; }
            Frame.Valid=false; Frame.CharacterBoneCount=0;
            if(!Bind(out reason)) return false;
            // Each output read REPLACES the shared runtime presentation latch;
            // character/board getters, including SK8V's, consume that latch.
            // The second read and bounded tick check cannot prove no host-frame
            // interference or disposal. This remains a synthetic-test probe.
            for(int attempt=0;attempt<2;attempt++)
            {
                IntPtr rt=getRuntime();
                if(rt==IntPtr.Zero) { reason="runtime-not-created"; return false; }
                CaptureOutput before=new CaptureOutput(), after=new CaptureOutput();
                before.Size=after.Size=(uint)sizeof(CaptureOutput);
                CaptureBoard board=new CaptureBoard(); board.Size=(uint)sizeof(CaptureBoard);
                if(getOutput(rt,&before)==0) { reason="output-unavailable"; return false; }
                if(before.Status!=3) { reason="inactive"; return false; }
                uint count;
                fixed(float* p=scratch) count=getCharacter(rt,p,MaxCharacterBones+1);
                // The actual ABI truncates to capacity, so 513 is a deliberate
                // sentinel slot: exactly 512 is valid, 513 signals overflow.
                if(count==0 || count>MaxCharacterBones) { reason="character-count"; return false; }
                if(getBoard(rt,&board)==0) { reason="board-unavailable"; return false; }
                if(board.Size!=sizeof(CaptureBoard) || board.BoneCount!=7) { reason="board-layout"; return false; }
                if(getOutput(rt,&after)==0) { reason="output-unavailable"; return false; }
                if(after.Status!=3) { reason="inactive"; return false; }
                if(getRuntime()!=rt || before.Tick!=after.Tick || board.Tick!=before.Tick) continue;
                if(hadCapture && previousHandle==rt && before.Tick<=previousTick) { reason="stale-tick"; return false; }
                if(before.Size!=sizeof(CaptureOutput) || after.Size!=sizeof(CaptureOutput) || !FiniteOutput(&before) || !FiniteOutput(&after)) { reason="invalid-output"; return false; }
                for(int i=0;i<count*16;i++) if(!Finite(scratch[i])) { reason="nonfinite-character"; return false; }
                for(int i=0;i<16;i++) if(!Finite(board.Entity[i])) { reason="nonfinite-board"; return false; }
                for(int i=0;i<112;i++) if(!Finite(board.World[i])) { reason="nonfinite-board"; return false; }
                Array.Copy(scratch,Frame.CharacterWorld,(int)count*16);
                for(int i=0;i<16;i++) Frame.BoardEntity[i]=board.Entity[i];
                for(int i=0;i<112;i++) Frame.BoardWorld[i]=board.World[i];
                Frame.Output=before; Frame.CharacterBoneCount=(int)count; Frame.Valid=true;
                previousHandle=rt; previousTick=before.Tick; hadCapture=true; reason=null; return true;
            }
            reason="snapshot-race"; return false;
        }
    }
}
