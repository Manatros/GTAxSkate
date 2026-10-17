//! `SkateGTA.ini`: `Key = value` lines, `;` comments. Unknown keys and bad
//! values are reported and leave that setting at its default.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    /// Skate 3's own follow camera.
    Skate,
    /// GTA's gameplay camera.
    Gta,
}

/// When the skater's pose is written onto the player's skeleton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseWrite {
    /// Never: the player keeps GTA's own animation on the board.
    Off,
    /// In the script tick.
    Tick,
    /// In the script tick, and held there by a thread that writes it again
    /// whenever GTA's own animation wrote over it (the fallback).
    Hold,
    /// Inside GTA's own animation pass, right after GTA wrote its pose
    /// (SK8V's pose hook, the default; Hold when the game differs).
    Hook,
    /// Just before each frame is presented.
    Present,
    Both,
}

/// How the board looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardLook {
    /// Your Skate 3 board (bought in clothing stores), drawn by the plugin.
    Skate3,
    /// A GTA model (`BoardModel`).
    Prop,
}

/// The tuning multipliers, in the engine's order. 1 is Skate 3's own value.
pub const TUNING: [&str; 11] = [
    "pop",
    "grindpop",
    "pushspeed",
    "pushpower",
    "braking",
    "steering",
    "wobble",
    "offboardjump",
    "grip",
    "turnpower",
    "manualdrag",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The converted Skate 3 data: the mashup's `skate-data\assets` folder.
    pub skate_assets: Option<PathBuf>,
    /// Windows virtual-key code that switches skate mode.
    pub toggle_key: u32,
    /// Clicking both sticks in also switches it.
    pub toggle_sticks: bool,
    pub camera: CameraMode,
    pub pose_write: PoseWrite,
    /// Extra turn (degrees) after the GTA ped's facing was lined up with the
    /// Skate skater's from their hips.
    pub facing_degrees: f32,
    pub board_model: String,
    /// The board model's own turn: yaw, pitch, roll in degrees.
    pub board_rotation: [f32; 3],
    /// The board model's offset: right, forward, up in metres.
    pub board_offset: [f32; 3],
    /// The board in hand on foot as a melee weapon (GTA's bat under it),
    /// F to get on: see `win::carry`.
    pub board_weapon: bool,
    /// Where the board sits in the hand: metres and degrees off GTA's
    /// right-hand bone (SKEL_R_Hand).
    pub carry_offset: [f32; 3],
    pub carry_rotation: [f32; 3],
    /// How far around the skater the world is scanned, in metres.
    pub scan_radius: f32,
    /// Distance between scan probes, in metres.
    pub scan_spacing: f32,
    /// Time the scan may take each frame, in milliseconds.
    pub scan_budget_ms: f32,
    pub difficulty: String,
    pub trucks: f32,
    pub wheels: f32,
    /// Painted wheels set their own hardness (else `wheels` always).
    pub wheels_from_board: bool,
    pub tuning: [f32; TUNING.len()],
    pub hold_fakie: bool,
    /// How high a hippie jump goes, times Skate 3's: 1.6 clears a car, the
    /// board rolling on underneath.
    pub hippie_jump: f32,
    pub board_look: BoardLook,
    /// Triangles across the board's texture, for its graphic's detail.
    pub board_detail: u32,
    /// Most triangles drawn for the board.
    pub board_triangles: usize,
    /// The Skate 3 board drawn with its textures (`DRAW_TEXTURED_POLY`),
    /// else in flat colours. Off while the skating crash is narrowed down:
    /// it began once the textured board was drawn.
    pub board_textures: bool,
    /// Walls taller than the ground probes' start found by looking sideways
    /// too (`terrain::WALL_LOOK`). Off: it cost two thirds of the frame rate.
    pub tall_walls: bool,
    /// Skate's pose on the ped with SK8V's retarget (`ped_rig`: GTA's own
    /// bone lengths, limb IK onto Skate's feet and hands, roll bones), else
    /// the older one (`rig::Retarget`).
    pub sk8v_rig: bool,
    /// The skate parks on GTA's map.
    pub park_blips: bool,
    /// GTA's GPS route to the waypoint while skating, as in a vehicle.
    pub gps: bool,
    /// Clothing stores sell Skate 3 boards.
    pub shops: bool,
    /// The radio station tuned while skating (Self Radio holds the Skate 3
    /// soundtrack get-skate-music.ps1 makes), or none.
    pub radio: Option<String>,
    /// Windows virtual-key code that skips to the next song while skating.
    pub radio_skip_key: u32,
    /// Keys for your Skate 3 skater: become them (and back), and edit them.
    pub character_key: u32,
    pub skater_editor_key: u32,
    /// Store interiors by name, and the chain each belongs to.
    pub shop_interiors: Vec<(String, crate::shop::Tier)>,
    /// Windows virtual-key code that opens the Skate options menu.
    pub options_key: u32,
    /// Mouse counts per unit of GTA's look input, for the mouse trick stick.
    pub mouse_scale: f32,
    /// How a fall off the board hurts.
    pub fall: skate_rules::FallRules,
    /// Skate 3's Hall of Meat injuries in a bail (see `crate::hom`).
    pub injuries: bool,
    /// Skating trains GTA's stamina, and push and pop follow it
    /// (`crate::stamina`).
    pub stamina: bool,
    /// GTA's own collision read live from its physics level (`crate::live`,
    /// as SK8V does it); off, the ray probes only.
    pub live_collision: bool,
    /// The board rolls as Skate 3's would on each surface's material
    /// (grass slows it, ice slides it); off, everything rolls like concrete.
    pub surfaces: bool,
    /// First or third person, and whether skating starts on foot.
    pub view: skate_rules::CameraRule,
    pub start: skate_rules::StartRule,
    /// The board's sounds, 0 (off) to 1.
    pub board_sound: f32,
    /// A bail goes back to the checkpoint by itself (or only on its key).
    pub auto_retry: bool,
    /// Vehicles and people: skated on and into, running the skater over,
    /// getting knocked over.
    pub traffic: bool,
    /// People react to the skater (step aside, get knocked over, speak).
    pub people: bool,
    /// Back on the board by itself after being run over or a swim.
    pub auto_resume: bool,
    /// Co-op over FiveM: the other player's plugin (`ip:port`) and the port
    /// this one listens on.
    pub coop_peer: Option<String>,
    pub coop_port: u16,
    /// In the board shops your character holds the board (or the board
    /// floats alone beside the menu).
    pub shop_skater: bool,
    /// The controls show on screen while skating and building.
    pub show_controls: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skate_assets: None,
            toggle_key: 0x78, // F9
            toggle_sticks: true,
            camera: CameraMode::Gta,
            pose_write: PoseWrite::Hook,
            facing_degrees: 0.,
            board_model: "v_res_skateboard".into(),
            board_rotation: [0.; 3],
            board_offset: [0.; 3],
            board_weapon: true,
            carry_offset: [0., -0.1, 0.],
            carry_rotation: [0., 0., 0.],
            scan_radius: 9.,
            scan_spacing: 0.2,
            scan_budget_ms: 3.,
            difficulty: "normal".into(),
            trucks: 0.7,
            wheels: 0.7,
            wheels_from_board: true,
            // Pops 15 % higher: GTA's curbs, benches and ledges are taller
            // than Skate 3's park.
            tuning: {
                let mut tuning = [1.; TUNING.len()];
                tuning[0] = 1.15;
                tuning[1] = 1.15;
                tuning
            },
            hold_fakie: false,
            hippie_jump: 1.6,
            board_look: BoardLook::Skate3,
            board_detail: 32,
            board_triangles: 2400,
            board_textures: false,
            tall_walls: true,
            sk8v_rig: true,
            park_blips: true,
            gps: true,
            shops: true,
            radio: Some("RADIO_19_USER".into()),
            radio_skip_key: 0xBE, // .
            character_key: 0x77,      // F8
            skater_editor_key: 0x7A, // F11
            shop_interiors: crate::shop::Tier::INTERIORS.iter().map(|(n, t)| ((*n).to_owned(), *t)).collect(),
            options_key: 0x79, // F10
            mouse_scale: 100.,
            fall: skate_rules::FallRules::Skate,
            injuries: true,
            stamina: true,
            // Off until it is proven in the game: on in the 10-10 tests the
            // board hovered, caught on nothing and fell through (the maintainer,
            // video 09-43); the ray probes are the tested ground.
            live_collision: false,
            surfaces: true,
            view: skate_rules::CameraRule::FirstThenThird,
            start: skate_rules::StartRule::OnFoot,
            board_sound: 0.8,
            auto_retry: true,
            traffic: true,
            people: true,
            auto_resume: true,
            coop_peer: None,
            coop_port: 47800,
            shop_skater: false,
            show_controls: true,
        }
    }
}

