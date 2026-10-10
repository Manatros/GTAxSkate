//! Finding GTA's physics level table: a Rust port of SK8V's
//! `host/src/physics_level.cpp` (https://github.com/becaring/SK8V,
//! Apache-2.0, Copyright 2026 becaring; adapted there from Sol4ra's
//! LS-Skate-LiveCollision).
//!
//! Every private read-write cell holding one of the physics instances GTA's
//! own peds, vehicles and objects have (exact or with state bits) votes for
//! the table base its level index (`inst + 0x18`) implies; the base most
//! voted for, by nearly every instance, is the table. Only pages in RAM are
//! read (a page out on disk stalled the game for seconds).

use std::collections::HashMap;
use std::ffi::c_void;

const MASK: u64 = 0x7FFF_FFFF_FFF0;
const STRIDE: usize = 0x30;

#[repr(C)]
struct MemoryInfo {
    base: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WorkingSetEx {
    address: *mut c_void,
    attributes: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualQuery(address: *const c_void, info: *mut MemoryInfo, length: usize) -> usize;
    fn ReadProcessMemory(process: isize, base: *const c_void, buffer: *mut c_void, size: usize, read: *mut usize) -> i32;
    fn GetCurrentProcess() -> isize;
    fn K32QueryWorkingSetEx(process: isize, info: *mut c_void, size: u32) -> i32;
}

const MEM_COMMIT: u32 = 0x1000;
const MEM_PRIVATE: u32 = 0x20000;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_GUARD: u32 = 0x100;

fn read(address: usize, out: &mut [u8]) -> usize {
    let mut got = 0usize;
    unsafe { ReadProcessMemory(GetCurrentProcess(), address as *const c_void, out.as_mut_ptr().cast(), out.len(), &mut got) };
    got
}

/// The table's base address, from the physics instances (`fragInst`s) of
/// entities in the world; none when no base is voted for by at least half
/// of them.
pub fn locate(instances: &[u64]) -> Option<usize> {
    let mut insts: Vec<u64> = instances.to_vec();
    insts.sort_unstable();
    insts.dedup();
    if insts.len() < 8 {
        return None;
    }
    let index: Vec<Option<u32>> = insts
        .iter()
        .map(|&i| {
            let mut b = [0u8; 2];
            (read(i as usize + 0x18, &mut b) == 2).then(|| u16::from_le_bytes(b) as u32)
        })
        .collect();
    let indexed = index.iter().filter(|i| i.is_some()).count() as u32;
    let settled = (indexed - indexed / 10).max(8);
    let (lo, hi) = (insts[0], *insts.last().unwrap());
    let own = (insts.as_ptr() as usize, insts.as_ptr() as usize + insts.len() * 8);
    let mut votes: HashMap<usize, u32> = HashMap::new();
    let (mut best, mut top) = (0usize, 0u32);
    const PAGE: usize = 0x1000;
    const CHUNK: usize = 1 << 20;
    let mut buf = vec![0u8; CHUNK];
    let mut ws = vec![WorkingSetEx { address: std::ptr::null_mut(), attributes: 0 }; CHUNK / PAGE];
    let mut at = 0x10000usize;
    while top < settled && at < 0x7FFF_FFFF_0000 {
        let mut info = std::mem::MaybeUninit::<MemoryInfo>::zeroed();
        if unsafe { VirtualQuery(at as *const c_void, info.as_mut_ptr(), size_of::<MemoryInfo>()) } == 0 {
            break;
        }
        let info = unsafe { info.assume_init() };
        let (region, size) = (info.base as usize, info.region_size);
        let next = region + size;
        if next <= at {
            break;
        }
        at = next;
        if info.state != MEM_COMMIT || info.kind != MEM_PRIVATE || info.protect & PAGE_GUARD != 0 || info.protect & PAGE_READWRITE == 0 {
            continue;
        }
        let mut p = region;
        while p < next && top < settled {
            let want = CHUNK.min(next - p);
            let pages = want / PAGE;
            for (k, w) in ws.iter_mut().take(pages).enumerate() {
                *w = WorkingSetEx { address: (p + k * PAGE) as *mut c_void, attributes: 0 };
            }
            let known = unsafe { K32QueryWorkingSetEx(GetCurrentProcess(), ws.as_mut_ptr().cast(), (pages * size_of::<WorkingSetEx>()) as u32) } != 0;
            let mut k0 = 0;
            while k0 < pages {
                if known && ws[k0].attributes & 1 == 0 {
                    k0 += 1;
                    continue;
                }
                let mut k1 = k0 + 1;
                while k1 < pages && (!known || ws[k1].attributes & 1 != 0) {
                    k1 += 1;
                }
                let run = p + k0 * PAGE;
                let got = read(run, &mut buf[..(k1 - k0) * PAGE]);
                let mut o = 0;
                while o + 8 <= got {
                    let v = u64::from_le_bytes(buf[o..o + 8].try_into().unwrap());
                    let m = v & MASK;
                    if m >= lo && m <= hi {
                        let cell = run + o;
                        if (cell < own.0 || cell >= own.1)
                            && let Ok(k) = insts.binary_search(&m)
                            && let Some(ix) = index[k]
                            && cell >= ix as usize * STRIDE
                        {
                            let base = cell - ix as usize * STRIDE;
                            let n = votes.entry(base).or_default();
                            *n += 1;
                            if *n > top {
                                top = *n;
                                best = base;
                            }
                        }
                    }
                    o += 8;
                }
                k0 = k1;
            }
            p += want;
        }
    }
    (top as usize >= (insts.len() / 2).max(8)).then_some(best)
}
