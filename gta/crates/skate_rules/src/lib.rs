//! Rules skate mode shares between the mashup and Skate 3 for GTA V.

pub mod tricks;
pub use tricks::{trick_name, trick_title};

/// How a fall off the board hurts. Skate 3 itself only bails the skater;
/// these turn a long drop into damage, a long enough one into death.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallRules {
    /// Modern Warfare 2's own heights: hurt from 128 inches, dead at 300.
    Mw2,
    /// Higher, for skating: stair sets and gaps hurt only when they are
    /// big; hurt from 4.5 metres, dead at 9.
    #[default]
    Skate,
    Off,
}

const INCH: f32 = 0.0254;

impl FallRules {
    pub const ALL: [Self; 3] = [Self::Mw2, Self::Skate, Self::Off];

    pub fn key(self) -> &'static str {
        match self {
            Self::Mw2 => "mw2",
            Self::Skate => "skate",
            Self::Off => "off",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.key().eq_ignore_ascii_case(text.trim()))
    }

    /// The heights (metres) where damage starts and where it kills.
    pub fn heights(self) -> Option<(f32, f32)> {
        match self {
            Self::Mw2 => Some((128. * INCH, 300. * INCH)),
            Self::Skate => Some((4.5, 9.)),
            Self::Off => None,
        }
    }

    /// Damage in percent of full health for a drop of `metres`: 0 below
    /// the first height, 100 or more at the second.
    pub fn damage(self, metres: f32) -> u32 {
        let Some((low, high)) = self.heights() else { return 0 };
        if !metres.is_finite() || metres <= low {
            return 0;
        }
        (((metres - low) / (high - low)) * 100.).ceil() as u32
    }
}

/// Follows a skater's height and state to find the drop of each landing.
/// Airborne states climb to a top; the landing is the first frame on
/// something again. A bail in the air (Skate's wipeout) keeps falling until
/// the skater stops going down.
#[derive(Clone, Debug, Default)]
pub struct FallTracker {
    top: Option<f32>,
}

/// Whether a Skate physical state (its debug name) is off the ground.
pub fn airborne(state: &str) -> bool {
    state.contains("Air")
}

impl FallTracker {
    /// One pose: Skate state name, height (metres, up) and vertical speed
    /// (metres per second, up). Returns the drop when the skater lands.
    pub fn update(&mut self, state: &str, height: f32, vertical_speed: f32) -> Option<f32> {
        if !height.is_finite() {
            return None;
        }
        let wiping_out = state.contains("Wipeout") && vertical_speed < -1.;
        if airborne(state) || wiping_out {
            self.top = Some(self.top.map_or(height, |top| top.max(height)));
            return None;
        }
        let top = self.top.take()?;
        let drop = top - height;
        (drop > 0.).then_some(drop)
    }

    pub fn reset(&mut self) {
        self.top = None;
    }
}

/// Which camera skating uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraRule {
    /// First person while on foot (pulling the board out), Skate 3's
    /// camera once on the board.
    #[default]
    FirstThenThird,
    Third,
    First,
}

impl CameraRule {
    pub const ALL: [Self; 3] = [Self::FirstThenThird, Self::Third, Self::First];

    pub fn key(self) -> &'static str {
        match self {
            Self::FirstThenThird => "auto",
            Self::Third => "third",
            Self::First => "first",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.key().eq_ignore_ascii_case(text.trim()))
    }

    /// Whether to look through the skater's eyes; `flipped` is the quick
    /// first/third switch.
    pub fn first_person(self, on_foot: bool, flipped: bool) -> bool {
        let base = match self {
            Self::FirstThenThird => on_foot,
            Self::Third => false,
            Self::First => true,
        };
        base != flipped
    }
}

/// How skate mode starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StartRule {
    /// On foot with the board in hand, as if just pulled out.
    #[default]
    OnFoot,
    OnBoard,
}

