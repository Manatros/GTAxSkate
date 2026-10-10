//! The in-game Skate options menu: Skate 3's options, the board, the
//! keyboard and mouse controls and every key, changed while playing.
//! Changes are written back into `SkateGTA.ini` (keeping its comments) and
//! `SkateGTA-controls.txt`.

use crate::config::{BoardLook, CameraMode, Config, TUNING};
use skate_input::{ACTIONS, Controls, MouseTrick};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Difficulty,
    Trucks,
    Wheels,
    HoldFakie,
    FallDamage,
    Surfaces,
    Tune(usize),
    Scheme,
    Mouse,
    MouseThrow,
    FlickMs,
    BufferMs,
    RampMs,
    Camera,
    View,
    Start,
    BoardLook,
    BoardDetail,
    BoardTriangles,
    Shops,
    Radio,
    BoardSound,
    Checkpoint,
    ShopView,
    ShowControls,
    Bind(usize),
    ResetKeys,
}

/// What a change touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changed {
    None,
    /// Skate 3's options (sent to the session).
    Skate,
    /// Keyboard and mouse controls.
    Controls,
    /// The board's look.
    Board,
    /// Other settings.
    Plugin,
    /// The skating radio.
    Radio,
}

pub fn rows() -> Vec<Setting> {
    let mut rows = vec![Setting::Difficulty, Setting::Trucks, Setting::Wheels, Setting::HoldFakie, Setting::FallDamage, Setting::Surfaces];
    rows.extend((0..TUNING.len()).map(Setting::Tune));
    rows.extend([
        Setting::Scheme,
        Setting::Mouse,
        Setting::MouseThrow,
        Setting::FlickMs,
        Setting::BufferMs,
        Setting::RampMs,
        Setting::Camera,
        Setting::View,
        Setting::Start,
        Setting::BoardLook,
        Setting::BoardDetail,
        Setting::BoardTriangles,
        Setting::Shops,
        Setting::Radio,
        Setting::BoardSound,
        Setting::Checkpoint,
        Setting::ShopView,
        Setting::ShowControls,
    ]);
    rows.extend((0..ACTIONS.len()).map(Setting::Bind));
    rows.push(Setting::ResetKeys);
    rows
}

fn on_off(v: bool) -> String {
    if v { "On".into() } else { "Off".into() }
}

fn step(value: f32, dir: i32, step: f32, low: f32, high: f32) -> f32 {
    let v = ((value / step).round() + dir as f32) * step;
    (v.clamp(low, high) * 1000.).round() / 1000.
}

impl Setting {
    pub fn label(self) -> String {
        match self {
            Self::Difficulty => "Difficulty".into(),
            Self::Trucks => "Trucks (loose - tight)".into(),
            Self::Wheels => "Wheels (soft - hard)".into(),
            Self::HoldFakie => "Hold fakie".into(),
            Self::FallDamage => "Fall damage".into(),
            Self::Surfaces => "Ground surfaces".into(),
            Self::Tune(i) => format!("Tuning: {}", TUNING[i]),
            Self::Scheme => "Keyboard tricks".into(),
            Self::Mouse => "Mouse trick stick".into(),
            Self::MouseThrow => "Mouse flick distance".into(),
            Self::FlickMs => "Key flick time".into(),
            Self::BufferMs => "Diagonal leniency".into(),
            Self::RampMs => "Lean ramp".into(),
            Self::Camera => "Default camera (V cycles)".into(),
            Self::View => "View".into(),
            Self::Start => "Start skating".into(),
            Self::BoardLook => "Board".into(),
            Self::BoardDetail => "Board detail".into(),
            Self::BoardTriangles => "Board triangles".into(),
            Self::Shops => "Board shops".into(),
            Self::Radio => "Radio while skating".into(),
            Self::BoardSound => "Board sound".into(),
            Self::Checkpoint => "Back to checkpoint after a bail".into(),
            Self::ShopView => "Board shop view".into(),
            Self::ShowControls => "On-screen controls".into(),
            Self::Bind(i) => format!("Key: {}", ACTIONS[i].2),
            Self::ResetKeys => "Reset all keys".into(),
        }
    }

