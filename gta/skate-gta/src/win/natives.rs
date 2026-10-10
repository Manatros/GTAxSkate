//! The GTA V natives the plugin uses. Hashes and signatures are from the
//! community native database for the Legacy game (alloc8or/gta5-nativedb-data).
//! Script thread only.

use super::shv::native;
use crate::terrain::{Hit, Probe};
use bevy::math::{Mat4, Quat, Vec3};
use std::ffi::CString;

/// Script Hook V's `Vector3`: each component in an 8-byte slot.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vector3 {
    x: f32,
    _x: u32,
    y: f32,
    _y: u32,
    z: f32,
    _z: u32,
}

impl Vector3 {
    fn vec(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

fn int(result: *mut u64) -> i32 {
    unsafe { *result as u32 as i32 }
}
fn float(result: *mut u64) -> f32 {
    unsafe { f32::from_bits(*result as u32) }
}
fn boolean(result: *mut u64) -> bool {
    int(result) != 0
}
fn vector(result: *mut u64) -> Vec3 {
    unsafe { (*(result as *const Vector3)).vec() }
}

pub fn player_ped() -> i32 {
    int(native!(0xD80958FC74E988A6))
}
pub fn entity_exists(entity: i32) -> bool {
    boolean(native!(0x7239B21A38F536BA, entity))
}
pub fn entity_coords(entity: i32) -> Vec3 {
    vector(native!(0x3FEF770D40960D5A, entity, true))
}
pub fn entity_heading(entity: i32) -> f32 {
    float(native!(0xE83D4F9BA2A38914, entity))
}
pub fn set_coords_no_offset(entity: i32, p: Vec3) {
    native!(0x239A3351AC1DA385, entity, p.x, p.y, p.z, false, false, false);
}
pub fn set_coords(entity: i32, p: Vec3) {
    native!(0x06843DA7060A026B, entity, p.x, p.y, p.z, false, false, false, false);
}
pub fn set_heading(entity: i32, heading: f32) {
    native!(0x8E2530AA8ADA980E, entity, heading);
}
pub fn set_quaternion(entity: i32, q: Quat) {
    native!(0x77B21BE7AC540F07, entity, q.x, q.y, q.z, q.w);
}
pub fn freeze(entity: i32, frozen: bool) {
    native!(0x428CA6DBD1094446, entity, frozen);
}
pub fn set_collision(entity: i32, on: bool) {
    native!(0x1A9205C1B9EE827F, entity, on, false);
}
pub fn set_invincible(entity: i32, on: bool) {
    native!(0x3882114BDE571AD4, entity, on, false);
}
pub fn set_can_ragdoll(ped: i32, on: bool) {
    native!(0xB128377056A54E2A, ped, on);
}
pub fn clear_tasks_now(ped: i32) {
    native!(0xAAA34F8A7CB32098, ped);
}
pub fn in_vehicle(ped: i32) -> bool {
    boolean(native!(0x997ABD671D25CA0B, ped, false))
}
pub fn is_dead(entity: i32) -> bool {
    boolean(native!(0x5F9532F3B5CC2551, entity, true))
}
pub fn pause_menu() -> bool {
    boolean(native!(0xB0034A223497FFCB))
}
pub fn frame_time() -> f32 {
    float(native!(0x15C40837039FFAF7))
}
/// Player controls (group 0) off for this frame.
pub fn disable_player_controls() {
    native!(0x5F4B6931816E599B, 0i32);
}

pub fn request_model(hash: u32) {
    native!(0x963D27A58DF860AC, hash);
}
pub fn model_loaded(hash: u32) -> bool {
    boolean(native!(0x98A4EB5D89A0C952, hash))
}
pub fn model_valid(hash: u32) -> bool {
    boolean(native!(0xC0296A2EDF545E92, hash))
}
pub fn release_model(hash: u32) {
    native!(0xE532F5D78798DAAB, hash);
}
pub fn create_object(hash: u32, at: Vec3) -> i32 {
    int(native!(0x509D5878EB39E842, hash, at.x, at.y, at.z, false, true, false))
}
pub fn player_id() -> i32 {
    int(native!(0x4F8644AF03D0E0D6))
}
/// Whether player `index` is in the session (NETWORK_IS_PLAYER_ACTIVE).
pub fn player_active(index: i32) -> bool {
    boolean(native!(0xB8DFD30D6973E135, index))
}
/// Player `index`'s ped (GET_PLAYER_PED).
pub fn player_ped_of(index: i32) -> i32 {
    int(native!(0x43A66C31C68491C0, index))
}
pub fn model_in_cdimage(hash: u32) -> bool {
    boolean(native!(0x35B9E0803292B641, hash))
}
/// CREATE_PED as a civilian male, not networked.
pub fn create_ped(model: u32, at: Vec3, heading: f32) -> i32 {
    int(native!(0xD49F9B0955C367DE, 4i32, model, at.x, at.y, at.z, heading, false, false))
}
/// The player becomes `ped` (CHANGE_PLAYER_PED).
pub fn change_player_ped(ped: i32) {
    native!(0x048189FAC643DEEE, player_id(), ped, true, true);
}
pub fn set_ped_component(ped: i32, component: u8, drawable: usize, texture: usize) {
    native!(0x262B14F48D29DE80, ped, i32::from(component), drawable as i32, texture as i32, 0i32);
}
pub fn set_ped_default_components(ped: i32) {
    native!(0x45EEE61580806D63, ped);
}
pub fn keep_entity(entity: i32) {
    native!(0xAD738C3085FE7E11, entity, true, true);
}
pub fn delete_ped(ped: &mut i32) {
    native!(0xAD738C3085FE7E11, *ped, true, true);
    native!(0x9614299DCB53E54B, ped as *mut i32);
    *ped = 0;
}
/// One control back on for this frame, after all were disabled
/// (ENABLE_CONTROL_ACTION).
pub fn enable_control(group: i32, control: i32) {
    native!(0x351220255D64C155, group, control, true);
}
/// The last input came from keyboard and mouse, not a controller
/// (IS_USING_KEYBOARD_AND_MOUSE).
pub fn using_keyboard() -> bool {
    boolean(native!(0xA571D46727E2B718, 2i32))
}
/// A control held, even while controls are disabled
/// (IS_DISABLED_CONTROL_PRESSED).
pub fn disabled_control_pressed(group: i32, control: i32) -> bool {
    boolean(native!(0xE2587F8CBBD87B1D, group, control))
}
/// The weapon the ped has out (GET_SELECTED_PED_WEAPON).
pub fn selected_weapon(ped: i32) -> u32 {
    int(native!(0x0A6DB4965674D243, ped)) as u32
}
/// A weapon's group: pistols, rifles, melee, ... (GET_WEAPONTYPE_GROUP).
pub fn weapon_group(weapon: u32) -> u32 {
    int(native!(0xC3287EE3050FB74C, weapon)) as u32
}
/// GET_AMMO_IN_PED_WEAPON.
pub fn ammo(ped: i32, weapon: u32) -> i32 {
    int(native!(0x015A522136D7F951, ped, weapon))
}
/// SET_PED_AMMO.
pub fn set_ammo(ped: i32, weapon: u32, ammo: i32) {
    native!(0x14E56BC5B5DB6A19, ped, weapon, ammo, false);
}
/// A weapon's damage per bullet (GET_WEAPON_DAMAGE).
pub fn weapon_damage(weapon: u32) -> f32 {
    float(native!(0x3133B907D8B32053, weapon, 0u32))
}
/// One bullet of `weapon` from `from` to `to`, fired by `owner`, heard and
/// seen (SHOOT_SINGLE_BULLET_BETWEEN_COORDS).
pub fn shoot_bullet(from: Vec3, to: Vec3, damage: i32, weapon: u32, owner: i32) {
    native!(0x867654CBC7606F2C, from.x, from.y, from.z, to.x, to.y, to.z, damage, true, weapon, owner, true, false, -1.0f32);
}
/// The material straight down from `top` to `bottom` (map and objects),
/// for the board's rolling sound.
pub fn material_below(top: Vec3, bottom: Vec3, ignore: i32) -> Option<u32> {
    let handle = int(native!(0x377906D8A31E5586, top.x, top.y, top.z, bottom.x, bottom.y, bottom.z, 17i32, ignore, 7i32));
    let mut hit = 0i32;
    let mut end = Vector3::default();
    let mut normal = Vector3::default();
    let mut entity = 0i32;
    let mut material = 0u32;
    native!(
        0x65287525D951F6BE,
        handle,
        &mut hit as *mut i32,
        &mut end as *mut Vector3,
        &mut normal as *mut Vector3,
        &mut material as *mut u32,
        &mut entity as *mut i32,
    );
    (hit != 0).then_some(material)
}
/// In a GTA Online session (NETWORK_IS_SESSION_STARTED).
pub fn online() -> bool {
    boolean(native!(0x9DE624D2FC4B603F))
}
/// A cutscene is playing (IS_CUTSCENE_PLAYING). Not IS_CUTSCENE_ACTIVE:
/// that is on as soon as a mission has one loaded, e.g. near a mission
/// marker (Legion Square), with nothing playing.
pub fn cutscene_active() -> bool {
    boolean(native!(0xD3C2E180A40F031E))
}
/// The game gives the player control (IS_PLAYER_CONTROL_ON): off in
/// cutscenes, character switches and scripted mission moments.
pub fn player_control_on() -> bool {
    boolean(native!(0x49C32D60007AFA47, player_id()))
}
/// The weapon's model as an object in the world (REQUEST_WEAPON_ASSET, then
/// CREATE_WEAPON_OBJECT), or 0 while it streams in.
pub fn create_weapon_object(weapon: u32, at: Vec3) -> i32 {
    native!(0x5443438F033E29C3, weapon, 31i32, 0i32);
    if !boolean(native!(0x36E353271F0E90EE, weapon)) {
        return 0;
    }
    int(native!(0x9541D3CF0D398F36, weapon, 0i32, at.x, at.y, at.z, true, 1.0f32, 0i32))
}
/// The game knows this weapon (IS_WEAPON_VALID): an add-on one is loaded.
pub fn weapon_valid(weapon: u32) -> bool {
    boolean(native!(0x937C71165CF334B3, weapon))
}
/// HAS_PED_GOT_WEAPON.
pub fn has_weapon(ped: i32, weapon: u32) -> bool {
    boolean(native!(0x8DECB02F88F428BC, ped, weapon, false))
}
/// GIVE_WEAPON_TO_PED, without taking it out.
pub fn give_weapon(ped: i32, weapon: u32) {
    native!(0xBF0FD6E56C964FCB, ped, weapon, 1i32, false, false);
}
/// SET_CURRENT_PED_WEAPON: takes it out.
pub fn set_current_weapon(ped: i32, weapon: u32) {
    native!(0xADF692B254977C0C, ped, weapon, true);
}
/// Shows or hides the weapon in the ped's hand (SET_PED_CURRENT_WEAPON_VISIBLE).
pub fn set_weapon_visible(ped: i32, visible: bool) {
    native!(0x0725A4CCFDED9A70, ped, visible, false, false, false);
}
/// The weapon object in the ped's hand (GET_CURRENT_PED_WEAPON_ENTITY_INDEX).
pub fn weapon_entity(ped: i32) -> i32 {
    int(native!(0x3B390A939AF0B5FC, ped, 0i32))
}
/// GTA's on-foot camera view: 0 near, 1 middle, 2 far, 4 first person
/// (GET_FOLLOW_PED_CAM_VIEW_MODE).
pub fn follow_view() -> i32 {
    int(native!(0x8D4D46230B2C353A))
}
/// Sets GTA's on-foot camera view (SET_FOLLOW_PED_CAM_VIEW_MODE).
pub fn set_follow_view(mode: i32) {
    native!(0x5A4F9EDF1673F704, mode);
}
/// Starts loading a movement clipset (REQUEST_ANIM_SET); true once loaded
/// (HAS_ANIM_SET_LOADED).
pub fn anim_set_ready(name: &str) -> bool {
    let name = CString::new(name).unwrap();
    native!(0x6EA47DAE7FAD0EED, name.as_ptr());
    boolean(native!(0xC4EA073D86FB29B0, name.as_ptr()))
}
/// SET_PED_MOVEMENT_CLIPSET.
pub fn set_movement_clipset(ped: i32, name: &str) {
    let name = CString::new(name).unwrap();
    native!(0xAF8A94EDE7712BEF, ped, name.as_ptr(), 1.0f32);
}
/// RESET_PED_MOVEMENT_CLIPSET.
pub fn reset_movement_clipset(ped: i32) {
    native!(0xAA74EC0CB0AAEA2C, ped, 0.0f32);
}
/// One control off for this frame (DISABLE_CONTROL_ACTION).
pub fn disable_control(group: i32, control: i32) {
    native!(0xFE99B66D079CF6BC, group, control, true);
}
pub fn disabled_control_just_pressed(group: i32, control: i32) -> bool {
    boolean(native!(0x91AEF906BCA88877, group, control))
}

/// Attaches `entity` to `to`'s origin, turned with it, no collision
/// (ATTACH_ENTITY_TO_ENTITY).
pub fn attach(entity: i32, to: i32) {
    native!(0x6B9BBD38AB0796DF, entity, to, 0i32, 0f32, 0f32, 0f32, 0f32, 0f32, 0f32, false, false, false, false, 2i32, true);
}
/// A named blip on the map (ADD_BLIP_FOR_COORD, its sprite, colour, short
/// range, name).
pub fn add_blip(at: Vec3, sprite: i32, colour: i32, name: &str) -> i32 {
    let blip = int(native!(0x5A039BB0BCA604B6, at.x, at.y, at.z));
    if blip == 0 {
        return 0;
    }
    native!(0xDF735600A4696DAF, blip, sprite);
    native!(0x03D7FB09E75D6B7E, blip, colour);
    native!(0xBE8BE4FE60E27B72, blip, true);
    let kind = CString::new("STRING").unwrap();
    let text = CString::new(name.replace('\0', "")).unwrap_or_default();
    native!(0xF9113A30DE5C6670, kind.as_ptr());
    native!(0x6C188BE134E074AA, text.as_ptr());
    native!(0xBC38B49BCB83BC9B, blip);
    blip
}
/// DOES_BLIP_EXIST.
pub fn blip_exists(blip: i32) -> bool {
    boolean(native!(0xA6DB27D19ECBB7DA, blip))
}
/// REMOVE_BLIP.
pub fn remove_blip(mut blip: i32) {
    native!(0x86A652570E5F25DD, &mut blip as *mut i32);
}
/// DETACH_ENTITY.
pub fn detach(entity: i32) {
    native!(0x961AC54BF0613F5D, entity, false, false);
}

pub fn delete_object(object: &mut i32) {
    native!(0xAD738C3085FE7E11, *object, true, true);
    native!(0x539E0AE3E6634B9F, object as *mut i32);
    *object = 0;
}

pub fn create_camera() -> i32 {
    let name = CString::new("DEFAULT_SCRIPTED_CAMERA").unwrap();
    int(native!(0xC3981DCE61D9E13F, name.as_ptr(), true))
}
pub fn camera_place(cam: i32, at: Vec3, rotation: [f32; 3], fov: f32) {
    native!(0x4D41783FB745E42E, cam, at.x, at.y, at.z);
    native!(0x85973643155D0B07, cam, rotation[0], rotation[1], rotation[2], 2i32);
    native!(0xB13C14F66A00D047, cam, fov);
}
pub fn camera_render(cam: i32, on: bool) {
    native!(0x026FB97D0A425F84, cam, on);
    native!(0x07E5B515DB0636FC, on, false, 0i32, true, false, 0i32);
}
pub fn destroy_camera(cam: i32) {
    native!(0x865908C81A2C22E9, cam, false);
}

pub fn health(entity: i32) -> i32 {
    int(native!(0xEEF059FAD016D209, entity))
}
pub fn max_health(entity: i32) -> i32 {
    int(native!(0x15D757606D170C3C, entity))
}
pub fn set_health(entity: i32, health: i32) {
    native!(0x6B76DC1F3AE6E6A3, entity, health, 0i32, 0u32);
}
pub fn ragdoll(ped: i32, milliseconds: i32) {
    native!(0xAE99FB955581844A, ped, milliseconds, milliseconds, 0i32, false, false, false);
}
pub fn set_visible(entity: i32, on: bool) {
    native!(0xEA1C610A04DB6BBB, entity, on, false);
}

/// One flat-coloured triangle in the world, this frame.
pub fn draw_poly(points: [Vec3; 3], rgba: [u8; 4]) {
    let [a, b, c] = points;
    let [r, g, bl, al] = rgba.map(i32::from);
    native!(0xAC26716048436851, a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z, r, g, bl, al);
}
/// DRAW_TEXTURED_POLY: a triangle with a texture of a streamed texture
/// dictionary, `uv` per corner (w 1), tinted by `rgba`.
pub fn draw_textured_poly(points: [Vec3; 3], rgba: [u8; 4], dict: &std::ffi::CStr, name: &std::ffi::CStr, uv: [[f32; 2]; 3]) {
    let [a, b, c] = points;
    let [r, g, bl, al] = rgba.map(i32::from);
    native!(
        0x29280002282F1928,
        a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z,
        r, g, bl, al,
        dict.as_ptr(), name.as_ptr(),
        uv[0][0], uv[0][1], 1f32, uv[1][0], uv[1][1], 1f32, uv[2][0], uv[2][1], 1f32
    );
}
/// REQUEST_STREAMED_TEXTURE_DICT.
pub fn request_texture_dict(dict: &std::ffi::CStr) {
    native!(0xDFA2EF8E04127DD5, dict.as_ptr(), false);
}
/// HAS_STREAMED_TEXTURE_DICT_LOADED.
pub fn texture_dict_loaded(dict: &std::ffi::CStr) -> bool {
    boolean(native!(0x0145F696AAAAD2E4, dict.as_ptr()))
}
/// FORCE_PED_AI_AND_ANIMATION_UPDATE: the ped's animation for this frame
/// is worked out now, so a pose written after it is not overwritten.
pub fn force_ped_animation_update(ped: i32) {
    native!(0x2208438012482A1A, ped, false, false);
}
pub fn backface_culling(on: bool) {
    native!(0x23BA6B0C2AD7B0D3, on);
}
/// Where the camera the frame renders from is.
pub fn camera_coord() -> Vec3 {
    vector(native!(0xA200EB1EE790F448))
}
/// The rendering camera's rotation: pitch, roll and yaw in degrees
/// (GET_FINAL_RENDERED_CAM_ROT, order 2).
pub fn camera_rot() -> Vec3 {
    vector(native!(0x5B4E4C817FCC2DFB, 2i32))
}
/// The rendering camera's forward, right and up directions.
pub fn camera_axes() -> (Vec3, Vec3, Vec3) {
    let r = camera_rot();
    let (pitch, yaw) = (r.x.to_radians(), r.z.to_radians());
    let forward = Vec3::new(-yaw.sin() * pitch.cos(), yaw.cos() * pitch.cos(), pitch.sin());
    let right = Vec3::new(yaw.cos(), yaw.sin(), 0.);
    (forward, right, right.cross(forward))
}
/// An entity's see-through-ness, 0 (clear) to 255 (solid).
pub fn set_alpha(entity: i32, alpha: i32) {
    if alpha >= 255 {
        native!(0x9B1E824FFBB7027A, entity);
    } else {
        native!(0x44A0870B7E92D7C0, entity, alpha, false);
    }
}
/// Sets an object on the ground under it (PLACE_OBJECT_ON_GROUND_PROPERLY).
pub fn place_on_ground(object: i32) -> bool {
    boolean(native!(0x58A850EAEE20FAA3, object))
}
pub fn clock_hours() -> i32 {
    int(native!(0x25223CA6B4D20B7F))
}
pub fn aspect_ratio() -> f32 {
    float(native!(0xF1307EF624A80D87, false))
}
pub fn entity_model(entity: i32) -> u32 {
    int(native!(0x9F47B058362C84B5, entity)) as u32
}
/// The name hash of the interior `entity` is in.
/// Where the interior `entity` is in stands (GET_INTERIOR_LOCATION_AND_NAMEHASH).
pub fn interior_place(entity: i32) -> Option<Vec3> {
    let interior = int(native!(0x2107BA504071A6BB, entity));
    if interior == 0 {
        return None;
    }
    let mut position = Vector3::default();
    let mut hash = 0u32;
    native!(0x252BDC06B73FA6EA, interior, &mut position as *mut Vector3, &mut hash as *mut u32);
    Some(position.vec())
}
pub fn interior_name(entity: i32) -> Option<u32> {
    let interior = int(native!(0x2107BA504071A6BB, entity));
    if interior == 0 {
        return None;
    }
    let mut position = Vector3::default();
    let mut hash = 0u32;
    native!(0x252BDC06B73FA6EA, interior, &mut position as *mut Vector3, &mut hash as *mut u32);
    (hash != 0).then_some(hash)
}
pub fn stat_int(stat: u32) -> Option<i32> {
    let mut value = 0i32;
    boolean(native!(0x767FBC2AC802EF3D, stat, &mut value as *mut i32, -1i32)).then_some(value)
}
pub fn set_stat_int(stat: u32, value: i32) {
    native!(0xB3271D7AB655B441, stat, value, true);
}
/// A control's value (-1 to 1), even while controls are disabled.
pub fn disabled_control_normal(group: i32, control: i32) -> f32 {
    float(native!(0x11E65974A982637C, group, control))
}
/// A control pressed this frame, even while controls are disabled.
pub fn control_just_pressed(group: i32, control: i32) -> bool {
    boolean(native!(0x91AEF906BCA88877, group, control))
}
pub fn sound(name: &str, set: &str) {
    let (name, set) = (CString::new(name).unwrap(), CString::new(set).unwrap());
    native!(0x67C540AA08E4A6F5, -1i32, name.as_ptr(), set.as_ptr(), true);
}
/// A rectangle by centre and size, in shares of the screen.
pub fn draw_rect(x: f32, y: f32, w: f32, h: f32, rgba: [u8; 4]) {
    let [r, g, b, a] = rgba.map(i32::from);
    native!(0x3A618A217E5154F0, x, y, w, h, r, g, b, a, false);
}
fn text_component(text: &str) -> CString {
    let text: String = text.replace('\0', " ").chars().take(98).collect();
    CString::new(text).unwrap()
}
/// Text at `x`, `y` (its left edge, or its right edge when `right`).
pub fn draw_text(text: &str, x: f32, y: f32, scale: f32, rgba: [u8; 4], right: bool) {
    let [r, g, b, a] = rgba.map(i32::from);
    native!(0x66E0276CC5F6B9DA, 4i32);
    native!(0x07C837F9A01C34C9, scale, scale);
    native!(0xBE6B23FFA53FB442, r, g, b, a);
    // An outline, so it reads over the world.
    native!(0x2513DFB0FB8400FE);
    if right {
        native!(0x4E096588B13FFECA, 2i32);
        native!(0x63145D9C883A1A70, 0.0f32, x);
    }
    let format = CString::new("STRING").unwrap();
    let text = text_component(text);
    native!(0x25FBB336DF1804CB, format.as_ptr());
    native!(0x6C188BE134E074AA, text.as_ptr());
    native!(0xCD015E5BB0D96A57, if right { 0.0f32 } else { x }, y, 0i32);
}
/// Text centred on `x` (or right-aligned to it when `right`), with an
/// outline so it reads over the world.
pub fn draw_text_centred(text: &str, x: f32, y: f32, scale: f32, rgba: [u8; 4], right: bool) {
    let [r, g, b, a] = rgba.map(i32::from);
    native!(0x66E0276CC5F6B9DA, 4i32);
    native!(0x07C837F9A01C34C9, scale, scale);
    native!(0xBE6B23FFA53FB442, r, g, b, a);
    native!(0x2513DFB0FB8400FE);
    if right {
        native!(0x4E096588B13FFECA, 2i32);
        native!(0x63145D9C883A1A70, 0.0f32, x);
    } else {
        native!(0xC02F4DBFB51D988B, true);
    }
    let format = CString::new("STRING").unwrap();
    let text = text_component(text);
    native!(0x25FBB336DF1804CB, format.as_ptr());
    native!(0x6C188BE134E074AA, text.as_ptr());
    native!(0xCD015E5BB0D96A57, if right { 0.0f32 } else { x }, y, 0i32);
}

/// Whether the game shows a help box now (a store's own counter, say).
pub fn help_shown() -> bool {
    boolean(native!(0x4D79439A6B55AC67))
}

/// The help box in the top left, this frame.
pub fn help(text: &str) {
    let format = CString::new("STRING").unwrap();
    let text = text_component(text);
    native!(0x8509B634FBE7DA11, format.as_ptr());
    native!(0x6C188BE134E074AA, text.as_ptr());
    native!(0x238FFE5C7B0498A6, 0i32, false, false, -1i32);
}

/// The radio station the player last listened to, if any.
pub fn player_radio_station() -> Option<String> {
    let result = native!(0xF6D733C32076AD03);
    let pointer = unsafe { *result as usize as *const std::ffi::c_char };
    if pointer.is_null() {
        return None;
    }
    let name = unsafe { std::ffi::CStr::from_ptr(pointer) }.to_string_lossy().into_owned();
    (!name.is_empty() && name != "OFF").then_some(name)
}

/// Plays `station` on foot (the phone's radio), or (`None`) stops that.
pub fn skate_radio(station: Option<&str>) {
    let on = station.is_some();
    native!(0x1098355A16064BB3, on); // SET_MOBILE_RADIO_ENABLED_DURING_GAMEPLAY
    native!(0xBF286C554784F3DF, on); // SET_MOBILE_PHONE_RADIO_STATE
    if let Some(station) = station {
        tune_radio(station);
    }
}

/// SET_RADIO_TO_STATION_NAME.
pub fn tune_radio(station: &str) {
    let name = CString::new(station.replace('\0', "")).unwrap();
    native!(0xC69EDA28699D5107, name.as_ptr());
}

/// SKIP_RADIO_FORWARD: the next song.
pub fn skip_radio() {
    native!(0x6DDBBDD98E2E9C25);
}

/// A message above the minimap.
pub fn notify(text: &str) {
    let format = CString::new("STRING").unwrap();
    let text = CString::new(text.replace('\0', " ")).unwrap();
    native!(0x202709F4C58A0424, format.as_ptr());
    native!(0x6C188BE134E074AA, text.as_ptr());
    native!(0x2ED7843F8F801023, false, false);
}

/// Downward probes against the map and objects, ignoring `ignore`.
pub struct GtaProbe {
    pub ignore: i32,
}

thread_local! {
    /// A ground probe GTA could not run (no handle, or no result).
    static PROBE_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static PROBE_FAILURES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Ground probes GTA could not run since last asked (for the log).
pub fn take_probe_failures() -> u32 {
    PROBE_FAILURES.with(|f| f.replace(0))
}

impl Probe for GtaProbe {
    fn failed(&mut self) -> bool {
        PROBE_FAILED.with(|f| f.replace(false))
    }

    fn down(&mut self, x: f32, y: f32, top: f32, bottom: f32) -> Option<Hit> {
        // Litter (a can, a stone, a bottle) is no ground: the board caught on
        // it. Such an object is looked through, a few times at most.
        let (mut top, mut ignore) = (top, self.ignore);
        for _ in 0..3 {
            let (hit, entity) = self.down_once(x, y, top, bottom, ignore)?;
            if entity == 0 || !is_object(entity) || !is_litter(entity) {
                return Some(hit);
            }
            top = hit.z - 0.005;
            ignore = entity;
        }
        None
    }

    /// A capsule swept down (START_SHAPE_TEST_CAPSULE): map collision and
    /// objects, answered a frame later.
    fn wide_start(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<i32> {
        let handle = int(native!(0x28579D1B8F8AAC80, from.x, from.y, from.z, to.x, to.y, to.z, radius, 17i32, self.ignore, 7i32));
        (handle != 0).then_some(handle)
    }

    /// GET_SHAPE_TEST_RESULT: 1 still pending, 2 done, 0 failed.
    fn wide_result(&mut self, id: i32) -> Option<Option<Vec3>> {
        let mut hit = 0i32;
        let mut end = Vector3::default();
        let mut normal = Vector3::default();
        let mut entity = 0i32;
        let status = int(native!(
            0x3D87450E15D98694,
            id,
            &mut hit as *mut i32,
            &mut end as *mut Vector3,
            &mut normal as *mut Vector3,
            &mut entity as *mut i32,
        ));
        match status {
            1 => None,
            2 => Some((hit != 0).then(|| end.vec())),
            _ => Some(None),
        }
    }

    /// Map collision and objects, as [`Self::down`], but glass counts: a
    /// shop window or a glass kiosk is a wall (options 6: skip only
    /// see-through and no-collision surfaces).
    fn across(&mut self, from: Vec3, to: Vec3) -> Option<Vec3> {
        across_once(from, to, self.ignore)
    }
}

/// Small loose things: an object model under 0.25 m tall and 0.6 m across.
fn is_litter(entity: i32) -> bool {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local!(static SIZES: RefCell<HashMap<u32, bool>> = RefCell::new(HashMap::new()));
    let model = entity_model(entity);
    SIZES.with(|sizes| {
        *sizes.borrow_mut().entry(model).or_insert_with(|| {
            let (lo, hi) = model_dimensions(model);
            let size = hi - lo;
            size.is_finite() && size.z < 0.25 && size.x.max(size.y) < 0.6
        })
    })
}

fn across_once(from: Vec3, to: Vec3, ignore: i32) -> Option<Vec3> {
    let handle = int(native!(0x377906D8A31E5586, from.x, from.y, from.z, to.x, to.y, to.z, 17i32, ignore, 6i32));
    let mut hit = 0i32;
    let mut end = Vector3::default();
    let mut normal = Vector3::default();
    let mut entity = 0i32;
    native!(
        0x3D87450E15D98694,
        handle,
        &mut hit as *mut i32,
        &mut end as *mut Vector3,
        &mut normal as *mut Vector3,
        &mut entity as *mut i32,
    );
    (hit != 0).then(|| end.vec())
}

impl GtaProbe {
    /// One probe down; the hit and the entity it met.
    fn down_once(&mut self, x: f32, y: f32, top: f32, bottom: f32, ignore: i32) -> Option<(Hit, i32)> {
        // Flags: map collision (1) and objects (16). Options 7: skip glass,
        // see-through and no-collision surfaces.
        let handle = int(native!(0x377906D8A31E5586, x, y, top, x, y, bottom, 17i32, ignore, 7i32));
        if handle == 0 {
            PROBE_FAILED.with(|f| f.set(true));
            PROBE_FAILURES.with(|f| f.set(f.get() + 1));
            return None;
        }
        let mut hit = 0i32;
        let mut end = Vector3::default();
        let mut normal = Vector3::default();
        let mut entity = 0i32;
        let mut material = 0u32;
        // GET_SHAPE_TEST_RESULT_INCLUDING_MATERIAL: also the surface's
        // material, for Skate's ground. Status 2: done.
        let status = int(native!(
            0x65287525D951F6BE,
            handle,
            &mut hit as *mut i32,
            &mut end as *mut Vector3,
            &mut normal as *mut Vector3,
            &mut material as *mut u32,
            &mut entity as *mut i32,
        ));
        if status != 2 {
            PROBE_FAILED.with(|f| f.set(true));
            PROBE_FAILURES.with(|f| f.set(f.get() + 1));
            return None;
        }
        (hit != 0).then(|| {
            (
                Hit {
                    z: end.z,
                    normal_z: normal.z,
                    ground: crate::ground::ground_of(material),
                },
                entity,
            )
        })
    }
}


/// An entity's rotation.
pub fn entity_quaternion(entity: i32) -> Quat {
    let (mut x, mut y, mut z, mut w) = (0f32, 0f32, 0f32, 1f32);
    native!(
        0x7B3703D2D32DFA18,
        entity,
        &mut x as *mut f32,
        &mut y as *mut f32,
        &mut z as *mut f32,
        &mut w as *mut f32,
    );
    let q = Quat::from_xyzw(x, y, z, w);
    if q.is_finite() && q.length_squared() > 0.5 { q.normalize() } else { Quat::IDENTITY }
}
pub fn entity_velocity(entity: i32) -> Vec3 {
    vector(native!(0x4805D2B1D8CF94A9, entity))
}
pub fn set_velocity(entity: i32, v: Vec3) {
    native!(0x1C99BB7B6E96D16F, entity, v.x, v.y, v.z);
}
/// A model's bounding box (its own space).
pub fn model_dimensions(model: u32) -> (Vec3, Vec3) {
    let mut min = Vector3::default();
    let mut max = Vector3::default();
    native!(0x03E8D3D5F549087A, model, &mut min as *mut Vector3, &mut max as *mut Vector3);
    (min.vec(), max.vec())
}
/// An entity's world transform, from its position and turn
/// (GET_ENTITY_COORDS, GET_ENTITY_QUATERNION).
pub fn entity_matrix(entity: i32) -> Mat4 {
    Mat4::from_rotation_translation(entity_quaternion(entity), entity_coords(entity))
}
/// Holds off GTA's idle camera (INVALIDATE_IDLE_CAM).
pub fn invalidate_idle_camera() {
    native!(0xF4F2C0D4EE209E20);
}
/// The game's day of the year, about (GET_CLOCK_MONTH, GET_CLOCK_DAY_OF_MONTH).
pub fn game_day() -> u32 {
    let month = int(native!(0xBBC72712E80257A1)).max(0) as u32;
    let day = int(native!(0x3D10BC92A4DB1D35)).max(0) as u32;
    month * 31 + day
}
pub fn is_train(model: u32) -> bool {
    boolean(native!(0xAB935175B22E822B, model))
}
/// The water's height at `x`, `y`, if there is water there.
pub fn water_height(x: f32, y: f32, z: f32) -> Option<f32> {
    let mut height = 0f32;
    boolean(native!(0xF6829842C06AE524, x, y, z, &mut height as *mut f32)).then_some(height)
}
pub fn in_water(entity: i32) -> bool {
    boolean(native!(0xCFB0A0D8EDD145A3, entity))
}
pub fn is_swimming(ped: i32) -> bool {
    boolean(native!(0x9DE327631295B4C2, ped))
}
pub fn is_ragdoll(ped: i32) -> bool {
    boolean(native!(0x47E4E977581C5B55, ped))
}
pub fn is_getting_up(ped: i32) -> bool {
    boolean(native!(0x2A74E1D5F2F00EEC, ped))
}
pub fn is_falling(ped: i32) -> bool {
    boolean(native!(0xFB92A102F1C4DFA3, ped))
}
pub fn is_on_foot(ped: i32) -> bool {
    boolean(native!(0x01FEE67DB37F59B2, ped))
}
pub fn is_human(ped: i32) -> bool {
    boolean(native!(0xB980061DA992779D, ped))
}
pub fn is_player(ped: i32) -> bool {
    boolean(native!(0x12534C348C6CB68B, ped))
}
pub fn is_dead_or_dying(ped: i32) -> bool {
    boolean(native!(0x3317DEDB88C95038, ped, true))
}
pub fn is_mission_entity(entity: i32) -> bool {
    boolean(native!(0x0A7B270912999B3C, entity))
}
/// Knocks `ped` over for `milliseconds`, thrown with `velocity`.
pub fn knock_down(ped: i32, milliseconds: i32, velocity: Vec3) {
    native!(0xAE99FB955581844A, ped, milliseconds, milliseconds * 2, 0i32, false, false, false);
    native!(0x1C99BB7B6E96D16F, ped, velocity.x, velocity.y, velocity.z);
}
/// Whether `entity` was hurt by `by` since last cleared
/// (HAS_ENTITY_BEEN_DAMAGED_BY_ENTITY).
pub fn damaged_by(entity: i32, by: i32) -> bool {
    boolean(native!(0xC86D67D52A707CF8, entity, by, true))
}
/// Forgets who last hurt `entity` (CLEAR_ENTITY_LAST_DAMAGE_ENTITY).
pub fn clear_damage(entity: i32) {
    native!(0xA72CD9CA74A5ECBA, entity);
}
/// Hurts a ped as a blow does (APPLY_DAMAGE_TO_PED, not armour first).
pub fn damage(ped: i32, amount: i32) {
    native!(0x697157CED63F18D4, ped, amount, false);
}
/// Says something from GTA's ambient speech (GENERIC_INSULT_HIGH, ...).
pub fn speak(ped: i32, line: &str) {
    let line = CString::new(line).unwrap_or_default();
    let params = CString::new("SPEECH_PARAMS_FORCE").unwrap();
    native!(0x8E04FEDD28D42462, ped, line.as_ptr(), params.as_ptr(), 0i32);
}
pub fn look_at(ped: i32, entity: i32, milliseconds: i32) {
    native!(0x69F4BE8C8CC4796C, ped, entity, milliseconds, 0i32, 2i32);
}
/// Sends `ped` quickly to `to` (a step out of the way).
pub fn step_to(ped: i32, to: Vec3) {
    native!(0xD76B57B44F1E6F8B, ped, to.x, to.y, to.z, 2.0f32, 1500i32, 40000.0f32, 0.0f32);
}
pub fn flee_from(ped: i32, from: i32) {
    native!(0x72C896464915D1B1, ped, from);
}

/// The first vehicle (flags 2) or ped (flags 4, 8) on the line from `a` to
/// `b`: where, and which entity.
/// Whether `entity` is an object (GET_ENTITY_TYPE 3), not a ped or vehicle.
pub fn is_object(entity: i32) -> bool {
    entity != 0 && int(native!(0x8ACD366038D14505, entity)) == 3
}
/// A push through the object's middle, an impulse in metres per second
/// times its mass (APPLY_FORCE_TO_ENTITY_CENTER_OF_MASS, type 1).
pub fn push(entity: i32, impulse: Vec3) {
    native!(0x18FF00FC7EFF559E, entity, 1i32, impulse.x, impulse.y, impulse.z, false, false, true, false);
}
pub fn probe_line(a: Vec3, b: Vec3, flags: i32, ignore: i32) -> Option<(Vec3, i32)> {
    let handle = int(native!(0x377906D8A31E5586, a.x, a.y, a.z, b.x, b.y, b.z, flags, ignore, 0i32));
    let mut hit = 0i32;
    let mut end = Vector3::default();
    let mut normal = Vector3::default();
    let mut entity = 0i32;
    native!(
        0x3D87450E15D98694,
        handle,
        &mut hit as *mut i32,
        &mut end as *mut Vector3,
        &mut normal as *mut Vector3,
        &mut entity as *mut i32,
    );
    (hit != 0).then(|| (end.vec(), entity))
}