impl StartRule {
    pub fn key(self) -> &'static str {
        match self {
            Self::OnFoot => "foot",
            Self::OnBoard => "board",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        [Self::OnFoot, Self::OnBoard].into_iter().find(|r| r.key().eq_ignore_ascii_case(text.trim()))
    }
}

/// What a skater may do besides skate, each on its own switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combat {
    /// Shoot, aim, reload and throw from the board (drive-bys).
    pub guns: bool,
    /// The melee button swings the board.
    pub melee: bool,
    /// Riding into an enemy fast enough hurts them, at speed kills them.
    pub ram: bool,
    /// Killstreaks (and their laptop or marker) from the board.
    pub streaks: bool,
    /// The Minecraft inventory and crafting open while rolling; the board
    /// coasts while they are open.
    pub crafting: bool,
    /// The controller button held to hand the triggers, bumpers and face
    /// buttons to the weapons instead of the board.
    pub pad_layer: PadLayer,
}

impl Default for Combat {
    fn default() -> Self {
        Self { guns: true, melee: true, ram: true, streaks: true, crafting: true, pad_layer: PadLayer::Back }
    }
}

impl Combat {
    /// The switches by console name, for `skate <name> on|off`.
    pub const SWITCHES: [&str; 5] = ["guns", "melee", "ram", "killstreaks", "crafting"];

    pub fn switch(&mut self, name: &str) -> Option<&mut bool> {
        Some(match name.to_ascii_lowercase().as_str() {
            "guns" => &mut self.guns,
            "melee" => &mut self.melee,
            "ram" => &mut self.ram,
            "killstreaks" | "streaks" => &mut self.streaks,
            "crafting" => &mut self.crafting,
            _ => return None,
        })
    }

    /// Whether the sim runs the rider's weapon (guns, the board swing, or a
    /// killstreak's laptop or marker).
    pub fn armed(&self) -> bool {
        self.guns || self.melee || self.streaks
    }
}

/// A controller button that, held, gives the weapons the pad. Back is
/// the one Skate 3 leaves free while skating (LB is its session-marker
/// button, the D-pad its own too).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadLayer {
    #[default]
    Back,
    Lb,
    Rb,
    LeftStick,
    DpadUp,
    DpadDown,
    /// The controller only skates.
    Off,
}

impl PadLayer {
    pub const ALL: [Self; 7] = [Self::Back, Self::Lb, Self::Rb, Self::LeftStick, Self::DpadUp, Self::DpadDown, Self::Off];

    pub fn key(self) -> &'static str {
        match self {
            Self::Back => "back",
            Self::Lb => "lb",
            Self::Rb => "rb",
            Self::LeftStick => "ls",
            Self::DpadUp => "up",
            Self::DpadDown => "down",
            Self::Off => "off",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.key().eq_ignore_ascii_case(text.trim()))
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Back => "Hold Back",
            Self::Lb => "Hold LB",
            Self::Rb => "Hold RB",
            Self::LeftStick => "Hold L3",
            Self::DpadUp => "Hold D-pad up",
            Self::DpadDown => "Hold D-pad down",
            Self::Off => "Off",
        }
    }

    /// The button's bit in XInput's layout.
    pub fn xinput_bit(self) -> Option<u16> {
        match self {
            Self::Back => Some(0x0020),
            Self::Lb => Some(0x0100),
            Self::Rb => Some(0x0200),
            Self::LeftStick => Some(0x0040),
            Self::DpadUp => Some(0x0001),
            Self::DpadDown => Some(0x0002),
            Self::Off => None,
        }
    }
}

/// While the weapons hold the controller, what of it still reaches the
/// board: the left stick (steering) and the D-pad. The trick stick becomes
/// the aim, the triggers, bumpers and face buttons the weapons'.
pub fn pad_for_board(buttons: u16, triggers: [u8; 2], right: [f32; 2], layer_held: bool) -> (u16, [u8; 2], [f32; 2]) {
    if layer_held {
        (buttons & 0x000f, [0; 2], [0.; 2])
    } else {
        (buttons, triggers, right)
    }
}

