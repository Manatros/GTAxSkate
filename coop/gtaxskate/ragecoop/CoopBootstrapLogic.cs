// Testable state machine behind the GTA.Script entry point in GTAxSkate.Coop.Bootstrap.dll.
// Compiled into GTAxSkate.Coop.Client.dll (no GTA.Script type here); the bootstrap only
// forwards KeyDown/Tick/Aborted and implements ICoopBootstrapHost with GTA calls.
//
// Default OFF: until CoopRuntimeControl says Enabled this class constructs no ClientAdapter,
// claims no owner, scans nothing and draws nothing unless the user opened the F11 panel.
using System;
using System.Collections.Concurrent;

namespace GTAxSkate.Coop.RageCoopAdapter
{
    public sealed class CoopPanelView
    {
        public readonly string Title;
        public readonly string[] Items;
        public readonly int Selected;
        public readonly string Status;
        public readonly string Help;

        public CoopPanelView(string title, string[] items, int selected, string status, string help)
        {
            // Anything on screen ends up in a stream: every string passes the sanitizer (design 2026-11-17-streamer-safe-coop).
            Title = SafeText.ForScreen(title);
            Items = new string[items == null ? 0 : items.Length];
            for (int i = 0; i < Items.Length; i++) Items[i] = SafeText.ForScreen(items[i]);
            Selected = selected;
            Status = SafeText.ForScreen(status);
            Help = SafeText.ForScreen(help);
        }
    }

    public interface ICoopBootstrapHost
    {
        bool IsWorldReady();
        bool IsWorldAndRageCoopReady();
        long NowMs();
        ClientAdapter CreateAdapter();
        void DrawPanel(CoopPanelView view);
        void Notify(string message);
        void LogWarning(string message);
        bool IsTransportConnected();
        void DisconnectTransport();
    }

    public sealed class CoopBootstrapLogic
    {
        public const int VkF11 = 0x7A, VkUp = 0x26, VkDown = 0x28, VkReturn = 0x0D, VkBack = 0x08, VkEscape = 0x1B;
        private const int MaxQueuedKeys = 64;
        private const int MaxStartupAttempts = 3;
        private const long RetrySpacingMs = 2000;
        private const long TransportGuardSpacingMs = 500;
        private const long OwnerStaleMs = 5000;

        private readonly ICoopBootstrapHost host;
        private readonly ConcurrentQueue<int> keys = new ConcurrentQueue<int>();
        private ClientAdapter adapter;
        private volatile bool started;
        private volatile bool panelOpen;
        private volatile bool f11Pending;
        private bool startupAbandoned, abandonNotified, openAdapterMenuOnStart;
        // Another loaded copy of the client assembly owns the session; this bootstrap stepped its own adapter down.
        private bool deferred;
        private int attempts, panelSelected;
        private long nextAttemptMs, nextGuardMs, deferGraceMs;
        private string lastError = string.Empty;

        public CoopBootstrapLogic(ICoopBootstrapHost host)
        {
            if (host == null) throw new ArgumentNullException("host");
            this.host = host;
        }

        public bool AdapterStarted { get { return started; } }
        public bool PanelOpen { get { return panelOpen; } }
        public bool DeferredToOtherCopy { get { return deferred; } }

        private static bool IsMenuKey(int vk)
        {
            return vk == VkF11 || vk == VkUp || vk == VkDown || vk == VkReturn || vk == VkBack || vk == VkEscape;
        }

        // KeyDown thread: enqueue only. Everything else (UI, startup, native calls) happens in Tick.
        // While the panel is closed and no adapter owns the menu, only F11 is accepted so normal
        // gameplay keys are never consumed or queued.
        public void EnqueueKey(int vk)
        {
            if (!IsMenuKey(vk)) return;
            if (vk != VkF11 && !panelOpen && !started && !f11Pending) return;
            if (keys.Count >= MaxQueuedKeys) return;
            if (vk == VkF11) f11Pending = true;
            keys.Enqueue(vk);
        }

        public void Tick()
        {
            if (!host.IsWorldReady())
            {
                // A loading screen also pauses the other copy's heartbeat: restart the staleness window.
                if (deferred) deferGraceMs = host.NowMs();
                return;
            }
            f11Pending = false; // every key queued so far is handled below, in order
            if (!CoopRuntimeControl.Enabled)
            {
                TickOff();
                return;
            }
            TickEnabled();
        }

        public void Abort()
        {
            int ignored;
            while (keys.TryDequeue(out ignored)) { }
            ClientAdapter local = adapter;
            adapter = null;
            started = false;
            deferred = false;
            panelOpen = false;
            openAdapterMenuOnStart = false;
            if (local != null && local.IsActive)
            {
                try { local.OnStop(); } catch { }
            }
        }

