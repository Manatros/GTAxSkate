//! Skate's pose written inside GTA's own animation pass: SK8V's pose hook
//! (`host/src/pose_hook.cpp`, Apache-2.0, Copyright 2026 becaring), ported
//! (B86). GTA build 1.0.3889.0.
//!
//! The `hold` thread wrote the pose from outside and raced GTA's animation
//! (about 1100 overwrites in 3 s: stutter, a frame of GTA's pose now and
//! then). Here the pose is written right after GTA wrote its own, on the
//! thread that wrote it:
//! - the player ped's vtable slot 65 (its per-frame animation/IK pass) is
//!   wrapped through a copy of its vtable, and
//! - the two anim job tasks that rewrite the skeleton afterwards on a
//!   worker (+0x15B9948 copies the animated locals and composes the object
//!   matrices, +0x15B8680 rewrites objects -> locals) get a 5-byte jump to a
//!   relay; after GTA's task returns, Skate's pose is written for the
//!   player's skeleton only.
//!
//! Both the object matrices (`crSkeleton + 0x18`) and the locals (`+ 0x10`)
//! are written, consistent with each other. A triple buffer hands the pose
//! from the script thread to the workers; fades blend with GTA's live pose.
//! Everything checks the game's bytes first and stays off if they differ.

use bevy_math::Mat4;
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};

const SLOT: usize = 65; // vtable offset 0x208
const SLOT_RVA: usize = 0x7B_0270; // CPed per-frame animation/IK pass
const COPIED_SLOTS: usize = 512;
const MAX_BONES: usize = 512;
const JOB_RVAS: [usize; 2] = [0x15B_9948, 0x15B_8680];
/// Params offset of each job's locals pointer.
const JOB_LOCALS: [usize; 2] = [0x20, 0x18];
/// mov rax, rsp; push rbp; push rbx; push rsi; push rdi; push r12..r15
const JOB_PROLOGUE: [u8; 15] = [0x48, 0x8B, 0xC4, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57];

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn VirtualAlloc(address: *mut c_void, size: usize, kind: u32, protect: u32) -> *mut c_void;
    fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
    fn FlushInstructionCache(process: *mut c_void, address: *const c_void, size: usize) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
    fn GetTickCount64() -> u64;
}
const MEM_COMMIT_RESERVE: u32 = 0x1000 | 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const PAGE_READWRITE: u32 = 0x04;