/// How hard riding into someone at `speed` (metres a second) hits, in
/// percent of full health: nothing at a push, more the faster, a kill from
/// about 9 m/s (bombing a hill).
pub fn ram_damage(speed: f32) -> u32 {
    const FROM: f32 = 3.;
    const KILL: f32 = 9.;
    if !speed.is_finite() || speed <= FROM {
        return 0;
    }
    (((speed - FROM) / (KILL - FROM)) * 100.).ceil().min(150.) as u32
}

/// Whether a Skate physical state (its debug name) is on foot.
pub fn on_foot(state: &str) -> bool {
    state.starts_with("Biped")
}

/// A first-person camera at the skater's head, in Skate space (metres, y
/// up). It looks where the skater walks, or along the board's travel when
/// riding (a skater stands sideways on the board), turning smoothly.
#[derive(Clone, Debug, Default)]
pub struct HeadCamera {
    yaw: Option<f32>,
}

fn yaw_of(v: [f32; 3]) -> Option<f32> {
    (v[0].hypot(v[2]) > 1e-3).then(|| v[0].atan2(v[2]))
}

fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

impl HeadCamera {
    pub fn reset(&mut self) {
        self.yaw = None;
    }

    /// Starts looking along `forward` (Skate space).
    pub fn start(&mut self, forward: [f32; 3]) {
        self.yaw = yaw_of(forward);
    }

    /// The eye position and look direction for this frame. `facing` is the
    /// body's forward, `board` the deck's forward axis.
    pub fn update(&mut self, head: [f32; 3], facing: [f32; 3], velocity: [f32; 3], board: [f32; 3], on_foot: bool, dt: f32) -> ([f32; 3], [f32; 3]) {
        let speed = velocity[0].hypot(velocity[2]);
        let wanted = if on_foot {
            yaw_of(facing)
        } else if speed > 1. {
            yaw_of(velocity)
        } else {
            // The deck points both ways: the end nearer the current view.
            yaw_of(board).map(|y| match self.yaw {
                Some(now) if wrap(y - now).abs() > std::f32::consts::FRAC_PI_2 => wrap(y + std::f32::consts::PI),
                _ => y,
            })
        };
        let yaw = match (self.yaw, wanted) {
            (Some(now), Some(target)) => wrap(now + wrap(target - now) * (1. - (-10. * dt.clamp(0., 0.1)).exp())),
            (None, Some(target)) => target,
            (Some(now), None) => now,
            (None, None) => 0.,
        };
        self.yaw = Some(yaw);
        let pitch: f32 = -0.12;
        let forward = [yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()];
        let eye = [head[0] + forward[0] * 0.12, head[1] + 0.06, head[2] + forward[2] * 0.12];
        (eye, forward)
    }
}

/// Skate 3's five ground surfaces (its `physics_surfaces` profiles), by
/// which a board rolls fast, slower, slow, slides, or barely moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Ground {
    #[default]
    Smooth = 1,
    Rough = 2,
    Slow = 3,
    Slippery = 4,
    VerySlow = 5,
}

impl Ground {
    pub fn from_index(index: u8) -> Self {
        match index {
            2 => Self::Rough,
            3 => Self::Slow,
            4 => Self::Slippery,
            5 => Self::VerySlow,
            _ => Self::Smooth,
        }
    }
}

/// The ground of an MW2 surface, from `SURF_TYPE` in its surface flags.
pub fn mw2_ground(surface_flags: u32) -> Ground {
    match (surface_flags >> 20) & 0x1f {
        // brick, gravel, rock, asphalt, bark
        2 | 11 | 17 | 22 | 1 => Ground::Rough,
        // carpet, cloth, dirt, foliage, grass, cushion, fruit, flesh
        3 | 4 | 6 | 8 | 10 | 26 | 27 | 7 => Ground::Slow,
        // ice
        12 => Ground::Slippery,
        // mud, sand, snow, water, slush
        14 | 18 | 19 | 20 | 30 => Ground::VerySlow,
        // default, concrete, glass, metal, paper, plaster, wood, ceramic,
        // plastic, rubber, painted metal, riot shield
        _ => Ground::Smooth,
    }
}

