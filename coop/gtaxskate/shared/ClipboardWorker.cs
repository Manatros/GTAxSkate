// Asynchronous clipboard access for the invite flows (design 2026-11-17-streamer-safe-coop, item 6).
// The game thread never waits: it Begins an operation and Polls its ticket on later ticks. All real clipboard
// calls run on ONE background STA thread (Windows needs an STA thread), strictly in the order they were begun,
// against an IClipboardBackend (the live WinForms one lives in client/ShvdnWorldBridge.cs; tests use a fake).
// The thread exits when idle, so a reloaded script domain leaks nothing. Pure, C# 5, no GTA references.
using System;
using System.Collections.Generic;
using System.Threading;

namespace GTAxSkate.Coop
{
    public enum ClipboardOpKind
    {
        CopyHidden = 0,     // set text; excluded from clipboard history, cloud clipboard and monitors
        Read = 1,           // read the current text
        ClearIfEquals = 2   // clear the clipboard only if it still holds exactly this text (ordinal, no trimming)
    }

    public enum ClipboardOpState
    {
        Pending = 0,
        Done = 1,
        Failed = 2
    }

    public sealed class ClipboardOpResult
    {
        public readonly ClipboardOpState State;
        public readonly string Text;      // Read: the clipboard text ("" when it holds none)
        public readonly bool Changed;     // CopyHidden: true; ClearIfEquals: true when it really cleared

        public ClipboardOpResult(ClipboardOpState state, string text, bool changed)
        {
            State = state; Text = text ?? string.Empty; Changed = changed;
        }

        public static readonly ClipboardOpResult PendingResult = new ClipboardOpResult(ClipboardOpState.Pending, null, false);
        public static readonly ClipboardOpResult FailedResult = new ClipboardOpResult(ClipboardOpState.Failed, null, false);
    }

    public interface IClipboardBackend
    {
        // Runs on the worker's STA thread. May throw; the worker turns that into Failed.
        string GetText();
        void SetTextHidden(string text);
        void Clear();
    }

    public sealed class ClipboardWorker : IDisposable
    {
        private const int IdleExitMs = 5000;
        private const int MaxKeptResults = 32;
        private sealed class Op
        {
            public int Ticket; public ClipboardOpKind Kind; public string Text; public bool Track;
        }

        private readonly IClipboardBackend backend;
        private readonly object gate = new object();
        private readonly Queue<Op> queue = new Queue<Op>();
        private readonly Dictionary<int, ClipboardOpResult> results = new Dictionary<int, ClipboardOpResult>();
        private readonly List<int> resultOrder = new List<int>();
        private Thread thread;
        private bool stopping;
        private int nextTicket;

        public ClipboardWorker(IClipboardBackend backend)
        {
            if (backend == null) throw new ArgumentNullException("backend");
            this.backend = backend;
        }

        // track=false: fire and forget (a clear after the invite expired); no result is kept.
        public int Begin(ClipboardOpKind kind, string text, bool track)
        {
            lock (gate)
            {
                if (stopping) return 0;
                int ticket = ++nextTicket;
                if (track) { results[ticket] = ClipboardOpResult.PendingResult; resultOrder.Add(ticket); }
                queue.Enqueue(new Op { Ticket = ticket, Kind = kind, Text = text, Track = track });
                if (thread == null)
                {
                    thread = new Thread(Loop);
                    thread.SetApartmentState(ApartmentState.STA);
                    thread.IsBackground = true;
                    thread.Name = "GTAxSkate clipboard";
                    thread.Start();
                }
                Monitor.PulseAll(gate);
                return ticket;
            }
        }

        // Pending until finished; a finished result is returned exactly once, later polls say Failed.
        public ClipboardOpResult Poll(int ticket)
        {
            lock (gate)
            {
                ClipboardOpResult r;
                if (ticket <= 0 || !results.TryGetValue(ticket, out r)) return ClipboardOpResult.FailedResult;
                if (r.State == ClipboardOpState.Pending) return r;
                results.Remove(ticket);
                resultOrder.Remove(ticket);
                return r;
            }
        }

        public int PendingCount { get { lock (gate) { return queue.Count; } } }

        // Lets queued operations finish (a clear on exit), then stops; waits at most waitMs.
        public void Shutdown(int waitMs)
        {
            Thread t;
            lock (gate)
            {
                stopping = true;
                t = thread;
                Monitor.PulseAll(gate);
            }
            if (t != null && waitMs > 0 && t != Thread.CurrentThread)
            {
                try { t.Join(waitMs); } catch { }
            }
        }

        public void Dispose() { Shutdown(1000); }

        private void Loop()
        {
            while (true)
            {
                Op op;
                lock (gate)
                {
                    while (queue.Count == 0)
                    {
                        if (stopping) { thread = null; return; }
                        if (!Monitor.Wait(gate, IdleExitMs) && queue.Count == 0)
                        {
                            thread = null;   // idle: the next Begin starts a fresh thread
                            return;
                        }
                    }
                    op = queue.Dequeue();
                }
                ClipboardOpResult result = Execute(op);
                if (!op.Track) continue;
                lock (gate)
                {
                    results[op.Ticket] = result;
                    while (resultOrder.Count > MaxKeptResults)
                    {
                        results.Remove(resultOrder[0]);
                        resultOrder.RemoveAt(0);
                    }
                }
            }
        }

        private ClipboardOpResult Execute(Op op)
        {
            try
            {
                switch (op.Kind)
                {
                    case ClipboardOpKind.CopyHidden:
                    {
                        backend.SetTextHidden(op.Text ?? string.Empty);
                        // Real completion status: read back what is on the clipboard now.
                        string now = backend.GetText();
                        if (!string.Equals(now ?? string.Empty, op.Text ?? string.Empty, StringComparison.Ordinal))
                            return ClipboardOpResult.FailedResult;
                        return new ClipboardOpResult(ClipboardOpState.Done, null, true);
                    }
                    case ClipboardOpKind.Read:
                        return new ClipboardOpResult(ClipboardOpState.Done, backend.GetText() ?? string.Empty, false);
                    case ClipboardOpKind.ClearIfEquals:
                    {
                        string current = backend.GetText();
                        if (!string.IsNullOrEmpty(current) && !string.IsNullOrEmpty(op.Text) &&
                            string.Equals(current, op.Text, StringComparison.Ordinal))   // exact: never erase a changed clipboard
                        {
                            backend.Clear();
                            return new ClipboardOpResult(ClipboardOpState.Done, null, true);
                        }
                        return new ClipboardOpResult(ClipboardOpState.Done, null, false);
                    }
                }
            }
            catch { }
            return ClipboardOpResult.FailedResult;
        }
    }
}