/// The file written on first start, with every setting at its default.
pub const TEMPLATE: &str = r"; Skate 3 for GTA V
;
; SkateAssets: the converted Skate 3 data. Set up skating in the
; 2010 Rust Rewrite Mashup once (it converts your own Skate 3 copy), then
; point this at its skate-data\assets folder, the one with rig.json in it.
SkateAssets =

; Switch skate mode: a key (F1-F12, A-Z, 0-9 or a hex virtual-key code
; like 0x78), and/or clicking both sticks in.
ToggleKey = F9
ToggleSticks = on
; The Skate options menu: settings, the board, keyboard and mouse keys.
OptionsKey = F10
; Keyboard and mouse: see SkateGTA-controls.txt (written on first start)
; or the options menu. MouseScale turns GTA's look input into mouse counts
; for the mouse trick stick.
MouseScale = 100

; Camera: the camera skate mode starts with. gta (GTA's own camera, turned
; with the mouse unless the mouse trick key is held) or skate (Skate 3's own
; camera). V while skating cycles GTA's views (near, middle, far, first
; person), then Skate 3's camera (third, then first person), and round.
Camera = gta
; View: auto (first person on foot, third on the board), third or first.
; V (see SkateGTA-controls.txt) switches first/third while skating.
View = auto
; Start: foot (pull the board out, on foot) or board (straight on it).
Start = foot

