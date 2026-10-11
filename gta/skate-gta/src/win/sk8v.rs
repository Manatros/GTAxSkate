//! The SK8V core (decision A28): when SK8V's ASI is loaded, it skates and
//! this plugin adds what is ours (store, HD board parts, paints, cameras,
//! radio, GPS, ...). The fork exports its runtime handle
//! (`SkateModsRuntime` in `SkateVLegacy.asi`); with it the state and the
//! board's pose are read from `SkateVRuntime.dll` (`skatev_runtime.h`, ABI 8).

use std::ffi::c_void;

use bevy_math::{Mat4, Quat, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SvVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SvQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// `SvOutput`: the skater after the last Skate tick.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SvOutput {
    pub size: u32,
    pub status: u32,
    pub tick: u64,
    pub skater_position: SvVec3,
    pub skater_rotation: SvQuat,
    pub skater_heading_degrees: f32,
    pub board_position: SvVec3,
    pub board_rotation: SvQuat,
    pub velocity: SvVec3,
    pub camera_valid: u32,
    pub camera_position: SvVec3,
    pub camera_rotation: SvVec3,
    pub camera_fov: f32,
    pub state_utf8: [u8; 64],
}

/// `SvLifecycleState`: whether Skate has the player.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SvLifecycleState {
    pub size: u32,
    pub phase: u32,
    pub prepared: u32,
    pub preparing: u32,
    pub building: u32,
    pub skate_state: u32,
    pub board_possession: u32,
    pub session_builds: u32,
    pub collision_builds: u32,
    pub last_prepare_ms: f32,
    pub last_activate_ms: f32,
    pub collision_center: SvVec3,
}

/// `SvBoardPose`: the board's bones in GTA space, RAGE row layout:
/// SKATEBOARD_ROOT, TRUCK_FRONT, TRUCK_BACK, then the four wheels.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SvBoardPose {
    pub size: u32,
    pub bone_count: u32,
    pub tick: u64,
    pub entity: [f32; 16],
    pub world: [[f32; 16]; 7],
}

/// `SV_LIFECYCLE_*`.
pub mod phase {
    pub const INACTIVE: u32 = 0;
    pub const MOUNTING: u32 = 1;
    pub const RIDING: u32 = 2;
    pub const DISMOUNTING: u32 = 3;
    pub const RELEASED: u32 = 4;
}

type GetOutput = unsafe extern "C" fn(*const c_void, *mut SvOutput) -> u32;
type GetLifecycle = unsafe extern "C" fn(*const c_void, *mut SvLifecycleState) -> u32;
type GetBoardPose = unsafe extern "C" fn(*const c_void, *mut SvBoardPose) -> u32;
type GetRuntime = unsafe extern "C" fn() -> *mut c_void;
type SetPlayerSetup = unsafe extern "C" fn(*mut c_void, f32, f32, *const f32, u32) -> u32;
type SetDifficulty = unsafe extern "C" fn(*mut c_void, u32) -> u32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

/// SK8V in this process: its functions, found once.
pub struct Core {
    runtime: GetRuntime,
    output: GetOutput,
    lifecycle: GetLifecycle,
    board: GetBoardPose,
    /// The fork's `sv_set_player_setup` (none in an older fork).
    setup: Option<SetPlayerSetup>,
    difficulty: Option<SetDifficulty>,
}

/// What the plugin needs of SK8V each frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub phase: u32,
    pub riding: bool,
    /// Skate's state id and who has the board (1 held, 2 released, ...).
    pub skate_state: u32,
    pub possession: u32,
    pub skater: Vec3,
    pub velocity: Vec3,
    pub board: Mat4,
    /// The board's root bone as an entity transform (where GTA's own board
    /// model would stand), when SK8V has its native pose.
    pub board_entity: Option<Mat4>,
}