/// State only one thread touches at a time (the pass and its two tasks run
/// one after another for the ped), as in SK8V.
struct Shared<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Shared<T> {}
impl<T> Shared<T> {
    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

struct PoseBuffer {
    bones: [Mat4; MAX_BONES],
    count: usize,
}
static BUFFERS: Shared<[PoseBuffer; 3]> = Shared(UnsafeCell::new(
    [const { PoseBuffer { bones: [Mat4::IDENTITY; MAX_BONES], count: 0 } }; 3],
));
static LATEST: AtomicI32 = AtomicI32::new(-1);
static SKELETON: AtomicUsize = AtomicUsize::new(0);
static ENTITY: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_TABLE: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_SLOT: AtomicUsize = AtomicUsize::new(0);
static COPY: AtomicUsize = AtomicUsize::new(0);

/// Hand-off fade: +1 in (GTA -> Skate), -1 out (Skate -> GTA), 0 none.
static FADE_DIR: AtomicI32 = AtomicI32::new(0);
static FADE_MS: AtomicI32 = AtomicI32::new(0);
static FADE_START: AtomicU64 = AtomicU64::new(0);
static FREEZE: AtomicBool = AtomicBool::new(false);
static OUT_DONE: AtomicBool = AtomicBool::new(false);

struct Writer {
    frozen: bool,
    have_written: bool,
    frozen_pose: [Mat4; MAX_BONES],
    written: [Mat4; MAX_BONES],
    object: [Mat4; MAX_BONES],
}
static WRITER: Shared<Writer> = Shared(UnsafeCell::new(Writer {
    frozen: false,
    have_written: false,
    frozen_pose: [Mat4::IDENTITY; MAX_BONES],
    written: [Mat4::IDENTITY; MAX_BONES],
    object: [Mat4::IDENTITY; MAX_BONES],
}));

struct Job {
    target: usize,
    original: u64,
    trampoline: usize,
}
static JOBS: Shared<[Job; 2]> = Shared(UnsafeCell::new([const { Job { target: 0, original: 0, trampoline: 0 } }; 2]));
static RELAY: AtomicUsize = AtomicUsize::new(0);

fn now_ms() -> u64 {
    unsafe { GetTickCount64() }
}

/// Skate's share of the written pose now (1 outside a fade).
fn skate_weight() -> f32 {
    let dir = FADE_DIR.load(Ordering::Acquire);
    if dir == 0 {
        return 1.;
    }
    let ms = FADE_MS.load(Ordering::Relaxed);
    let t = if ms > 0 { (now_ms().saturating_sub(FADE_START.load(Ordering::Relaxed))) as f32 / ms as f32 } else { 1. };
    let u = t.clamp(0., 1.);
    if dir > 0 { u } else { 1. - u }
}

unsafe fn load(at: usize) -> Mat4 {
    unsafe { Mat4::from_cols_array(&*(at as *const [f32; 16])) }
}

/// Writes the rotation and place of `m` (each matrix row's x, y, z; GTA's
/// row w stays as it is).
unsafe fn store_xyz(at: usize, m: &Mat4) {
    let d = at as *mut f32;
    let cols = m.to_cols_array();
    for row in 0..4 {
        for c in 0..3 {
            unsafe { d.add(row * 4 + c).write(cols[row * 4 + c]) };
        }
    }
}

fn blend(a: &Mat4, b: &Mat4, t: f32) -> Mat4 {
    let (_, qa, ta) = a.to_scale_rotation_translation();
    let (_, qb, tb) = b.to_scale_rotation_translation();
    Mat4::from_rotation_translation(qa.normalize().slerp(qb.normalize(), t).normalize(), ta.lerp(tb, t))
}

/// After GTA's pass: object matrices (and matching locals) from Skate's pose.
unsafe fn apply(s: usize) {
    unsafe {
        let data = *(s as *const usize);
        let parent = *((s + 0x08) as *const usize);
        let locals = *((s + 0x10) as *const usize);
        let objects = *((s + 0x18) as *const usize);
        let count = *((s + 0x20) as *const i32);
        if data == 0 || parent == 0 || locals == 0 || objects == 0 || count <= 0 || count as usize > MAX_BONES {
            return;
        }
        let count = count as usize;
        let parents = *((data + 0x38) as *const *const i16);
        if parents.is_null() {
            return;
        }
        let latest = LATEST.load(Ordering::Acquire);
        if latest < 0 {
            return;
        }
        let buffer = &BUFFERS.get()[latest as usize];
        if buffer.count != count {
            return; // a pose published for this skeleton only
        }
        let w = WRITER.get();
        let weight = skate_weight();
        let out = FADE_DIR.load(Ordering::Relaxed) < 0;
        if out {
            if FREEZE.swap(false, Ordering::AcqRel) {
                w.frozen = false;
            }
            if weight <= 0. {
                OUT_DONE.store(true, Ordering::Release);
                return; // GTA's pose alone
            }
        }
        if out && w.frozen {
            w.object[..count].copy_from_slice(&w.frozen_pose[..count]);
        } else {
            w.object[..count].copy_from_slice(&buffer.bones[..count]);
            if out {
                w.frozen_pose[..count].copy_from_slice(&w.object[..count]);
                w.frozen = true;
            }
        }
        if weight < 1. {
            // A later writer of the same frame finds our blend in place:
            // blending again would compound the weight.
            let ours = w.have_written
                && (0..count).all(|i| {
                    let cur = load(objects + 64 * i).to_cols_array();
                    let was = w.written[i].to_cols_array();
                    (0..4).all(|r| (0..3).all(|k| (cur[r * 4 + k] - was[r * 4 + k]).abs() < 1e-5))
                });
            if ours {
                return;
            }
            for i in 0..count {
                w.object[i] = blend(&load(objects + 64 * i), &w.object[i], weight);
            }
        }
        w.written[..count].copy_from_slice(&w.object[..count]);
        w.have_written = true;
        for i in 0..count {
            store_xyz(objects + 64 * i, &w.object[i]);
            let p = *parents.add(i);
            let local = if i > 0 && p >= 0 && (p as usize) < count {
                let inv = w.object[p as usize].inverse();
                if inv.is_finite() { inv * w.object[i] } else { w.object[i] }
            } else {
                w.object[i]
            };
            store_xyz(locals + 64 * i, &local);
        }
    }
}

type SlotFn = unsafe extern "C" fn(usize, bool) -> bool;
type JobFn = unsafe extern "C" fn(usize, usize, usize, usize) -> usize;

unsafe extern "C" fn slot(this: usize, flag: bool) -> bool {
    let original: SlotFn = unsafe { std::mem::transmute(ORIGINAL_SLOT.load(Ordering::Acquire)) };
    let result = unsafe { original(this, flag) };
    if this == ENTITY.load(Ordering::Relaxed) {
        let s = SKELETON.load(Ordering::Relaxed);
        if s != 0 {
            unsafe { apply(s) };
        }
    }
    result
}

unsafe fn job_wrap(index: usize, params: usize, b: usize, c: usize, d: usize) -> usize {
    let job = &JOBS.get()[index];
    let original: JobFn = unsafe { std::mem::transmute(job.trampoline) };
    let result = unsafe { original(params, b, c, d) };
    let s = SKELETON.load(Ordering::Acquire);
    if s != 0 && params != 0 {
        unsafe {
            let locals = *((s + 0x10) as *const usize);
            if locals != 0 && *((params + JOB_LOCALS[index]) as *const usize) == locals {
                apply(s);
            }
        }
    }
    result
}
unsafe extern "C" fn job0(params: usize, b: usize, c: usize, d: usize) -> usize {
    unsafe { job_wrap(0, params, b, c, d) }
}
unsafe extern "C" fn job1(params: usize, b: usize, c: usize, d: usize) -> usize {
    unsafe { job_wrap(1, params, b, c, d) }
}

fn base() -> usize {
    unsafe { GetModuleHandleW(std::ptr::null()) as usize }
}

unsafe fn patch_qword(at: usize, value: u64) {
    unsafe {
        let mut old = 0u32;
        VirtualProtect(at as *mut c_void, 8, PAGE_EXECUTE_READWRITE, &mut old);
        (*(at as *const AtomicU64)).store(value, Ordering::SeqCst);
        VirtualProtect(at as *mut c_void, 8, old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), at as *const c_void, 8);
    }
}