; PoseMode: how Skate 3's animation is put on your character. hook (inside
; GTA's own animation pass, SK8V's way: smooth), hold (written each frame and
; held against GTA's animation by a watching thread), tick, present, both,
; or off (your character keeps GTA's animation).
PoseMode = hook
; FacingExtra: an extra turn of the skater on the GTA skeleton, in degrees.
; The facing itself is found from the hips; 0 unless the body looks turned.
FacingExtra = 0

; BoardLook: skate3 (your Skate 3 board, bought in GTA's clothing stores,
; drawn by the plugin) or prop (the GTA model below).
BoardLook = skate3
; The Skate 3 board's detail: triangles across its graphic, and the most
; triangles drawn (fewer is faster).
BoardDetail = 32
BoardTriangles = 2400
; BoardTextures: on draws the board with its graphic (needs the
; skate3boards pack), off in flat colours. Off for now: GTA crashed while
; skating once the textured board was drawn.
BoardTextures = off
; TallWalls: on finds walls taller than 2.2 m above your feet too (else the
; skater rides through them, and through shop windows). Off saves about 1 ms
; a frame.
TallWalls = on
; Rig: how Skate's animation is put on your character. sk8v (SK8V's: GTA's
; own proportions, feet and grabbing hands on Skate's) or classic (the older one).
Rig = sk8v
; ParkBlips: GTA's skate spots on the map (Vespucci Beach, Hawick, Little
; Seoul, Strawberry, La Mesa).
ParkBlips = on

; Street clothing stores (Binco, the Discount Store, Suburban) sell Skate 3
; boards beside their clothes: complete boards, or every part to build your
; own. Press G (or left on the d-pad) inside one; E stays the clothes.
Shops = on
; Radio: while skating, GTA's radio plays this station on foot too.
; self is Self Radio, where get-skate-music.ps1 puts the Skate 3
; soundtrack (run a Full Scan in Settings > Audio once); a station name
; such as RADIO_01_CLASS_ROCK plays that one; off leaves the radio alone.
Radio = self
; RadioSkipKey: the next song while skating.
RadioSkipKey = 0xBE
; Your Skate 3 skater (the skate3 add-on ped, see the guide): CharacterKey
; becomes the skater and back (GTA's fourth-character key); SkaterEditorKey
; opens the Create-a-Skater editor while you are the skater.
CharacterKey = F8
SkaterEditorKey = F11
; ShowControls: on (the controls show on screen while skating, on your
; own keys) or off.
ShowControls = on
; ShopView: board (the board beside the menu, turned with the mouse) or
; skater (your character holds it, posed as in Skate 3). Tab switches.
ShopView = board
; The store interiors, as name:chain (low, mid or high).
ShopInteriors = v_clotheslo:low v_clothesmid:mid v_clotheshi:high

; The board as a GTA model: the model, its own turn (yaw pitch roll,
; degrees) and offset (right forward up, metres) to sit on the Skate board.
BoardModel = v_res_skateboard
BoardRotation = 0 0 0
BoardOffset = 0 0 0
; BoardWeapon: on carries the board on foot like a baseball bat (it is the
; bat in GTA's weapon wheel, and swings like one); F with it out gets on and
; off the board. CarryOffset (metres) and CarryRotation (degrees) place it
; in the hand, off GTA's right-hand bone.
BoardWeapon = on
CarryOffset = 0 -0.1 0
CarryRotation = 0 0 0

; The world scan around the skater: radius and probe spacing in metres,
; and the time it may take each frame in milliseconds.
ScanRadius = 9
ScanSpacing = 0.2
ScanBudgetMs = 3

; Skate 3's options. Difficulty: easy, normal or hardcore.
Difficulty = normal
; Trucks and wheels: 0 (loose / soft) to 1 (tight / hard).
; WheelsFromBoard: on, painted wheels bring their own hardness (matte soft,
; gloss medium, metallic hard); off, Wheels below always holds.
WheelsFromBoard = on
Trucks = 0.7
Wheels = 0.7
HoldFakie = off
; HippieJump: how high a hippie jump goes, times Skate 3's (1). 1.6 clears a
; car while the board rolls on underneath (cars leave a gap at the ground).
HippieJump = 1.6
; FallDamage: a long drop off the board hurts, a longer one is WASTED.
; skate (from 4.5 m, deadly at 9 m), mw2 (3.25 m and 7.6 m) or off.
FallDamage = skate
; Injuries: Skate 3's Hall of Meat in a bail: each bone's knock scrapes,
; bruises, sprains, dislocates or breaks it (Skate's own thresholds); the
; worst show as a message and cost health, a broken leg limps. on or off.
Injuries = on
; LiveCollision: GTA's own collision read from the game (as the SK8V mod does
; it: exact kerbs, rails, ramps), on; off (the default, the tested way), the
; ground is felt with rays.
LiveCollision = off
; Surfaces: on, the board rolls as on Skate 3's ground for each material
; (concrete fast, tarmac and brick rougher, grass slow, sand barely, ice
; slides); off, everything rolls like concrete.
Surfaces = on
; BoardSound: the board's own sounds (rolling, pops, landings, grinds),
; 0 (off) to 100.
BoardSound = 80
; Checkpoint: auto goes back to the session marker (or, without one, to
; where your failed line started) a moment after a bail; key only on the
; checkpoint key (LB + left on a controller, comma on the keyboard).
Checkpoint = auto
; Traffic: vehicles and people are there to skate on and into: cars and
; trucks to ride on (a moving one carries you), vehicles run you over,
; people get out of the way or get knocked over.
Traffic = on
; People: passers-by step aside, get knocked over and have a word (off:
; they are only something to bump into).
People = on
; FastStart: GTA starts without the Rockstar logo film and the legal
; notices (off: as GTA starts them).
FastStart = on
; AutoResume: back on the board by itself once you are up again after being
; run over, or out of the water after a swim.
AutoResume = on
; Co-op over FiveM (see the guide): CoopPeer is the other player's
; computer and port (192.168.1.20:47800), CoopPort the port this one
; listens on. Empty: no co-op.
CoopPeer =
CoopPort = 47800
; Tuning multipliers, 0.1 to 5; 1 is Skate 3's own value. Pops are
; 15 % higher (the maintainer: 1.25 was a tick too high): GTA's curbs and ledges are taller.
pop = 1.15
grindpop = 1.15
pushspeed = 1
pushpower = 1
braking = 1
steering = 1
wobble = 1
offboardjump = 1
grip = 1
turnpower = 1
manualdrag = 1
";

/// A Windows virtual-key code from its name.
pub fn virtual_key(name: &str) -> Option<u32> {
    let name = name.trim();
    if let Some(hex) = name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok().filter(|v| (1..=0xFE).contains(v));
    }
    let upper = name.to_ascii_uppercase();
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok())
        && (1..=24).contains(&n)
    {
        return Some(0x6F + n);
    }
    let mut chars = upper.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() || c.is_ascii_digit() => Some(c as u32),
        _ => None,
    }
}

