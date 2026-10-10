//! When GTA crashes: where, and what the plugin was doing, in
//! `SkateGTA-crash.txt` beside it (the guided test sends it along).
//!
//! The script marks what it is about to do ([`phase`]); a crash handler
//! writes the exception, the module and offset it happened in, and the
//! last mark, then hands the crash on to whatever handled it before.

use std::ffi::{c_char, c_void};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

#[repr(C)]
struct ExceptionRecord {
    code: u32,
    flags: u32,
    record: *mut ExceptionRecord,
    address: *mut c_void,
    parameters: u32,
    information: [usize; 15],
}

#[repr(C)]
struct ExceptionPointers {
    record: *mut ExceptionRecord,
    context: *mut c_void,
}

type Filter = Option<unsafe extern "system" fn(*mut ExceptionPointers) -> i32>;

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(filter: Filter) -> Filter;
    fn GetModuleHandleExA(flags: u32, address: *const c_char, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameA(module: *mut c_void, name: *mut c_char, size: u32) -> u32;
    fn GetCurrentThreadId() -> u32;
}

const FROM_ADDRESS: u32 = 0x4;
const UNCHANGED_REFCOUNT: u32 = 0x2;
const CONTINUE_SEARCH: i32 = 0;

static PATH: OnceLock<std::path::PathBuf> = OnceLock::new();
static PREVIOUS: AtomicUsize = AtomicUsize::new(0);
static PHASE: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());
static PHASE_LEN: AtomicUsize = AtomicUsize::new(0);
static MAIN_THREAD: AtomicU32 = AtomicU32::new(0);
static STARTED: OnceLock<std::time::Instant> = OnceLock::new();

/// What the script is doing now (a crash reports the last one). Also times
/// each phase, for [`take_times`].
pub fn phase(what: &'static str) {
    PHASE_LEN.store(0, Ordering::Relaxed);
    PHASE.store(what.as_ptr() as *mut u8, Ordering::Relaxed);
    PHASE_LEN.store(what.len(), Ordering::Relaxed);
    if let Ok(mut times) = TIMES.try_lock() {
        let now = std::time::Instant::now();
        if let Some((since, last)) = times.0.replace((now, what)) {
            let spent = now - since;
            match times.1.iter_mut().find(|(name, _)| std::ptr::eq(*name, last)) {
                Some(entry) => entry.1 += spent,
                None => times.1.push((last, spent)),
            }
        }
    }
}

/// The phase running since when, and the time spent in each phase so far.
type Times = (Option<(std::time::Instant, &'static str)>, Vec<(&'static str, std::time::Duration)>);
static TIMES: std::sync::Mutex<Times> = std::sync::Mutex::new((None, Vec::new()));

/// The time spent in each phase since the last call, longest first.
pub fn take_times() -> Vec<(&'static str, std::time::Duration)> {
    let Ok(mut times) = TIMES.lock() else { return Vec::new() };
    let mut taken = std::mem::take(&mut times.1);
    taken.sort_by(|a, b| b.1.cmp(&a.1));
    taken
}

