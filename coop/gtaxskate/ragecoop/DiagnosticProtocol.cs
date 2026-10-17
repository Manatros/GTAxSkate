using System;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    // This handshake grants no pose/native/policy capability. Inbound client
    // callbacks expose no transport origin, so their content is diagnostic only.
    internal static class DiagnosticProtocol
    {
        internal const int Version = 1;
        internal const string Hello = "GTAxSkate.Coop.v1.Hello";
        internal const string Challenge = "GTAxSkate.Coop.v1.Challenge";
        internal const string Ack = "GTAxSkate.Coop.v1.Ack";
        internal const string Result = "GTAxSkate.Coop.v1.Result";
        internal static bool Header(object[] args, int count)
        { return args != null && args.Length == count && args[0] is int && (int)args[0] == Version; }
        internal static bool Nonce(object value)
        { return value is byte[] && ((byte[])value).Length == 16 && new Guid((byte[])value) != Guid.Empty; }
        internal static bool Equal(byte[] a, byte[] b)
        {
            if (a == null || b == null || a.Length != 16 || b.Length != 16) return false;
            int difference = 0;
            for (int i = 0; i < 16; i++) difference |= a[i] ^ b[i];
            return difference == 0;
        }
    }
}