fn on_off(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "on" | "1" | "true" | "yes" => Some(true),
        "off" | "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn number(value: &str, low: f32, high: f32) -> Option<f32> {
    value.parse::<f32>().ok().filter(|v| v.is_finite() && (low..=high).contains(v))
}

fn triple(value: &str, low: f32, high: f32) -> Option<[f32; 3]> {
    let parts: Vec<f32> = value
        .split_whitespace()
        .map(|p| number(p, low, high))
        .collect::<Option<_>>()?;
    parts.try_into().ok()
}

impl Config {
    /// The settings in `text`, with a message for each line not used.
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut config = Self::default();
        let mut problems = Vec::new();
        // An editor's byte-order mark is not part of the first key.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (number_of_line, line) in text.lines().enumerate() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                problems.push(format!("line {}: no '='", number_of_line + 1));
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            if let Err(e) = config.set(&key, value) {
                problems.push(format!("line {}: {e}", number_of_line + 1));
            }
        }
        (config, problems)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let bad = || format!("{key} cannot be {value:?}");
        match key {
            "skateassets" => {
                self.skate_assets = (!value.is_empty()).then(|| PathBuf::from(value.trim_matches('"')));
            }
            "togglekey" => self.toggle_key = virtual_key(value).ok_or_else(bad)?,
            "togglesticks" => self.toggle_sticks = on_off(value).ok_or_else(bad)?,
            "optionskey" => self.options_key = virtual_key(value).ok_or_else(bad)?,
            "mousescale" => self.mouse_scale = number(value, 1., 10000.).ok_or_else(bad)?,
            "camera" => {
                self.camera = match value.to_ascii_lowercase().as_str() {
                    "skate" => CameraMode::Skate,
                    "gta" => CameraMode::Gta,
                    _ => return Err(bad()),
                }
            }
            // PoseBones is the older setting: tick, present or both there
            // were tried before hold existed, so it is left alone.
            "posebones" => {}
            "posemode" => {
                self.pose_write = match value.to_ascii_lowercase().as_str() {
                    "off" => PoseWrite::Off,
                    "tick" => PoseWrite::Tick,
                    "hold" => PoseWrite::Hold,
                    "hook" => PoseWrite::Hook,
                    "present" => PoseWrite::Present,
                    "both" => PoseWrite::Both,
                    _ => return Err(bad()),
                }
            }
            // The old fixed turn (180) is found from the hips now.
            "facingdegrees" => {}
            "facingextra" => self.facing_degrees = number(value, -360., 360.).ok_or_else(bad)?,
            "boardmodel" => {
                if value.is_empty() {
                    return Err(bad());
                }
                self.board_model = value.to_owned();
            }
            "boardrotation" => self.board_rotation = triple(value, -360., 360.).ok_or_else(bad)?,
            "boardoffset" => self.board_offset = triple(value, -2., 2.).ok_or_else(bad)?,
            "boardweapon" => self.board_weapon = on_off(value).ok_or_else(bad)?,
            "carryoffset" => self.carry_offset = triple(value, -2., 2.).ok_or_else(bad)?,
            "carryrotation" => self.carry_rotation = triple(value, -360., 360.).ok_or_else(bad)?,
            "scanradius" => self.scan_radius = number(value, 3., 30.).ok_or_else(bad)?,
            "scanspacing" => self.scan_spacing = number(value, 0.05, 1.).ok_or_else(bad)?,
            "scanbudgetms" => self.scan_budget_ms = number(value, 0.2, 30.).ok_or_else(bad)?,
            "difficulty" => {
                let d = value.to_ascii_lowercase();
                if !["easy", "normal", "hardcore"].contains(&d.as_str()) {
                    return Err(bad());
                }
                self.difficulty = d;
            }
            "trucks" => self.trucks = number(value, 0., 1.).ok_or_else(bad)?,
            "wheels" => self.wheels = number(value, 0., 1.).ok_or_else(bad)?,
            "wheelsfromboard" => self.wheels_from_board = on_off(value).ok_or_else(bad)?,
            "holdfakie" => self.hold_fakie = on_off(value).ok_or_else(bad)?,
            "hippiejump" => self.hippie_jump = number(value, 0.5, 3.).ok_or_else(bad)?,
            "falldamage" => self.fall = skate_rules::FallRules::parse(value).ok_or_else(bad)?,
            "injuries" => self.injuries = on_off(value).ok_or_else(bad)?,
            "stamina" => self.stamina = on_off(value).ok_or_else(bad)?,
            "livecollision" => self.live_collision = on_off(value).ok_or_else(bad)?,
            "boardsound" => self.board_sound = number(value, 0., 100.).ok_or_else(bad)? / 100.,
            "checkpoint" => {
                self.auto_retry = match value.to_ascii_lowercase().as_str() {
                    "auto" | "on" => true,
                    "key" | "off" => false,
                    _ => return Err(bad()),
                }
            }
            "traffic" => self.traffic = on_off(value).ok_or_else(bad)?,
            "people" => self.people = on_off(value).ok_or_else(bad)?,
            // Read by the plugin as it loads (`win::fast_start`).
            "faststart" => {
                on_off(value).ok_or_else(bad)?;
            }
            "autoresume" => self.auto_resume = on_off(value).ok_or_else(bad)?,
            "cooppeer" => {
                let peer = value.trim();
                if peer.is_empty() {
                    self.coop_peer = None;
                } else if peer.parse::<std::net::SocketAddr>().is_ok() {
                    self.coop_peer = Some(peer.to_owned());
                } else {
                    return Err(bad());
                }
            }
            "coopport" => self.coop_port = number(value, 1024., 65535.).ok_or_else(bad)? as u16,
            "surfaces" => self.surfaces = on_off(value).ok_or_else(bad)?,
            "view" => self.view = skate_rules::CameraRule::parse(value).ok_or_else(bad)?,
            "start" => self.start = skate_rules::StartRule::parse(value).ok_or_else(bad)?,
            "boardlook" => {
                self.board_look = match value.to_ascii_lowercase().as_str() {
                    "skate3" => BoardLook::Skate3,
                    "prop" => BoardLook::Prop,
                    _ => return Err(bad()),
                }
            }
            "boarddetail" => self.board_detail = number(value, 4., 128.).ok_or_else(bad)? as u32,
            "boardtriangles" => self.board_triangles = number(value, 100., 10000.).ok_or_else(bad)? as usize,
            "boardtextures" => self.board_textures = on_off(value).ok_or_else(bad)?,
            "tallwalls" => self.tall_walls = on_off(value).ok_or_else(bad)?,
            "rig" => {
                self.sk8v_rig = match value.to_ascii_lowercase().as_str() {
                    "sk8v" => true,
                    "classic" => false,
                    _ => return Err(bad()),
                }
            }
            "parkblips" => self.park_blips = on_off(value).ok_or_else(bad)?,
            "gps" => self.gps = on_off(value).ok_or_else(bad)?,
            "shops" => self.shops = on_off(value).ok_or_else(bad)?,
            "showcontrols" => self.show_controls = on_off(value).ok_or_else(bad)?,
            "shopview" => {
                self.shop_skater = match value.to_ascii_lowercase().as_str() {
                    "skater" | "character" => true,
                    "board" => false,
                    _ => return Err(bad()),
                }
            }
            "radio" => {
                self.radio = match value.trim() {
                    "" => return Err(bad()),
                    v if v.eq_ignore_ascii_case("off") => None,
                    v if v.eq_ignore_ascii_case("self") || v.eq_ignore_ascii_case("on") => Some("RADIO_19_USER".into()),
                    v => Some(v.to_ascii_uppercase()),
                }
            }
            "radioskipkey" => self.radio_skip_key = virtual_key(value).ok_or_else(bad)?,
            "characterkey" => self.character_key = virtual_key(value).ok_or_else(bad)?,
            "skatereditorkey" => self.skater_editor_key = virtual_key(value).ok_or_else(bad)?,
            "shopinteriors" => {
                self.shop_interiors = value
                    .split_whitespace()
                    .map(|pair| {
                        let (name, tier) = pair.split_once(':')?;
                        Some((name.to_owned(), crate::shop::Tier::parse(tier)?))
                    })
                    .collect::<Option<_>>()
                    .ok_or_else(bad)?;
            }
            _ => {
                let index = TUNING
                    .iter()
                    .position(|name| *name == key)
                    .ok_or_else(|| format!("unknown setting {key:?}"))?;
                self.tuning[index] = number(value, 0.1, 5.).ok_or_else(bad)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_is_the_defaults() {
        let (config, problems) = Config::parse(TEMPLATE);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn settings_parse_and_bad_lines_are_reported() {
        let text = "SkateAssets = C:\\Example\\skate-data\\assets ; mine\nToggleKey = F5\nCamera = skate\n\
                    PoseMode = both\nDifficulty = Hardcore\npop = 1.5\nBoardRotation = 90 0 0\n\
                    Trucks = 3\nnonsense\nJetpack = on\n";
        let (config, problems) = Config::parse(text);
        assert_eq!(config.skate_assets, Some(PathBuf::from("C:\\Example\\skate-data\\assets")));
        let (marked, _) = Config::parse("\u{feff}SkateAssets = C:\\x\n");
        assert_eq!(marked.skate_assets, Some(PathBuf::from("C:\\x")), "a byte-order mark is skipped");
        assert_eq!(config.toggle_key, 0x74);
        assert_eq!(config.camera, CameraMode::Skate);
        assert_eq!(config.pose_write, PoseWrite::Both);
        assert_eq!(config.difficulty, "hardcore");
        assert_eq!(config.tuning[0], 1.5);
        assert_eq!(config.board_rotation, [90., 0., 0.]);
        assert_eq!(config.trucks, 0.7, "an out-of-range value keeps the default");
        assert_eq!(problems.len(), 3, "{problems:?}");
        let (config, problems) = Config::parse("BoardLook = prop\nShopInteriors = a:low b:ponsonbys\nBoardDetail = 2\n");
        assert_eq!(config.board_look, BoardLook::Prop);
        assert_eq!(config.shop_interiors, vec![("a".into(), crate::shop::Tier::Low), ("b".into(), crate::shop::Tier::High)]);
        assert_eq!(problems.len(), 1, "detail out of range");
        assert!(!Config::default().board_textures, "flat until the skating crash is understood");
        assert!(Config::parse("BoardTextures = on\n").0.board_textures);
        let edited = crate::options::write_ini(TEMPLATE, &Config { board_textures: true, ..Config::default() });
        assert!(Config::parse(&edited).0.board_textures, "the options menu keeps it");
    }

    #[test]
    fn the_skating_radio() {
        assert_eq!(Config::parse("Radio = off\n").0.radio, None);
        assert_eq!(Config::parse("Radio = self\n").0.radio.as_deref(), Some("RADIO_19_USER"));
        assert_eq!(Config::parse("Radio = radio_01_class_rock\n").0.radio.as_deref(), Some("RADIO_01_CLASS_ROCK"));
        assert_eq!(Config::parse("RadioSkipKey = N\n").0.radio_skip_key, 0x4E);
        let edited = crate::options::write_ini(TEMPLATE, &Config { radio: None, ..Config::default() });
        assert_eq!(Config::parse(&edited).0.radio, None);
    }

    #[test]
    fn key_names() {
        assert_eq!(virtual_key("F9"), Some(0x78));
        assert_eq!(virtual_key("f1"), Some(0x70));
        assert_eq!(virtual_key("K"), Some(0x4B));
        assert_eq!(virtual_key("7"), Some(0x37));
        assert_eq!(virtual_key("0x78"), Some(0x78));
        assert_eq!(virtual_key("F99"), None);
        assert_eq!(virtual_key("Enter"), None);
    }
}