        // ---- Off ----------------------------------------------------------------------------

        private void TickOff()
        {
            // Switched off while an adapter existed (external SetEnabled): tear it down once, silently.
            if (adapter != null || started || deferred)
            {
                ClientAdapter local = adapter ?? ClientAdapter.ActiveOwner;
                adapter = null;
                started = false;
                deferred = false;
                if (local != null && local.IsActive)
                {
                    try { local.StopForDisable(); } catch { }
                }
            }

            int vk;
            while (keys.TryDequeue(out vk))
            {
                if (vk == VkF11) { panelOpen = !panelOpen; panelSelected = 0; continue; }
                if (!panelOpen) continue;
                if (vk == VkUp || vk == VkDown) { panelSelected = panelSelected == 0 ? 1 : 0; continue; }
                if (vk == VkEscape || vk == VkBack) { panelOpen = false; continue; }
                if (vk == VkReturn)
                {
                    if (panelSelected == 0) { RequestEnable(); break; }
                    panelOpen = false;
                }
            }

            if (panelOpen && !CoopRuntimeControl.Enabled)
            {
                host.DrawPanel(new CoopPanelView(
                    "Multiplayer: Off",
                    new[] { "Enable Multiplayer", "Close" },
                    panelSelected,
                    "Solo Story Mode. Nothing coop-related runs while this is off.",
                    "Up/Down: Select  |  Enter: Activate  |  F11/Esc: Close  |  Remembered in scripts\\RageCoop\\FriendsCoopPrefs.json"));
            }

            // Fail-closed: a raw F9/external RAGECOOP connection observed while multiplayer is off is
            // dropped silently. No adapter is started and no custom event is sent.
            long now = host.NowMs();
            if (now >= nextGuardMs)
            {
                nextGuardMs = now + TransportGuardSpacingMs;
                if (host.IsWorldAndRageCoopReady() && host.IsTransportConnected())
                {
                    try { host.DisconnectTransport(); } catch { }
                }
            }
        }

        private void RequestEnable()
        {
            string error;
            bool saved = CoopRuntimeControl.SetEnabled(true, out error);
            panelOpen = false;
            openAdapterMenuOnStart = true;
            attempts = 0;
            startupAbandoned = false;
            abandonNotified = false;
            nextAttemptMs = 0;
            lastError = string.Empty;
            if (!saved)
                host.Notify(SafeText.ForScreen("[F11 Coop] Multiplayer is on for this session, but the preference could not be saved: " + error));
        }

        // ---- Enabled ------------------------------------------------------------------------

        private void TickEnabled()
        {
            if (started)
            {
                ClientAdapter owner = ClientAdapter.ActiveOwner ?? adapter;
                if (owner != null && owner.IsActive && !owner.IsLive)
                {
                    if (owner.IsDisplaced)
                    {
                        // Another loaded copy (RageCoop resource loader) owns the session. Stepping down must not
                        // touch the transport or the server: both belong to that live copy. Do not restart over it.
                        try { owner.StepDownDisplaced(); } catch { }
                        adapter = null;
                        started = false;
                        deferred = true;
                        deferGraceMs = host.NowMs();
                        return;
                    }
                    // Stale generation (disabled and re-enabled behind our back): drop it and start fresh.
                    try { owner.StopForDisable(); } catch { }
                }
                else if (owner != null && owner.IsActive)
                {
                    int vk;
                    while (keys.TryDequeue(out vk))
                    {
                        if (!owner.IsActive) break;
                        owner.HandleMenuKeyCode(vk);
                    }
                    if (owner.IsActive)
                        owner.StepEveryFrame();
                    return;
                }
                // Owner lost while still enabled (inactive-owner recovery): start again.
                adapter = null;
                started = false;
                attempts = 0;
                startupAbandoned = false;
                abandonNotified = false;
                nextAttemptMs = 0;
                return;
            }

            bool stillDeferred = deferred && !LeaveDeferredIfOtherCopyGone();
            if (!ProcessKeysAndPanel()) return; // disabled from the panel
            if (!stillDeferred) TickStartIfDue();
        }

        // Deferred to another live copy: nothing to start. Returns true when the deferral ended
        // (token released, or its heartbeat went stale for OwnerStaleMs = crashed copy) and the normal start path may run.
        private bool LeaveDeferredIfOtherCopyGone()
        {
            long now = host.NowMs();
            if (CoopRuntimeControl.HasLiveOtherOwner(Guid.Empty, now, OwnerStaleMs)) return false;
            // Released token: restart at once. Still held but stale: require a full window of our own observation,
            // so a pause of the other copy (load screen) never triggers a restart.
            if (CoopRuntimeControl.HasOtherOwner(Guid.Empty) && now - deferGraceMs < OwnerStaleMs) return false;
            deferred = false;
            attempts = 0;
            startupAbandoned = false;
            abandonNotified = false;
            nextAttemptMs = 0;
            return true;
        }

