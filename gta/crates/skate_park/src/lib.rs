//! Skate 3's object dropper: kickers, ramps, a quarter pipe, banks, rails,
//! ledges, manual pads and funboxes dropped anywhere to build a park, as
//! Skate 3 lets you anywhere in San Vanelona.
//!
//! [`shape`] is each object's geometry (metres, x right, y away from the
//! one dropping it, z up, centred on its footprint) with the edges that
//! grind as rails; [`Dropper`] is the dropper's controls: pick an object,
//! move and turn its ghost, drop it, pick one back up.

pub mod shapes;

pub use shapes::{Material, Shape, shape};

/// What can be dropped, in the dropper's order.
pub const KINDS: [&str; 10] = [
    "Kicker",
    "Launch ramp",
    "Quarter pipe",
    "Bank",
    "Flat rail",
    "Down rail",
    "Ledge",
    "Manual pad",
    "Funbox",
    "Crate",
];

/// Most objects one skater keeps dropped.
pub const MAX_OBJECTS: usize = 24;

/// A dropped object: which ([`KINDS`]), where its footprint's centre sits
/// on the ground and which way it faces (radians about the up axis, 0
/// facing +y), in the host game's units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParkObject {
    pub kind: u8,
    pub at: [f32; 3],
    pub yaw: f32,
}

impl ParkObject {
    /// Whether this is a usable object: a known kind at a finite place.
    pub fn valid(&self) -> bool {
        usize::from(self.kind) < KINDS.len() && self.at.iter().all(|v| v.is_finite()) && self.yaw.is_finite()
    }

    /// Its shape placed in the world, in the host's units (`unit` metres
    /// each).
    pub fn placed(&self, unit: f32) -> Shape {
        let mut s = shape(usize::from(self.kind));
        let (sin, cos) = self.yaw.sin_cos();
        let place = |p: [f32; 3]| {
            let x = p[0] * cos - p[1] * sin;
            let y = p[0] * sin + p[1] * cos;
            [self.at[0] + x / unit, self.at[1] + y / unit, self.at[2] + p[2] / unit]
        };
        let turn = |n: [f32; 3]| [n[0] * cos - n[1] * sin, n[0] * sin + n[1] * cos, n[2]];
        for t in &mut s.triangles {
            t.points = t.points.map(place);
            t.normal = turn(t.normal);
        }
        for rail in &mut s.rails {
            for p in rail.iter_mut() {
                *p = place(*p);
            }
        }
        s
    }
}

/// The controller as the dropper reads it: XInput button bits, sticks -1
/// to 1 (y up) and triggers 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    pub buttons: u16,
    pub left: [f32; 2],
    pub right: [f32; 2],
    pub triggers: [f32; 2],
}

pub mod buttons {
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const LB: u16 = 0x0100;
    pub const RB: u16 = 0x0200;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

/// What the dropper did this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Opened,
    Closed,
    /// The skater's objects changed.
    Changed,
    /// The ghost cannot go there: the skater has the most objects already.
    Full,
}

/// Metres the ghost first sits ahead, moves a second, and may stray.
const AHEAD: f32 = 4.;
const MOVE_SPEED: f32 = 5.;
const REACH: f32 = 25.;
/// Radians a second it turns, and metres a second it rises or sinks.
const TURN_SPEED: f32 = 2.;
const LIFT_SPEED: f32 = 1.5;
/// Metres within which the ghost picks a dropped object back up.
const PICK_UP: f32 = 2.5;

/// The object dropper's state for one skater: the ghost and what they
/// dropped. Positions are metres in the host's world, z up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dropper {
    pub open: bool,
    /// The object the ghost is.
    pub kind: usize,
    /// The ghost: where, facing, raised or lowered from the skater's feet.
    pub at: [f32; 3],
    pub yaw: f32,
    pub lift: f32,
    /// What this skater dropped (metres).
    pub placed: Vec<ParkObject>,
    last: u16,
    /// The bumper held was part of a D-pad combination.
    bumper_spent: bool,
    /// How many objects there are to pick from: [`KINDS`] unless the host
    /// game has a list of its own.
    pub kinds: usize,
}

impl Dropper {
    fn count(&self) -> usize {
        if self.kinds == 0 { KINDS.len() } else { self.kinds }
    }

