//! The plugin's script: skate mode on the player.

use super::memory::{self, Skeleton};
use super::natives as n;
use super::shv;
use super::hold;
use super::input;
use super::panel;
use super::store::Store;
use crate::options::{self, Changed, Setting};
use crate::board_look::{BoardLook, daylight};
use crate::config::{BoardLook as Look, CameraMode, Config, PoseWrite, TEMPLATE};
use crate::coords::{
    basis, camera_rotation, entity_transform, from_skate, gta_heading, joaat, rotation_quaternion, skate_heading,
    to_skate, transform_from_skate,
};
use crate::rig::{Retarget, bind_pose, load_reference};
use crate::terrain::{Probe, Terrain};
use crate::worker::{Event, Frame, Options, Worker};
use bevy::math::{EulerRot, Mat4, Quat, Vec3, Vec4};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg_attr(windows, link(name = "user32"))]
unsafe extern "system" {
    fn GetAsyncKeyState(key: i32) -> i16;
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

const _: () = assert!(size_of::<XInputState>() == 16);

/// Whether any controller has both sticks clicked in.
fn sticks_clicked() -> bool {
    (0..4).any(|index| {
        let mut state = XInputState::default();
        unsafe { XInputGetState(index, &mut state) == 0 && state.buttons & 0x00C0 == 0x00C0 }
    })
}

fn key_down(key: u32) -> bool {
    let state = unsafe { GetAsyncKeyState(key as i32) };
    state as u16 & 0x8000 != 0
}

pub extern "C" fn on_present(_swap_chain: *mut std::ffi::c_void) {
    hold::write_now();
}

/// Puts a pose (each bone relative to the character) on a character the
/// way the settings say: written now, held against GTA's animation by the
/// pose thread, or written again as each frame is shown.
fn show_pose(mode: PoseWrite, slot: hold::Slot, skeleton: &Skeleton, objects: &[Mat4]) {
    if mode == PoseWrite::Off {
        return;
    }
    let pose = memory::Written::new(skeleton, objects);
    if matches!(mode, PoseWrite::Tick | PoseWrite::Both | PoseWrite::Hold) {
        pose.write();
    }
    match mode {
        PoseWrite::Hold => hold::set(slot, pose, true),
        PoseWrite::Present | PoseWrite::Both => hold::set(slot, pose, false),
        PoseWrite::Off | PoseWrite::Tick => {}
    }
}

/// The Skate bones a co-op packet carries when this player's skeleton is
/// not posed (the body's; the board's are added).
const BODY_BONES: [&str; 22] = [
    "HIPS", "SPINE", "SPINE1", "SPINE3", "NECK", "HEAD", "LEFTSHOULDER", "LEFTARM", "LEFTFOREARM", "LEFTHAND",
    "RIGHTSHOULDER", "RIGHTARM", "RIGHTFOREARM", "RIGHTHAND", "LEFTUPLEG", "LEFTLEG", "LEFTFOOT", "LEFTTOEBASE",
    "RIGHTUPLEG", "RIGHTLEG", "RIGHTFOOT", "RIGHTTOEBASE",
];

/// In a board shop: the player's character holding the board they look
/// at, in Skate's on-foot pose, seen from in front.
struct Showroom {
    ped: i32,
    origin: Vec3,
    /// Where the character stood, to stand there again.
    home: Vec3,
    heading: f32,
    camera: i32,
    skeleton: Option<(Skeleton, Retarget)>,
    frame: Option<Frame>,
    board: Option<(skate_boards::BoardChoice, Option<(BoardLook, Option<Vec<usize>>)>, Option<(BoardLook, Option<Vec<usize>>)>)>,
}

/// The other player's skater as shown here.
struct RemoteSkater {
    ped: i32,
    /// A ped made here to stand in (no network ped near them).
    stand_in: bool,
    skeleton: Option<(Skeleton, Retarget)>,
    heading: f32,
    last: Vec3,
}

struct Log(Option<std::fs::File>);

impl Log {
    fn line(&mut self, text: &str) {
        if let Some(file) = self.0.as_mut() {
            let _ = writeln!(file, "{text}");
            let _ = file.flush();
        }
    }
}

/// What skate mode keeps while it is on.
struct Riding {
    ped: i32,
    origin: Vec3,
    board: i32,
    camera: i32,
    /// Whether the scripted camera is the one rendering.
    camera_on: bool,
    skeleton: Option<(Skeleton, Retarget)>,
    frame: Option<Frame>,
    heading: f32,
    built: (u64, Instant, Vec3),
    /// Skate's origin when collision was last sent (it moves with a vehicle
    /// carrying the skater).
    sent_origin: Vec3,
    /// The station playing before skating tuned the radio, to go back to.
    radio_before: Option<Option<String>>,
    /// The pose GTA showed when skating began, blended into Skate's.
    blend_in: Option<(Instant, Vec<Mat4>)>,
    /// The last pose written, for blending back into GTA's on getting off.
    last_pose: Vec<Mat4>,
}

/// After skating: the last Skate pose blended into GTA's own animation.
struct BlendOut {
    ped: i32,
    global: usize,
    skeleton: Option<Skeleton>,
    from: Vec<Mat4>,
    started: Instant,
}

/// Whether a weapon group fires while the button is held, and the seconds
/// between shots; none for what is no gun (fists, melee, thrown, heavy).
fn gun_rate(group: u32) -> Option<(bool, f32)> {
    Some(match group {
        0x18D5_FA97 => (false, 0.18),
        0xC6E9_A5C5 => (true, 0.08),
        0x39D5_C192 => (true, 0.11),
        0x451B_04BC => (true, 0.1),
        0x3343_1399 => (false, 0.7),
        0xB7BB_D827 => (false, 1.1),
        _ => return None,
    })
}

/// How long poses blend when skating begins and ends.
const BLEND_SECONDS: f32 = 0.35;

/// `a` to `b` by `t` (0 to 1): rotations turned, positions and scales moved.
/// Where the board models go for Skate's deck `board` (Skate space): the
/// models lie along their Z, the deck facing -X (wheels +X), held 0.1 m in
/// from the tail; onto the deck's frame (+Y its back, Z up), their middle on
/// the deck, with the ini's own turn and offset.
fn deck_model(board: Mat4, origin: Vec3, config: &Config) -> Mat4 {
    let [yaw, pitch, roll] = config.board_rotation.map(f32::to_radians);
    transform_from_skate(board, origin)
        * Mat4::from_rotation_z(std::f32::consts::PI)
        * Mat4::from_quat(Quat::from_euler(EulerRot::ZXY, yaw, pitch, roll))
        * Mat4::from_translation(Vec3::from_array(config.board_offset))
        * Mat4::from_cols(-Vec4::Z, Vec4::X, -Vec4::Y, Vec4::W)
        * Mat4::from_translation(Vec3::new(0., 0., -0.35))
}

fn blend_pose(a: &Mat4, b: &Mat4, t: f32) -> Mat4 {
    crate::rig::blend_bone(a, b, t)
}

/// Eased progress of a blend begun at `started`: 0 to 1.
fn blend_progress(started: Instant) -> f32 {
    let x = (started.elapsed().as_secs_f32() / BLEND_SECONDS).clamp(0., 1.);
    x * x * (3. - 2. * x)
}

/// The board library and the default skater's board, read once.
struct BoardSources {
    library: Option<skate_boards::Library>,
    defaults: Result<skate_boards::Board, String>,
}

/// The Skate 3 board being drawn: the choice it shows and its bones in the
/// Skate pose.
struct DrawnBoard {
    choice: skate_boards::BoardChoice,
    look: BoardLook,
    bound: Option<Vec<usize>>,
    /// The same board in its own triangles, drawn with the skater pack's
    /// board textures once GTA has them loaded.
    textured: BoardLook,
    textured_bound: Option<Vec<usize>>,
    /// The textures are loaded (checked each frame until they are).
    textures_ready: bool,
    /// Its triangle count went to the log.
    logged: bool,
}

/// Texture size limit for the drawn board's colours.
const BOARD_TEXTURE: u32 = 256;

/// The options menu while it is open.
struct OptionsMenu {
    row: usize,
    /// The action waiting for a key.
    capture: Option<usize>,
    edges: input::Edges,
}

const OPTION_ROWS: usize = 12;

pub struct Script {
    config: Config,
    dir: PathBuf,
    controls: skate_input::Controls,
    keyboard: skate_input::Translator,
    menu: Option<OptionsMenu>,
    options_key_was_down: bool,
    radio_skip_was_down: bool,
    radio_hint_shown: bool,
    /// Skate mode ended with the board in hand: the skating station plays
    /// on (the station from before, to go back to once the board is put
    /// away or a vehicle is entered). the maintainer: the radio between skating and
    /// GTA mode (video 19-54 11:53).
    radio_held: Option<Option<String>>,
    /// When the skating station was last checked to still play.
    radio_checked: Option<Instant>,
    /// How closed in the skater is, for the board's echo (0 to 1).
    closed_in: f32,
    /// Frames with the board in hand (blows are looked for every few).
    board_blows: u32,
    /// The ridden deck's scuffs are worked out (after loading, after a buy).
    scuff_known: bool,
    /// The cinematic camera (R) and whether it is on.
    action_cam: crate::action_cam::ActionCam,
    action_cam_on: bool,
    /// Logging Skate's states as skating starts: since when, the last one.
    start_states: Option<(Instant, String)>,
    /// Pushing (A) from when skating started at a run, for this long.
    push_off: Option<(Instant, Duration)>,
    hud: crate::hud::Hud,
    clock: Instant,
    /// The quick first/third person switch, and the first-person camera.
    view_flipped: bool,
    /// GTA's own collision read live (`crate::live`, SK8V's way): the
    /// reader thread, the area it built last, and the table search.
    live: crate::live::Service,
    live_area: Option<crate::live::Area>,
    live_search: Option<std::sync::mpsc::Receiver<Option<usize>>>,
    live_next: Option<Instant>,
    /// The camera V has cycled to (see `crate::cam_cycle`).
    view: crate::cam_cycle::View,
    /// First person: where the mouse turned the head from straight ahead
    /// (yaw, pitch in radians), and when it last moved.
    look: (f32, f32),
    look_moved: Option<Instant>,
    head: skate_rules::HeadCamera,
    /// Leaving skate mode: stepping off the board first, until this time.
    leaving: Option<Instant>,
    fall: skate_rules::FallTracker,
    /// Damage (percent) from a landing, to apply on the script's next turn.
    fallen: Option<u32>,
    /// Skate's Hall of Meat through a bail (see `crate::hom`), the last
    /// tick seen in it, and the injuries to apply on the script's next turn.
    hom: Option<crate::hom::Tracker>,
    hom_tick: Option<u64>,
    injured: Option<crate::hom::Injuries>,
    /// Limping on a broken leg since then.
    limping: Option<Instant>,
    store: Store,
    /// Your Skate 3 skater, the fourth character.
    character: super::character::Character,
    sources: Option<BoardSources>,
    drawn: Option<DrawnBoard>,
    log: Log,
    /// Skating frames logged step by step since skate mode came on (the
    /// first few), so a crash there shows where.
    traced: u32,
    /// Getting off: Skate's last pose fading into GTA's.
    blend_out: Option<BlendOut>,
    /// This frame is one of those.
    tracing: bool,
    /// When skating began and the pose thread's count then, to log once
    /// how often GTA wrote its own pose over Skate's.
    pose_check: Option<(Instant, u64)>,
    worker: Option<Worker>,
    names: Option<Vec<String>>,
    /// Skate mode was asked for while the session was still loading.
    waiting: bool,
    riding: Option<Riding>,
    terrain: Terrain,
    /// Where the ground was last scanned from at skating's start: getting
    /// back on nearby keeps what was scanned instead of probing it all again.
    terrain_centre: Option<Vec3>,
    /// Skate's session was started loading in the background at the first
    /// tick, before the toggle.
    preloaded: bool,
    /// The ridden board as drawn in the hand on foot (`BoardWeapon`), for
    /// which board.
    carry_look: Option<(skate_boards::BoardChoice, BoardLook, crate::board_look::Rest)>,
    /// The board as the pack's HD part models: under the feet, in the hand.
    ride_parts: super::board_parts::Parts,
    /// Since when GTA has the player's control off, if it has.
    control_off_since: Option<Instant>,
    /// The skate parks' exact collision (`<assets>\\parks`), loaded once.
    parks: Option<Vec<(&'static crate::parks::Park, crate::parks::ParkMesh)>>,
    /// The parks' map blips.
    park_blips: Vec<i32>,
    park_blips_checked: Option<Instant>,
    hand_parts: super::board_parts::Parts,
    /// The board the character holds in the store's skater view.
    showroom_parts: super::board_parts::Parts,
    /// The rolling sound's surface under the board, and a frame count to
    /// look for it every few frames.
    ground_sound: Option<&'static str>,
    ground_look: u32,
    /// Where the skater last stood on something (falling far below it
    /// is out of the world: back there).
    safe_feet: Option<Vec3>,
    /// The player's skeleton and retarget, read once: reading them took half
    /// a second every time skating began.
    skeleton_cache: Option<(i32, Skeleton, Retarget)>,
    /// The board in hand on foot (`BoardWeapon`).
    carry: super::carry::Carry,
    /// The terrain's collision as last built: its revision, centre and mesh.
    terrain_mesh: Option<(u64, Vec3, bool, crate::terrain::Mesh)>,
    /// Vehicles and people around the skater.
    traffic: super::traffic::Traffic,
    /// Skate mode ended without being asked (run over, into the water): it
    /// comes back once the player is up on their feet, since then.
    resume: Option<Option<Instant>>,
    vfunc: Option<Result<usize, String>>,
    toggle_was_down: bool,
    present_hooked: bool,
    /// The board's sounds, once the sound output is open.
    sound: Option<super::sound::BoardSound>,
    sound_tried: bool,
    /// Skate 3's object dropper.
    dropper: super::dropper::GtaDropper,
    /// LB with the D-pad's sides held last frame (dropper, checkpoint).
    combo_last: u16,
    /// Back to the checkpoint at this time (after a bail, or its key).
    retry_at: Option<Instant>,
    /// Since when frames are timed for the `perf` log line, and how many.
    perf: (Instant, u32),
    perf_checks: u64,
    /// Aiming a gun: the point aimed at, the bones that bend for it (for
    /// which skeleton), how far round from the hips the aim was last frame,
    /// where the gun hand was, and when the next shot may go.
    aim_target: Option<Vec3>,
    aim_rig: Option<(usize, crate::aim::AimRig)>,
    aim_turn: f32,
    aim_hand: Vec3,
    next_shot: Instant,
    retries: u32,
    landings_seen: Option<u32>,
    /// What riding did since it last wore the board, and when the wear was
    /// last handed over and saved.
    usage: skate_boards::wear::Usage,
    usage_at: Instant,
    wear_saved_at: Instant,
    /// Co-op: the link to the other player's plugin, their skater, board,
    /// sounds and dropped objects as shown here.
    coop: Option<super::coop::Link>,
    coop_tried: bool,
    remote: Option<RemoteSkater>,
    remote_board: Option<(String, Option<(BoardLook, Option<Vec<usize>>)>)>,
    remote_props: Vec<(skate_park::ParkObject, i32)>,
    remote_sound: Option<super::sound::BoardSound>,
    showroom: Option<Showroom>,
}

/// `forward` turned by the mouse look: yaw about up, then pitch.
fn look_turned(forward: Vec3, (yaw, pitch): (f32, f32)) -> Vec3 {
    let turned = bevy::math::Quat::from_rotation_z(yaw) * forward;
    let right = turned.cross(Vec3::Z).normalize_or_zero();
    if right == Vec3::ZERO {
        return turned;
    }
    (bevy::math::Quat::from_axis_angle(right, pitch) * turned).normalize_or_zero()
}

/// Seconds after a bail before going back to the checkpoint.
/// Falling this far (m) below where the skater last stood is through the
/// world, not off a ledge.
const FELL_THROUGH: f32 = 10.;
/// The parks' blip: a white circle in yellow, short range like a shop's.
const PARK_SPRITE: i32 = 1;
const PARK_COLOUR: i32 = 5;
/// Running this fast (m/s), skating starts on the board.
const RUN_ONTO_BOARD: f32 = 3.;
const RETRY_DELAY: Duration = Duration::from_millis(1200);

const ACTIVATION_RADIUS: f32 = 4.;

/// GTA's standing animation played under Skate's pose.


impl Script {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::remove_file(dir.join("SkateGTA-crash.txt"));
        super::crash::install(&dir);
        // The last session's log is kept (a test is often two sessions).
        let _ = std::fs::rename(dir.join("SkateGTA.log"), dir.join("SkateGTA-previous.log"));
        let mut log = Log(std::fs::File::create(dir.join("SkateGTA.log")).ok());
        let ini = dir.join("SkateGTA.ini");
        if !ini.exists() {
            let _ = std::fs::write(&ini, TEMPLATE);
            log.line("wrote SkateGTA.ini with the defaults");
        }
        let text = std::fs::read_to_string(&ini).unwrap_or_default();
        let (config, problems) = Config::parse(&text);
        for problem in &problems {
            log.line(&format!("SkateGTA.ini {problem}"));
        }
        log.line(&format!("settings: {config:?}"));
        let terrain = Terrain::new(config.scan_spacing).with_tall_walls(config.tall_walls);
        let store = Store::new(&dir);
        let (character, problem) = super::character::Character::new(&dir);
        if let Some(problem) = problem {
            log.line(&problem);
        }
        let controls_path = dir.join("SkateGTA-controls.txt");
        let controls = match std::fs::read_to_string(&controls_path) {
            // Keys from before GTA's own layout: kept aside, GTA's in place.
            Ok(text) if crate::options::controls_outdated(&text) => {
                let _ = std::fs::write(dir.join("SkateGTA-controls-old.txt"), &text);
                let controls = crate::options::gta_controls();
                let _ = std::fs::write(&controls_path, controls.to_text());
                log.line("SkateGTA-controls.txt: GTA's key layout now (the old keys are in SkateGTA-controls-old.txt)");
                controls
            }
            Ok(text) => {
                let (controls, problems) = skate_input::Controls::from_text(&text);
                for problem in &problems {
                    log.line(&format!("SkateGTA-controls.txt {problem}"));
                }
                controls
            }
            Err(_) => {
                let controls = crate::options::gta_controls();
                let _ = std::fs::write(&controls_path, controls.to_text());
                controls
            }
        };
        let mut script = Self {
            config,
            dir: dir.clone(),
            controls,
            keyboard: skate_input::Translator::new(),
            menu: None,
            options_key_was_down: false,
            radio_skip_was_down: false,
            radio_hint_shown: false,
            radio_held: None,
            radio_checked: None,
            closed_in: 0.,
            board_blows: 0,
            scuff_known: false,
            action_cam: Default::default(),
            action_cam_on: false,
            start_states: None,
            push_off: None,
            hud: crate::hud::Hud::default(),
            clock: Instant::now(),
            view_flipped: false,
            view: crate::cam_cycle::View::Gta(0),
            live: Default::default(),
            live_area: None,
            live_search: None,
            live_next: None,
            look: (0., 0.),
            look_moved: None,
            head: skate_rules::HeadCamera::default(),
            leaving: None,
            fall: skate_rules::FallTracker::default(),
            fallen: None,
            hom: None,
            hom_tick: None,
            injured: None,
            limping: None,
            store,
            character,
            sources: None,
            drawn: None,
            log,
            traced: 0,
            blend_out: None,
            tracing: false,
            pose_check: None,
            worker: None,
            names: None,
            waiting: false,
            riding: None,
            terrain,
            terrain_mesh: None,
            traffic: super::traffic::Traffic::new(),
            resume: None,
            vfunc: None,
            toggle_was_down: false,
            present_hooked: false,
            sound: None,
            sound_tried: false,
            dropper: super::dropper::GtaDropper::default(),
            combo_last: 0,
            retry_at: None,
            perf: (Instant::now(), 0),
            perf_checks: 0,
            terrain_centre: None,
            preloaded: false,
            carry_look: None,
            ride_parts: Default::default(),
            control_off_since: None,
            parks: None,
            park_blips: Vec::new(),
            park_blips_checked: None,
            hand_parts: Default::default(),
            showroom_parts: Default::default(),
            ground_sound: None,
            ground_look: 0,
            safe_feet: None,
            skeleton_cache: None,
            carry: Default::default(),
            aim_target: None,
            aim_rig: None,
            aim_turn: 0.,
            aim_hand: Vec3::ZERO,
            next_shot: Instant::now(),
            retries: 0,
            landings_seen: None,
            usage: skate_boards::wear::Usage::default(),
            usage_at: Instant::now(),
            wear_saved_at: Instant::now(),
            coop: None,
            coop_tried: false,
            remote: None,
            remote_board: None,
            remote_props: Vec::new(),
            remote_sound: None,
            showroom: None,
        };
        script.store.skater_view = script.config.shop_skater;
        // Load the session now: it takes seconds, and the first switch on
        // should not wait for it.
        script.start_worker();
        script
    }

    fn assets(&self) -> Result<PathBuf, String> {
        let assets = self
            .config
            .skate_assets
            .clone()
            .ok_or("Set SkateAssets in SkateGTA.ini to the mashup's skate-data\\assets folder.")?;
        if !assets.join("rig.json").is_file() {
            return Err(format!(
                "{} has no rig.json. Skate once in the mashup so it prepares the Skate data, then try again.",
                assets.display()
            ));
        }
        Ok(assets)
    }

    fn start_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        match self.assets().and_then(Worker::start) {
            Ok(worker) => {
                self.log.line("loading the Skate session");
                self.worker = Some(worker);
            }
            Err(e) => self.log.line(&format!("not loading Skate: {e}")),
        }
    }

    pub fn tick(&mut self) {
        // Never online: no keys taken (F8 is GTA Online's), nothing done.
        // (With BattlEye on the plugin is not loaded at all.)
        if n::online() {
            if self.riding.is_some() {
                self.stop("");
            }
            return;
        }
        // Skate loads (about ten seconds) while the game starts, not on the
        // first press of the toggle.
        if !self.preloaded {
            self.preloaded = true;
            if self.assets().is_ok() {
                self.start_worker();
            }
        }
        if self.limping.is_some() {
            self.limp(n::player_ped());
        }
        self.live_tick();
        super::crash::phase("blending out of a pose");
        self.blend_out_tick();
        super::crash::phase("waiting to get back on the board");
        self.resume_tick();
        let toggle = {
            let down = key_down(self.config.toggle_key) || (self.config.toggle_sticks && sticks_clicked());
            let pressed = down && !self.toggle_was_down;
            self.toggle_was_down = down;
            pressed
        };
        if toggle {
            let riding = self.riding.as_ref().and_then(|r| r.frame.as_ref()).is_some_and(|f| !skate_rules::on_foot(&f.state));
            if riding && self.config.start == skate_rules::StartRule::OnFoot && self.leaving.is_none() {
                // Step off first; skate mode ends once the skater stands.
                self.leaving = Some(Instant::now());
            } else if self.riding.is_some() || self.waiting {
                self.stop("");
            } else if self.resume.take().is_some() {
                // Asked off while waiting to get back on: stays off.
            } else {
                self.request();
            }
        }
        // A cutscene, a character switch, a mission taking control: skating
        // ends and does not come back by itself (it skated on through a
        // mission's cutscene after a fall).
        // GTA takes the player's control away for a moment now and then (an
        // ambient event, a phone call): only a longer spell is a mission's.
        if n::player_control_on() {
            self.control_off_since = None;
        } else if self.control_off_since.is_none() {
            self.control_off_since = Some(Instant::now());
        }
        let control_off = self.control_off_since.is_some_and(|at| at.elapsed() > Duration::from_millis(1500));
        let cutscene = n::cutscene_active();
        let switched = self.riding.as_ref().is_some_and(|r| r.ped != n::player_ped());
        let held_by_game = cutscene || control_off || switched;
        if held_by_game && (self.riding.is_some() || self.resume.is_some() || self.waiting) {
            let why = if cutscene { "a cutscene" } else if switched { "the player's character changed" } else { "the game took the player's control" };
            self.log.line(&format!("skating stopped: {why}"));
            self.stop("");
            self.resume = None;
        }
        // F8 is the Skate skater's (the fourth character, beside GTA's F5 to
        // F7): GTA's own F8 (to GTA Online) never fires.
        n::disable_control(0, 169);
        // GTA's wheel or F5 to F7 as the Skate skater: back to the story
        // character first, so GTA can switch.
        if self.character.is_skater() && [19, 166, 167, 168].iter().any(|c| n::disabled_control_pressed(0, *c)) {
            self.log.line("skater: back to the story character for GTA's switch");
            self.character.toggle();
        }
        // Switching character (Alt held for the wheel, F5 to F7) while skating:
        // skating ends so GTA can switch. Frozen and with its controls off, the
        // player could not, and Alt opened the recording menu instead.
        if self.riding.is_some() && [19, 166, 167, 168].iter().any(|c| n::disabled_control_pressed(0, *c)) {
            self.log.line("skating stopped: switching character");
            self.stop("");
            self.resume = None;
        }
        // The board as a weapon: F with it out gets on, F while riding gets
        // off (the board back in hand).
        let mut in_hand = false;
        if self.config.board_weapon {
            if self.riding.is_some() {
                self.carry.put_away();
                // F again: back to GTA on foot, the board in hand. On the
                // board, Skate's own get-off first (the board comes into the
                // hand), then GTA; on foot, at once.
                if n::disabled_control_just_pressed(0, super::carry::ENTER) && self.leaving.is_none() {
                    let on_board = self.riding.as_ref().and_then(|r| r.frame.as_ref()).is_some_and(|f| !skate_rules::on_foot(&f.state));
                    if on_board {
                        self.leaving = Some(Instant::now());
                    } else {
                        self.stop("");
                    }
                }
            } else if !held_by_game && !self.waiting && self.resume.is_none() && self.menu.is_none() && !self.store.open() {
                if self.carry.tick() {
                    self.request();

                } else if self.carry.holding() {
                    self.draw_carried_board(n::player_ped());
                } else if self.carry.holding_board() && self.parts_ready() && n::follow_view() != 4 {
                    // The weapon's own model hidden, the ridden board's
                    // parts on it. Not in first person: there the board
                    // filled half the screen (video 19-54 05:05).
                    // Not hidden: what is attached to a hidden entity hid
                    // with it, and the board never showed in the hand. The
                    // weapon is drawn see-through, the parts placed on it.
                    let held = n::weapon_entity(n::player_ped());
                    if held != 0 {
                        n::set_visible(held, true);
                        n::set_alpha(held, 0);
                        let choice = self.ridden_choice();
                        let extras = self.store.extras();
                        self.hand_parts.show(&choice, &extras, super::board_parts::Place::At(n::entity_matrix(held)));
                        in_hand = true;
                        // A blow struck with the board scuffs it.
                        self.board_blows += 1;
                        if self.board_blows % 6 == 0 {
                            let me = n::player_ped();
                            let at = n::entity_coords(me);
                            let mut hits = 0;
                            for ped in super::shv::all_peds() {
                                if ped != me && n::entity_coords(ped).distance(at) < 3.5 && n::damaged_by(ped, me) {
                                    n::clear_damage(ped);
                                    hits += 1;
                                }
                            }
                            if hits > 0 {
                                let library = self.sources.as_ref().and_then(|s| s.library.as_ref());
                                self.store.wear_board(library, &skate_boards::wear::Usage { hits, ..Default::default() });
                                self.store.update_scuff(library);
                                self.store.save_wear();
                            }
                        }
                    }
                }
            } else {
                self.carry.put_away();
            }
        }
        if !in_hand && self.hand_parts.shown() {
            self.hand_parts.clear();
        }
        self.park_blips();
        super::crash::phase("reading Skate's frames");
        self.handle_events();
        super::crash::phase("loading the board library");
        self.sources();
        if !self.scuff_known && let Some(library) = self.sources.as_ref().and_then(|s| s.library.as_ref()) {
            self.store.update_scuff(Some(library));
            self.scuff_known = true;
        }
        let options_key = input::vk_down(self.config.options_key);
        if options_key && !self.options_key_was_down && input::game_focused() && !self.store.open() {
            self.menu = match self.menu.take() {
                Some(_) => None,
                None => Some(OptionsMenu { row: 0, capture: None, edges: input::Edges::default() }),
            };
        }
        self.options_key_was_down = options_key;
        let menu_open = self.menu.is_some() || self.store.open();
        if let super::character::Event::StopSkating = self.character.tick(&self.config, self.riding.is_some() || self.waiting, menu_open) {
            self.stop("");
            self.character.toggle();
        }
        if self.menu.is_some() {
            self.options_tick();
        } else if !self.character.editing() {
            let library = self.sources.as_ref().and_then(|s| s.library.as_ref());
            let defaults = self.sources.as_ref().and_then(|s| s.defaults.as_ref().ok());
            self.store.posed = self.showroom.is_some();
            super::crash::phase("in a store");
            for note in std::mem::take(&mut self.store.notes) {
                self.log.line(&note);
            }
            if self.store.tick(&self.config, library, defaults, self.riding.is_some()) {
                self.drawn = None;
                // A new part bought: its wear (and scuffs) start over.
                self.scuff_known = false;
                // Painted wheels bring their hardness.
                if let Some(worker) = &self.worker {
                    worker.options(self.options());
                }
            }
            // In a shop, the character holds the board they look at.
            let posed = self.store.open()
                && self.store.skater_view
                && self.names.is_some()
                && self.riding.is_none()
                && self.config.board_look == Look::Skate3;
            if posed && self.showroom.is_none() {
                self.open_showroom();
            } else if !posed && self.showroom.is_some() {
                self.close_showroom();
            }
            if self.showroom.is_some() {
                super::crash::phase("posing the character in a store");
                self.showroom_tick();
            }
        }
        super::crash::phase("co-op");
        self.coop_tick();
        if self.riding.is_some() {
            let skip = input::vk_down(self.config.radio_skip_key) && input::game_focused();
            if skip && !self.radio_skip_was_down && self.riding.as_ref().is_some_and(|r| r.radio_before.is_some()) {
                n::skip_radio();
            }
            self.radio_skip_was_down = skip;
            // GTA switched the phone radio off (an interior, the phone, a
            // cutscene): on again.
            if self.radio_checked.is_none_or(|at| at.elapsed() > Duration::from_secs(3)) {
                self.radio_checked = Some(Instant::now());
                if let Some(station) = self.config.radio.clone()
                    && self.riding.as_ref().is_some_and(|r| r.radio_before.is_some())
                    && n::player_radio_station().as_deref() != Some(station.as_str())
                {
                    n::skate_radio(Some(&station));
                }
            }
            self.ride();
        } else if self.radio_held.is_some() {
            // Board put away or into a vehicle: GTA's own radio again.
            let ped = n::player_ped();
            if n::in_vehicle(ped) || n::selected_weapon(ped) != super::carry::board_weapon() || n::is_dead(ped) {
                n::skate_radio(None);
                if let Some(Some(station)) = self.radio_held.take() {
                    n::tune_radio(&station);
                }
                self.radio_held = None;
            }
        }
    }

    /// While skating, GTA's radio plays the skating station on foot (Self
    /// Radio, with the Skate 3 soundtrack); with the radio off in the
    /// options, the one from before comes back.
    fn start_radio(&mut self) {
        let Some(riding) = self.riding.as_mut() else { return };
        match (&self.config.radio, riding.radio_before.is_some()) {
            (Some(station), _) => {
                if riding.radio_before.is_none() {
                    // Still playing from the last ride: keep its "before".
                    riding.radio_before = Some(self.radio_held.take().unwrap_or_else(n::player_radio_station));
                }
                let station = station.clone();
                n::skate_radio(Some(&station));
                // Self Radio plays nothing until GTA has scanned its music
                // folder: say how, once.
                shv::wait(0);
                if !self.radio_hint_shown && n::player_radio_station().as_deref() != Some(station.as_str()) {
                    self.radio_hint_shown = true;
                    self.log.line(&format!("radio: GTA did not tune to {station} (Self Radio not scanned?)"));
                    n::notify("Skate 3 radio: in GTA's Settings > Audio set Self Radio Mode to Radio and run Perform Full Scan once.");
                }
            }
            (None, true) => {
                n::skate_radio(None);
                if let Some(Some(station)) = riding.radio_before.take() {
                    n::tune_radio(&station);
                }
            }
            (None, false) => {}
        }
    }

    /// Reads the board library and the default board once Skate's data is
    /// known.
    fn sources(&mut self) {
        if self.sources.is_some() {
            return;
        }
        let Ok(assets) = self.assets() else { return };
        let library = if skate_boards::Library::exists(&assets) {
            match skate_boards::Library::load(&assets) {
                Ok(library) => Some(library),
                Err(e) => {
                    self.log.line(&format!("board library: {e}"));
                    None
                }
            }
        } else {
            self.log.line("no board library: the mashup has not prepared Skate 3's board catalog");
            None
        };
        let defaults = skate_boards::Board::default_board(&assets, BOARD_TEXTURE);
        if let Err(e) = &defaults {
            self.log.line(&format!("default board: {e}"));
        }
        self.sources = Some(BoardSources { library, defaults });
    }

    /// Builds the Skate 3 board for the ridden choice; false when it cannot
    /// be drawn and the GTA model stands in.
    fn prepare_board(&mut self) -> bool {
        // The skate3weapon pack's board model: GTA draws and lights it.
        if self.config.board_look != Look::Skate3 || self.board_model_ready() || self.parts_ready() {
            self.drawn = None;
            return false;
        }
        self.sources();
        // The board as bought, its worn-through parts in their worn finish.
        let choice = self.store.ridden_look(self.sources.as_ref().and_then(|s| s.library.as_ref()));
        if self.drawn.as_ref().is_some_and(|d| d.choice == choice) {
            return true;
        }
        let Some(sources) = self.sources.as_ref() else { return false };
        let Ok(defaults) = sources.defaults.as_ref() else { return false };
        match skate_boards::assemble(sources.library.as_ref(), defaults, &choice, BOARD_TEXTURE) {
            Ok(board) => {
                let look = BoardLook::new(&board, self.config.board_detail, self.config.board_triangles);
                let textured = BoardLook::textured(&board);
                self.log.line(&format!(
                    "Skate 3 board: {} triangles ({} textured)",
                    look.triangle_count(),
                    textured.triangle_count()
                ));
                if self.config.board_textures {
                    for (dict, _) in textured.textures() {
                        n::request_texture_dict(dict);
                    }
                }
                self.drawn = Some(DrawnBoard {
                    choice,
                    look,
                    bound: None,
                    textured,
                    textured_bound: None,
                    textures_ready: false,
                    logged: false,
                });
                true
            }
            Err(e) => {
                self.log.line(&format!("Skate 3 board: {e}"));
                self.drawn = None;
                false
            }
        }
    }

    fn request(&mut self) {
        if let Err(e) = self.assets() {
            n::notify(&format!("Skate 3: {e}"));
            return;
        }
        self.start_worker();
        if self.names.is_some() {
            self.activate();
        } else {
            self.waiting = true;
            n::notify("Skate 3: loading Skate...");
        }
    }

    fn handle_events(&mut self) {
        let events = self.worker.as_ref().map(Worker::events).unwrap_or_default();
        for event in events {
            match event {
                Event::Ready { names } => {
                    self.log.line(&format!("Skate ready: {} bones", names.len()));
                    self.names = Some(names);
                    if std::mem::take(&mut self.waiting) {
                        self.activate();
                    }
                }
                Event::Activated(frame) => {
                    self.fall.reset();
                    if let Some(riding) = self.riding.as_mut() {
                        riding.frame = Some(frame);
                    } else if let Some(showroom) = self.showroom.as_mut() {
                        showroom.frame = Some(frame);
                    }
                }
                Event::Frame(frame) => {
                    // A bail goes back to the checkpoint a moment later.
                    let landings = frame.score.landings;
                    if self.landings_seen.is_some_and(|seen| seen != landings) {
                        if frame.score.bailed {
                            self.usage.bails += 1;
                            if self.config.auto_retry {
                                self.retry_at = Some(Instant::now() + RETRY_DELAY);
                            }
                        } else {
                            self.usage.landings += 1;
                            if let Some(frame) = self.riding.as_ref().and_then(|r| r.frame.as_ref()) {
                                let at = transform_from_skate(frame.root, self.riding.as_ref().map_or(Vec3::ZERO, |r| r.origin)).w_axis.truncate();
                                if frame.score.last_reward > 2000. {
                                    self.traffic.cheer(at + Vec3::Z);
                                }
                            }
                        }
                    }
                    self.landings_seen = Some(landings);
                    self.track_injuries(&frame);
                    if let Some(drop) = self.fall.update(&frame.state, frame.root.w_axis.y, frame.velocity.y) {
                        let percent = self.config.fall.damage(drop);
                        if percent > 0 {
                            self.log.line(&format!("fall of {drop:.1} m: {percent}%"));
                            self.fallen = Some(percent);
                        }
                    }
                    if let Some(riding) = self.riding.as_mut() {
                        riding.frame = Some(frame);
                    } else if let Some(showroom) = self.showroom.as_mut() {
                        showroom.frame = Some(frame);
                    }
                }
                Event::Note(text) => self.log.line(&text),
                Event::Failed(e) => {
                    self.log.line(&format!("Skate failed: {e}"));
                    n::notify(&format!("Skate 3 stopped: {e}"));
                    self.worker = None;
                    self.names = None;
                    self.waiting = false;
                    self.stop("");
                }
            }
        }
    }

    /// The wheels' hardness: painted wheels' own (unless the ini says
    /// otherwise), else the setting.
    fn wheel_hardness(&self) -> f32 {
        let painted = self.config.wheels_from_board.then(|| self.store.riding().get(skate_boards::Part::Wheels).and_then(skate_boards::paints::of)).flatten();
        painted.map_or(self.config.wheels, |p| p.finish.hardness())
    }

    fn options(&self) -> Options {
        Options {
            difficulty: self.config.difficulty.clone(),
            trucks: self.config.trucks,
            wheels: self.wheel_hardness(),
            tuning: self.config.tuning,
            hold_fakie: self.config.hold_fakie,
            hippie_jump: self.config.hippie_jump,
        }
    }

    /// Sends the ground around `centre` (as scanned) and the vehicles and
    /// people near it as Skate collision, Skate's origin at `origin`.
    fn send_collision(&mut self, origin: Vec3, centre: Vec3, now: bool) -> u64 {
        let revision = self.terrain.revision();
        // On foot, stairs are walked up as slopes.
        let on_foot = self.riding.as_ref().and_then(|r| r.frame.as_ref()).is_none_or(|f| skate_rules::on_foot(&f.state));
        if self.terrain_mesh.as_ref().is_none_or(|(r, c, f, _)| *r != revision || *c != centre || *f != on_foot) {
            self.terrain_mesh = Some((revision, centre, on_foot, self.terrain.build_for(centre, self.config.scan_radius, on_foot)));
        }
        if self.parks.is_none() {
            let parks = self.assets().map(|a| crate::parks::load_all(&a.join("parks"))).unwrap_or_default();
            self.log.line(&format!("skate parks with their own collision: {}", parks.iter().map(|(p, m)| format!("{} ({} triangles, {} rails)", p.id, m.triangles.len(), m.rails.len())).collect::<Vec<_>>().join(", ")));
            self.parks = Some(parks);
        }
        if self.live_covers(centre) {
            return self.send_live_collision(origin, centre, now, revision);
        }
        let Some((_, _, _, mesh)) = self.terrain_mesh.as_ref() else { return revision };
        // In a skate park its own collision takes the place of the probed
        // ground (whose steps the board caught on in bowls and on ramps).
        let reach = self.config.scan_radius + 3.;
        let parks: Vec<&crate::parks::ParkMesh> = self
            .parks
            .iter()
            .flatten()
            .filter(|(park, _)| park.centre.distance(centre.truncate()) < park.radius + reach)
            .map(|(_, mesh)| mesh)
            .collect();
        let in_park = |p: Vec3| parks.iter().any(|m| m.covers(p));
        // Probed floors go where the park has its own; probed walls go only
        // where the park has steep faces of its own (a bowl, a ramp: probed,
        // they were steps to fall through). A wall the park lacks (a
        // building that is its own object) stays, or the skater got stuck
        // inside it.
        let keep: Vec<bool> = mesh
            .triangles
            .iter()
            .map(|t| {
                let mid = (t[0] + t[1] + t[2]) / 3.;
                let up = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero().z.abs();
                if up < 0.5 { !parks.iter().any(|m| m.has_wall(mid)) } else { !in_park(mid) }
            })
            .collect();
        let mut triangles: Vec<[[f32; 3]; 3]> = mesh
            .triangles
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(t, _)| t.map(|p| to_skate(p - origin).to_array()))
            .collect();
        let mut surfaces = self.config.surfaces.then(|| mesh.surfaces.iter().zip(&keep).filter(|(_, k)| **k).map(|(s, _)| *s).collect::<Vec<u8>>());
        for park in &parks {
            let before = triangles.len();
            triangles.extend(park.near(centre, reach).map(|t| t.map(|p| to_skate(p - origin).to_array())));
            // Smooth concrete.
            if let Some(surfaces) = surfaces.as_mut() {
                surfaces.resize(surfaces.len() + triangles.len() - before, 0);
            }
        }
        // The ground's rails, the parks' (coping, ledges) and the vehicles'
        // (roof, hood and trunk edges).
        let rails: Vec<Vec<[f32; 3]>> = mesh
            .rails
            .iter()
            .filter(|r| !r.iter().any(|p| in_park(*p)))
            .cloned()
            .chain(parks.iter().flat_map(|m| m.rails_near(centre, reach).cloned()))
            // A roof edge out of an ollie's reach is no rail: the grind
            // assist pulled the skater up onto it.
            .chain(self.traffic.rails().into_iter().filter(|r| r.iter().any(|p| p.z - centre.z <= crate::terrain::MAX_RAIL_HEIGHT)))
            .map(|r| r.iter().map(|p| to_skate(*p - origin).to_array()).collect())
            .collect();
        let bodies = self.traffic.triangles();
        if !bodies.is_empty() {
            triangles.extend(bodies.iter().map(|t| t.map(|p| to_skate(p - origin).to_array())));
            // Paint and clothes: smooth.
            if let Some(surfaces) = surfaces.as_mut() {
                surfaces.resize(triangles.len(), 1);
            }
        }
        self.traffic.mark_sent();
        if now {
            self.log.line(&format!("collision: {} triangles ({} of vehicles and people), {} rails", triangles.len(), bodies.len(), rails.len()));
        }
        // Nothing not finite reaches Skate's physics (one NaN stops it).
        let finite = |p: &[f32; 3]| p.iter().all(|v| v.is_finite());
        let keep: Vec<bool> = triangles.iter().map(|t| t.iter().all(finite)).collect();
        if keep.iter().any(|k| !k) {
            let mut k = keep.iter();
            triangles.retain(|_| *k.next().unwrap_or(&true));
            if let Some(surfaces) = surfaces.as_mut() {
                let mut k = keep.iter();
                surfaces.retain(|_| *k.next().unwrap_or(&true));
            }
        }
        let rails: Vec<Vec<[f32; 3]>> = rails.into_iter().filter(|r| r.len() >= 2 && r.iter().all(finite)).collect();
        if let Some(worker) = &self.worker {
            worker.collision(triangles, surfaces, rails, now);
        }
        revision
    }

    fn activate(&mut self) {
        let ped = n::player_ped();
        if n::is_dead(ped) || n::in_vehicle(ped) {
            n::notify("Skate 3: get out of the vehicle first.");
            return;
        }
        let at = n::entity_coords(ped);
        let mut probe = n::GtaProbe { ignore: ped };
        let feet = probe
            .down(at.x, at.y, at.z, at.z - 3.)
            .map_or(at - Vec3::Z, |hit| Vec3::new(at.x, at.y, hit.z));
        let origin = feet;

        // The ground right around the player before the first tick, the
        // rest as the skater rides.
        let started = Instant::now();
        // Back on near where skating last began (after a bail, a run-over, a
        // swim): the ground scanned then is still there.
        if self.terrain_centre.is_none_or(|c| c.distance(feet) > 40.) {
            self.terrain = Terrain::new(self.config.scan_spacing).with_tall_walls(self.config.tall_walls);
        }
        self.terrain_centre = Some(feet);
        self.terrain_mesh = None;
        self.resume = None;
        // Where skating starts is safe ground: through the world at once,
        // the skater comes back here.
        self.safe_feet = Some(feet);
        self.traffic.reset();
        self.traffic.update(&[ped], feet, ACTIVATION_RADIUS, Duration::from_millis(4));
        let probes = self.terrain.scan(&mut probe, feet, ACTIVATION_RADIUS, || true);
        let scanned = started.elapsed();
        let revision = self.send_collision(origin, feet, true);
        let Some(worker) = &self.worker else { return };
        worker.options(self.options());
        let heading = n::entity_heading(ped);
        // At a run: straight onto the board and pushing on (Skate starts
        // still: on foot, the run stopped dead and the skater stumbled).
        let run = n::entity_velocity(ped).truncate().length();
        let on_foot = self.config.start == skate_rules::StartRule::OnFoot && run < RUN_ONTO_BOARD;
        // Rolling on at the run's speed (a sprint about 7 m/s; Skate's
        // push tops out near that); a short push after to keep it.
        let roll = if on_foot { 0. } else { run.min(8.) };
        worker.activate(to_skate(feet - origin).to_array(), skate_heading(heading), on_foot, roll);
        self.push_off = (!on_foot && run >= RUN_ONTO_BOARD).then(|| (Instant::now(), Duration::from_millis(150)));
        self.start_states = Some((Instant::now(), String::new()));
        if self.push_off.is_some() {
            self.log.line(&format!("start: at a run ({run:.1} m/s), onto the board"));
        }
        if self.config.board_weapon {
            n::set_current_weapon(ped, super::carry::UNARMED);
        }

        if !self.sound_tried {
            self.sound_tried = true;
            self.sound = super::sound::BoardSound::start(self.assets().ok().map(|a| a.join("sounds")));
            if self.sound.is_none() {
                self.log.line("no sound output for the board's sounds");
            }
        }
        self.landings_seen = None;
        self.retry_at = None;
        n::clear_tasks_now(ped);
        n::freeze(ped, true);
        n::set_collision(ped, false);
        n::set_can_ragdoll(ped, false);
        n::set_invincible(ped, true);
        super::crash::did("skate mode came on");
        self.perf = (Instant::now(), 0);
        let _ = super::crash::take_times();

        self.traced = 0;
        self.pose_check = Some((Instant::now(), hold::rewrites()));
        let before_skeleton = Instant::now();
        let skeleton = self.skeleton(ped);
        let skeleton_took = before_skeleton.elapsed();
        self.blend_out = None;
        // GTA's last pose, in the world: skating moves the ped to its pelvis,
        // and blended as relative to the ped it warped through itself.
        let standing = entity_transform(n::entity_coords(ped), heading);
        let blend_in = skeleton
            .as_ref()
            .and_then(|(s, _)| memory::read_pose(s.global, s.bones.len()))
            .map(|pose| (Instant::now(), pose.iter().map(|m| standing * *m).collect()));
        let board = if self.prepare_board() { 0 } else { self.spawn_board(feet) };
        self.log.line("start: camera");
        // One scripted camera for Skate's view and the first-person one;
        // GTA's own camera is used when neither is.
        let camera = n::create_camera();
        self.view_flipped = false;
        self.view = crate::cam_cycle::View::start(self.config.camera, n::follow_view());
        self.leaving = None;
        let (sin, cos) = heading.to_radians().sin_cos();
        self.head.start([-sin, 0., -cos]);
        self.riding = Some(Riding {
            ped,
            origin,
            board,
            camera,
            camera_on: false,
            skeleton,
            frame: None,
            heading,
            built: (revision, Instant::now(), feet),
            sent_origin: origin,
            radio_before: None,
            blend_in,
            last_pose: Vec::new(),
        });
        self.log.line("start: radio");
        self.start_radio();
        self.log.line(&format!(
            "start: on after {:.0} ms (ground {probes} probes {:.0} ms, skeleton {:.0} ms)",
            started.elapsed().as_secs_f32() * 1000.,
            scanned.as_secs_f32() * 1000.,
            skeleton_took.as_secs_f32() * 1000.
        ));
        n::notify("Skate 3: on. Press the toggle again to get off.");
    }

    /// The player's skeleton and the retarget onto it, unless posing is off
    /// or the skeleton cannot be read in this game version.
    fn skeleton(&mut self, ped: i32) -> Option<(Skeleton, Retarget)> {
        if self.config.pose_write == PoseWrite::Off {
            return None;
        }
        match self.read_skeleton(ped) {
            Ok((skeleton, retarget)) => {
                self.log.line(&format!(
                    "posing {} GTA bones, {} following Skate bones",
                    skeleton.bones.len(),
                    retarget.mapped()
                ));
                if matches!(self.config.pose_write, PoseWrite::Present | PoseWrite::Both) && !self.present_hooked {
                    shv::on_present(on_present);
                    self.present_hooked = true;
                }
                Some((skeleton, retarget))
            }
            Err(e) => {
                self.log.line(&format!("no Skate animations on the player: {e}"));
                n::notify(&format!("Skate 3: animations unavailable ({e}). Skating without them."));
                None
            }
        }
    }

    fn read_skeleton(&mut self, ped: i32) -> Result<(Skeleton, Retarget), String> {
        if let Some((cached, skeleton, retarget)) = &self.skeleton_cache
            && *cached == ped
            && skeleton.still_there()
        {
            return Ok((skeleton.clone(), retarget.clone()));
        }
        let offset = self.vfunc.get_or_insert_with(memory::frag_inst_vfunc_offset).clone()?;
        let skeleton = memory::skeleton(shv::entity_address(ped) as usize, offset)?;
        let gta = bind_pose(&skeleton.bones);
        let reference = load_reference(&self.assets()?.join("rig.json"))?;
        let names = self.names.as_deref().ok_or("Skate is not loaded")?;
        let retarget = Retarget::new(&gta, &reference, names, self.config.facing_degrees)?;
        if ped == n::player_ped() {
            self.skeleton_cache = Some((ped, skeleton.clone(), retarget.clone()));
        }
        Ok((skeleton, retarget))
    }

    /// The Skate 3 board as GTA's own model (the skate3weapon pack's), not
    /// drawn triangles: lit and filtered with the world.
    /// The skate parks on GTA's map.
    fn park_blips(&mut self) {
        if !self.config.park_blips || self.park_blips_checked.is_some_and(|at| at.elapsed() < Duration::from_secs(5)) {
            return;
        }
        self.park_blips_checked = Some(Instant::now());
        if !self.park_blips.is_empty() && self.park_blips.iter().all(|b| n::blip_exists(*b)) {
            return;
        }
        for blip in self.park_blips.drain(..) {
            if n::blip_exists(blip) {
                n::remove_blip(blip);
            }
        }
        for park in crate::parks::PARKS {
            // Far away the ground is not loaded to find; the map is flat.
            let blip = n::add_blip(park.centre.extend(0.), PARK_SPRITE, PARK_COLOUR, park.name);
            if blip != 0 {
                self.park_blips.push(blip);
            }
        }
    }

    fn parts_ready(&self) -> bool {
        self.config.board_look == Look::Skate3 && super::board_parts::available()
    }

    /// The board ridden as it looks now (worn parts worn).
    fn ridden_choice(&self) -> skate_boards::BoardChoice {
        self.store.ridden_look(self.sources.as_ref().and_then(|s| s.library.as_ref()))
    }

    fn board_model_ready(&self) -> bool {
        self.config.board_look == Look::Skate3 && super::carry::board_weapon() == super::carry::skateboard()
    }

    fn spawn_board(&mut self, at: Vec3) -> i32 {
        // The part models are placed by `present_frame`.
        if self.parts_ready() {
            let names = skate_boards::Part::ALL.map(|p| super::board_parts::model_name(p, &self.ridden_choice()));
            self.log.line(&format!("board: HD part models {names:?}"));
            return 0;
        }
        self.log.line("board: no HD part models in the skate3weapon pack");
        if self.board_model_ready() {
            let weapon = super::carry::skateboard();
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let board = n::create_weapon_object(weapon, at);
                if board != 0 {
                    n::set_collision(board, false);
                    return board;
                }
                if Instant::now() > deadline {
                    self.log.line("the board model did not load: drawn instead");
                    break;
                }
                shv::wait(0);
            }
        }        let hash = joaat(&self.config.board_model);
        if !n::model_valid(hash) {
            self.log.line(&format!("board model {} does not exist", self.config.board_model));
            return 0;
        }
        n::request_model(hash);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !n::model_loaded(hash) && Instant::now() < deadline {
            shv::wait(0);
        }
        if !n::model_loaded(hash) {
            self.log.line("board model did not load");
            return 0;
        }
        let board = n::create_object(hash, at);
        n::release_model(hash);
        n::set_collision(board, false);
        board
    }

    fn ride(&mut self) {
        let Some(riding) = self.riding.as_ref() else { return };
        let ped = riding.ped;
        if !n::entity_exists(ped) || n::is_dead(ped) || ped != n::player_ped() {
            self.stop("");
            return;
        }
        if n::pause_menu() {
            if let Some(worker) = &self.worker {
                worker.suspend();
            }
            if let Some(sound) = &self.sound {
                sound.quiet();
            }
            return;
        }
        if let Some(hurt) = self.injured.take() {
            let worst = hurt.worst(3);
            let points = hurt.points();
            self.log.line(&format!("hall of meat: ${points}, {worst:?}"));
            let text: Vec<String> = worst.iter().map(|(bone, level)| format!("{level} {bone}")).collect();
            n::notify(&format!("Hall of Meat: ${points}~n~{}", text.join("~n~")));
            if hurt.leg_broken() {
                self.limping = Some(Instant::now());
            }
            // Skate's points as health (a broken skull, $5000, costs a
            // quarter, at most 35 %), and a bail alone never kills.
            let percent = (points / 200).min(35) as i32;
            if percent > 0 {
                let above = (n::max_health(ped) - 100).max(1);
                let left = (n::health(ped) - above * percent / 100).max(101 + above / 10);
                n::set_health(ped, left.min(n::health(ped)));
            }
        }
        if let Some(percent) = self.fallen.take() {
            // GTA's characters die at 100 health: the damage is a share of
            // what lies above that.
            let above = (n::max_health(ped) - 100).max(1);
            let left = n::health(ped) - above * percent as i32 / 100;
            if percent >= 100 || left <= 100 {
                self.stop("");
                n::set_health(ped, 0);
                return;
            }
            n::set_health(ped, left);
        }
        n::disable_player_controls();
        // GTA's pause menu (Esc) stays open to the player while skating;
        // Skate is suspended while it shows. (P is Skate's 360 flip.)
        n::enable_control(0, 200);
        // GTA's interaction menu (M) too.
        n::enable_control(0, 244);
        // R: the cinematic camera, as in a car (not while aiming).
        if n::disabled_control_just_pressed(0, 45) && !n::disabled_control_pressed(0, 25) {
            self.action_cam_on = !self.action_cam_on;
            self.action_cam.reset();
            self.log.line(&format!("cinematic camera {}", if self.action_cam_on { "on" } else { "off" }));
        }
        const TRACED: u32 = 4;
        let tracing = self.traced < TRACED;
        self.tracing = tracing;
        if tracing {
            self.traced += 1;
            self.log.line(&format!("frame {}: scan", self.traced));
        }

        // Scan ahead of the skater within this frame's budget, and send new
        // collision when the scan grew or the skater moved on.
        let origin = riding.origin;
        let feet = riding
            .frame
            .as_ref()
            .map_or(origin, |f| transform_from_skate(f.root, origin).w_axis.truncate());
        let budget = Duration::from_secs_f32(self.config.scan_budget_ms / 1000.);
        let started = Instant::now();
        let mut probe = n::GtaProbe { ignore: ped };
        super::crash::phase("scanning the ground");
        // GTA's own collision round the skater (asked for as they move on);
        // the probes only where it is not (yet) there.
        if self.live.level.is_some() && self.live.asked.is_none_or(|a| a.truncate().distance(feet.truncate()) > crate::live::RECENTRE) {
            self.live.request(feet);
        }
        if !self.live_covers(feet) {
            self.terrain.scan(&mut probe, feet, self.config.scan_radius, || started.elapsed() < budget);
        }
        // Vehicles and people around; a vehicle under the skater carries
        // them (Skate's origin moves along with it).
        let skip: Vec<i32> = std::iter::once(ped).chain(self.remote.as_ref().map(|r| r.ped)).collect();
        let traffic_budget = Duration::from_secs_f32(self.config.scan_budget_ms / 2000.);
        super::crash::phase("finding vehicles and people");
        self.traffic.update(&skip, feet, self.config.scan_radius, traffic_budget);
        super::crash::phase("riding along on a vehicle");
        let (state, velocity) = riding
            .frame
            .as_ref()
            .map_or((String::new(), Vec3::ZERO), |f| (f.state.clone(), from_skate(f.velocity)));
        let grounded = !state.contains("Air");
        let carried = self.traffic.carry(feet, grounded, n::frame_time().clamp(0., 0.1));
        let mut origin = origin;
        if carried != Vec3::ZERO && carried.length() < 5. {
            origin += carried;
            if let Some(riding) = self.riding.as_mut() {
                riding.origin = origin;
            }
        }
        let feet = feet + carried;

        // Out of the world (a gap in GTA's collision, off a pier into water
        // it did not report): back to where the skater last stood.
        // Safe only standing still upright on GTA's own ground: falling
        // through the world, Skate's state need not say so, and the place to
        // come back to went down with the skater.
        let solid = grounded && !state.is_empty() && velocity.z.abs() < 3. && {
            let mut probe = n::GtaProbe { ignore: ped };
            probe.down(feet.x, feet.y, feet.z + 0.5, feet.z - 1.5).is_some()
        };
        if solid {
            self.safe_feet = Some(feet);
        }
        if let Some(safe) = self.safe_feet
            && feet.z < safe.z - FELL_THROUGH
        {
            self.log.line(&format!("fell out of the world at {feet}: back to {safe}"));
            // The fall is no wipeout to count.
            if let Some(hom) = self.hom.as_mut() {
                hom.finish();
            }
            self.hom_tick = None;
            self.stop("");
            n::set_coords(ped, safe + Vec3::Z);
            return;
        }

        // Run over by a vehicle: knocked off the board, onto the street.
        super::crash::phase("checking for vehicles hitting the skater");
        let points = [feet + Vec3::Z * 0.3, feet + Vec3::Z * 0.95, feet + Vec3::Z * 1.5];
        if self.config.traffic
            && let Some(impact) = self.traffic.run_over(&points, velocity)
        {
            self.log.line(&format!("run over at {:.1} m/s", impact.speed));
            super::crash::did("the skater was run over");
            self.knocked_down(impact.velocity * 1.1 + Vec3::Z * (1.5 + 0.12 * impact.speed), impact.speed);
            return;
        }
        super::crash::phase("checking for water");
        // Into deep water: GTA swims from here.
        if let Some(surface) = n::water_height(feet.x, feet.y, feet.z + 3.)
            && surface > feet.z + 0.6
        {
            self.log.line("into the water: swimming");
            super::crash::did("the skater went into the water");
            self.stop("");
            n::set_velocity(ped, velocity * 0.5);
            if self.config.auto_resume {
                self.resume = Some(None);
            }
            return;
        }
        super::crash::phase("pushing doors and things");
        if self.config.traffic {
            self.traffic.push_objects(ped, feet, velocity);
        }
        super::crash::phase("people reacting");
        if self.config.traffic && self.config.people {
            let knocked = self.traffic.people(ped, feet + Vec3::Z * 0.95, velocity);
            if knocked > 0 {
                self.log.line(&format!("knocked {knocked} over"));
            }
        }

        let Some(riding) = self.riding.as_ref() else { return };
        let (built_revision, built_at, built_centre) = riding.built;
        let since = built_at.elapsed();
        let moved_origin = riding.sent_origin.distance(origin) > 0.03;
        // Getting on or off the board: stairs change between steps and slopes.
        let foot_changed = riding.frame.as_ref().is_some_and(|f| {
            self.terrain_mesh.as_ref().is_some_and(|m| m.2 != skate_rules::on_foot(&f.state))
        });
        if foot_changed
            || (self.terrain.revision() != built_revision && since > Duration::from_millis(250))
            || built_centre.distance(feet) > 2.
            || ((self.traffic.changed() || moved_origin) && since > Duration::from_millis(90))
        {
            super::crash::phase("sending collision to Skate");
            let revision = self.send_collision(origin, feet, false);
            if let Some(riding) = self.riding.as_mut() {
                riding.built = (revision, Instant::now(), feet);
                riding.sent_origin = origin;
            }
        }

        let dt = n::frame_time().clamp(0., 0.1);
        // The controller and, while GTA has the keyboard, keys and mouse.
        let held = input::controller().unwrap_or_default();
        let typed = if self.menu.is_none() && input::game_focused() {
            let mouse = input::mouse_motion(self.config.mouse_scale);
            let pad = self.keyboard.update(&self.controls, input::key_down, mouse, dt);
            if self.keyboard.view_pressed() {
                self.next_view();
            }
            pad
        } else {
            self.keyboard.reset();
            skate_input::Pad::default()
        };
        let mut pad = held.merge(typed);
        // V (GTA's next camera) cycles GTA's views and then Skate's camera.
        if self.menu.is_none() && n::using_keyboard() && n::disabled_control_just_pressed(0, 0) {
            self.next_view();
        }
        // GTA's camera: the mouse turns it as on foot, unless it works the
        // trick stick (its key held).
        if self.view.camera() == CameraMode::Gta && self.menu.is_none() {
            if n::using_keyboard() && !self.keyboard.mouse_tricks() {
                n::enable_control(0, 1);
                n::enable_control(0, 2);
            }
        }
        self.weapons(&mut pad);
        // LB with the D-pad's right opens the object dropper, with its left
        // goes back to the checkpoint (keyboard: their own keys, as LB + D-pad).
        use skate_park::buttons as pb;
        let combo = if pad.buttons & pb::LB != 0 { pad.buttons & (pb::DPAD_LEFT | pb::DPAD_RIGHT) } else { 0 };
        let combo_pressed = combo & !self.combo_last;
        self.combo_last = combo;
        let (camera_forward, _, _) = n::camera_axes();
        if combo_pressed & pb::DPAD_RIGHT != 0 && self.leaving.is_none() {
            self.dropper.toggle(feet, camera_forward);
        }
        if combo_pressed & pb::DPAD_LEFT != 0 && !self.dropper.open() {
            self.retry_at = Some(Instant::now());
        }
        if pad.buttons & pb::LB != 0 {
            pad.buttons &= !(pb::DPAD_LEFT | pb::DPAD_RIGHT);
        }
        if let Some(at) = self.retry_at
            && Instant::now() >= at
            && let Some(worker) = &self.worker
        {
            worker.retry();
            self.retry_at = None;
            self.retries = self.retries.wrapping_add(1);
        }
        if self.dropper.open() {
            // Building with the dropper, the skater waits as in Skate 3's
            // menus; a changed park is scanned anew.
            if self.dropper.tick(&pad, camera_forward, feet, dt) {
                self.terrain = Terrain::new(self.config.scan_spacing).with_tall_walls(self.config.tall_walls);
                if let Some(riding) = self.riding.as_mut() {
                    riding.built.0 = u64::MAX;
                }
            }
            if let Some(worker) = &self.worker {
                worker.suspend();
            }
            if let Some(sound) = &self.sound {
                sound.quiet();
            }
            self.present_frame();
            self.draw_board();
            self.draw_hud();
            return;
        }
        // After a moment standing on the board (Skate settles the feet on
        // it first: pushing at once, the skater bailed), a short push.
        if let Some((since, long)) = self.push_off {
            let settle = Duration::from_millis(400);
            let t = since.elapsed();
            if t >= settle && t < settle + long {
                pad.buttons |= pb::A;
            } else if t >= settle + long {
                self.push_off = None;
            }
        }
        // Skate's states in the first two seconds of skating, for the log.
        if let Some((since, last)) = self.start_states.as_mut() {
            let state = self.riding.as_ref().and_then(|r| r.frame.as_ref()).map(|f| f.state.clone()).unwrap_or_default();
            if since.elapsed() > Duration::from_secs(2) {
                self.start_states = None;
            } else if state != *last {
                let at = since.elapsed().as_millis();
                *last = state.clone();
                self.log.line(&format!("start: {at} ms {state}"));
            }
        }
        if let Some(since) = self.leaving {
            let on_foot = self.riding.as_ref().and_then(|r| r.frame.as_ref()).is_some_and(|f| skate_rules::on_foot(&f.state));
            if on_foot || since.elapsed() > Duration::from_millis(2500) {
                self.leaving = None;
                self.stop("");
                return;
            }
            // Skate's own get-off (Y) once, then hands off.
            let first = since.elapsed() < Duration::from_millis(40);
            pad = skate_input::Pad { buttons: if first { 0x8000 } else { 0 }, ..Default::default() };
        }
        if tracing {
            self.log.line("  step");
        }
        super::crash::phase("stepping Skate");
        if let Some(worker) = &self.worker {
            worker.step(dt, n::aspect_ratio().max(0.5), Some(pad));
        }
        if tracing {
            self.log.line("  sound");
        }
        super::crash::phase("board sounds and wear");
        self.board_sound_and_wear(dt);
        // A board bought or changed in a store shows from the next ride;
        // switched from here when its look was cleared mid-ride.
        if self.drawn.is_none() && self.config.board_look == Look::Skate3 && self.riding.as_ref().is_some_and(|r| r.board == 0) {
            self.prepare_board();
        }
        if tracing {
            self.log.line(&format!("  pose (skate frame {})", self.riding.as_ref().is_some_and(|r| r.frame.is_some())));
        }
        super::crash::phase("posing the player, camera");
        self.present_frame();
        if tracing {
            self.log.line("  board");
        }
        super::crash::phase("drawing the board");
        self.draw_board();
        if tracing {
            self.log.line("  hud");
        }
        super::crash::phase("drawing the HUD");
        self.draw_hud();
        super::crash::phase("between frames");
        if tracing {
            self.log.line("  done");
        }
        if let Some((since, count)) = self.pose_check
            && since.elapsed() > Duration::from_secs(3)
        {
            self.pose_check = None;
            let seconds = since.elapsed().as_secs_f32();
            self.log.line(&format!(
                "pose ({:?}): GTA wrote its own pose over Skate's {} times in {seconds:.1} s",
                self.config.pose_write,
                hold::rewrites() - count
            ));
        }
        // Where each frame's time goes, every few seconds: GTA's own share
        // is "between frames".
        self.perf.1 += 1;
        if self.perf.0.elapsed() > Duration::from_secs(5) {
            let (seconds, frames) = (self.perf.0.elapsed().as_secs_f32(), self.perf.1.max(1));
            let parts: Vec<String> = super::crash::take_times()
                .iter()
                .map(|(name, spent)| format!("{name} {:.1}", spent.as_secs_f32() * 1000. / frames as f32))
                .collect();
            let checks = hold::checks();
            self.log.line(&format!(
                "perf: {:.0} fps; pose thread {:.0} looks/s; ground probes GTA could not run {}; ms per frame: {}",
                frames as f32 / seconds,
                (checks - self.perf_checks) as f32 / seconds,
                n::take_probe_failures(),
                parts.join(", ")
            ));
            self.perf_checks = checks;
            self.perf = (Instant::now(), 0);
        }
    }

    /// The board's sounds, and the wear riding puts on it.
    fn board_sound_and_wear(&mut self, dt: f32) {
        let Some(riding) = self.riding.as_ref() else { return };
        let Some(frame) = riding.frame.as_ref() else { return };
        // What is rolled on, for its sound: GTA's material under the board,
        // looked at a few times a second.
        self.ground_look = self.ground_look.wrapping_add(1);
        if self.ground_look % 6 == 0 {
            let feet = transform_from_skate(frame.root, riding.origin).w_axis.truncate();
            self.ground_sound = n::material_below(feet + Vec3::Z * 0.5, feet - Vec3::Z * 0.6, riding.ped).map(crate::ground::rolling_sound_of);
        }
        // How closed in the skater is, twice a second: lines out from the
        // head (up and round about) that meet something within 14 m; in a
        // building at least most.
        if self.ground_look % 30 == 0 {
            let head = transform_from_skate(frame.root, riding.origin).w_axis.truncate() + Vec3::Z * 1.6;
            let dirs = [Vec3::Z, Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::new(0.7, 0.7, 0.3), Vec3::new(-0.7, -0.7, 0.3)];
            let met = dirs.iter().filter(|d| n::probe_line(head, head + **d * 14., 1, riding.ped).is_some()).count();
            let mut closed = met as f32 / dirs.len() as f32;
            // Open sky above: not a room, whatever is round about.
            if n::probe_line(head, head + Vec3::Z * 30., 1, riding.ped).is_none() {
                closed *= 0.35;
            }
            if n::interior_place(riding.ped).is_some() {
                closed = closed.max(0.8);
            }
            self.closed_in = closed;
        }
        if let Some(sound) = &self.sound {
            let deck = transform_from_skate(frame.board, riding.origin).w_axis.truncate();
            let (_, right, _) = n::camera_axes();
            sound.set_space(crate::space::Space::heard(n::camera_coord().to_array(), right.to_array(), deck.to_array(), self.closed_in));
        }
        if let Some(sound) = &self.sound {
            sound.update(
                &frame.state,
                frame.velocity.length(),
                frame.velocity.y,
                frame.wheels_down,
                frame.surface,
                dt,
                self.config.board_sound,
                self.ground_sound,
                self.wheel_hardness() >= 0.5,
            );
        }
        match frame.state.as_str() {
            "GrindFiftyFifty" | "GrindFiveO" | "GrindBackslash" => self.usage.truck_grind += dt,
            "GrindBoardslide" | "GrindTipslide" | "GrindDarkslide" => self.usage.deck_slide += dt,
            "SlideGround" => self.usage.powerslide += dt,
            _ => {}
        }
        if frame.wheels_down > 0 && !skate_rules::on_foot(&frame.state) {
            self.usage.rolled += frame.velocity.length() * dt;
        }
        if self.usage_at.elapsed() > Duration::from_secs(2) {
            self.usage_at = Instant::now();
            let usage = std::mem::take(&mut self.usage);
            let library = self.sources.as_ref().and_then(|s| s.library.as_ref());
            if self.store.wear_board(library, &usage) {
                n::notify("Skate 3: a board part wore through. Buy it again in a clothing store for a new one.");
                self.drawn = None;
            }
            self.store.update_scuff(library);
        }
        if self.wear_saved_at.elapsed() > Duration::from_secs(30) {
            self.wear_saved_at = Instant::now();
            self.store.save_wear();
        }
    }

    /// Skate 3's trick and score HUD.
    fn draw_hud(&mut self) {
        let Some(frame) = self.riding.as_ref().and_then(|r| r.frame.as_ref()) else { return };
        if self.menu.is_some() {
            return;
        }
        let now = self.clock.elapsed().as_secs_f64();
        let pad = input::controller().is_some();
        let on_foot = skate_rules::on_foot(&frame.state);
        let mut lines = self.hud.lines(&frame.score, now);
        lines.extend(self.hud.marker_lines(&frame.marker, &self.controls, pad, now));
        let view = self.dropper.view();
        let view = crate::hud::DropperView {
            open: view.open,
            object: view.object,
            previous: view.previous,
            next: view.next,
            placed: view.placed,
            max: view.max,
            pad,
        };
        if self.config.show_controls && !view.open {
            lines.extend(self.hud.controls_lines(&self.controls, pad, on_foot));
        }
        lines.extend(self.hud.dropper_lines(&view, self.retries, &self.controls, now));
        for line in lines {
            if line.from_left() {
                n::draw_text(&line.text, line.x, line.y, line.scale, line.rgba, false);
            } else {
                n::draw_text_centred(&line.text, line.x, line.y, line.scale, line.rgba, line.right);
            }
        }
    }

    /// Draws the Skate 3 board on the Skate pose's board bones: textured
    /// when the skater pack's board textures are loaded, flat otherwise.
    fn draw_board(&mut self) {
        let (Some(riding), Some(drawn)) = (self.riding.as_ref(), self.drawn.as_mut()) else { return };
        let Some(frame) = riding.frame.as_ref() else { return };
        if !drawn.textures_ready && self.config.board_textures {
            let mut ready = true;
            for (dict, _) in drawn.textured.textures() {
                if !n::texture_dict_loaded(dict) {
                    n::request_texture_dict(dict);
                    ready = false;
                }
            }
            if ready {
                drawn.textures_ready = true;
                self.log.line("board: textured");
                super::crash::did("the board is drawn textured");
            }
        }
        let names = self.names.as_deref();
        if drawn.bound.is_none() {
            drawn.bound = names.and_then(|names| drawn.look.bind(names));
        }
        if drawn.textured_bound.is_none() {
            drawn.textured_bound = names.and_then(|names| drawn.textured.bind(names));
        }
        let light = daylight(n::clock_hours());
        let eye = n::camera_coord();
        n::backface_culling(false);
        if drawn.textures_ready
            && let Some(bound) = drawn.textured_bound.as_ref()
        {
            let polys = drawn.textured.polys(&frame.bones, bound, riding.origin, eye, light);
            if self.tracing && !drawn.logged {
                drawn.logged = true;
                self.log.line(&format!("  board: {} textured triangles to draw", polys.len()));
            }
            let textures = drawn.textured.textures();
            for poly in polys {
                let Some((dict, name)) = textures.get(usize::from(poly.texture)) else { continue };
                let v = (poly.shade.clamp(0., 1.) * 255.) as u8;
                n::draw_textured_poly(poly.points, [v, v, v, 255], dict, name, poly.uv);
            }
            return;
        }
        let Some(bound) = drawn.bound.as_ref() else { return };
        let polys = drawn.look.polys(&frame.bones, bound, riding.origin, eye, light);
        if self.tracing && !drawn.logged {
            drawn.logged = true;
            self.log.line(&format!("  board: {} triangles to draw", polys.len()));
        }
        for poly in polys {
            n::draw_poly(poly.points, poly.rgba);
        }
    }

    /// Skate's Hall of Meat: the bones' knocks while Skate's state is a
    /// wipeout; the injuries when it ends.
    fn track_injuries(&mut self, frame: &crate::worker::Frame) {
        if !self.config.injuries {
            return;
        }
        let wiping_out = frame.state.contains("Wipeout");
        if wiping_out {
            if self.hom.is_none() {
                let Some(names) = self.names.as_deref() else { return };
                self.hom = Some(crate::hom::Tracker::new(names));
            }
            let dt = self.hom_tick.map_or(0., |t| frame.tick.saturating_sub(t) as f32 / 60.);
            self.hom_tick = Some(frame.tick);
            let at: Vec<[f32; 3]> = frame.bones.iter().map(|m| m.w_axis.truncate().to_array()).collect();
            if let Some(hom) = self.hom.as_mut() {
                hom.frame(&at, dt);
            }
        } else if self.hom_tick.take().is_some()
            && let Some(hom) = self.hom.as_mut()
        {
            let hurt = hom.finish();
            if hurt.any() {
                self.injured = Some(hurt);
            }
        }
    }

    /// A broken leg limps on foot (GTA's injured walk) for two minutes.
    fn limp(&mut self, ped: i32) {
        let Some(since) = self.limping else { return };
        if since.elapsed().as_secs() > 120 || n::health(ped) >= n::max_health(ped) {
            n::reset_movement_clipset(ped);
            self.limping = None;
        } else if self.riding.is_none() && n::anim_set_ready("move_m@injured") {
            n::set_movement_clipset(ped, "move_m@injured");
        }
    }

    /// GTA's collision table: searched for (on a thread, every 5 s until
    /// found, once the world has 20 peds, vehicles and objects), handed to
    /// the reader; finished areas taken and the collision resent.
    fn live_tick(&mut self) {
        if let Some(search) = &self.live_search
            && let Ok(found) = search.try_recv()
        {
            self.live_search = None;
            match found {
                Some(base) => {
                    let base_image = super::memory::game_image_range();
                    self.log.line(&format!("gta collision: physics level table at {base:#x}: GTA's own collision is the ground now"));
                    self.live.set_level(crate::live::level::Level { base, image: base_image });
                }
                None => self.log.line("gta collision: table not found yet; the probes stay"),
            }
        }
        if self.live.level.is_none()
            && self.live_search.is_none()
            && self.config.live_collision
            && self.live_next.is_none_or(|t| Instant::now() >= t)
        {
            self.live_next = Some(Instant::now() + Duration::from_secs(5));
            if let Ok(offset) = self.live_offset() {
                let insts: Vec<u64> = super::shv::all_peds()
                    .into_iter()
                    .chain(super::shv::all_vehicles())
                    .chain(super::shv::all_objects())
                    .map(|h| super::memory::frag_inst(super::shv::entity_address(h) as usize, offset) as u64)
                    .filter(|&i| i != 0)
                    .collect();
                if insts.len() >= 20 {
                    let (send, receive) = std::sync::mpsc::channel();
                    let _ = std::thread::Builder::new().name("gta collision search".into()).spawn(move || {
                        let _ = send.send(crate::live::locate::locate(&insts));
                    });
                    self.live_search = Some(receive);
                }
            }
        }
        if let Some(built) = self.live.take() {
            match built {
                Ok(area) => {
                    self.log.line(&format!("gta collision: {} triangles, {} rails round {:.0} {:.0} ({})", area.triangles.len(), area.rails.len(), area.centre.x, area.centre.y, area.note));
                    self.live_area = Some(area);
                    // Sent at once.
                    if let Some(riding) = self.riding.as_mut() {
                        riding.built.0 = u64::MAX;
                    }
                }
                Err(e) => self.log.line(&format!("gta collision: {e}")),
            }
        }
    }

    fn live_offset(&mut self) -> Result<usize, String> {
        static OFFSET: std::sync::OnceLock<Result<usize, String>> = std::sync::OnceLock::new();
        OFFSET.get_or_init(super::memory::frag_inst_vfunc_offset).clone()
    }

    /// GTA's own collision covers the scan round `feet`.
    fn live_covers(&self, feet: Vec3) -> bool {
        self.live_area.as_ref().is_some_and(|a| {
            let d = (a.centre - feet).truncate().abs();
            d.x.max(d.y) < crate::live::HALF - self.config.scan_radius - 4. && (a.centre.z - feet.z).abs() < 60.
        })
    }

    /// As `send_collision`, from GTA's own collision: its triangles and rails
    /// within the scan round `centre`, the vehicles and people as before.
    fn send_live_collision(&mut self, origin: Vec3, centre: Vec3, now: bool, revision: u64) -> u64 {
        let Some(area) = self.live_area.as_ref() else { return revision };
        let reach = self.config.scan_radius + 6.;
        let near = |p: Vec3| (p - centre).truncate().length() <= reach && (p.z - centre.z).abs() < 30.;
        let mut triangles = Vec::new();
        let mut surfaces = Vec::new();
        for (t, g) in area.triangles.iter().zip(&area.grounds) {
            let mid = (t[0] + t[1] + t[2]) / 3.;
            // Big ones (a street) by their nearest corner too.
            if near(mid) || t.iter().any(|p| near(*p)) || (mid - centre).truncate().length() < reach + 20. && (t[0] - t[1]).length().max((t[1] - t[2]).length()) > 10. {
                triangles.push(t.map(|p| to_skate(p - origin).to_array()));
                surfaces.push(*g);
            }
        }
        let statics = triangles.len();
        let rails: Vec<Vec<[f32; 3]>> = area
            .rails
            .iter()
            .filter(|r| r.iter().any(|p| near(*p)))
            .cloned()
            .chain(self.traffic.rails().into_iter().filter(|r| r.iter().any(|p| p.z - centre.z <= crate::terrain::MAX_RAIL_HEIGHT)))
            .map(|r| r.iter().map(|p| to_skate(*p - origin).to_array()).collect())
            .filter(|r: &Vec<[f32; 3]>| r.len() >= 2 && r.iter().all(|p| p.iter().all(|v| v.is_finite())))
            .collect();
        let bodies = self.traffic.triangles();
        triangles.extend(bodies.iter().map(|t| t.map(|p| to_skate(p - origin).to_array())));
        surfaces.resize(triangles.len(), 1);
        self.traffic.mark_sent();
        if now {
            self.log.line(&format!("collision (GTA's own): {statics} triangles, {} of vehicles and people, {} rails", bodies.len(), rails.len()));
        }
        let finite = |p: &[f32; 3]| p.iter().all(|v| v.is_finite());
        let keep: Vec<bool> = triangles.iter().map(|t| t.iter().all(finite)).collect();
        let mut k = keep.iter();
        triangles.retain(|_| *k.next().unwrap_or(&true));
        let mut k = keep.iter();
        surfaces.retain(|_| *k.next().unwrap_or(&true));
        let surfaces = self.config.surfaces.then_some(surfaces);
        if let Some(worker) = &self.worker {
            worker.collision(triangles, surfaces, rails, now);
        }
        revision
    }

    /// The next camera (V): GTA's views, then Skate 3's, and round.
    fn next_view(&mut self) {
        if let crate::cam_cycle::View::Gta(_) = self.view {
            self.view = crate::cam_cycle::View::Gta(n::follow_view());
        }
        self.view = self.view.next();
        if let crate::cam_cycle::View::Gta(mode) = self.view {
            n::set_follow_view(mode);
        }
        n::help(self.view.label());
        self.log.line(&format!("camera: {}", self.view.label()));
    }

    /// GTA's weapons while skating (keyboard and mouse: the controller's
    /// buttons are all Skate's): the weapon wheel (Tab, 1-8, mouse wheel),
    /// aiming with the right mouse button (the body bends toward it, see
    /// `crate::aim`) and shooting with the left. Walking goes the way GTA's
    /// camera looks, as on foot in GTA.
    fn weapons(&mut self, pad: &mut skate_input::Pad) {
        self.aim_target = None;
        if self.view.camera() != CameraMode::Gta || self.menu.is_some() {
            return;
        }
        let Some(riding) = self.riding.as_ref() else { return };
        let ped = riding.ped;
        let on_foot = riding.frame.as_ref().is_some_and(|f| skate_rules::on_foot(&f.state));
        if on_foot && let Some((_, skate_camera, _)) = riding.frame.as_ref().and_then(|f| f.camera) {
            let skate_yaw = gta_heading(basis().transform_vector3(skate_camera.z_axis));
            pad.left = crate::aim::stick_for_camera(pad.left, n::camera_rot().z, skate_yaw);
        }
        if !n::using_keyboard() {
            return;
        }
        for control in [12, 13, 14, 15, 16, 17, 37].into_iter().chain(157..=164) {
            n::enable_control(0, control);
        }
        let weapon = n::selected_weapon(ped);
        let Some((automatic, interval)) = gun_rate(n::weapon_group(weapon)) else { return };
        // INPUT_AIM, INPUT_ATTACK.
        if !n::disabled_control_pressed(0, 25) {
            return;
        }
        let eye = n::camera_coord();
        let (forward, _, _) = n::camera_axes();
        let far = eye + forward * 300.;
        let target = n::probe_line(eye, far, 287, ped).map_or(far, |(hit, _)| hit);
        self.aim_target = Some(target);
        n::draw_rect(0.5, 0.5, 0.003, 0.005, [255, 255, 255, 220]);
        if !on_foot {
            pad.left[0] = (pad.left[0] + crate::aim::steer_toward(self.aim_turn)).clamp(-1., 1.);
        }
        let fire = if automatic { n::disabled_control_pressed(0, 24) } else { n::disabled_control_just_pressed(0, 24) };
        let ammo = n::ammo(ped, weapon);
        if fire && ammo > 0 && Instant::now() >= self.next_shot && self.aim_hand != Vec3::ZERO {
            let from = self.aim_hand + (target - self.aim_hand).normalize_or_zero() * 0.35;
            n::shoot_bullet(from, target, n::weapon_damage(weapon).round() as i32, weapon, ped);
            n::set_ammo(ped, weapon, ammo - 1);
            self.next_shot = Instant::now() + Duration::from_secs_f32(interval);
        }
    }

    /// The ridden Skate 3 board in the right hand on foot (`BoardWeapon`),
    /// placed by the hand's bone as GTA animates it.
    fn draw_carried_board(&mut self, ped: i32) {
        let Some(sources) = self.sources.as_ref() else { return };
        let library = sources.library.as_ref();
        let Ok(defaults) = sources.defaults.as_ref() else { return };
        let choice = self.store.ridden_look(library);
        if self.carry_look.as_ref().is_none_or(|(c, ..)| *c != choice) {
            self.carry_look = skate_boards::assemble(library, defaults, &choice, BOARD_TEXTURE).ok().map(|board| {
                let look = BoardLook::new(&board, 24, 1800);
                let rest = look.rest(defaults);
                (choice, look, rest)
            });
        }
        if self.read_skeleton(ped).is_err() {
            return;
        }
        let Some((_, skeleton, _)) = self.skeleton_cache.as_ref() else { return };
        let Some(hand) = skeleton.bones.iter().position(|b| b.0 == "SKEL_R_Hand") else { return };
        let Some(pose) = memory::read_pose(skeleton.global, skeleton.bones.len()) else { return };
        let Some((_, look, rest)) = self.carry_look.as_ref() else { return };
        let entity = Mat4::from_rotation_translation(n::entity_quaternion(ped), n::entity_coords(ped));
        let (_, turn, at) = (entity * pose[hand]).to_scale_rotation_translation();
        let [rx, ry, rz] = self.config.carry_rotation.map(f32::to_radians);
        let held = turn * Quat::from_euler(EulerRot::ZXY, rz, rx, ry);
        let origin = at + turn * Vec3::from_array(self.config.carry_offset);
        let bones = crate::board_look::showcase(rest, to_skate(held * Vec3::X), to_skate(held * Vec3::Y), to_skate(held * Vec3::Z), 0.);
        let (eye, light) = (n::camera_coord(), daylight(n::clock_hours()));
        n::backface_culling(false);
        for poly in look.polys(&bones, &rest.bound, origin, eye, light) {
            n::draw_poly(poly.points, poly.rgba);
        }
    }

    fn present_frame(&mut self) {
        let config = self.config.clone();
        let parts = self.parts_ready().then(|| self.ridden_choice());
        let extras = self.store.extras();
        let board_model = self.board_model_ready() || parts.is_some();
        let Some(riding) = self.riding.as_mut() else { return };
        let Some(frame) = riding.frame.as_ref() else { return };
        let origin = riding.origin;
        let root = transform_from_skate(frame.root, origin);

        // Face the way the board travels.
        let velocity = from_skate(frame.velocity);
        if velocity.truncate().length() > 0.5 {
            riding.heading = gta_heading(velocity);
        }

        // Bones, then the ped where its pelvis is.
        let posed = riding.skeleton.as_ref().map(|(skeleton, retarget)| {
            let world = retarget.pose(&frame.bones, origin, root);
            let pelvis = skeleton.bones.iter().position(|b| b.0 == "SKEL_Pelvis");
            (world, pelvis)
        });
        // Aiming: the body bends toward it as far as a body can.
        let mut posed = posed;
        if let (Some(target), Some((world, _)), Some((skeleton, _))) = (self.aim_target, posed.as_mut(), riding.skeleton.as_ref()) {
            if self.aim_rig.as_ref().map(|(at, _)| *at) != Some(skeleton.address) {
                self.aim_rig = crate::aim::AimRig::new(&skeleton.bones).map(|rig| (skeleton.address, rig));
            }
            if let Some((_, rig)) = &self.aim_rig {
                (self.aim_turn, self.aim_hand) = rig.aim(world, target);
            }
        } else {
            self.aim_hand = Vec3::ZERO;
        }
        let position = posed
            .as_ref()
            .and_then(|(world, pelvis)| pelvis.and_then(|p| world.get(p)).map(|m| m.w_axis.truncate()))
            .unwrap_or(root.w_axis.truncate() + Vec3::Z);
        n::set_coords_no_offset(riding.ped, position);
        n::set_heading(riding.ped, riding.heading);
        if let (Some((skeleton, _)), Some((world, _))) = (riding.skeleton.as_ref(), posed) {
            let entity = entity_transform(position, riding.heading);
            let inverse = entity.inverse();
            let mut local: Vec<Mat4> = world.iter().map(|m| inverse * *m).collect();
            local.truncate(skeleton.bones.len());
            // From GTA's pose into Skate's as skating begins.
            if let Some((started, from)) = riding.blend_in.as_ref() {
                let t = blend_progress(*started);
                if t < 1. {
                    for (m, f) in local.iter_mut().zip(from) {
                        *m = blend_pose(&(inverse * *f), m, t);
                    }
                } else {
                    riding.blend_in = None;
                }
            }
            riding.last_pose.clone_from(&local);
            if matches!(config.pose_write, PoseWrite::Tick | PoseWrite::Both) {
                // GTA's animation for this frame now, so it does not
                // overwrite the pose after it is written.
                n::force_ped_animation_update(riding.ped);
            }
            if self.tracing {
                self.log.line(&format!("  write {} bones ({:?})", local.len(), config.pose_write));
            }
            super::crash::phase("posing: writing and handing over the pose");
            show_pose(config.pose_write, hold::Slot::Player, skeleton, &local);
        }
        super::crash::phase("posing: the board model and camera");

        // The board: Skate's deck (GTA-local +Y is its back), the model's own
        // turn and offset.
        if riding.board != 0 || parts.is_some() {
            let [yaw, pitch, roll] = config.board_rotation.map(f32::to_radians);
            let mut deck = transform_from_skate(frame.board, origin)
                * Mat4::from_rotation_z(std::f32::consts::PI)
                * Mat4::from_quat(Quat::from_euler(EulerRot::ZXY, yaw, pitch, roll))
                * Mat4::from_translation(Vec3::from_array(config.board_offset));
            if board_model {
                // The weapon model lies along its Z, its deck facing -X (wheels
                // +X), held 0.1 m in from the tail: onto the deck's frame (+Y
                // its back, Z up), its middle on the deck. (With the deck
                // facing +X it lay upside down.)
                deck = deck * Mat4::from_cols(-Vec4::Z, Vec4::X, -Vec4::Y, Vec4::W) * Mat4::from_translation(Vec3::new(0., 0., -0.35));
            }
            if let Some(choice) = &parts {
                self.ride_parts.show(choice, &extras, super::board_parts::Place::At(deck));
            } else {
                n::set_coords_no_offset(riding.board, deck.w_axis.truncate());
                n::set_quaternion(riding.board, rotation_quaternion(deck));
            }
        }

        // First person at the head, or Skate's camera (its basis's Z looks
        // ahead, Y is up), or GTA's own.
        let on_foot = skate_rules::on_foot(&frame.state);
        let first = match self.view {
            crate::cam_cycle::View::Skate { first } => first,
            crate::cam_cycle::View::Gta(_) => config.view.first_person(on_foot, self.view_flipped),
        };
        let skate_camera = self.view.camera() == CameraMode::Skate;
        // First person, GTA's view too, is the head camera; the mouse looks
        // round with it unless it works the trick stick (its key held).
        let head_camera = (first && skate_camera) || matches!(self.view, crate::cam_cycle::View::Gta(4));
        if head_camera && n::using_keyboard() && !self.keyboard.mouse_tricks() && self.menu.is_none() {
            let (lr, ud) = (n::disabled_control_normal(0, 1), n::disabled_control_normal(0, 2));
            if lr.abs() + ud.abs() > 0.001 {
                self.look.0 = (self.look.0 - lr * 0.12).clamp(-1.9, 1.9);
                self.look.1 = (self.look.1 - ud * 0.09).clamp(-1.1, 1.1);
                self.look_moved = Some(Instant::now());
            }
        }
        // Left alone for a moment, the head turns back to the way ahead.
        if self.look_moved.is_none_or(|at| at.elapsed() > Duration::from_millis(1500)) {
            let k = (n::frame_time() * 2.5).clamp(0., 1.);
            self.look.0 -= self.look.0 * k;
            self.look.1 -= self.look.1 * k;
        }
        let head = self
            .names
            .as_deref()
            .and_then(|names| names.iter().position(|n| n.eq_ignore_ascii_case("HEAD")))
            .and_then(|i| frame.bones.get(i))
            .map(|m| m.w_axis.truncate());
        let placed = match (first, head) {
            // The cinematic camera (R), kept out of walls.
            _ if self.action_cam_on => {
                let dt = n::frame_time().clamp(0., 0.1);
                let shot = self.action_cam.update(position, velocity, dt);
                let mut eye = shot.eye;
                // Never below the ground under it.
                if let Some((ground, _)) = n::probe_line(eye + Vec3::Z * 2., eye - Vec3::Z * 3., 1, riding.ped) {
                    eye.z = eye.z.max(ground.z + 0.25);
                }
                if let Some((hit, _)) = n::probe_line(shot.look, eye, 1, riding.ped) {
                    eye = hit + (shot.look - hit).normalize_or_zero() * 0.3;
                }
                n::camera_place(riding.camera, eye, camera_rotation((shot.look - eye).normalize_or_zero()), shot.fov);
                true
            }
            (_, Some(head)) if head_camera => {
                let v = frame.velocity.to_array();
                let board = frame.board.z_axis.truncate().to_array();
                let dt = n::frame_time().clamp(0., 0.1);
                let (eye, forward) = self.head.update(head.to_array(), v, v, board, on_foot, dt);
                let forward = basis().transform_vector3(Vec3::from_array(forward));
                let forward = look_turned(forward, self.look);
                n::camera_place(riding.camera, from_skate(Vec3::from_array(eye)) + origin, camera_rotation(forward), 75.);
                true
            }
            _ if skate_camera => match frame.camera {
                Some((position, camera_basis, fov)) => {
                    let forward = basis().transform_vector3(camera_basis.z_axis);
                    n::camera_place(riding.camera, from_skate(position) + origin, camera_rotation(forward), fov);
                    true
                }
                None => false,
            },
            _ => false,
        };
        if placed != riding.camera_on && riding.camera != 0 {
            n::camera_render(riding.camera, placed);
            riding.camera_on = placed;
        }
    }

    /// The options menu for this frame.
    fn options_tick(&mut self) {
        n::disable_player_controls();
        let rows = options::rows();
        let Some(menu) = self.menu.as_mut() else { return };
        let mut changed = Changed::None;
        if let Some(index) = menu.capture {
            if let Some(key) = menu.edges.pressed() {
                match key {
                    "Escape" => {}
                    "Delete" => {
                        self.controls.unbind(index);
                        changed = Changed::Controls;
                    }
                    key => {
                        if self.controls.bind(index, key).is_ok() {
                            changed = Changed::Controls;
                        }
                    }
                }
                menu.capture = None;
            }
        } else {
            const UP: i32 = 172;
            const DOWN: i32 = 173;
            const LEFT: i32 = 174;
            const RIGHT: i32 = 175;
            const ACCEPT: i32 = 201;
            const CANCEL: i32 = 202;
            let setting = rows[menu.row];
            if n::control_just_pressed(2, UP) {
                menu.row = (menu.row + rows.len() - 1) % rows.len();
            } else if n::control_just_pressed(2, DOWN) {
                menu.row = (menu.row + 1) % rows.len();
            } else if n::control_just_pressed(2, LEFT) {
                changed = setting.adjust(&mut self.config, &mut self.controls, -1);
            } else if n::control_just_pressed(2, RIGHT) {
                changed = setting.adjust(&mut self.config, &mut self.controls, 1);
            } else if n::control_just_pressed(2, ACCEPT) {
                if let Setting::Bind(index) = setting {
                    menu.capture = Some(index);
                    // What is held now (Enter) is not the answer.
                    menu.edges.pressed();
                } else {
                    changed = setting.adjust(&mut self.config, &mut self.controls, 1);
                }
            } else if n::control_just_pressed(2, CANCEL) {
                self.menu = None;
                return;
            }
        }
        match changed {
            Changed::None => {}
            Changed::Controls => {
                let _ = std::fs::write(self.dir.join("SkateGTA-controls.txt"), self.controls.to_text());
            }
            other => {
                let ini = self.dir.join("SkateGTA.ini");
                let text = std::fs::read_to_string(&ini).unwrap_or_default();
                let _ = std::fs::write(&ini, options::write_ini(&text, &self.config));
                if other == Changed::Skate
                    && let Some(worker) = &self.worker
                {
                    worker.options(self.options());
                }
                if other == Changed::Board {
                    self.drawn = None;
                }
                if other == Changed::Radio {
                    self.start_radio();
                }
                self.store.skater_view = self.config.shop_skater;
            }
        }
        let Some(menu) = self.menu.as_ref() else { return };
        let list: Vec<(String, String)> = rows
            .iter()
            .map(|setting| {
                let value = match setting {
                    Setting::Bind(i) if menu.capture == Some(*i) => "Press a key...".to_owned(),
                    _ => setting.value(&self.config, &self.controls),
                };
                (setting.label(), value)
            })
            .collect();
        let footer = if menu.capture.is_some() {
            "Press a key or mouse button. Esc cancels, Delete clears."
        } else {
            "Left/Right: change   Enter: set a key   Backspace: close"
        };
        panel::draw_list(
            "Skate 3",
            &format!("OPTIONS   {}/{}", menu.row + 1, list.len()),
            &list,
            menu.row,
            panel::first_visible(menu.row, list.len(), OPTION_ROWS),
            OPTION_ROWS,
            footer,
        );
    }

    /// Run over (or thrown off): skate mode ends, the player is thrown
    /// with `push` (m/s) and hurt by an impact at `speed` (m/s); back on the
    /// board once they are up again.
    fn knocked_down(&mut self, push: Vec3, speed: f32) {
        let Some(ped) = self.riding.as_ref().map(|r| r.ped) else { return };
        self.stop("");
        let percent = ((speed - 4.) * 5.).clamp(5., 100.) as i32;
        let above = (n::max_health(ped) - 100).max(1);
        let left = n::health(ped) - above * percent / 100;
        if left <= 100 {
            n::set_health(ped, 0);
            return;
        }
        n::set_health(ped, left);
        n::knock_down(ped, 1500 + (speed * 120.) as i32, push);
        if self.config.auto_resume {
            self.resume = Some(None);
        }
    }

    /// After being run over or a swim: back on the board once the player
    /// has stood on their feet for a moment.
    fn resume_tick(&mut self) {
        let Some(since) = self.resume else { return };
        if self.riding.is_some() || !self.config.auto_resume {
            self.resume = None;
            return;
        }
        let ped = n::player_ped();
        if n::is_dead(ped) || n::in_vehicle(ped) {
            self.resume = None;
            return;
        }
        let up = n::is_on_foot(ped)
            && !n::is_ragdoll(ped)
            && !n::is_getting_up(ped)
            && !n::is_falling(ped)
            && !n::is_swimming(ped)
            && !n::in_water(ped);
        match (up, since) {
            (false, _) => self.resume = Some(None),
            (true, None) => self.resume = Some(Some(Instant::now())),
            (true, Some(at)) if at.elapsed() > Duration::from_millis(700) => {
                self.resume = None;
                self.request();
            }
            _ => {}
        }
    }

    /// Ends skate mode and gives the player back to GTA.
    /// One frame of the blend from Skate's last pose into what GTA's own
    /// animation shows now.
    fn blend_out_tick(&mut self) {
        let Some(blend) = self.blend_out.as_ref() else { return };
        let t = blend_progress(blend.started);
        if t >= 1. || self.riding.is_some() || !n::entity_exists(blend.ped) || blend.ped != n::player_ped() {
            self.blend_out = None;
            return;
        }
        let Some(now) = memory::read_pose(blend.global, blend.from.len()) else {
            self.blend_out = None;
            return;
        };
        let mixed: Vec<Mat4> = blend.from.iter().zip(&now).map(|(f, g)| blend_pose(f, g, t)).collect();
        // Written in the script's turn only: GTA's own animation takes over
        // as it fades in.
        if let Some(skeleton) = blend.skeleton.as_ref() {
            memory::Written::new(skeleton, &mixed).write();
        }
    }

    fn stop(&mut self, note: &str) {
        self.waiting = false;
        hold::clear(hold::Slot::Player);
        self.hud.reset();
        self.dropper.close();
        self.retry_at = None;
        if let Some(sound) = &self.sound {
            sound.quiet();
        }
        let Some(mut riding) = self.riding.take() else { return };
        self.store.save_wear();
        if !riding.last_pose.is_empty()
            && let Some((skeleton, _)) = riding.skeleton.take()
        {
            self.blend_out = Some(BlendOut {
                ped: riding.ped,
                global: skeleton.global,
                skeleton: Some(skeleton),
                from: std::mem::take(&mut riding.last_pose),
                started: Instant::now(),
            });
        }
        if let Some(worker) = &self.worker {
            worker.suspend();
        }
        let ped = riding.ped;
        if n::entity_exists(ped) {
            let feet = riding
                .frame
                .as_ref()
                .map_or(riding.origin, |f| transform_from_skate(f.root, riding.origin).w_axis.truncate());
            n::freeze(ped, false);
            n::set_collision(ped, true);
            n::set_can_ragdoll(ped, true);
            n::set_invincible(ped, false);
            n::set_coords(ped, feet + Vec3::Z);
            n::set_heading(ped, riding.heading);
            n::clear_tasks_now(ped);
            // Off the board: back in hand, ready to swing or get on again.
            if self.config.board_weapon && !n::is_dead(ped) {
                n::set_current_weapon(ped, super::carry::board_weapon());
            }
        }
        if let Some(before) = riding.radio_before.take() {
            // With the board in hand the station plays on (see `radio_held`).
            if self.config.board_weapon && !n::is_dead(ped) && !n::in_vehicle(ped) {
                self.radio_held = Some(before);
            } else {
                n::skate_radio(None);
                if let Some(station) = before {
                    n::tune_radio(&station);
                }
            }
        }
        if riding.board != 0 {
            n::delete_object(&mut riding.board);
        }
        self.ride_parts.clear();
        if riding.camera != 0 {
            n::camera_render(riding.camera, false);
            n::destroy_camera(riding.camera);
        }
        if !note.is_empty() {
            n::notify(note);
        }
    }

    /// Poses the character in the shop: Skate on foot, the board in hand,
    /// a camera in front of them leaving the menu its side of the screen.
    fn open_showroom(&mut self) {
        let ped = n::player_ped();
        let at = n::entity_coords(ped);
        let mut probe = n::GtaProbe { ignore: ped };
        let feet = probe.down(at.x, at.y, at.z, at.z - 3.).map_or(at - Vec3::Z, |hit| Vec3::new(at.x, at.y, hit.z));
        self.terrain = Terrain::new(self.config.scan_spacing).with_tall_walls(self.config.tall_walls);
        self.terrain.scan(&mut probe, feet, ACTIVATION_RADIUS, || true);
        self.send_collision(feet, feet, true);
        let heading = n::entity_heading(ped);
        let Some(worker) = &self.worker else { return };
        worker.options(self.options());
        worker.activate([0.; 3], skate_heading(heading), true, 0.);
        let skeleton = self.skeleton(ped);
        n::freeze(ped, true);
        // In front of them, looking a little to their side so they stand
        // clear of the menu on the left.
        let (sin, cos) = heading.to_radians().sin_cos();
        let facing = Vec3::new(-sin, cos, 0.);
        let eye = feet + facing * 2.4 + Vec3::Z * 1.15;
        let look = -facing;
        let right = Vec3::new(look.y, -look.x, 0.);
        let target = feet + Vec3::Z * 0.95 - right * 0.7;
        let camera = n::create_camera();
        n::camera_place(camera, eye, camera_rotation(target - eye), 45.);
        n::camera_render(camera, true);
        self.showroom = Some(Showroom { ped, origin: feet, home: at, heading, camera, skeleton, frame: None, board: None });
    }

    fn showroom_tick(&mut self) {
        if let Some(worker) = &self.worker {
            worker.step(n::frame_time().clamp(0., 0.1), n::aspect_ratio().max(0.5), None);
        }
        let library = self.sources.as_ref().and_then(|s| s.library.as_ref());
        let wanted = self.store.shown_board(library).unwrap_or_else(|| self.store.ridden_look(library));
        let names = self.names.clone().unwrap_or_default();
        let config = self.config.clone();
        let parts = self.parts_ready();
        let extras = self.store.extras_shown();
        let Some(showroom) = self.showroom.as_mut() else { return };
        let Some(frame) = showroom.frame.as_ref() else { return };
        // The character, posed.
        let root = transform_from_skate(frame.root, showroom.origin);
        if let Some((skeleton, retarget)) = showroom.skeleton.as_ref() {
            let world = retarget.pose(&frame.bones, showroom.origin, root);
            let pelvis = skeleton.bones.iter().position(|b| b.0 == "SKEL_Pelvis");
            let position = pelvis.and_then(|p| world.get(p)).map_or(root.w_axis.truncate() + Vec3::Z, |m| m.w_axis.truncate());
            n::set_coords_no_offset(showroom.ped, position);
            n::set_heading(showroom.ped, showroom.heading);
            let entity = entity_transform(position, showroom.heading);
            let inverse = entity.inverse();
            let mut local: Vec<Mat4> = world.iter().map(|m| inverse * *m).collect();
            local.truncate(skeleton.bones.len());
            let _ = entity;
            show_pose(config.pose_write, hold::Slot::Player, skeleton, &local);
        }
        // The board they look at, in their hands: GTA's HD part models.
        if parts {
            let at = deck_model(frame.board, showroom.origin, &config);
            self.showroom_parts.show(&wanted, &extras, super::board_parts::Place::At(at));
            return;
        }
        if showroom.board.as_ref().is_none_or(|(shown, ..)| *shown != wanted) {
            let board = self.sources.as_ref().and_then(|sources| {
                let defaults = sources.defaults.as_ref().ok()?;
                skate_boards::assemble(sources.library.as_ref(), defaults, &wanted, BOARD_TEXTURE).ok()
            });
            let flat = board.as_ref().map(|board| {
                let look = BoardLook::new(board, config.board_detail, config.board_triangles);
                let bound = look.bind(&names);
                (look, bound)
            });
            let textured = board.as_ref().filter(|_| config.board_textures).map(|board| {
                let look = BoardLook::textured(board);
                for (dict, _) in look.textures() {
                    n::request_texture_dict(dict);
                }
                let bound = look.bind(&names);
                (look, bound)
            });
            showroom.board = Some((wanted, flat, textured));
        }
        if let Some((_, flat, textured)) = showroom.board.as_ref() {
            n::backface_culling(false);
            let eye = n::camera_coord();
            let ready = |look: &BoardLook| look.textures().iter().all(|(dict, _)| n::texture_dict_loaded(dict));
            match (textured, flat) {
                (Some((look, Some(bound))), _) if ready(look) => {
                    let textures = look.textures();
                    for poly in look.polys(&frame.bones, bound, showroom.origin, eye, 1.) {
                        let Some((dict, name)) = textures.get(usize::from(poly.texture)) else { continue };
                        let v = (poly.shade.clamp(0., 1.) * 255.) as u8;
                        n::draw_textured_poly(poly.points, [v, v, v, 255], dict, name, poly.uv);
                    }
                }
                (_, Some((look, Some(bound)))) => {
                    for poly in look.polys(&frame.bones, bound, showroom.origin, eye, 1.) {
                        n::draw_poly(poly.points, poly.rgba);
                    }
                }
                _ => {}
            }
        }
    }

    /// Back to the shop as it was.
    fn close_showroom(&mut self) {
        let Some(showroom) = self.showroom.take() else { return };
        self.showroom_parts.clear();
        if let Some(worker) = &self.worker {
            worker.suspend();
        }
        hold::clear(hold::Slot::Player);
        if showroom.camera != 0 {
            n::camera_render(showroom.camera, false);
            n::destroy_camera(showroom.camera);
        }
        if n::entity_exists(showroom.ped) {
            n::freeze(showroom.ped, false);
            n::set_coords(showroom.ped, showroom.home);
            n::set_heading(showroom.ped, showroom.heading);
            n::clear_tasks_now(showroom.ped);
        }
    }

    /// Co-op: tells the other player's plugin about this skater, and shows
    /// theirs.
    fn coop_tick(&mut self) {
        if !self.coop_tried {
            self.coop_tried = true;
            if let Some(peer) = self.config.coop_peer.clone() {
                match super::coop::Link::open(&peer, self.config.coop_port) {
                    Ok(link) => {
                        self.log.line(&format!("co-op: listening on {}, talking to {peer}", self.config.coop_port));
                        self.coop = Some(link);
                    }
                    Err(e) => {
                        self.log.line(&format!("co-op: {e}"));
                        n::notify(&format!("Skate 3 co-op: {e}"));
                    }
                }
            }
        }
        if self.coop.is_none() {
            return;
        }
        let packet = self.local_packet();
        let rate = if packet.skating { 20. } else { 2. };
        let Some(link) = self.coop.as_mut() else { return };
        link.receive();
        link.send(packet, rate);
        let current = link.current().cloned();
        let objects = current.as_ref().map(|p| p.objects.clone()).unwrap_or_default();
        self.sync_remote_props(&objects);
        match current.filter(|p| p.skating) {
            Some(packet) => self.show_remote(&packet),
            None => self.hide_remote(),
        }
    }

    /// This player's skater for the other player.
    fn local_packet(&self) -> crate::coop::Packet {
        let mut packet = crate::coop::Packet { objects: self.dropper.placed().to_vec(), ..Default::default() };
        let (Some(riding), Some(names)) = (self.riding.as_ref(), self.names.as_ref()) else { return packet };
        let Some(frame) = riding.frame.as_ref() else { return packet };
        let mut indices: Vec<usize> = match riding.skeleton.as_ref() {
            Some((_, retarget)) => retarget.used(),
            None => BODY_BONES.iter().filter_map(|b| names.iter().position(|n| n == b)).collect(),
        };
        if let Some(bound) = self.drawn.as_ref().and_then(|d| d.bound.as_ref()) {
            indices.extend(bound.iter().copied());
        }
        indices.sort_unstable();
        indices.dedup();
        indices.truncate(crate::coop::MAX_BONES);
        packet.skating = true;
        packet.origin = riding.origin;
        packet.root = frame.root;
        packet.bones = indices
            .into_iter()
            .filter_map(|i| frame.bones.get(i).map(|m| (i as u16, *m)))
            .collect();
        packet.board = self.store.ridden_look(self.sources.as_ref().and_then(|s| s.library.as_ref())).to_text();
        packet.state = skate_sound::state_code(&frame.state);
        packet.speed = frame.velocity.length();
        packet.vertical = frame.velocity.y;
        packet.wheels = frame.wheels_down;
        packet.surface = frame.surface;
        packet
    }

    /// The other player's skater: their ped posed, their board drawn, their
    /// board's sounds from where they ride.
    fn show_remote(&mut self, packet: &crate::coop::Packet) {
        let Some(names) = self.names.clone() else { return };
        let pose = packet.pose(names.len());
        let root = transform_from_skate(packet.root, packet.origin);
        let at = root.w_axis.truncate();
        if self.remote.as_ref().is_none_or(|r| !n::entity_exists(r.ped)) {
            self.hide_remote();
            let (ped, stand_in) = match super::coop::network_ped_near(at, 60.) {
                Some(ped) => (ped, false),
                None => (self.spawn_stand_in(at), true),
            };
            if ped == 0 {
                return;
            }
            let skeleton = self.read_skeleton(ped).map_err(|e| self.log.line(&format!("co-op skater: {e}"))).ok();
            self.remote = Some(RemoteSkater { ped, stand_in, skeleton, heading: 0., last: at });
        }
        let Some(remote) = self.remote.as_mut() else { return };
        let moved = at - remote.last;
        if moved.truncate().length() > 0.05 {
            remote.heading = gta_heading(moved);
        }
        remote.last = at;
        if let Some((skeleton, retarget)) = remote.skeleton.as_ref() {
            let world = retarget.pose(&pose, packet.origin, root);
            let pelvis = skeleton.bones.iter().position(|b| b.0 == "SKEL_Pelvis");
            let position = pelvis.and_then(|p| world.get(p)).map_or(at + Vec3::Z, |m| m.w_axis.truncate());
            let entity = if remote.stand_in {
                n::set_coords_no_offset(remote.ped, position);
                n::set_heading(remote.ped, remote.heading);
                entity_transform(position, remote.heading)
            } else {
                entity_transform(n::entity_coords(remote.ped), n::entity_heading(remote.ped))
            };
            let inverse = entity.inverse();
            let mut local: Vec<Mat4> = world.iter().map(|m| inverse * *m).collect();
            local.truncate(skeleton.bones.len());
            let mode = if self.config.pose_write == PoseWrite::Off { PoseWrite::Hold } else { self.config.pose_write };
            if matches!(mode, PoseWrite::Present | PoseWrite::Both) && !self.present_hooked {
                shv::on_present(on_present);
                self.present_hooked = true;
            }
            show_pose(mode, hold::Slot::Remote, skeleton, &local);
        } else if remote.stand_in {
            n::set_coords_no_offset(remote.ped, at + Vec3::Z);
            n::set_heading(remote.ped, remote.heading);
        }
        // Their board.
        if self.remote_board.as_ref().is_none_or(|(text, _)| *text != packet.board) {
            self.sources();
            let built = self.sources.as_ref().and_then(|sources| {
                let defaults = sources.defaults.as_ref().ok()?;
                let choice = skate_boards::BoardChoice::from_text(&packet.board);
                let board = skate_boards::assemble(sources.library.as_ref(), defaults, &choice, BOARD_TEXTURE).ok()?;
                let look = BoardLook::new(&board, self.config.board_detail, self.config.board_triangles / 2);
                let bound = look.bind(&names);
                Some((look, bound))
            });
            self.remote_board = Some((packet.board.clone(), built));
        }
        let eye = n::camera_coord();
        if let Some((_, Some((look, Some(bound))))) = self.remote_board.as_ref() {
            n::backface_culling(false);
            for poly in look.polys(&pose, bound, packet.origin, eye, daylight(n::clock_hours())) {
                n::draw_poly(poly.points, poly.rgba);
            }
        }
        // Their board's sounds, fading with distance.
        if self.remote_sound.is_none() && self.sound_tried {
            self.remote_sound = super::sound::BoardSound::start(None);
        }
        if let Some(sound) = &self.remote_sound {
            let near = (1. - eye.distance(at) / 40.).clamp(0., 1.);
            let dt = n::frame_time().clamp(0., 0.1);
            let state = skate_sound::state_name(packet.state);
            sound.update(state, packet.speed, packet.vertical, packet.wheels, packet.surface, dt, self.config.board_sound * near, None, false);
        }
    }

    /// A skater ped standing in for the other player.
    fn spawn_stand_in(&mut self, at: Vec3) -> i32 {
        let model = joaat(super::coop::STAND_IN);
        if !n::model_in_cdimage(model) {
            return 0;
        }
        n::request_model(model);
        let deadline = Instant::now() + Duration::from_millis(1500);
        while !n::model_loaded(model) && Instant::now() < deadline {
            shv::wait(0);
        }
        if !n::model_loaded(model) {
            return 0;
        }
        let ped = n::create_ped(model, at, 0.);
        n::release_model(model);
        if ped != 0 {
            n::keep_entity(ped);
            n::freeze(ped, true);
            n::set_collision(ped, false);
            n::set_invincible(ped, true);
        }
        ped
    }

    fn hide_remote(&mut self) {
        hold::clear(hold::Slot::Remote);
        if let Some(sound) = &self.remote_sound {
            sound.quiet();
        }
        if let Some(mut remote) = self.remote.take()
            && remote.stand_in
            && n::entity_exists(remote.ped)
        {
            n::delete_ped(&mut remote.ped);
        }
    }

    /// The other player's dropped objects, put down here too.
    fn sync_remote_props(&mut self, wanted: &[skate_park::ParkObject]) {
        if self.remote_props.len() == wanted.len() && self.remote_props.iter().zip(wanted).all(|((a, _), b)| a == b) {
            return;
        }
        let mut kept = Vec::new();
        for (object, mut handle) in std::mem::take(&mut self.remote_props) {
            if wanted.contains(&object) && n::entity_exists(handle) {
                kept.push((object, handle));
            } else {
                n::delete_object(&mut handle);
            }
        }
        for object in wanted {
            if kept.iter().any(|(o, _)| o == object) {
                continue;
            }
            let Some(model) = self.dropper.model(usize::from(object.kind)) else { continue };
            let handle = super::dropper::spawn(model, Vec3::from_array(object.at), object.yaw.to_degrees(), false);
            if handle != 0 {
                kept.push((*object, handle));
            }
        }
        self.remote_props = kept;
        // The scan finds them as ground.
        self.terrain = Terrain::new(self.config.scan_spacing).with_tall_walls(self.config.tall_walls);
        if let Some(riding) = self.riding.as_mut() {
            riding.built.0 = u64::MAX;
        }
    }

    /// Called once if the script panics, to leave the player usable.
    pub fn recover(&mut self, what: &str) {
        self.log.line(&format!("recovered from a fault: {what}"));
        self.stop("Skate 3 hit a problem and switched off; see SkateGTA.log.");
    }
}
