//! Controller, keyboard and mouse for skating, read on the script thread.

use super::natives as n;
use std::ffi::c_void;

#[cfg_attr(windows, link(name = "user32"))]
unsafe extern "system" {
    fn GetAsyncKeyState(key: i32) -> i16;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
}

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
}

#[repr(C)]
#[derive(Default)]
struct XInputState {
    packet: u32,
    buttons: u16,
    left_trigger: u8,
    right_trigger: u8,
    sticks: [i16; 4],
}

#[cfg_attr(windows, link(name = "xinput"))]
unsafe extern "system" {
    fn XInputGetState(index: u32, state: *mut XInputState) -> u32;
}

/// Whether GTA's window has the keyboard (keys pressed in other windows are
/// not skating).
pub fn game_focused() -> bool {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return false;
    }
    let mut process = 0u32;
    unsafe { GetWindowThreadProcessId(window, &mut process) };
    process == unsafe { GetCurrentProcessId() }
}

pub fn vk_down(vk: u32) -> bool {
    (unsafe { GetAsyncKeyState(vk as i32) } as u16) & 0x8000 != 0
}

/// Whether a skate control key (by name) is held.
pub fn key_down(name: &str) -> bool {
    skate_input::keys::virtual_key(name).is_some_and(vk_down)
}

/// The first connected controller, as Skate reads one.
pub fn controller() -> Option<skate_input::Pad> {
    (0..4).find_map(|index| {
        let mut state = XInputState::default();
        (unsafe { XInputGetState(index, &mut state) } == 0).then(|| {
            let axis = |v: i16| (f32::from(v) / 32767.).clamp(-1., 1.);
            skate_input::Pad {
                buttons: state.buttons,
                triggers: [state.left_trigger, state.right_trigger],
                left: [axis(state.sticks[0]), axis(state.sticks[1])],
                right: [axis(state.sticks[2]), axis(state.sticks[3])],
            }
        })
    })
}

/// This frame's mouse motion in counts (y down), from GTA's look input.
pub fn mouse_motion(scale: f32) -> [f32; 2] {
    // INPUT_LOOK_LR and INPUT_LOOK_UD, read even while disabled.
    [n::disabled_control_normal(0, 1) * scale, n::disabled_control_normal(0, 2) * scale]
}

/// Keys pressed since the last call, for binding a key in the menu.
#[derive(Default)]
pub struct Edges {
    held: Vec<u32>,
}

impl Edges {
    pub fn pressed(&mut self) -> Option<&'static str> {
        let now: Vec<u32> = skate_input::keys::KEYS.iter().map(|(_, vk)| *vk).filter(|vk| vk_down(*vk)).collect();
        let fresh = now.iter().find(|vk| !self.held.contains(vk)).copied();
        self.held = now;
        fresh.and_then(skate_input::keys::from_virtual_key)
    }
}