impl Core {
    /// SK8V's ASI and runtime when both are loaded and the fork exports
    /// its runtime (`SkateModsRuntime`).
    pub fn find() -> Option<Self> {
        unsafe {
            let host = GetModuleHandleA(c"SkateVLegacy.asi".as_ptr().cast());
            let dll = GetModuleHandleA(c"SkateVRuntime.dll".as_ptr().cast());
            if host.is_null() || dll.is_null() {
                return None;
            }
            let get = |m: *mut c_void, name: &std::ffi::CStr| GetProcAddress(m, name.as_ptr().cast());
            let (runtime, output, lifecycle, board) = (
                get(host, c"SkateModsRuntime"),
                get(dll, c"sv_get_output"),
                get(dll, c"sv_lifecycle_get_state"),
                get(dll, c"sv_get_board_pose"),
            );
            if runtime.is_null() || output.is_null() || lifecycle.is_null() || board.is_null() {
                return None;
            }
            let setup = get(dll, c"sv_set_player_setup");
            let difficulty = get(dll, c"sv_set_difficulty");
            Some(Self {
                setup: (!setup.is_null()).then(|| std::mem::transmute::<*mut c_void, SetPlayerSetup>(setup)),
                difficulty: (!difficulty.is_null()).then(|| std::mem::transmute::<*mut c_void, SetDifficulty>(difficulty)),
                runtime: std::mem::transmute::<*mut c_void, GetRuntime>(runtime),
                output: std::mem::transmute::<*mut c_void, GetOutput>(output),
                lifecycle: std::mem::transmute::<*mut c_void, GetLifecycle>(lifecycle),
                board: std::mem::transmute::<*mut c_void, GetBoardPose>(board),
            })
        }
    }

    /// Hands SK8V the options menu's setup (trucks, wheels, the 11 tuning
    /// multipliers, hold-fakie); false when the fork has no such export or
    /// Skate is not loaded yet.
    pub fn player_setup(&self, trucks: f32, wheels: f32, tuning: &[f32; 11], hold_fakie: bool) -> bool {
        let Some(set) = self.setup else { return false };
        unsafe {
            let rt = (self.runtime)();
            !rt.is_null() && set(rt, trucks, wheels, tuning.as_ptr(), hold_fakie as u32) != 0
        }
    }

    /// Skate 3's difficulty (`easy`, `normal`, `hardcore`, `motorized`).
    pub fn difficulty(&self, name: &str) -> bool {
        let index = match name.to_ascii_lowercase().as_str() {
            "easy" => 0,
            "normal" => 1,
            "hardcore" => 2,
            "motorized" => 3,
            _ => return false,
        };
        let Some(set) = self.difficulty else { return false };
        unsafe {
            let rt = (self.runtime)();
            !rt.is_null() && set(rt, index) != 0
        }
    }

    /// Whether SK8V's ASI is loaded at all (even an unforked one).
    pub fn loaded() -> bool {
        unsafe { !GetModuleHandleA(c"SkateVLegacy.asi".as_ptr().cast()).is_null() }
    }

    /// SK8V's runtime has the board shop (contrib/board-shop): it sells and
    /// shows the boards itself, so the extras leave the store and the
    /// ridden board's parts to it.
    pub fn has_trainer() -> bool {
        // SK8V's own trainer tuning and stamina (contrib/trainer-tuning):
        // its SkateVLegacy.ini and menu set them, not ours.
        unsafe {
            let dll = GetModuleHandleA(c"SkateVRuntime.dll".as_ptr().cast());
            !dll.is_null() && !GetProcAddress(dll, c"sv_set_trainer".as_ptr().cast()).is_null()
        }
    }

    /// SK8V's shop shows the boards in the store itself (sv_shop_display).
    pub fn has_display() -> bool {
        unsafe {
            let dll = GetModuleHandleA(c"SkateVRuntime.dll".as_ptr().cast());
            !dll.is_null() && !GetProcAddress(dll, c"sv_shop_display".as_ptr().cast()).is_null()
        }
    }

    pub fn has_shop() -> bool {
        unsafe {
            let dll = GetModuleHandleA(c"SkateVRuntime.dll".as_ptr().cast());
            !dll.is_null() && !GetProcAddress(dll, c"sv_shop_start".as_ptr().cast()).is_null()
        }
    }