/// Executable memory within a 32-bit jump of `origin`.
fn alloc_near(origin: usize, size: usize) -> usize {
    const GRANULARITY: usize = 0x1_0000;
    let mut delta = 0x1000_0000usize;
    while delta < 0x7000_0000 {
        for a in [origin.wrapping_sub(delta), origin + delta] {
            let p = unsafe { VirtualAlloc((a & !(GRANULARITY - 1)) as *mut c_void, size, MEM_COMMIT_RESERVE, PAGE_EXECUTE_READWRITE) };
            if !p.is_null() {
                return p as usize;
            }
        }
        delta += GRANULARITY * 16;
    }
    0
}

/// Detours the two anim job tasks; false (and nothing changed) when the
/// game's bytes are not the expected ones.
fn install_jobs(log: &mut dyn FnMut(&str)) -> bool {
    let jobs = JOBS.get();
    if jobs[0].target != 0 {
        return true;
    }
    let base = base();
    for rva in JOB_RVAS {
        let t = base + rva;
        let bytes = unsafe { std::slice::from_raw_parts(t as *const u8, JOB_PROLOGUE.len()) };
        if t & 7 != 0 || bytes != JOB_PROLOGUE {
            log(&format!("pose hook: anim task +{rva:#x} differs from build 3889; job hooks not installed"));
            return false;
        }
    }
    let mut mem = RELAY.load(Ordering::Acquire); // reused on reinstall, never freed
    if mem == 0 {
        mem = alloc_near(base, 4096);
        RELAY.store(mem, Ordering::Release);
    }
    if mem == 0 {
        log("pose hook: no memory near the game image; job hooks not installed");
        return false;
    }
    for rva in JOB_RVAS {
        let distance = mem as i64 - (base + rva + 5) as i64;
        if distance < i32::MIN as i64 + 4096 || distance > i32::MAX as i64 - 4096 {
            log("pose hook: relay memory out of jump range; job hooks not installed");
            return false;
        }
    }
    let wraps = [job0 as usize, job1 as usize];
    for (i, rva) in JOB_RVAS.into_iter().enumerate() {
        let t = base + rva;
        unsafe {
            // relay: jmp [rip+0]; dq wrap
            let relay = (mem + 64 * i) as *mut u8;
            relay.write(0xFF);
            relay.add(1).write(0x25);
            std::ptr::write_bytes(relay.add(2), 0, 4);
            std::ptr::copy_nonoverlapping(wraps[i].to_le_bytes().as_ptr(), relay.add(6), 8);
            // trampoline: the 5 prologue bytes; jmp [rip+0]; dq t+5
            let tramp = (mem + 64 * i + 32) as *mut u8;
            std::ptr::copy_nonoverlapping(JOB_PROLOGUE.as_ptr(), tramp, 5);
            tramp.add(5).write(0xFF);
            tramp.add(6).write(0x25);
            std::ptr::write_bytes(tramp.add(7), 0, 4);
            std::ptr::copy_nonoverlapping((t + 5).to_le_bytes().as_ptr(), tramp.add(11), 8);
            jobs[i].trampoline = tramp as usize;
            jobs[i].original = *(t as *const u64);
        }
    }
    unsafe { FlushInstructionCache(GetCurrentProcess(), mem as *const c_void, 4096) };
    for (i, rva) in JOB_RVAS.into_iter().enumerate() {
        let t = base + rva;
        let mut patch = jobs[i].original.to_le_bytes();
        let rel = ((mem + 64 * i) as i64 - (t + 5) as i64) as i32;
        patch[0] = 0xE9;
        patch[1..5].copy_from_slice(&rel.to_le_bytes());
        jobs[i].target = t;
        unsafe { patch_qword(t, u64::from_le_bytes(patch)) };
    }
    log("pose hook: anim job tasks +0x15B9948 and +0x15B8680 detoured; Skate's pose follows them on the worker");
    true
}

