//! Script Hook V's exports. They are C++ functions, imported by their
//! decorated names, so no SDK import library is needed: Windows loads
//! ScriptHookV.dll for the plugin as it does for the SDK's samples.

use std::ffi::c_void;

pub type ScriptMain = extern "C" fn();
pub type PresentCallback = extern "C" fn(*mut c_void);

#[cfg_attr(windows, link(name = "ScriptHookV", kind = "raw-dylib"))]
unsafe extern "C" {
    #[link_name = "?scriptRegister@@YAXPEAUHINSTANCE__@@P6AXXZ@Z"]
    fn scriptRegister(module: *mut c_void, main: ScriptMain);
    #[link_name = "?scriptUnregister@@YAXPEAUHINSTANCE__@@@Z"]
    fn scriptUnregister(module: *mut c_void);
    #[link_name = "?scriptWait@@YAXK@Z"]
    fn scriptWait(milliseconds: u32);
    #[link_name = "?nativeInit@@YAX_K@Z"]
    fn nativeInit(hash: u64);
    #[link_name = "?nativePush64@@YAX_K@Z"]
    fn nativePush64(value: u64);
    #[link_name = "?nativeCall@@YAPEA_KXZ"]
    fn nativeCall() -> *mut u64;
    #[link_name = "?getScriptHandleBaseAddress@@YAPEAEH@Z"]
    fn getScriptHandleBaseAddress(handle: i32) -> *mut u8;
    #[link_name = "?presentCallbackRegister@@YAXP6AXPEAX@Z@Z"]
    fn presentCallbackRegister(callback: PresentCallback);
    #[link_name = "?presentCallbackUnregister@@YAXP6AXPEAX@Z@Z"]
    fn presentCallbackUnregister(callback: PresentCallback);
    #[link_name = "?worldGetAllVehicles@@YAHPEAHH@Z"]
    fn worldGetAllVehicles(arr: *mut i32, size: i32) -> i32;
    #[link_name = "?worldGetAllPeds@@YAHPEAHH@Z"]
    fn worldGetAllPeds(arr: *mut i32, size: i32) -> i32;
    #[link_name = "?worldGetAllObjects@@YAHPEAHH@Z"]
    fn worldGetAllObjects(arr: *mut i32, size: i32) -> i32;
    #[link_name = "?createTexture@@YAHPEBD@Z"]
    fn createTexture(file: *const std::ffi::c_char) -> i32;
    #[link_name = "?drawTexture@@YAXHHHHMMMMMMMMMMMM@Z"]
    fn drawTexture(
        id: i32,
        index: i32,
        level: i32,
        time: i32,
        size_x: f32,
        size_y: f32,
        center_x: f32,
        center_y: f32,
        pos_x: f32,
        pos_y: f32,
        rotation: f32,
        screen_height_scale: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
}

/// Loads an image file (PNG) for [`draw_texture`]; Script Hook V draws it
/// over the game. Each file is loaded once.
pub fn create_texture(path: &std::path::Path) -> Option<i32> {
    let name = std::ffi::CString::new(path.to_string_lossy().as_bytes()).ok()?;
    let id = unsafe { createTexture(name.as_ptr()) };
    (id >= 0).then_some(id)
}

/// Draws a loaded texture for the next `time_ms`, `width` wide (a share of
/// the screen's width), `height` high (in the same units), centred at
/// `x`, `y` (shares of the screen).
pub fn draw_texture(id: i32, width: f32, height: f32, x: f32, y: f32, aspect: f32) {
    unsafe { drawTexture(id, 0, 9999, 60, width, height, 0.5, 0.5, x, y, 0.0, aspect, 1.0, 1.0, 1.0, 1.0) }
}

pub fn register(module: *mut c_void, main: ScriptMain) {
    unsafe { scriptRegister(module, main) }
}

pub fn unregister(module: *mut c_void) {
    unsafe { scriptUnregister(module) }
}

/// Yields to the game; the script continues next frame (0) or later.
pub fn wait(milliseconds: u32) {
    unsafe { scriptWait(milliseconds) }
}

/// Every vehicle in the world (up to 512).
pub fn all_vehicles() -> Vec<i32> {
    let mut handles = vec![0i32; 512];
    let count = unsafe { worldGetAllVehicles(handles.as_mut_ptr(), handles.len() as i32) };
    handles.truncate(count.clamp(0, 512) as usize);
    handles
}

/// Every ped in the world (up to 512).
pub fn all_peds() -> Vec<i32> {
    let mut handles = vec![0i32; 512];
    let count = unsafe { worldGetAllPeds(handles.as_mut_ptr(), handles.len() as i32) };
    handles.truncate(count.clamp(0, 512) as usize);
    handles
}

/// Every object in the world (up to 1024).
pub fn all_objects() -> Vec<i32> {
    let mut handles = vec![0i32; 1024];
    let count = unsafe { worldGetAllObjects(handles.as_mut_ptr(), handles.len() as i32) };
    handles.truncate(count.clamp(0, 1024) as usize);
    handles
}

pub fn entity_address(handle: i32) -> *mut u8 {
    unsafe { getScriptHandleBaseAddress(handle) }
}

pub fn on_present(callback: PresentCallback) {
    unsafe { presentCallbackRegister(callback) }
}

pub fn off_present(callback: PresentCallback) {
    unsafe { presentCallbackUnregister(callback) }
}

/// One argument as Script Hook V pushes it: the value in the low bytes of
/// a zeroed 64-bit slot.
pub trait Arg {
    fn raw(self) -> u64;
}
impl Arg for i32 {
    fn raw(self) -> u64 {
        u64::from(self as u32)
    }
}
impl Arg for u32 {
    fn raw(self) -> u64 {
        u64::from(self)
    }
}
impl Arg for f32 {
    fn raw(self) -> u64 {
        u64::from(self.to_bits())
    }
}
impl Arg for bool {
    fn raw(self) -> u64 {
        u64::from(self)
    }
}
impl<T> Arg for *const T {
    fn raw(self) -> u64 {
        self as u64
    }
}
impl<T> Arg for *mut T {
    fn raw(self) -> u64 {
        self as u64
    }
}

/// Calls native `hash` with raw arguments; the result's first slot.
///
/// # Safety
/// Only from the script thread, with the arguments the native expects.
pub unsafe fn call(hash: u64, args: &[u64]) -> *mut u64 {
    unsafe {
        nativeInit(hash);
        for &arg in args {
            nativePush64(arg);
        }
        nativeCall()
    }
}

/// `native!(HASH, args...)` -> a pointer to the result.
macro_rules! native {
    ($hash:expr $(, $arg:expr)* $(,)?) => {
        unsafe { $crate::win::shv::call($hash, &[$($crate::win::shv::Arg::raw($arg)),*]) }
    };
}
pub(crate) use native;