    /// This frame's state; none before Skate is loaded.
    pub fn frame(&self) -> Option<Frame> {
        unsafe {
            let rt = (self.runtime)();
            if rt.is_null() {
                return None;
            }
            let mut life = SvLifecycleState { size: size_of::<SvLifecycleState>() as u32, ..Default::default() };
            if (self.lifecycle)(rt, &mut life) == 0 {
                return None;
            }
            let mut out: SvOutput = std::mem::zeroed();
            out.size = size_of::<SvOutput>() as u32;
            if (self.output)(rt, &mut out) == 0 {
                return None;
            }
            let mut pose: SvBoardPose = std::mem::zeroed();
            pose.size = size_of::<SvBoardPose>() as u32;
            let native = (self.board)(rt, &mut pose) != 0 && pose.bone_count == 7;
            let v = |v: SvVec3| Vec3::new(v.x, v.y, v.z);
            let q = out.board_rotation;
            let board = Mat4::from_rotation_translation(Quat::from_xyzw(q.x, q.y, q.z, q.w).normalize(), v(out.board_position));
            Some(Frame {
                phase: life.phase,
                skate_state: life.skate_state,
                possession: life.board_possession,
                riding: matches!(life.phase, phase::MOUNTING | phase::RIDING | phase::DISMOUNTING),
                skater: v(out.skater_position),
                velocity: v(out.velocity),
                board,
                board_entity: native.then(|| rage_rows(&pose.entity)),
            })
        }
    }
}

/// `XINPUT_GAMEPAD`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XInputGamepad {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
}

impl XInputGamepad {
    pub fn from_pad(pad: &skate_input::Pad) -> Self {
        let (left, right) = pad.axes();
        Self {
            buttons: pad.buttons,
            left_trigger: pad.triggers[0],
            right_trigger: pad.triggers[1],
            left_x: left[0],
            left_y: left[1],
            right_x: right[0],
            right_y: right[1],
        }
    }
}

/// The keyboard and mouse as a pad with the maintainer's layout (BACKLOG A3), handed
/// to SK8V, which reads only XInput: set each frame while GTA has the
/// keyboard, none while a controller is used.
static KEYBOARD_PAD: std::sync::Mutex<Option<(std::time::Instant, XInputGamepad)>> = std::sync::Mutex::new(None);

pub fn publish_keyboard_pad(pad: Option<XInputGamepad>) {
    if let Ok(mut slot) = KEYBOARD_PAD.lock() {
        *slot = pad.map(|p| (std::time::Instant::now(), p));
    }
}

/// For the fork (`ReadPadRaw`): 1 and the keyboard's pad while the keyboard
/// is in use (renewed within 200 ms), else 0 (SK8V reads its controller).
#[unsafe(no_mangle)]
pub extern "C" fn SkateModsKeyboardPad(out: *mut XInputGamepad) -> i32 {
    let Ok(slot) = KEYBOARD_PAD.lock() else { return 0 };
    match *slot {
        Some((at, pad)) if at.elapsed() < std::time::Duration::from_millis(200) && !out.is_null() => {
            unsafe { *out = pad };
            1
        }
        _ => 0,
    }
}

/// A RAGE row-layout matrix (rows: right, forward, up, position) as a
/// column-major `Mat4`.
pub fn rage_rows(m: &[f32; 16]) -> Mat4 {
    Mat4::from_cols_array(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_skatev_runtime_h() {
        assert_eq!(size_of::<SvBoardPose>(), 528);
        assert_eq!(std::mem::offset_of!(SvBoardPose, entity), 16);
        assert_eq!(std::mem::offset_of!(SvBoardPose, world), 80);
        assert_eq!(size_of::<SvOutput>(), 184);
        assert_eq!(std::mem::offset_of!(SvOutput, board_position), 48);
        assert_eq!(std::mem::offset_of!(SvOutput, state_utf8), 120);
        assert_eq!(size_of::<SvLifecycleState>(), 56);
        assert_eq!(size_of::<XInputGamepad>(), 12);
        let pad = skate_input::Pad { buttons: 0x1000, triggers: [0, 255], left: [1., -1.], right: [0., 0.5] };
        let x = XInputGamepad::from_pad(&pad);
        assert_eq!((x.buttons, x.right_trigger, x.left_x, x.left_y, x.right_y), (0x1000, 255, 32767, -32767, 16384));
    }
}