    pub fn value(self, config: &Config, controls: &Controls) -> String {
        match self {
            Self::Difficulty => config.difficulty.clone(),
            Self::Trucks => format!("{:.0}%", config.trucks * 100.),
            Self::Wheels => format!("{:.0}%", config.wheels * 100.),
            Self::HoldFakie => on_off(config.hold_fakie),
            Self::FallDamage => match config.fall {
                skate_rules::FallRules::Skate => "Skate heights",
                skate_rules::FallRules::Mw2 => "MW2 heights",
                skate_rules::FallRules::Off => "Off",
            }
            .into(),
            Self::Surfaces => on_off(config.surfaces),
            Self::BoardSound => format!("{:.0}%", config.board_sound * 100.),
            Self::Checkpoint => on_off(config.auto_retry),
            Self::ShopView => if config.shop_skater { "You holding the board" } else { "Board only" }.into(),
            Self::ShowControls => on_off(config.show_controls),
            Self::Tune(i) => format!("{:.1}x", config.tuning[i]),
            Self::Scheme => match (controls.classic, controls.modern) {
                (true, true) => "Classic + Modern",
                (true, false) => "Classic (motions)",
                (false, true) => "Modern (one key)",
                (false, false) => "Off",
            }
            .into(),
            Self::Mouse => match controls.mouse {
                MouseTrick::Off => "Off",
                MouseTrick::Hold => "Hold a key",
                MouseTrick::Always => "Always",
            }
            .into(),
            Self::MouseThrow => format!("{:.0}", controls.mouse_throw),
            Self::FlickMs => format!("{:.0} ms", controls.flick_ms),
            Self::BufferMs => format!("{:.0} ms", controls.buffer_ms),
            Self::RampMs => format!("{:.0} ms", controls.ramp_ms),
            Self::Camera => match config.camera {
                CameraMode::Skate => "Skate 3",
                CameraMode::Gta => "GTA",
            }
            .into(),
            Self::View => match config.view {
                skate_rules::CameraRule::FirstThenThird => "First on foot, third on board",
                skate_rules::CameraRule::Third => "Third person",
                skate_rules::CameraRule::First => "First person",
            }
            .into(),
            Self::Start => match config.start {
                skate_rules::StartRule::OnFoot => "On foot, board in hand",
                skate_rules::StartRule::OnBoard => "On the board",
            }
            .into(),
            Self::BoardLook => match config.board_look {
                BoardLook::Skate3 => "Your Skate 3 board",
                BoardLook::Prop => "GTA model",
            }
            .into(),
            Self::BoardDetail => config.board_detail.to_string(),
            Self::BoardTriangles => config.board_triangles.to_string(),
            Self::Shops => on_off(config.shops),
            Self::Radio => match config.radio.as_deref() {
                None => "Off".into(),
                Some("RADIO_19_USER") => "Self Radio (Skate 3)".into(),
                Some(other) => other.to_owned(),
            },
            Self::Bind(i) => {
                let keys = controls.keys_text(i);
                if controls.uses(ACTIONS[i].0) { keys } else { format!("{keys} (off)") }
            }
            Self::ResetKeys => String::new(),
        }
    }