    /// Opens the ghost ahead of the skater at `feet` facing `heading`
    /// (radians, 0 along +y, as an object's yaw).
    pub fn open(&mut self, feet: [f32; 3], heading: f32) {
        self.open = true;
        let (sin, cos) = heading.sin_cos();
        self.at = [feet[0] - sin * AHEAD, feet[1] + cos * AHEAD, feet[2]];
        self.yaw = heading;
        self.lift = 0.;
        // The buttons held now opened it; they wait to be let go.
        self.last = u16::MAX;
        self.bumper_spent = true;
    }

    /// The ghost as an object.
    pub fn ghost(&self) -> ParkObject {
        ParkObject {
            kind: self.kind.min(self.count() - 1) as u8,
            at: [self.at[0], self.at[1], self.at[2] + self.lift],
            yaw: self.yaw,
        }
    }

    /// One frame of the dropper's controls, `camera_yaw` turning the
    /// sticks to the view (radians, 0 along +y). Left stick moves the
    /// ghost, right stick turns it, triggers lower and raise it, LB / RB
    /// change the object, A drops it, X picks up the one under it, Y picks
    /// up all, B closes.
    pub fn update(&mut self, pad: Pad, camera_yaw: f32, feet: [f32; 3], dt: f32) -> Vec<Event> {
        use buttons::*;
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        let dt = if dt.is_finite() { dt.clamp(0., 0.1) } else { 0. };
        let pressed = |bit: u16| pad.buttons & bit != 0 && self.last & bit == 0;
        let dead = |v: f32| if v.abs() < 0.2 { 0. } else { v.clamp(-1., 1.) };
        let (sin, cos) = camera_yaw.sin_cos();
        let (mx, my) = (dead(pad.left[0]), dead(pad.left[1]));
        // Right is +x turned by the view, forward is +y turned.
        self.at[0] += (mx * cos - my * sin) * MOVE_SPEED * dt;
        self.at[1] += (mx * sin + my * cos) * MOVE_SPEED * dt;
        let (dx, dy) = (self.at[0] - feet[0], self.at[1] - feet[1]);
        let away = dx.hypot(dy);
        if away > REACH {
            self.at[0] = feet[0] + dx / away * REACH;
            self.at[1] = feet[1] + dy / away * REACH;
        }
        self.yaw -= dead(pad.right[0]) * TURN_SPEED * dt;
        self.lift = (self.lift + (pad.triggers[1] - pad.triggers[0]) * LIFT_SPEED * dt).clamp(-3., 6.);
        // A bumper changes the object when let go, unless the D-pad was
        // pressed with it (LB + right closes the dropper).
        if pad.buttons & (DPAD_LEFT | DPAD_RIGHT) != 0 {
            self.bumper_spent = true;
        }
        let released = |bit: u16| pad.buttons & bit == 0 && self.last & bit != 0 && self.last != u16::MAX;
        let count = self.count();
        if released(LB) && !self.bumper_spent {
            self.kind = (self.kind + count - 1) % count;
        }
        if released(RB) && !self.bumper_spent {
            self.kind = (self.kind + 1) % count;
        }
        if pad.buttons & (LB | RB) == 0 {
            self.bumper_spent = false;
        }
        if pressed(A) {
            if self.placed.len() >= MAX_OBJECTS {
                events.push(Event::Full);
            } else {
                self.placed.push(self.ghost());
                events.push(Event::Changed);
            }
        }
        if pressed(X) {
            let ghost = self.ghost().at;
            let nearest = self
                .placed
                .iter()
                .enumerate()
                .map(|(i, o)| (i, (o.at[0] - ghost[0]).hypot(o.at[1] - ghost[1])))
                .filter(|(_, d)| *d <= PICK_UP)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = nearest {
                let picked = self.placed.remove(i);
                self.kind = usize::from(picked.kind);
                self.yaw = picked.yaw;
                events.push(Event::Changed);
            }
        }
        if pressed(Y) && !self.placed.is_empty() {
            self.placed.clear();
            events.push(Event::Changed);
        }
        if pressed(B) {
            self.open = false;
            events.push(Event::Closed);
        }
        self.last = if self.last == u16::MAX && pad.buttons != 0 { u16::MAX } else { pad.buttons };
        events
    }