        // Returns false when the user disabled multiplayer from the panel (nothing else may run this tick).
        private bool ProcessKeysAndPanel()
        {
            bool ready = host.IsWorldAndRageCoopReady();
            int key;
            while (keys.TryDequeue(out key))
            {
                if (key == VkF11) { panelOpen = !panelOpen; panelSelected = 0; continue; }
                if (!panelOpen) continue;
                string[] items = EnabledPanelItems();
                if (key == VkUp) { panelSelected = (panelSelected + items.Length - 1) % items.Length; continue; }
                if (key == VkDown) { panelSelected = (panelSelected + 1) % items.Length; continue; }
                if (key == VkEscape || key == VkBack) { panelOpen = false; continue; }
                if (key == VkReturn)
                {
                    string chosen = items[panelSelected];
                    if (chosen == CoopInGameMenuState.DisableLabel)
                    {
                        DisableFromPanel();
                        return false;
                    }
                    if (chosen == "Retry now")
                    {
                        attempts = 0; startupAbandoned = false; abandonNotified = false; nextAttemptMs = 0; lastError = string.Empty;
                    }
                    panelOpen = false;
                }
            }

            if (panelOpen)
            {
                string status = deferred
                    ? "Running in another loaded copy of the coop client."
                    : startupAbandoned
                    ? "Start failed: " + (string.IsNullOrEmpty(lastError) ? "unknown error" : lastError)
                    : (ready ? "Starting..." : "Waiting for RageCoop to initialize...");
                host.DrawPanel(new CoopPanelView(
                    "Multiplayer: On (starting)",
                    EnabledPanelItems(),
                    panelSelected,
                    status,
                    "Up/Down: Select  |  Enter: Activate  |  F11/Esc: Close"));
            }

            return true;
        }

        private void TickStartIfDue()
        {
            if (startupAbandoned || deferred || !host.IsWorldAndRageCoopReady()) return;
            long now = host.NowMs();
            if (nextAttemptMs != 0 && now < nextAttemptMs) return;
            TryStartAdapter(now);
        }

        private string[] EnabledPanelItems()
        {
            return startupAbandoned
                ? new[] { "Retry now", CoopInGameMenuState.DisableLabel, "Close" }
                : new[] { CoopInGameMenuState.DisableLabel, "Close" };
        }

        private void DisableFromPanel()
        {
            string error;
            CoopRuntimeControl.SetEnabled(false, out error);
            string uiError = CoopRuntimeControl.TakeUiLockError();
            if (!string.IsNullOrEmpty(uiError)) host.Notify(SafeText.ForScreen("[F11 Coop] " + uiError));
            panelOpen = false;
            openAdapterMenuOnStart = false;
            int ignored;
            while (keys.TryDequeue(out ignored)) { }
            if (!string.IsNullOrEmpty(error))
                host.LogWarning("GTAxSkate Coop bootstrap: could not save multiplayer preference: " + error);
        }

        private void TryStartAdapter(long nowMs)
        {
            attempts++;
            ClientAdapter candidate = null;
            try
            {
                // Reuse an active RageCoop-loaded owner if one already started in this AppDomain.
                ClientAdapter existing = ClientAdapter.ActiveOwner;
                if (existing != null && existing.IsActive)
                {
                    adapter = existing;
                }
                else
                {
                    candidate = host.CreateAdapter();
                    candidate.OnStart();
                    if (!candidate.IsActive)
                        throw new InvalidOperationException("adapter did not start (multiplayer switched off)");
                    adapter = candidate;
                }
                started = true;
                if (openAdapterMenuOnStart || panelOpen)
                {
                    openAdapterMenuOnStart = false;
                    panelOpen = false;
                    ClientAdapter owner = ClientAdapter.ActiveOwner ?? adapter;
                    if (owner != null && owner.IsActive)
                        owner.HandleMenuKeyCode(VkF11);
                }
            }
            catch (Exception ex)
            {
                lastError = ex.Message;
                try { host.LogWarning("GTAxSkate Coop bootstrap attempt " + attempts + " failed: " + ex.Message); } catch { }
                if (candidate != null)
                {
                    try { candidate.OnStop(); } catch { }
                }
                if (attempts >= MaxStartupAttempts)
                {
                    startupAbandoned = true;
                    if (!abandonNotified)
                    {
                        abandonNotified = true;
                        host.Notify(SafeText.ForScreen("[F11 Coop] Init failed: " + (string.IsNullOrEmpty(ex.Message) ? "unknown error" : ex.Message)));
                    }
                }
                else
                {
                    nextAttemptMs = nowMs + RetrySpacingMs;
                }
            }
        }
    }
}
