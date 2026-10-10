//! The Script Hook V plugin: `SkateGTA.asi`.
#![cfg_attr(not(windows), allow(dead_code))]

mod character;
mod hold;
mod crash;
mod fast_start;
mod traffic;
mod input;
mod memory;
mod panel;
mod natives;
mod script;
mod shv;
mod store;
mod sound;
mod dropper;
mod coop;
mod carry;
mod board_parts;
mod sk8v;

use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

static MODULE: AtomicUsize = AtomicUsize::new(0);

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn GetModuleFileNameA(module: *mut c_void, name: *mut c_char, size: u32) -> u32;
}

/// The folder this plugin was loaded from.
fn plugin_folder() -> std::path::PathBuf {
    let mut buffer = [0u8; 1024];
    let length = unsafe {
        GetModuleFileNameA(MODULE.load(Ordering::Relaxed) as *mut c_void, buffer.as_mut_ptr().cast(), buffer.len() as u32)
    } as usize;
    let path = std::path::PathBuf::from(String::from_utf8_lossy(&buffer[..length.min(buffer.len())]).into_owned());
    path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
}

extern "C" fn script_main() {
    let mut script = match std::panic::catch_unwind(|| script::Script::new(plugin_folder())) {
        Ok(script) => script,
        Err(_) => return,
    };
    loop {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| script.tick()));
        if let Err(panic) = result {
            let what = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| script.recover(&what)));
        }
        shv::wait(0);
    }
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    const PROCESS_ATTACH: u32 = 1;
    const PROCESS_DETACH: u32 = 0;
    match reason {
        PROCESS_ATTACH => {
            MODULE.store(module as usize, Ordering::Relaxed);
            // The intro and legal notices go before the scripts start.
            fast_start::start(plugin_folder());
            shv::register(module, script_main);
        }
        PROCESS_DETACH => {
            shv::off_present(script::on_present);
            shv::unregister(module);
        }
        _ => {}
    }
    1
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn type_check_only() {
    let _ = script_main as extern "C" fn();
}