fn uninstall_jobs(log: &mut dyn FnMut(&str)) {
    let jobs = JOBS.get();
    if jobs[0].target == 0 {
        return;
    }
    for job in jobs.iter_mut() {
        unsafe { patch_qword(job.target, job.original) };
        job.target = 0;
    }
    // Relays and trampolines stay: a worker may still be inside one.
    log("pose hook: anim job tasks restored");
}

/// Wraps the ped `entity`'s animation pass and the anim job tasks for its
/// skeleton `skeleton` (`crSkeleton`). False when the game differs from
/// build 3889 (the caller then keeps the `hold` thread).
pub fn install(entity: usize, skeleton: usize, log: &mut dyn FnMut(&str)) -> bool {
    if entity == 0 || skeleton == 0 {
        return false;
    }
    SKELETON.store(skeleton, Ordering::Release);
    if ENTITY.load(Ordering::Acquire) == entity {
        return true;
    }
    if ENTITY.load(Ordering::Acquire) != 0 {
        uninstall(log);
        SKELETON.store(skeleton, Ordering::Release);
    }
    let base = base();
    let table = unsafe { *(entity as *const usize) };
    let original = unsafe { *((table + 8 * SLOT) as *const usize) };
    if original != base + SLOT_RVA {
        log(&format!(
            "pose hook: ped vtable slot {SLOT} is image +{:#x}, expected +{SLOT_RVA:#x}; not installed",
            original.wrapping_sub(base)
        ));
        SKELETON.store(0, Ordering::Release);
        return false;
    }
    if !install_jobs(log) {
        SKELETON.store(0, Ordering::Release);
        return false;
    }
    let copy = unsafe { VirtualAlloc(std::ptr::null_mut(), 8 * (COPIED_SLOTS + 1), MEM_COMMIT_RESERVE, PAGE_READWRITE) } as *mut usize;
    if copy.is_null() {
        return false;
    }
    unsafe {
        // RTTI locator + slots.
        std::ptr::copy_nonoverlapping((table - 8) as *const usize, copy, COPIED_SLOTS + 1);
        *copy.add(1 + SLOT) = slot as usize;
    }
    // A pose left from an earlier session belongs to another ped: until
    // this one publishes, GTA's own pose stands.
    LATEST.store(-1, Ordering::Release);
    ORIGINAL_SLOT.store(original, Ordering::Release);
    ORIGINAL_TABLE.store(table, Ordering::Release);
    COPY.store(copy as usize, Ordering::Release);
    ENTITY.store(entity, Ordering::Release);
    unsafe { (*(entity as *const AtomicUsize)).store(copy.add(1) as usize, Ordering::SeqCst) };
    log("pose hook: the player's animation pass (vtable slot 65) wrapped; Skate's pose follows it");
    true
}

