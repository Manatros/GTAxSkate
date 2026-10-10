//! A faster start: GTA's Rockstar logo film and its legal notices skipped
//! (`FastStart` in SkateGTA.ini). Both are found in the running game the
//! first moment they can be, from a thread the plugin starts as it loads:
//!
//! - the logo film, by its path in the game's data: renamed to a film that
//!   is not there, which the game skips as it skips a missing film;
//! - the legal notices, by the time they stay up (as FiveM finds it, from
//!   the instruction that reads it): set to nothing.
//!
//! Each is changed only when it is found exactly once where it should be;
//! a game version where it is not stays as it is.

use super::memory::readable;
use std::ffi::c_void;
use std::time::{Duration, Instant};

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
    fn GetModuleHandleA(name: *const std::ffi::c_char) -> *mut c_void;
}

const PAGE_READWRITE: u32 = 0x04;
const LOGO_FILM: &[u8] = b"platform:/movies/rockstar_logos";
/// `xorps xmm1, xmm1; sub rax, rcx; lea rcx, [rip + legal time]`.
const LEGAL_TIME: [Option<u8>; 9] =
    [Some(0x0F), Some(0x57), Some(0xC9), Some(0x48), Some(0x2B), Some(0xC1), Some(0x48), Some(0x8D), Some(0x0D)];

/// Starts looking (once, as the plugin loads) when the ini asks for it.
pub fn start(folder: std::path::PathBuf) {
    let _ = std::thread::Builder::new().name("skate-gta-fast-start".into()).spawn(move || {
        let ini = std::fs::read_to_string(folder.join("SkateGTA.ini")).unwrap_or_default();
        let off = ini.lines().any(|line| {
            let line = line.trim().to_ascii_lowercase();
            line.starts_with("faststart") && line.split('=').nth(1).is_some_and(|v| v.trim() == "off")
        });
        if !off {
            run();
        }
    });
}

fn image() -> Option<(usize, usize)> {
    let base = unsafe { GetModuleHandleA(std::ptr::null()) } as usize;
    if !readable(base, 0x400) {
        return None;
    }
    let header = unsafe { std::ptr::read_unaligned((base + 0x3C) as *const u32) } as usize;
    if !readable(base + header + 0x54, 4) {
        return None;
    }
    let size = unsafe { std::ptr::read_unaligned((base + header + 0x50) as *const u32) } as usize;
    Some((base, size))
}

/// Every place `pattern` (None: any byte) is in the readable parts of the
/// image, up to `most`.
fn find(base: usize, size: usize, pattern: &[Option<u8>], most: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let page = 0x10000;
    let mut start = 0;
    while start < size && found.len() < most {
        let end = (start + page + pattern.len()).min(size);
        if readable(base + start, end - start) {
            let bytes = unsafe { std::slice::from_raw_parts((base + start) as *const u8, end - start) };
            for (i, window) in bytes.windows(pattern.len()).enumerate() {
                if i >= page {
                    break;
                }
                if window.iter().zip(pattern).all(|(b, p)| p.is_none_or(|p| p == *b)) {
                    found.push(base + start + i);
                    if found.len() >= most {
                        break;
                    }
                }
            }
        }
        start += page;
    }
    found
}

fn write(at: usize, bytes: &[u8]) -> bool {
    let mut old = 0u32;
    if unsafe { VirtualProtect(at as *mut c_void, bytes.len(), PAGE_READWRITE, &mut old) } == 0 {
        return false;
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len()) };
    let mut back = 0u32;
    unsafe { VirtualProtect(at as *mut c_void, bytes.len(), old, &mut back) };
    true
}

fn run() {
    let started = Instant::now();
    let (mut logo, mut legal) = (false, false);
    let logo_pattern: Vec<Option<u8>> = LOGO_FILM.iter().map(|b| Some(*b)).collect();
    while !(logo && legal) && started.elapsed() < Duration::from_secs(60) {
        if let Some((base, size)) = image() {
            if !logo {
                let places = find(base, size, &logo_pattern, 4);
                if !places.is_empty() {
                    // "rockstar_logos" becomes "rockstar_logoX": no such film.
                    for at in &places {
                        write(at + LOGO_FILM.len() - 1, b"X");
                    }
                    logo = true;
                }
            }
            if !legal {
                let places = find(base, size, &LEGAL_TIME, 2);
                if places.len() == 1 {
                    let operand = places[0] + LEGAL_TIME.len();
                    let offset = unsafe { std::ptr::read_unaligned(operand as *const i32) };
                    let time = (operand as i64 + 4 + i64::from(offset)) as usize;
                    // A time of a few seconds, inside the game's image.
                    if time > base && time + 4 < base + size && readable(time, 4) {
                        let value = unsafe { std::ptr::read_unaligned(time as *const u32) };
                        if value < 60_000 {
                            write(time, &0u32.to_le_bytes());
                        }
                        legal = true;
                    }
                } else if places.len() > 1 {
                    // Not the one place it should be: left alone.
                    legal = true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