    /// The dropper's controls, for its HUD.
    pub fn hints(keyboard: bool) -> &'static str {
        if keyboard {
            "WASD move   arrows turn   Z/C lower/raise   X/B object   Space drop   LShift pick up   Q pick up all   LCtrl done"
        } else {
            "Left stick move   right stick turn   LT/RT lower/raise   LB/RB object   A drop   X pick up   Y pick up all   B done"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(buttons: u16) -> Pad {
        Pad { buttons, ..Default::default() }
    }

    #[test]
    fn the_dropper_drops_moves_and_picks_up() {
        let mut d = Dropper::default();
        assert!(d.update(pad(buttons::A), 0., [0.; 3], 0.1).is_empty(), "closed, it does nothing");
        d.open([10., 20., 1.], 0.);
        assert_eq!(d.at, [10., 24., 1.], "ahead of the skater");
        // The buttons that opened it are let go first.
        assert!(d.update(pad(buttons::A), 0., [10., 20., 1.], 0.01).is_empty());
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        d.update(pad(buttons::RB), 0., [10., 20., 1.], 0.01);
        assert_eq!(KINDS[d.kind], "Kicker", "on letting go");
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(KINDS[d.kind], "Launch ramp");
        // LB then the D-pad (the combination that closes it) changes nothing.
        d.update(pad(buttons::LB), 0., [10., 20., 1.], 0.01);
        d.update(pad(buttons::LB | buttons::DPAD_RIGHT), 0., [10., 20., 1.], 0.01);
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(KINDS[d.kind], "Launch ramp");
        assert_eq!(d.update(pad(buttons::A), 0., [10., 20., 1.], 0.01), vec![Event::Changed]);
        assert_eq!(d.placed[0].kind, 1);
        // Pushed right for a second with the view turned a quarter left.
        let right = Pad { left: [1., 0.], ..Default::default() };
        d.update(right, std::f32::consts::FRAC_PI_2, [10., 20., 1.], 0.1);
        assert!((d.at[1] - 24.5).abs() < 1e-4, "{:?}", d.at);
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(d.update(pad(buttons::X), 0., [10., 20., 1.], 0.01), vec![Event::Changed]);
        assert!(d.placed.is_empty(), "picked back up");
        for _ in 0..MAX_OBJECTS {
            d.update(pad(0), 0., [10., 20., 1.], 0.01);
            d.update(pad(buttons::A), 0., [10., 20., 1.], 0.01);
        }
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(d.update(pad(buttons::A), 0., [10., 20., 1.], 0.01), vec![Event::Full]);
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(d.update(pad(buttons::Y), 0., [10., 20., 1.], 0.01), vec![Event::Changed]);
        assert!(d.placed.is_empty());
        // Far pushes stop at its reach.
        let far = Pad { left: [0., 1.], ..Default::default() };
        for _ in 0..100 {
            d.update(far, 0., [10., 20., 1.], 0.1);
        }
        assert!(((d.at[0] - 10.).hypot(d.at[1] - 20.) - REACH).abs() < 1e-3);
        d.update(pad(0), 0., [10., 20., 1.], 0.01);
        assert_eq!(d.update(pad(buttons::B), 0., [10., 20., 1.], 0.01), vec![Event::Closed]);
        assert!(!d.open);
    }

    #[test]
    fn objects_are_placed_and_turned() {
        let o = ParkObject { kind: 4, at: [100., 200., 10.], yaw: std::f32::consts::FRAC_PI_2 };
        assert!(o.valid());
        let placed = o.placed(0.0254);
        // The flat rail runs along its y; turned a quarter it runs along -x.
        let rail = &placed.rails[0];
        let (a, b) = (rail[0], rail[rail.len() - 1]);
        assert!((a[1] - b[1]).abs() < 1e-2 && (a[0] - b[0]).abs() > 100., "{a:?} {b:?}");
        assert!(rail.iter().all(|p| p[2] > 10.), "above the ground");
        assert!(!ParkObject { kind: 99, ..o }.valid());
        assert!(!ParkObject { at: [f32::NAN, 0., 0.], ..o }.valid());
    }
}