    /// Moves the setting one step left (-1) or right (1).
    pub fn adjust(self, config: &mut Config, controls: &mut Controls, dir: i32) -> Changed {
        let cycle = |n: usize, at: usize| ((at as i32 + dir).rem_euclid(n as i32)) as usize;
        match self {
            Self::Difficulty => {
                let all = ["easy", "normal", "hardcore"];
                let at = all.iter().position(|d| *d == config.difficulty).unwrap_or(0);
                config.difficulty = all[cycle(3, at)].into();
                Changed::Skate
            }
            Self::Trucks => {
                config.trucks = step(config.trucks, dir, 0.05, 0., 1.);
                Changed::Skate
            }
            Self::Wheels => {
                config.wheels = step(config.wheels, dir, 0.05, 0., 1.);
                Changed::Skate
            }
            Self::HoldFakie => {
                config.hold_fakie = !config.hold_fakie;
                Changed::Skate
            }
            Self::FallDamage => {
                let all = skate_rules::FallRules::ALL;
                let at = all.iter().position(|r| *r == config.fall).unwrap_or(0);
                config.fall = all[cycle(all.len(), at)];
                Changed::Plugin
            }
            Self::Surfaces => {
                config.surfaces = !config.surfaces;
                Changed::Plugin
            }
            Self::BoardSound => {
                config.board_sound = step(config.board_sound, dir, 0.05, 0., 1.);
                Changed::Plugin
            }
            Self::Checkpoint => {
                config.auto_retry = !config.auto_retry;
                Changed::Plugin
            }
            Self::ShopView => {
                config.shop_skater = !config.shop_skater;
                Changed::Plugin
            }
            Self::ShowControls => {
                config.show_controls = !config.show_controls;
                Changed::Plugin
            }
            Self::Tune(i) => {
                config.tuning[i] = step(config.tuning[i], dir, 0.1, 0.1, 5.);
                Changed::Skate
            }
            Self::Scheme => {
                let at = usize::from(controls.classic) | usize::from(controls.modern) << 1;
                let next = cycle(4, at);
                controls.classic = next & 1 != 0;
                controls.modern = next & 2 != 0;
                Changed::Controls
            }
            Self::Mouse => {
                let all = [MouseTrick::Off, MouseTrick::Hold, MouseTrick::Always];
                let at = all.iter().position(|m| *m == controls.mouse).unwrap_or(0);
                controls.mouse = all[cycle(3, at)];
                Changed::Controls
            }
            Self::MouseThrow => {
                controls.mouse_throw = step(controls.mouse_throw, dir, 5., 10., 300.);
                Changed::Controls
            }
            Self::FlickMs => {
                controls.flick_ms = step(controls.flick_ms, dir, 5., 10., 120.);
                Changed::Controls
            }
            Self::BufferMs => {
                controls.buffer_ms = step(controls.buffer_ms, dir, 2., 0., 100.);
                Changed::Controls
            }
            Self::RampMs => {
                controls.ramp_ms = step(controls.ramp_ms, dir, 10., 0., 300.);
                Changed::Controls
            }
            Self::Camera => {
                config.camera = match config.camera {
                    CameraMode::Skate => CameraMode::Gta,
                    CameraMode::Gta => CameraMode::Skate,
                };
                Changed::Plugin
            }
            Self::View => {
                let all = skate_rules::CameraRule::ALL;
                let at = all.iter().position(|r| *r == config.view).unwrap_or(0);
                config.view = all[cycle(all.len(), at)];
                Changed::Plugin
            }
            Self::Start => {
                config.start = match config.start {
                    skate_rules::StartRule::OnFoot => skate_rules::StartRule::OnBoard,
                    skate_rules::StartRule::OnBoard => skate_rules::StartRule::OnFoot,
                };
                Changed::Plugin
            }
            Self::BoardLook => {
                config.board_look = match config.board_look {
                    BoardLook::Skate3 => BoardLook::Prop,
                    BoardLook::Prop => BoardLook::Skate3,
                };
                Changed::Board
            }
            Self::BoardDetail => {
                config.board_detail = step(config.board_detail as f32, dir, 4., 4., 128.) as u32;
                Changed::Board
            }
            Self::BoardTriangles => {
                config.board_triangles = step(config.board_triangles as f32, dir, 200., 200., 10000.) as usize;
                Changed::Board
            }
            Self::Shops => {
                config.shops = !config.shops;
                Changed::Plugin
            }
            Self::Radio => {
                config.radio = match config.radio {
                    None => Some("RADIO_19_USER".into()),
                    Some(_) => None,
                };
                Changed::Radio
            }
            Self::Bind(_) => Changed::None,
            Self::ResetKeys => {
                controls.keys = gta_controls().keys;
                Changed::Controls
            }
        }
    }
}