/// Restores the ped's vtable and the job tasks.
pub fn uninstall(log: &mut dyn FnMut(&str)) {
    SKELETON.store(0, Ordering::Release);
    FADE_DIR.store(0, Ordering::Release);
    WRITER.get().have_written = false;
    uninstall_jobs(log);
    let entity = ENTITY.swap(0, Ordering::AcqRel);
    if entity == 0 {
        return;
    }
    let slot0 = unsafe { &*(entity as *const AtomicUsize) };
    if slot0.load(Ordering::Acquire) == COPY.load(Ordering::Acquire) + 8 {
        slot0.store(ORIGINAL_TABLE.load(Ordering::Acquire), Ordering::SeqCst);
    }
    // The copied table stays allocated: a game thread may still use it.
    log("pose hook: the ped's own vtable restored");
}

/// Whether the hook is on for `entity`.
pub fn installed_for(entity: usize) -> bool {
    entity != 0 && ENTITY.load(Ordering::Acquire) == entity
}

/// Script thread: this frame's pose, each bone relative to the entity as
/// it was just placed (`objects`, the skeleton's bone order).
pub fn publish(objects: &[Mat4]) {
    let count = objects.len().min(MAX_BONES);
    if SKELETON.load(Ordering::Acquire) == 0 || !objects.iter().all(|m| m.is_finite()) {
        return;
    }
    // The buffer after the published one: readers only hold the published.
    let next = (LATEST.load(Ordering::Relaxed) + 1).rem_euclid(3) as usize;
    let buffer = &mut BUFFERS.get()[next];
    buffer.bones[..count].copy_from_slice(&objects[..count]);
    buffer.count = count;
    LATEST.store(next as i32, Ordering::Release);
}

/// GTA's pose to Skate's over `ms`.
pub fn fade_in(ms: i32) {
    OUT_DONE.store(false, Ordering::Release);
    FADE_MS.store(ms, Ordering::Relaxed);
    FADE_START.store(now_ms(), Ordering::Relaxed);
    FADE_DIR.store(1, Ordering::Release);
}

/// Fades in, unless a fade in is running or done (each frame while
/// skating: after a fade out, or at the first pose).
pub fn fade_in_unless_in(ms: i32) {
    if FADE_DIR.load(Ordering::Acquire) <= 0 {
        fade_in(ms);
    }
}

/// Skate's last pose to GTA's over `ms` (Skate's frozen where it was).
pub fn fade_out(ms: i32) {
    OUT_DONE.store(false, Ordering::Release);
    FADE_MS.store(ms, Ordering::Relaxed);
    FADE_START.store(now_ms(), Ordering::Relaxed);
    FREEZE.store(true, Ordering::Release);
    FADE_DIR.store(-1, Ordering::Release);
}