/// What the plugin last did to the game (beyond its own skater): the
/// most recent few, with when.
static DID: std::sync::Mutex<Vec<(std::time::Instant, &'static str)>> = std::sync::Mutex::new(Vec::new());

pub fn did(what: &'static str) {
    if let Ok(mut did) = DID.try_lock() {
        if did.len() >= 8 {
            did.remove(0);
        }
        did.push((std::time::Instant::now(), what));
    }
}

/// Installs the handler (once, from the script's thread).
pub fn install(folder: &std::path::Path) {
    if PATH.set(folder.join("SkateGTA-crash.txt")).is_err() {
        return;
    }
    let _ = STARTED.set(std::time::Instant::now());
    MAIN_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    let previous = unsafe { SetUnhandledExceptionFilter(Some(on_crash)) };
    PREVIOUS.store(previous.map_or(0, |f| f as usize), Ordering::Relaxed);
}

fn module_of(address: usize) -> (String, usize) {
    let mut module: *mut c_void = std::ptr::null_mut();
    let ok = unsafe { GetModuleHandleExA(FROM_ADDRESS | UNCHANGED_REFCOUNT, address as *const c_char, &mut module) };
    if ok == 0 || module.is_null() {
        return ("(no module)".into(), address);
    }
    let mut name = [0u8; 260];
    let n = unsafe { GetModuleFileNameA(module, name.as_mut_ptr().cast(), name.len() as u32) } as usize;
    let path = String::from_utf8_lossy(&name[..n.min(name.len())]).into_owned();
    let file = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
    (file, address - module as usize)
}

/// The registers, the code around the crash (GTA5.exe is encrypted on disk,
/// so only the running game's bytes can be disassembled) and every address on
/// the stack that lies in a module: the calls that led there.
fn machine_state(context: usize) -> String {
    use super::memory::readable;
    // x64 CONTEXT: Rax at 0x78 .. R15 at 0xF0, then Rip.
    const NAMES: [&str; 17] =
        ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15", "rip"];
    if !readable(context + 0x78, NAMES.len() * 8) {
        return String::new();
    }
    let reg = |i: usize| unsafe { std::ptr::read_unaligned((context + 0x78 + i * 8) as *const usize) };
    let mut text = String::from("registers:\n");
    for (i, name) in NAMES.iter().enumerate() {
        let value = reg(i);
        let (module, offset) = module_of(value);
        let place = if module == "(no module)" { String::new() } else { format!("  ({module}+0x{offset:X})") };
        text.push_str(&format!("  {name:<3} = 0x{value:016X}{place}\n"));
    }
    let (rip, rsp) = (reg(16), reg(4));
    let (before, after) = (64, 48);
    if readable(rip.wrapping_sub(before), before + after) {
        let bytes = unsafe { std::slice::from_raw_parts((rip - before) as *const u8, before + after) };
        text.push_str(&format!("code from rip-0x{before:X}:\n "));
        for b in bytes {
            text.push_str(&format!(" {b:02X}"));
        }
        text.push('\n');
    }
    text.push_str("stack (addresses in modules, from rsp):\n");
    let mut shown = 0;
    for slot in 0..4096usize {
        let at = rsp + slot * 8;
        if shown >= 40 || !readable(at, 8) {
            break;
        }
        let value = unsafe { std::ptr::read_unaligned(at as *const usize) };
        if value < 0x10000 {
            continue;
        }
        let (module, offset) = module_of(value);
        if module != "(no module)" {
            text.push_str(&format!("  rsp+0x{:X}: {module}+0x{offset:X}\n", slot * 8));
            shown += 1;
        }
    }
    text
}

unsafe extern "system" fn on_crash(info: *mut ExceptionPointers) -> i32 {
    if let (Some(path), Some(record)) = (PATH.get(), unsafe { info.as_ref() }.and_then(|i| unsafe { i.record.as_ref() })) {
        let (module, offset) = module_of(record.address as usize);
        let phase = {
            let (p, n) = (PHASE.load(Ordering::Relaxed), PHASE_LEN.load(Ordering::Relaxed));
            if p.is_null() || n == 0 {
                "(none yet)"
            } else {
                std::str::from_utf8(unsafe { std::slice::from_raw_parts(p, n) }).unwrap_or("?")
            }
        };
        let thread = unsafe { GetCurrentThreadId() };
        let main = MAIN_THREAD.load(Ordering::Relaxed);
        let mut text = format!(
            "GTA crashed: exception 0x{:08X} at {module}+0x{offset:X}\n\
             thread: {}\n\
             the Skate plugin was last: {phase}\n\
             seconds since the plugin started: {:.1}\n",
            record.code,
            if thread == main { "the script's".to_owned() } else { format!("another ({thread})") },
            STARTED.get().map_or(0., |s| s.elapsed().as_secs_f32()),
        );
        if let Ok(did) = DID.try_lock() {
            for (at, what) in did.iter().rev() {
                text.push_str(&format!("{:.1} s before: {what}\n", at.elapsed().as_secs_f32()));
            }
        }
        if record.code == 0xC000_0005 && record.parameters >= 2 {
            let kind = match record.information[0] {
                0 => "reading",
                1 => "writing",
                _ => "executing",
            };
            text.push_str(&format!("{kind} address 0x{:X}\n", record.information[1]));
        }
        if let Some(context) = unsafe { info.as_ref() }.map(|i| i.context as usize).filter(|&c| c != 0) {
            text.push_str(&machine_state(context));
        }
        let _ = std::fs::write(path, text);
    }
    let previous = PREVIOUS.load(Ordering::Relaxed);
    if previous != 0 {
        let previous: unsafe extern "system" fn(*mut ExceptionPointers) -> i32 = unsafe { std::mem::transmute(previous) };
        return unsafe { previous(info) };
    }
    CONTINUE_SEARCH
}
