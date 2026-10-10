//! Holding Skate's pose on a GTA character against GTA's own animation.
//!
//! GTA animates and skins its characters after scripts have run, so a pose
//! written in the script's turn is overwritten before the frame is drawn:
//! the character shows GTA's own pose (a T-pose for the frozen player), and
//! written again just before a frame is shown it flickers between the two.
//! A thread watches the bone arrays instead and writes the pose again the
//! moment GTA has written its own, so what GTA skins the character with is
//! Skate's pose. The script hands it the newest pose every frame; a pose not
//! renewed for a moment, or whose character went away, is dropped.

use super::memory::Written;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Whose pose: the player's (skating or in a board shop), or the co-op
/// partner's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Player = 0,
    Remote = 1,
}

struct Held {
    pose: Arc<Written>,
    until: Instant,
}

/// The poses held. The lock is only ever held for a moment: the thread
/// takes the poses out to check and write them. Held while it checked and
/// wrote (~3800 times a second), it kept the script waiting up to 100 ms
/// a frame to hand over the next pose.
static SLOTS: Mutex<[Option<Held>; 2]> = Mutex::new([None, None]);
static RUNNING: AtomicBool = AtomicBool::new(false);
static REWRITES: AtomicU64 = AtomicU64::new(0);
static CHECKS: AtomicU64 = AtomicU64::new(0);

/// A pose not renewed for this long is dropped.
const KEEP: Duration = Duration::from_millis(150);

/// Holds `pose` on its character until the next one (or a moment), with
/// the watching thread when `watch` (else the present callback writes it).
pub fn set(slot: Slot, pose: Written, watch: bool) {
    let held = Held { pose: Arc::new(pose), until: Instant::now() + KEEP };
    if let Ok(mut slots) = SLOTS.lock() {
        slots[slot as usize] = Some(held);
    }
    if watch && !RUNNING.swap(true, Ordering::AcqRel) {
        if std::thread::Builder::new().name("skate-gta-pose".into()).spawn(run).is_err() {
            RUNNING.store(false, Ordering::Release);
        } else {
            super::crash::did("the pose thread started");
        }
    }
}

/// Lets go of the pose in `slot`: GTA's own animation shows again.
pub fn clear(slot: Slot) {
    if let Ok(mut slots) = SLOTS.lock() {
        slots[slot as usize] = None;
    }
}

/// The poses held now and until when, taken out of the lock.
fn current() -> Vec<(usize, Arc<Written>, Instant)> {
    let Ok(slots) = SLOTS.lock() else { return Vec::new() };
    slots
        .iter()
        .enumerate()
        .filter_map(|(i, held)| held.as_ref().map(|h| (i, Arc::clone(&h.pose), h.until)))
        .collect()
}

/// Writes the held poses once (the present callback's way).
pub fn write_now() {
    let now = Instant::now();
    for (_, pose, until) in current() {
        if now <= until && pose.still_there() {
            pose.write();
        }
    }
}

/// How many times the thread looked at the held poses so far.
pub fn checks() -> u64 {
    CHECKS.load(Ordering::Relaxed)
}

/// How many times GTA wrote its own pose over a held one so far.
pub fn rewrites() -> u64 {
    REWRITES.load(Ordering::Relaxed)
}

fn run() {
    let mut idle_since: Option<Instant> = None;
    loop {
        let mut active = false;
        let now = Instant::now();
        for (slot, pose, until) in current() {
            if now > until || !pose.still_there() {
                // Dropped unless a newer pose came meanwhile.
                if let Ok(mut slots) = SLOTS.lock()
                    && slots[slot].as_ref().is_some_and(|h| Arc::ptr_eq(&h.pose, &pose))
                {
                    slots[slot] = None;
                }
                continue;
            }
            active = true;
            CHECKS.fetch_add(1, Ordering::Relaxed);
            if pose.overwritten() {
                pose.write();
                REWRITES.fetch_add(1, Ordering::Relaxed);
            }
        }
        if active {
            idle_since = None;
            // Back within microseconds: GTA skins the character soon after
            // it animates it.
            std::thread::yield_now();
        } else {
            let since = *idle_since.get_or_insert_with(Instant::now);
            if since.elapsed() > Duration::from_secs(2) {
                RUNNING.store(false, Ordering::Release);
                // A pose set while stopping: carry on unless that started
                // another thread.
                let pending = SLOTS.lock().map(|s| s.iter().any(Option::is_some)).unwrap_or(false);
                if pending && !RUNNING.swap(true, Ordering::AcqRel) {
                    idle_since = None;
                    continue;
                }
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}