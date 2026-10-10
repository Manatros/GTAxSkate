//! A ped's skeleton in game memory, found the way ScriptHookVDotNet finds it
//! (`NativeMemory.cs`, `Structs/Skeleton`): the entity's fragment instance
//! through a virtual function located by byte pattern, its cache entry's
//! skeleton, and the bone matrices there. Every pointer is checked to be
//! readable memory before it is followed, so a game update that moves
//! things makes posing unavailable instead of crashing the game.

use bevy_math::{Mat4, Quat, Vec3};
use std::ffi::{c_char, c_void};

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

#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn VirtualQuery(address: *const c_void, info: *mut MemoryInfo, length: usize) -> usize;
    fn GetModuleHandleA(name: *const c_char) -> *mut c_void;
}

const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;

/// Whether `length` bytes at `address` are committed, readable memory,
/// across as many regions as they span.
pub fn readable(address: usize, length: usize) -> bool {
    if address < 0x10000 || address.checked_add(length).is_none() {
        return false;
    }
    let end = address + length.max(1);
    let mut at = address;
    while at < end {
        let mut info = std::mem::MaybeUninit::<MemoryInfo>::zeroed();
        let got = unsafe { VirtualQuery(at as *const c_void, info.as_mut_ptr(), size_of::<MemoryInfo>()) };
        if got == 0 {
            return false;
        }
        let info = unsafe { info.assume_init() };
        if info.state != MEM_COMMIT || info.protect & (PAGE_NOACCESS | PAGE_GUARD) != 0 {
            return false;
        }
        let region_end = info.base as usize + info.region_size;
        if region_end <= at {
            return false;
        }
        at = region_end;
    }
    true
}

fn read<T: Copy>(address: usize) -> Result<T, String> {
    if !readable(address, size_of::<T>()) {
        return Err(format!("unreadable memory at {address:#x}"));
    }
    Ok(unsafe { std::ptr::read_unaligned(address as *const T) })
}

/// The game executable's image's address range.
pub fn game_image_range() -> (usize, usize) {
    game_image().map_or((0, 0), |i| (i.as_ptr() as usize, i.as_ptr() as usize + i.len()))
}

/// The game executable's image.
fn game_image() -> Option<&'static [u8]> {
    let base = unsafe { GetModuleHandleA(std::ptr::null()) } as usize;
    let header = read::<u32>(base + 0x3C).ok()? as usize;
    let size = read::<u32>(base + header + 0x50).ok()? as usize;
    readable(base, 0x1000).then(|| unsafe { std::slice::from_raw_parts(base as *const u8, size) })
}

/// The offset of the entity virtual function that returns its fragment
/// instance (0x78 in build 2944), read from the instruction that calls it.
pub fn frag_inst_vfunc_offset() -> Result<usize, String> {
    const PATTERN: [u8; 29] = [
        0x0F, 0x84, 0x8F, 0x00, 0x00, 0x00, 0x8A, 0x48, 0x28, 0x80, 0xE9, 0x02, 0x80, 0xF9, 0x03, 0x0F, 0x87, 0x80,
        0x00, 0x00, 0x00, 0x48, 0x8B, 0x10, 0x48, 0x8B, 0xC8, 0xFF, 0x52,
    ];
    let image = game_image().ok_or("the game image is unreadable")?;
    let mut found = None;
    // Read the image page by page: not all of it is mapped readable.
    let page = 0x1000;
    let mut start = 0;
    while start < image.len() {
        let end = (start + page + PATTERN.len()).min(image.len());
        if readable(image.as_ptr() as usize + start, end - start) {
            if let Some(i) = image[start..end].windows(PATTERN.len()).position(|w| w == PATTERN) {
                found = Some(start + i);
                break;
            }
        }
        start += page;
    }
    let at = found.ok_or("the skeleton lookup pattern is not in this game version")?;
    let offset = *image.get(at + 0x1D).ok_or("pattern at the end of the image")? as i8;
    if offset <= 0 || offset % 8 != 0 {
        return Err(format!("unexpected virtual function offset {offset}"));
    }
    Ok(offset as usize)
}

/// An entity's physics instance (its `fragInst` / `phInst`), 0 if none.
pub fn frag_inst(entity: usize, vfunc_offset: usize) -> usize {
    let Ok(vtable) = read::<usize>(entity) else { return 0 };
    let Ok(function) = read::<usize>(vtable + vfunc_offset) else { return 0 };
    if !readable(function, 16) {
        return 0;
    }
    let get_frag_inst: extern "C" fn(usize) -> usize = unsafe { std::mem::transmute(function) };
    get_frag_inst(entity)
}

