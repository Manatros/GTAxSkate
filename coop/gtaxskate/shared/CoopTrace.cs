// Durable coop trace (2026-11-16 evening test): RageCoop's own client/server logs were
// missing after the session, so friend-auth failures left no evidence. Every line is
// appended and flushed at once to %LOCALAPPDATA%\SkateV\coop-<side>.log (side = client
// or server), capped at 1 MB (older content moves to .prev.log). Never throws.
using System;
using System.IO;
using System.Text;

namespace GTAxSkate.Coop
{
    public static class CoopTrace
    {
        // "<InnerTypeName> 0xHRESULT" for traces: reflection/aggregate wrappers are unwrapped so the trace names the real
        // failure; never the message (it can carry addresses or paths).
        public static string Describe(Exception e)
        {
            if (e == null) return "none";
            for (int i = 0; i < 8; i++)
            {
                Exception inner = null;
                if (e is System.Reflection.TargetInvocationException) inner = e.InnerException;
                else if (e is AggregateException && e.InnerException != null) inner = e.InnerException;
                if (inner == null) break;
                e = inner;
            }
            return e.GetType().Name + " 0x" + e.HResult.ToString("X8");
        }

        private static readonly object Gate = new object();
        public static string DirectoryOverride;  // tests point this at a scratch folder

        public static void Write(string side, string message)
        {
#if COOP_FIXTURE
            if (DirectoryOverride == null) return;  // fixture runs never write to the real profile
#endif
            try
            {
                string dir = DirectoryOverride ?? Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "SkateV");
                if (string.IsNullOrEmpty(dir)) return;
                string path = Path.Combine(dir, "coop-" + side + ".log");
                string line = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff") + " " + message + Environment.NewLine;
                lock (Gate)
                {
                    Directory.CreateDirectory(dir);
                    var info = new FileInfo(path);
                    if (info.Exists && info.Length > 1024 * 1024)
                    {
                        string prev = Path.Combine(dir, "coop-" + side + ".prev.log");
                        if (File.Exists(prev)) File.Delete(prev);
                        File.Move(path, prev);
                    }
                    File.AppendAllText(path, line, new UTF8Encoding(false));
                }
            }
            catch { }
        }
    }
}