/// `SkateGTA.ini` with `config`'s values written into its lines; comments
/// and order are kept, missing settings appended.
pub fn write_ini(text: &str, config: &Config) -> String {
    let on = |v: bool| if v { "on" } else { "off" }.to_owned();
    let mut values: Vec<(String, String)> = vec![
        ("Difficulty".into(), config.difficulty.clone()),
        ("Trucks".into(), config.trucks.to_string()),
        ("Wheels".into(), config.wheels.to_string()),
        ("HoldFakie".into(), on(config.hold_fakie)),
        ("HippieJump".into(), config.hippie_jump.to_string()),
        ("FallDamage".into(), config.fall.key().to_owned()),
        ("Surfaces".into(), on(config.surfaces)),
        ("BoardSound".into(), (config.board_sound * 100.).round().to_string()),
        ("Checkpoint".into(), if config.auto_retry { "auto" } else { "key" }.to_owned()),
        ("ShopView".into(), if config.shop_skater { "skater" } else { "board" }.to_owned()),
        ("ShowControls".into(), if config.show_controls { "on" } else { "off" }.to_owned()),
        ("View".into(), config.view.key().to_owned()),
        ("Start".into(), config.start.key().to_owned()),
        (
            "Camera".into(),
            match config.camera {
                CameraMode::Skate => "skate".into(),
                CameraMode::Gta => "gta".into(),
            },
        ),
        (
            "BoardLook".into(),
            match config.board_look {
                BoardLook::Skate3 => "skate3".into(),
                BoardLook::Prop => "prop".into(),
            },
        ),
        ("BoardDetail".into(), config.board_detail.to_string()),
        ("BoardTriangles".into(), config.board_triangles.to_string()),
        ("BoardTextures".into(), on(config.board_textures)),
        ("TallWalls".into(), on(config.tall_walls)),
        ("Shops".into(), on(config.shops)),
        (
            "Radio".into(),
            match config.radio.as_deref() {
                None => "off".into(),
                Some("RADIO_19_USER") => "self".into(),
                Some(other) => other.to_owned(),
            },
        ),
    ];
    values.extend(TUNING.iter().enumerate().map(|(i, name)| ((*name).to_owned(), config.tuning[i].to_string())));
    let mut written = vec![false; values.len()];
    let mut out = String::new();
    for line in text.lines() {
        let key = line.split(';').next().unwrap_or("").split('=').next().unwrap_or("").trim();
        match values.iter().position(|(k, _)| k.eq_ignore_ascii_case(key)).filter(|_| line.contains('=')) {
            Some(i) => {
                out.push_str(&format!("{} = {}\n", values[i].0, values[i].1));
                written[i] = true;
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for (i, (key, value)) in values.iter().enumerate() {
        if !written[i] {
            out.push_str(&format!("{key} = {value}\n"));
        }
    }
    out
}

/// The keys for GTA, laid out the way GTA's own are, one key for each thing
/// and one thing on each key: Shift pushes (as it sprints), Space pops
/// (held it crouches, let go it ollies, with a trick arrow held it flips),
/// the arrows are the trick stick, Q held is the hippie jump, Ctrl brakes,
/// E gets on and off the board in skate mode (Skate's own walking and
/// running jump on). F starts and ends skate mode; the camera is GTA's.
/// No numpad or whole-trick keys and no mouse trick stick: they did again
/// what the arrows and Space do.
pub fn gta_controls() -> Controls {
    let mut controls = Controls::default();
    let index = |id: &str| Controls::action_index(id).expect("a Skate action");
    // Alt is GTA's character wheel: not Skate's left stick click.
    for id in ["trick_7", "trick_8", "trick_9", "trick_4", "trick_6", "trick_1", "trick_2", "trick_3", "camera", "mouse_trick", "ls"] {
        controls.unbind(index(id));
    }
    for (id, key) in [("pop", "Space"), ("a", "LShift"), ("x", "T"), ("b", "LCtrl"), ("hippie", "Q"), ("y", "E")] {
        controls.bind(index(id), key).expect("a key");
    }
    controls.modern = false;
    controls.mouse = skate_input::MouseTrick::Off;
    controls
}

/// Whether a saved key file predates [`gta_controls`] (it has no hippie key,
/// Alt is still Skate's, getting on has no key, or R is still Skate's X:
/// R is the cinematic camera).
pub fn controls_outdated(text: &str) -> bool {
    !text.lines().any(|l| l.trim_start().starts_with("bind hippie"))
        || text.lines().any(|l| l.trim() == "bind ls = LAlt" || l.trim() == "bind y =" || l.trim() == "bind x = R")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TEMPLATE;

    #[test]
    fn settings_step_and_cycle() {
        let mut config = Config::default();
        let mut controls = Controls::default();
        assert_eq!(Setting::Difficulty.adjust(&mut config, &mut controls, -1), Changed::Skate);
        assert_eq!(config.difficulty, "easy");
        Setting::Difficulty.adjust(&mut config, &mut controls, -1);
        assert_eq!(config.difficulty, "hardcore", "wraps round");
        for _ in 0..10 {
            Setting::Trucks.adjust(&mut config, &mut controls, 1);
        }
        assert_eq!(config.trucks, 1.0, "stops at tight");
        assert_eq!(Setting::Trucks.value(&config, &controls), "100%");
        Setting::Scheme.adjust(&mut config, &mut controls, 1);
        assert!(!controls.classic && !controls.modern, "3 + 1 wraps to off");
        assert_eq!(Setting::Scheme.value(&config, &controls), "Off");
        assert_eq!(Setting::Bind(0).adjust(&mut config, &mut controls, 1), Changed::None);
        assert!(Setting::Bind(18).value(&config, &controls).ends_with("(off)"), "modern keys off");
        assert_eq!(rows().iter().filter(|r| matches!(r, Setting::Bind(_))).count(), ACTIONS.len());
    }

    #[test]
    fn the_ini_keeps_its_comments() {
        let mut config = Config::default();
        config.difficulty = "hardcore".into();
        config.tuning[0] = 2.5;
        config.board_look = BoardLook::Prop;
        let text = write_ini(TEMPLATE, &config);
        assert!(text.contains("; Skate 3 for GTA V"));
        let (read, problems) = Config::parse(&text);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(read, config);
        assert_eq!(text.matches("Difficulty =").count(), 1);
        let appended = write_ini("SkateAssets = C:\\x\n", &config);
        assert!(appended.contains("pop = 2.5"));
    }

    #[test]
    fn gtas_keys_push_with_shift_and_pop_with_space() {
        let controls = gta_controls();
        let keys = |id: &str| controls.keys[Controls::action_index(id).unwrap()].clone();
        assert_eq!(keys("a"), vec!["LShift"]);
        assert_eq!(keys("pop"), vec!["Space"]);
        assert_eq!(keys("hippie"), vec!["Q"]);
        assert!(keys("ls").is_empty(), "Alt is GTA's character wheel");
        assert_eq!(keys("y"), vec!["E"], "on and off the board in skate mode");
        assert!(controls_outdated("bind hippie = Q\nbind ls = LAlt\n"));
        // One key for each thing: no key is bound twice.
        let mut all: Vec<&str> = controls.claimed();
        let count = all.len();
        all.dedup();
        assert_eq!(all.len(), count);
        let bound: usize = controls.keys.iter().enumerate().filter(|(i, _)| controls.uses(skate_input::ACTIONS[*i].0)).map(|(_, k)| k.len()).sum();
        assert_eq!(bound, count, "each key is on one action");
        assert!(!keys("x").contains(&"LShift"));
        assert!(controls_outdated("bind a = Space\nbind pop = Space\n"));
        assert!(!controls_outdated(&controls.to_text()));
    }
}