/// A ped's bones: names, parents and bind transforms, and where the game
/// keeps their current matrices.
#[derive(Clone)]
pub struct Skeleton {
    pub bones: Vec<(String, Option<usize>, Quat, Vec3)>,
    /// The `crSkeleton` itself, to check it still holds these arrays.
    pub address: usize,
    /// The pose matrices (`crSkeleton + 0x10`); writing them does not show.
    pub object: usize,
    /// Each bone relative to the entity (`+ 0x18`): what a ped is skinned
    /// with, 64 bytes each.
    pub global: usize,
}

impl Skeleton {
    /// The skeleton still holds the arrays it was read with (the ped was not
    /// removed and its memory not reused).
    pub fn still_there(&self) -> bool {
        readable(self.address, 0x28)
            && read::<usize>(self.address + 0x10).ok() == Some(self.object)
            && read::<usize>(self.address + 0x18).ok() == Some(self.global)
            && read::<i32>(self.address + 0x20).ok() == Some(self.bones.len() as i32)
    }
}

const BONE_DATA_SIZE: usize = 0x50;
const MATRIX_SIZE: usize = 0x40;

/// The skeleton of the entity at `entity` (`getScriptHandleBaseAddress`).
pub fn skeleton(entity: usize, vfunc_offset: usize) -> Result<Skeleton, String> {
    if entity == 0 {
        return Err("no entity".into());
    }
    let vtable = read::<usize>(entity)?;
    let function = read::<usize>(vtable + vfunc_offset)?;
    if !readable(function, 16) {
        return Err("the fragment function is unreadable".into());
    }
    let get_frag_inst: extern "C" fn(usize) -> usize = unsafe { std::mem::transmute(function) };
    let frag_inst = get_frag_inst(entity);
    if frag_inst == 0 {
        return Err("the ped has no fragment instance".into());
    }
    let cache_entry = read::<usize>(frag_inst + 0x68)?;
    let skeleton = read::<usize>(cache_entry + 0x178)?;
    let data = read::<usize>(skeleton)?;
    let object = read::<usize>(skeleton + 0x10)?;
    let global = read::<usize>(skeleton + 0x18)?;
    let count = read::<i32>(skeleton + 0x20)?;
    let bone_data = read::<usize>(data + 0x20)?;
    let data_count = read::<u16>(data + 0x5E)?;
    if !(1..=400).contains(&count) || count as usize != usize::from(data_count) {
        return Err(format!("implausible bone count {count} / {data_count}"));
    }
    let count = count as usize;
    if !readable(object, count * MATRIX_SIZE) || !readable(global, count * MATRIX_SIZE) || !readable(bone_data, count * BONE_DATA_SIZE) {
        return Err("the bone arrays are unreadable".into());
    }
    let mut bones = Vec::with_capacity(count);
    for index in 0..count {
        let bone = bone_data + index * BONE_DATA_SIZE;
        let r = read::<[f32; 4]>(bone)?;
        let t = read::<[f32; 3]>(bone + 0x10)?;
        let parent = read::<u16>(bone + 0x32)?;
        let name_at = read::<usize>(bone + 0x38)?;
        let name = if name_at != 0 && readable(name_at, 64) {
            // One check for the whole name: checked byte by byte, reading
            // all names took half a second.
            let bytes = unsafe { std::slice::from_raw_parts(name_at as *const u8, 64) };
            let end = bytes.iter().position(|b| *b == 0).unwrap_or(64);
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        } else if name_at != 0 && readable(name_at, 1) {
            let mut bytes = Vec::new();
            for k in 0..64 {
                if !readable(name_at + k, 1) {
                    break;
                }
                let b = unsafe { *((name_at + k) as *const u8) };
                if b == 0 {
                    break;
                }
                bytes.push(b);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        } else {
            String::new()
        };
        let rotation = Quat::from_xyzw(r[0], r[1], r[2], r[3]);
        if !rotation.is_finite() || !t.iter().all(|v| v.is_finite()) {
            return Err(format!("bone {index} has a broken bind pose"));
        }
        let parent = (parent != 0xFFFF && usize::from(parent) < count).then_some(usize::from(parent));
        bones.push((name, parent, rotation, Vec3::from_array(t)));
    }
    if !bones.iter().any(|b| b.0 == "SKEL_Pelvis") {
        return Err("the skeleton has no SKEL_Pelvis; bone names were not readable".into());
    }
    Ok(Skeleton { bones, address: skeleton, object, global })
}

/// Each bone relative to its parent (the root's relative to the entity),
/// from each bone relative to the entity: what `crSkeleton + 0x10` holds.
pub fn locals_of(bones: &[(String, Option<usize>, Quat, Vec3)], objects: &[Mat4]) -> Vec<Mat4> {
    objects
        .iter()
        .enumerate()
        .map(|(i, m)| match bones.get(i).and_then(|b| b.1).and_then(|p| objects.get(p)) {
            Some(parent) => parent.inverse() * *m,
            None => *m,
        })
        .collect()
}

fn write_matrices(at: usize, matrices: &[[f32; 16]]) {
    for (index, m) in matrices.iter().enumerate() {
        unsafe { std::ptr::write_unaligned((at + index * MATRIX_SIZE) as *mut [f32; 16], *m) };
    }
}

/// A pose to write: the skeleton's arrays and each bone's matrix relative to
/// the entity (`crSkeleton + 0x18`, what a ped is skinned with) and to its
/// parent (`+ 0x10`, what GTA rebuilds the first from).
pub struct Written {
    pub skeleton: usize,
    pub object: usize,
    pub global: usize,
    pub objects: Vec<[f32; 16]>,
    /// Every matrix is finite: a pose with any NaN or infinity is never
    /// written (GTA's physics would take it).
    finite: bool,
}

impl Written {
    pub fn new(skeleton: &Skeleton, objects: &[Mat4]) -> Self {
        let count = objects.len().min(skeleton.bones.len());
        let objects = &objects[..count];
        let objects: Vec<[f32; 16]> = objects.iter().map(Mat4::to_cols_array).collect();
        // Sensible numbers too: no bone kilometres off.
        let finite = objects.iter().all(|m| m.iter().all(|v| v.is_finite() && v.abs() < 1.0e4));
        Self { skeleton: skeleton.address, object: skeleton.object, global: skeleton.global, objects, finite }
    }

    /// The skeleton still holds the arrays this pose is for: the character
    /// was not removed (or the memory reused) since.
    pub fn still_there(&self) -> bool {
        self.skeleton != 0
            && readable(self.skeleton, 0x28)
            && read::<usize>(self.skeleton + 0x10).ok() == Some(self.object)
            && read::<usize>(self.skeleton + 0x18).ok() == Some(self.global)
            && read::<i32>(self.skeleton + 0x20).ok().is_some_and(|n| n as usize >= self.objects.len())
    }

    /// Whether the game's arrays hold something other than this pose (GTA
    /// wrote its own over it).
    pub fn overwritten(&self) -> bool {
        let length = self.objects.len() * MATRIX_SIZE;
        if !readable(self.global, length) {
            return false;
        }
        self.objects.iter().enumerate().rev().any(|(index, m)| {
            let now = unsafe { std::ptr::read_unaligned((self.global + index * MATRIX_SIZE) as *const [f32; 16]) };
            now != *m
        })
    }

    /// Writes the pose where the ped is skinned from (each bone relative to
    /// the character). The bones relative to their parents are left to
    /// GTA: written there too, GTA crashed shortly after skating began.
    pub fn write(&self) {
        let length = self.objects.len() * MATRIX_SIZE;
        if !self.finite || !readable(self.global, length) {
            return;
        }
        write_matrices(self.global, &self.objects);
    }
}

/// Whether the bone array at `global` holds something other than `objects`.
pub fn differs(global: usize, objects: &[[f32; 16]]) -> bool {
    if !readable(global, objects.len() * MATRIX_SIZE) {
        return false;
    }
    objects.iter().enumerate().rev().any(|(index, m)| {
        let now = unsafe { std::ptr::read_unaligned((global + index * MATRIX_SIZE) as *const [f32; 16]) };
        now != *m
    })
}

/// Writes `objects` (each finite) into the bone array at `global`.
pub fn write_raw(global: usize, objects: &[[f32; 16]]) {
    let finite = objects.iter().all(|m| m.iter().all(|v| v.is_finite() && v.abs() < 1.0e4));
    if finite && readable(global, objects.len() * MATRIX_SIZE) {
        write_matrices(global, objects);
    }
}

/// The pose the game shows now: each bone relative to the entity, as
/// [`Written`] writes it. None when the array cannot be read.
pub fn read_pose(global: usize, count: usize) -> Option<Vec<Mat4>> {
    if count == 0 || !readable(global, count * MATRIX_SIZE) {
        return None;
    }
    let pose: Vec<Mat4> = (0..count)
        .map(|i| Mat4::from_cols_array(&unsafe { std::ptr::read_unaligned((global + i * MATRIX_SIZE) as *const [f32; 16]) }))
        .collect();
    pose.iter().all(|m| m.is_finite()).then_some(pose)
}