/// The ground of a named material: a Minecraft block (`grass_block`) or a
/// GTA V surface material (`SAND_LOOSE`).
pub fn named_ground(name: &str) -> Ground {
    let name = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    // Ice first: GTA's ICE_TARMAC is black ice, not tarmac. Then the
    // stones whose names hold a soft word (sandstone, mud bricks).
    if has(&["ice"]) && !has(&["dice", "lice", "spice", "slice"]) {
        Ground::Slippery
    } else if has(&["sandstone", "brick", "packed_mud"]) {
        Ground::Rough
    } else if has(&["sand", "mud", "snow", "slush", "water", "marsh", "soul_s", "honey_block", "slime", "powder"]) {
        Ground::VerySlow
    } else if has(&["cobble", "gravel", "tarmac", "asphalt", "rock", "bark", "rumble", "netherrack", "basalt", "blackstone", "tuff", "deepslate"]) {
        Ground::Rough
    } else if has(&["grass", "dirt", "soil", "farmland", "podzol", "mycelium", "moss", "leaves", "hay", "wool", "carpet", "cloth", "fabric", "bush", "twig", "woodchip", "clay", "foliage", "path", "nylium"]) {
        Ground::Slow
    } else {
        Ground::Smooth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riding_with_weapons() {
        let mut combat = Combat::default();
        assert!(combat.armed());
        *combat.switch("guns").unwrap() = false;
        *combat.switch("Melee").unwrap() = false;
        assert!(combat.armed(), "killstreaks still need the weapon");
        *combat.switch("killstreaks").unwrap() = false;
        assert!(!combat.armed());
        assert!(combat.switch("jetpack").is_none());
        assert_eq!(PadLayer::parse("RB"), Some(PadLayer::Rb));
        assert_eq!(PadLayer::Off.xinput_bit(), None);
        // The layer keeps the D-pad and drops the rest.
        assert_eq!(pad_for_board(0x1301, [200, 90], [0.5, 0.], true), (0x0001, [0, 0], [0., 0.]));
        assert_eq!(pad_for_board(0x1301, [200, 90], [0.5, 0.], false), (0x1301, [200, 90], [0.5, 0.]));
        assert_eq!(ram_damage(2.), 0);
        assert!(ram_damage(5.) > 20 && ram_damage(5.) < 50);
        assert!(ram_damage(9.) >= 100);
        assert_eq!(ram_damage(40.), 150);
    }

    #[test]
    fn grounds_from_each_game() {
        assert_eq!(mw2_ground(17 << 20 | 0x0200_0000), Ground::Rough, "rock");
        assert_eq!(mw2_ground(5 << 20), Ground::Smooth, "concrete");
        assert_eq!(mw2_ground(10 << 20), Ground::Slow, "grass");
        assert_eq!(mw2_ground(18 << 20), Ground::VerySlow, "sand");
        assert_eq!(named_ground("grass_block"), Ground::Slow);
        assert_eq!(named_ground("packed_ice"), Ground::Slippery);
        assert_eq!(named_ground("sand"), Ground::VerySlow);
        assert_eq!(named_ground("SANDSTONE_SOLID"), Ground::Rough);
        assert_eq!(named_ground("TARMAC"), Ground::Rough);
        assert_eq!(named_ground("CONCRETE"), Ground::Smooth);
        assert_eq!(named_ground("oak_planks"), Ground::Smooth);
        assert_eq!(named_ground("dirt_path"), Ground::Slow);
        assert_eq!(named_ground("ICE_TARMAC"), Ground::Slippery);
        assert_eq!(named_ground("mud_bricks"), Ground::Rough);
        assert_eq!(named_ground("SNOW_TARMAC"), Ground::VerySlow);
        assert_eq!(named_ground("PAVING_SLAB"), Ground::Smooth);
        assert_eq!(Ground::from_index(9), Ground::Smooth);
    }

    #[test]
    fn the_camera_rule_follows_the_feet_and_the_switch() {
        let auto = CameraRule::FirstThenThird;
        assert!(auto.first_person(true, false));
        assert!(!auto.first_person(false, false));
        assert!(auto.first_person(false, true), "switched to first");
        assert!(!CameraRule::First.first_person(true, true));
        assert_eq!(CameraRule::parse("THIRD"), Some(CameraRule::Third));
        assert!(on_foot("BipedGround") && !on_foot("PhysicsGround"));
    }

    #[test]
    fn the_head_camera_looks_along_the_ride() {
        let mut cam = HeadCamera::default();
        // Riding along +x while facing +z (sideways on the board).
        let (eye, forward) = cam.update([0., 1.7, 0.], [0., 0., 1.], [5., 0., 0.], [1., 0., 0.], false, 0.016);
        assert!(forward[0] > 0.9, "{forward:?}");
        assert!(eye[1] > 1.7);
        // On foot it turns towards the body's facing, not at once.
        let (_, turning) = cam.update([0., 1.7, 0.], [0., 0., 1.], [0., 0., 0.], [1., 0., 0.], true, 0.016);
        assert!(turning[0] > 0.5 && turning[2] > 0.05, "{turning:?}");
        for _ in 0..200 {
            cam.update([0., 1.7, 0.], [0., 0., 1.], [0., 0., 0.], [1., 0., 0.], true, 0.016);
        }
        let (_, settled) = cam.update([0., 1.7, 0.], [0., 0., 1.], [0., 0., 0.], [1., 0., 0.], true, 0.016);
        assert!(settled[2] > 0.95);
        // Standing on the board, it keeps the deck end it already faces.
        let mut cam = HeadCamera::default();
        cam.update([0., 1., 0.], [0., 0., 1.], [-3., 0., 0.], [1., 0., 0.], false, 0.016);
        let (_, still) = cam.update([0., 1., 0.], [0., 0., 1.], [0., 0., 0.], [1., 0., 0.], false, 0.016);
        assert!(still[0] < -0.9, "{still:?}");
    }

    #[test]
    fn falls_hurt_by_height() {
        assert_eq!(FallRules::Skate.damage(4.0), 0);
        assert_eq!(FallRules::Skate.damage(6.75), 50);
        assert!(FallRules::Skate.damage(9.0) >= 100);
        assert_eq!(FallRules::Mw2.damage(3.0), 0, "under 128 inches");
        assert!(FallRules::Mw2.damage(7.7) >= 100, "over 300 inches");
        assert_eq!(FallRules::Off.damage(100.), 0);
        assert_eq!(FallRules::parse("MW2"), Some(FallRules::Mw2));
    }

    #[test]
    fn a_landing_reports_the_drop_from_the_top() {
        let mut t = FallTracker::default();
        assert_eq!(t.update("PhysicsGround", 10., 0.), None);
        assert_eq!(t.update("PhysicsAir", 11., 3.), None);
        assert_eq!(t.update("PhysicsAir", 12., 0.), None);
        assert_eq!(t.update("KnownAir", 5., -10.), None);
        assert_eq!(t.update("PhysicsGround", 2., -1.), Some(10.));
        assert_eq!(t.update("PhysicsGround", 2., 0.), None, "once");
        // A bail in mid-air still lands.
        t.update("PhysicsAir", 20., 0.);
        assert_eq!(t.update("WipeoutGround", 15., -8.), None);
        assert_eq!(t.update("WipeoutGround", 4., 0.), Some(16.));
        // Rolling down a ramp is not a fall.
        assert_eq!(t.update("PhysicsGround", 1., -6.), None);
    }
}
