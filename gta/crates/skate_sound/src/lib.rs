//! The board's sounds while skating, made from what the board does rather
//! than from recordings: wheels rolling on each of Skate 3's ground surfaces
//! (with the cracks in a pavement clacking under the trucks), powerslides,
//! truck grinds and board slides, the tail's pop, landings, the feet on the
//! deck, steps on foot and a bail's clattering board.
//!
//! [`Tracker`] turns the skater's state, speed and wheels, frame by frame,
//! into what is heard ([`Rolling`]) and what happened ([`Hit`]); [`Synth`]
//! plays them as mono samples at [`RATE`].

use std::f32::consts::TAU;

/// Samples a second.
pub const RATE: u32 = 44_100;

/// What grinds or slides on an edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grind {
    #[default]
    None,
    /// The metal trucks: 50-50, 5-0, crooked.
    Trucks,
    /// The wooden deck: boardslide, tailslide, darkslide.
    Deck,
}

/// The continuous sounds: what rolls, grinds or slides, and how fast.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rolling {
    /// Metres a second.
    pub speed: f32,
    /// Wheels on the ground, 0 to 4.
    pub wheels: u8,
    /// Skate 3's surface: 1 smooth, 2 rough, 3 slow, 4 slippery, 5 very slow.
    pub surface: u8,
    pub grind: Grind,
    /// Powersliding.
    pub slide: bool,
}

/// One-off sounds, each with a strength from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// The tail pops.
    Pop(f32),
    /// The wheels come down.
    Land(f32),
    /// The trucks or deck lock onto an edge.
    GrindOn(Grind),
    /// Feet onto the deck: getting on.
    Catch,
    /// A step on foot.
    Step(f32),
    /// The skater falls; the board clatters away.
    Bail,
    /// A crack in the pavement under a truck.
    Joint(f32),
}

/// Skate 3's state names (`PhysicalStateId`, as `Debug` prints it), so a
/// state travels as a number: its index plus one, 0 for none.
pub const STATES: [&str; 26] = [
    "PhysicsGround", "SlideGround", "RevertGround", "GroundAnimation", "Skitching", "FollowPath",
    "PhysicsAir", "KnownAir", "PhysicsAirSecondary", "WipeoutGround", "GrindBoardslide", "GrindFiftyFifty",
    "GrindTipslide", "GrindFiveO", "GrindBackslash", "GrindDarkslide", "BipedGround", "BipedAir",
    "OffBoardPushing", "LandingOnDeck", "HandPlant", "FootPlant", "Boneless", "Sleeping", "Nonspecific",
    "Teleporting",
];

/// A state's number (see [`STATES`]).
pub fn state_code(name: &str) -> u8 {
    STATES.iter().position(|s| *s == name).map_or(0, |i| i as u8 + 1)
}

/// The state numbered `code`, or "" for none.
pub fn state_name(code: u8) -> &'static str {
    usize::from(code).checked_sub(1).and_then(|i| STATES.get(i)).copied().unwrap_or("")
}

/// How a state name of Skate 3's (`PhysicalStateId`, as `Debug` prints it)
/// sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Ground,
    Slide,
    Air,
    Grind(Grind),
    Wipeout,
    Foot,
    FootAir,
    Other,
}

fn phase(state: &str) -> Phase {
    match state {
        "PhysicsGround" | "RevertGround" | "GroundAnimation" | "Skitching" | "FollowPath" | "LandingOnDeck" => {
            Phase::Ground
        }
        "SlideGround" => Phase::Slide,
        "PhysicsAir" | "KnownAir" | "PhysicsAirSecondary" | "HandPlant" | "FootPlant" | "Boneless" => Phase::Air,
        "GrindFiftyFifty" | "GrindFiveO" | "GrindBackslash" => Phase::Grind(Grind::Trucks),
        "GrindBoardslide" | "GrindTipslide" | "GrindDarkslide" => Phase::Grind(Grind::Deck),
        "WipeoutGround" => Phase::Wipeout,
        "BipedGround" | "OffBoardPushing" => Phase::Foot,
        "BipedAir" => Phase::FootAir,
        _ => Phase::Other,
    }
}

/// A pavement's slabs, metres between cracks, and a skateboard's
/// wheelbase.
const SLAB: f32 = 1.6;
const WHEELBASE: f32 = 0.36;
/// Metres between steps on foot.
const STRIDE: f32 = 0.75;

/// Reads the skater frame by frame.
#[derive(Clone, Debug)]
pub struct Tracker {
    last: Phase,
    /// The fastest fall while in the air, metres a second (positive down).
    falling: f32,
    walked: f32,
    rolled: f32,
    /// A crack the back truck still has to cross: metres to go.
    back_truck: Option<f32>,
}

impl Default for Tracker {
    fn default() -> Self {
        Self { last: Phase::Other, falling: 0., walked: 0., rolled: 0., back_truck: None }
    }
}

impl Tracker {
    /// Forgets the last frame: skating starts anew.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame: Skate's state name, speed and vertical speed (metres a
    /// second, up positive), wheels down, surface, and the frame's seconds.
    /// Returns what is heard now and what happened since the last frame.
    pub fn update(
        &mut self,
        state: &str,
        speed: f32,
        vertical: f32,
        wheels: u8,
        surface: u8,
        dt: f32,
    ) -> (Rolling, Vec<Hit>) {
        let now = phase(state);
        let speed = if speed.is_finite() { speed.max(0.) } else { 0. };
        let vertical = if vertical.is_finite() { vertical } else { 0. };
        let dt = if dt.is_finite() { dt.clamp(0., 0.25) } else { 0. };
        let mut hits = Vec::new();
        let was = self.last;
        if now != was {
            match (was, now) {
                (Phase::Ground | Phase::Slide, Phase::Air) if vertical > 1.0 => {
                    hits.push(Hit::Pop((vertical / 4.).clamp(0.3, 1.)));
                }
                (Phase::Grind(_), Phase::Air) => hits.push(Hit::Pop(0.6)),
                (Phase::Air, Phase::Ground | Phase::Slide) => {
                    hits.push(Hit::Land((self.falling / 6.).clamp(0.25, 1.)));
                }
                (Phase::Air | Phase::Ground | Phase::Slide | Phase::Grind(_), Phase::Grind(kind)) => hits.push(Hit::GrindOn(kind)),
                (Phase::Wipeout, _) => {}
                (_, Phase::Wipeout) => hits.push(Hit::Bail),
                (Phase::FootAir | Phase::Air, Phase::Foot) => hits.push(Hit::Step(0.9)),
                (Phase::Foot | Phase::FootAir, Phase::Ground | Phase::Air) => hits.push(Hit::Catch),
                _ => {}
            }
            if matches!(now, Phase::Air | Phase::FootAir) {
                self.falling = 0.;
            }
        }
        if matches!(now, Phase::Air | Phase::FootAir) {
            self.falling = self.falling.max(-vertical);
        }
        // Steps on foot, and the cracks a pavement's wheels cross.
        if now == Phase::Foot && speed > 0.4 {
            self.walked += speed * dt;
            if self.walked >= STRIDE {
                self.walked -= STRIDE;
                hits.push(Hit::Step((speed / 4.).clamp(0.35, 0.8)));
            }
        } else {
            self.walked = STRIDE * 0.6;
        }
        let rolling = matches!(now, Phase::Ground | Phase::Slide) && wheels > 0;
        if rolling && surface <= 1 && speed > 0.8 {
            let strength = (speed / 8.).clamp(0.2, 0.7);
            self.rolled += speed * dt;
            if self.rolled >= SLAB {
                self.rolled -= SLAB;
                hits.push(Hit::Joint(strength));
                self.back_truck = Some(WHEELBASE);
            }
            if let Some(left) = self.back_truck.as_mut() {
                *left -= speed * dt;
                if *left <= 0. {
                    self.back_truck = None;
                    hits.push(Hit::Joint(strength * 0.8));
                }
            }
        } else {
            self.back_truck = None;
        }
        self.last = now;
        let heard = Rolling {
            speed,
            wheels: if rolling { wheels.min(4) } else { 0 },
            surface: surface.clamp(1, 5),
            grind: match now {
                Phase::Grind(kind) => kind,
                _ => Grind::None,
            },
            slide: now == Phase::Slide,
        };
        (heard, hits)
    }
}

/// A state-variable filter (Chamberlin), low and band outputs.
#[derive(Clone, Copy, Debug, Default)]
struct Svf {
    low: f32,
    band: f32,
}

impl Svf {
    fn run(&mut self, x: f32, cutoff: f32, q: f32) -> (f32, f32) {
        let f = 2. * (std::f32::consts::PI * cutoff.clamp(10., 12_000.) / RATE as f32).sin();
        let damp = 1. / q.max(0.5);
        let high = x - self.low - damp * self.band;
        self.band += f * high;
        self.low += f * self.band;
        // Never let a filter run away on a bad parameter.
        if !self.low.is_finite() || !self.band.is_finite() {
            *self = Self::default();
        }
        (self.low, self.band)
    }
}

/// A one-shot playing.
#[derive(Clone, Copy, Debug)]
struct Voice {
    hit: Hit,
    /// Seconds since it started.
    t: f32,
    /// Its own pitch spread, around 1.
    pitch: f32,
    lp: f32,
}

impl Voice {
    fn length(&self) -> f32 {
        match self.hit {
            Hit::Pop(_) => 0.3,
            Hit::Land(_) => 0.45,
            Hit::GrindOn(_) => 0.5,
            Hit::Catch => 0.12,
            Hit::Step(_) => 0.15,
            Hit::Bail => 1.3,
            Hit::Joint(_) => 0.08,
        }
    }
}

/// A wooden knock: the deck struck.
fn knock(t: f32, freq: f32, decay: f32) -> f32 {
    if t < 0. {
        return 0.;
    }
    let e = (-t / decay).exp();
    ((TAU * freq * t).sin() + 0.45 * (TAU * freq * 2.71 * t).sin() * (-t / (decay * 0.4)).exp()) * e
}

/// A low thump: a body or the wheels coming down.
fn thump(t: f32, freq: f32, decay: f32) -> f32 {
    if t < 0. {
        return 0.;
    }
    // The pitch drops as it rings, as a struck board does.
    let f = freq * (1. + 0.6 * (-t / 0.015).exp());
    (TAU * f * t).sin() * (-t / decay).exp()
}

/// Plays the board's sounds.
pub struct Synth {
    seed: u32,
    target: Rolling,
    /// Seconds since the last [`Synth::set`]: the sound dies away when no
    /// one says what the board does (a paused game).
    stale: f32,
    roll_gain: f32,
    grind_gain: [f32; 2],
    slide_gain: f32,
    speed: f32,
    roll: Svf,
    grit: Svf,
    metal: Svf,
    wood: Svf,
    squeal: Svf,
    phase: [f32; 3],
    shimmer: f32,
    crackle: f32,
    voices: Vec<Voice>,
    /// Master volume, 0 to 1.
    pub volume: f32,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            seed: 0x2545_f491,
            target: Rolling::default(),
            stale: 0.,
            roll_gain: 0.,
            grind_gain: [0.; 2],
            slide_gain: 0.,
            speed: 0.,
            roll: Svf::default(),
            grit: Svf::default(),
            metal: Svf::default(),
            wood: Svf::default(),
            squeal: Svf::default(),
            phase: [0.; 3],
            shimmer: 0.,
            crackle: 0.,
            voices: Vec::new(),
            volume: 1.,
        }
    }

    /// What the board does now.
    pub fn set(&mut self, rolling: Rolling) {
        self.target = rolling;
        self.stale = 0.;
    }

    /// Starts a one-shot.
    pub fn hit(&mut self, hit: Hit) {
        let pitch = 0.9 + 0.2 * (self.noise() * 0.5 + 0.5);
        if self.voices.len() >= 24 {
            self.voices.remove(0);
        }
        self.voices.push(Voice { hit, t: 0., pitch, lp: 0. });
    }

    /// Whether anything is sounding.
    pub fn silent(&self) -> bool {
        self.voices.is_empty() && self.roll_gain < 1e-4 && self.grind_gain.iter().all(|g| *g < 1e-4) && self.slide_gain < 1e-4
    }

    fn noise(&mut self) -> f32 {
        // xorshift32
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x as f32 / u32::MAX as f32) * 2. - 1.
    }

    /// Fills `out` with the next mono samples, from -1 to 1.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1. / RATE as f32;
        for sample in out.iter_mut() {
            *sample = self.next(dt);
        }
    }

    fn next(&mut self, dt: f32) -> f32 {
        self.stale += dt;
        let live = self.stale < 0.25;
        let t = self.target;
        // Gains glide, so state changes never click.
        let glide = |current: &mut f32, target: f32, seconds: f32| {
            *current += (target - *current) * (dt / seconds).min(1.);
        };
        let surface_gain = match t.surface {
            2 => 1.25,
            3 => 0.55,
            4 => 0.35,
            5 => 0.45,
            _ => 1.,
        };
        let rolling = if live && t.wheels > 0 && t.grind == Grind::None {
            (t.speed / 7.).clamp(0., 1.).powf(0.8) * surface_gain * (0.6 + 0.1 * f32::from(t.wheels))
        } else {
            0.
        };
        glide(&mut self.roll_gain, rolling, 0.05);
        glide(&mut self.speed, t.speed, 0.08);
        let grind_level = (t.speed / 4.).clamp(0.45, 1.);
        glide(&mut self.grind_gain[0], if live && t.grind == Grind::Trucks { grind_level } else { 0. }, 0.03);
        glide(&mut self.grind_gain[1], if live && t.grind == Grind::Deck { grind_level } else { 0. }, 0.03);
        glide(&mut self.slide_gain, if live && t.slide && t.wheels > 0 { (t.speed / 5.).clamp(0.3, 1.) } else { 0. }, 0.04);

        let mut mix = 0.;
        let v = self.speed;
        if self.roll_gain > 1e-4 {
            let n = self.noise();
            // The wheels' rumble: lower and duller on soft ground.
            let cutoff = match t.surface {
                3 | 5 => 80. + 25. * v,
                4 => 300. + 120. * v,
                2 => 160. + 90. * v,
                _ => 130. + 70. * v,
            };
            let (low, band) = self.roll.run(n, cutoff, 1.2);
            let mut roll = low * 2.2 + band * 0.8;
            // Grain: asphalt's grit, grass and sand's crunch, ice's hiss.
            let (grit_freq, grit) = match t.surface {
                2 => (2400., 0.9),
                3 => (1700., 0.35),
                4 => (5200., 0.25),
                5 => (900., 0.6),
                _ => (3000., 0.18),
            };
            let n = self.noise();
            let (_, g) = self.grit.run(n, grit_freq, 0.9);
            roll += g * grit;
            // Stones and gravel under the wheels, on rough and soft ground.
            if matches!(t.surface, 2 | 5) {
                let rate = 25. * v * dt;
                if self.noise() * 0.5 + 0.5 < rate {
                    self.crackle = 0.8;
                }
            }
            roll += self.crackle * self.noise();
            self.crackle *= 0.93;
            mix += roll * self.roll_gain * 0.42;
        }
        if self.grind_gain[0] > 1e-4 {
            // Metal scraping metal: a bright, ringing band of noise and the
            // truck's partials, wavering as the grind chatters.
            let wobble = {
                self.shimmer += dt * (7. + v);
                0.7 + 0.3 * (TAU * self.shimmer).sin()
            };
            let n = self.noise();
            let (_, band) = self.metal.run(n, 2700. + 150. * v, 6.);
            let mut ring = 0.;
            for (i, freq) in [1170., 1830., 3270.].into_iter().enumerate() {
                self.phase[i] = (self.phase[i] + freq * dt).fract();
                ring += (TAU * self.phase[i]).sin() * [0.25, 0.18, 0.1][i];
            }
            mix += (band * 1.4 + ring * wobble) * self.grind_gain[0] * 0.38;
        }
        if self.grind_gain[1] > 1e-4 {
            // Wood sliding on concrete or metal: a dry, low rasp.
            let n = self.noise();
            let (low, band) = self.wood.run(n, 650. + 60. * v, 1.4);
            mix += (band * 1.1 + low * 0.6) * self.grind_gain[1] * 0.45;
        }
        if self.slide_gain > 1e-4 {
            // Urethane scrubbing sideways.
            let n = self.noise();
            let cutoff = 1500. + 200. * (TAU * self.shimmer * 0.5).sin();
            let (_, band) = self.squeal.run(n, cutoff, 3.);
            self.shimmer += dt * 3.;
            mix += band * self.slide_gain * 0.35;
        }
        mix += self.one_shots(dt);
        if !mix.is_finite() {
            *self = Self { volume: self.volume, ..Self::new() };
            return 0.;
        }
        // A soft limiter.
        (mix * self.volume.clamp(0., 1.)).tanh()
    }

    fn one_shots(&mut self, dt: f32) -> f32 {
        let mut mix = 0.;
        let mut voices = std::mem::take(&mut self.voices);
        for v in voices.iter_mut() {
            let n = self.noise();
            let t = v.t;
            let p = v.pitch;
            let s = match v.hit {
                Hit::Pop(k) => {
                    // The tail strikes the ground, then the deck rings.
                    let crack = n * (-t / 0.0025).exp();
                    k * (crack * 0.9 + knock(t, 540. * p, 0.035) * 0.7 + thump(t, 95. * p, 0.05) * 0.5)
                }
                Hit::Land(k) => {
                    v.lp += (n - v.lp) * 0.2;
                    let slap = v.lp * (-t / 0.02).exp() * 1.4;
                    let second = t - 0.028 / p;
                    k * (thump(t, 62. * p, 0.1) * 1.1
                        + knock(t, 390. * p, 0.045) * 0.5
                        + slap
                        + thump(second, 70. * p, 0.07) * 0.6
                        + knock(second, 420. * p, 0.03) * 0.3)
                }
                Hit::GrindOn(Grind::Trucks) | Hit::GrindOn(Grind::None) => {
                    // A clank: the truck's metal rings.
                    let e = (-t / 0.16).exp();
                    let ring = (TAU * 1170. * p * t).sin() * 0.5
                        + (TAU * 2390. * p * t).sin() * 0.35
                        + (TAU * 3420. * p * t).sin() * 0.25;
                    ring * e + n * (-t / 0.004).exp() * 0.8 + thump(t, 80., 0.05) * 0.4
                }
                Hit::GrindOn(Grind::Deck) => knock(t, 330. * p, 0.05) * 0.9 + n * (-t / 0.006).exp() * 0.6,
                Hit::Catch => knock(t, 700. * p, 0.012) * 0.35 + thump(t, 120. * p, 0.02) * 0.25,
                Hit::Step(k) => {
                    v.lp += (n - v.lp) * 0.08;
                    k * (v.lp * (-t / 0.04).exp() * 1.8 + thump(t, 110. * p, 0.03) * 0.35)
                }
                Hit::Bail => {
                    // The board tumbling: knocks that come faster and
                    // quieter, and a scrape under them.
                    let mut s = 0.;
                    for (i, at) in [0., 0.13, 0.23, 0.31, 0.38, 0.44].into_iter().enumerate() {
                        let k = 1. - i as f32 * 0.14;
                        s += knock(t - at, (380. + 70. * i as f32) * p, 0.04) * k * 0.8;
                        s += thump(t - at, 90. * p, 0.04) * k * 0.5;
                    }
                    v.lp += (n - v.lp) * 0.15;
                    s + v.lp * (-t / 0.4).exp() * 0.6
                }
                Hit::Joint(k) => k * (n * (-t / 0.002).exp() * 0.6 + thump(t, 170. * p, 0.018) * 0.7),
            };
            mix += s;
            v.t += dt;
        }
        voices.retain(|v| v.t < v.length());
        self.voices = voices;
        mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(synth: &mut Synth, seconds: f32) -> Vec<f32> {
        let mut out = vec![0.; (seconds * RATE as f32) as usize];
        synth.render(&mut out);
        out
    }

    fn loudness(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
    }

    #[test]
    fn states_travel_as_numbers() {
        for name in STATES {
            assert_eq!(state_name(state_code(name)), name);
        }
        assert_eq!(state_code("Flying"), 0);
        assert_eq!(state_name(0), "");
        assert_eq!(state_name(200), "");
    }

    #[test]
    fn an_ollie_pops_and_lands() {
        let mut tracker = Tracker::default();
        let dt = 1. / 60.;
        let (_, hits) = tracker.update("PhysicsGround", 4., 0., 4, 1, dt);
        assert!(hits.is_empty());
        let (heard, hits) = tracker.update("PhysicsAir", 4., 3., 0, 1, dt);
        assert_eq!(heard.wheels, 0);
        assert!(matches!(hits[..], [Hit::Pop(k)] if k > 0.5));
        tracker.update("PhysicsAir", 4., -5., 0, 1, dt);
        let (heard, hits) = tracker.update("PhysicsGround", 4., 0., 4, 1, dt);
        assert!(matches!(hits[..], [Hit::Land(k)] if (k - 5. / 6.).abs() < 1e-4));
        assert_eq!(heard.wheels, 4);
        // Rolling off a ledge is no pop.
        let (_, hits) = tracker.update("PhysicsAir", 4., -0.5, 0, 1, dt);
        assert!(hits.is_empty());
    }

    #[test]
    fn grinds_bails_and_steps() {
        let mut tracker = Tracker::default();
        let dt = 1. / 60.;
        tracker.update("PhysicsAir", 3., 1., 0, 1, dt);
        let (heard, hits) = tracker.update("GrindFiftyFifty", 3., 0., 0, 1, dt);
        assert_eq!(hits, vec![Hit::GrindOn(Grind::Trucks)]);
        assert_eq!(heard.grind, Grind::Trucks);
        let (_, hits) = tracker.update("GrindBoardslide", 3., 0., 0, 1, dt);
        assert_eq!(hits, vec![Hit::GrindOn(Grind::Deck)]);
        let (_, hits) = tracker.update("WipeoutGround", 3., 0., 0, 1, dt);
        assert_eq!(hits, vec![Hit::Bail]);
        tracker.update("BipedGround", 0., 0., 0, 1, dt);
        let steps: usize = (0..120)
            .map(|_| tracker.update("BipedGround", 1.5, 0., 0, 1, dt).1.len())
            .sum();
        // Three metres walked, a step every 0.75 m.
        assert!((3..=5).contains(&steps), "{steps} steps");
    }

    #[test]
    fn pavement_cracks_click_under_both_trucks() {
        let mut tracker = Tracker::default();
        let dt = 1. / 60.;
        let joints: Vec<Hit> = (0..60)
            .flat_map(|_| tracker.update("PhysicsGround", 5., 0., 4, 1, dt).1)
            .collect();
        // Five metres: cracks at 1.6, 3.2 and 4.8 m under the front truck,
        // the first two under the back one too.
        assert_eq!(joints.len(), 5, "{joints:?}");
        // Grass has none.
        let none: usize = (0..60).map(|_| tracker.update("PhysicsGround", 5., 0., 4, 3, dt).1.len()).sum();
        assert_eq!(none, 0);
    }

    #[test]
    fn the_synth_rolls_louder_when_faster_and_falls_silent() {
        let mut synth = Synth::new();
        let rolling = |speed| Rolling { speed, wheels: 4, surface: 1, ..Default::default() };
        synth.set(rolling(1.));
        let slow = loudness(&run(&mut synth, 0.2)[4000..]);
        synth.set(rolling(7.));
        let fast = loudness(&run(&mut synth, 0.2)[4000..]);
        assert!(fast > slow * 1.5 && slow > 0.001, "slow {slow}, fast {fast}");
        // Nobody says what the board does any more: it fades out.
        let after = run(&mut synth, 0.6);
        assert!(loudness(&after[after.len() - 4000..]) < 1e-3);
        assert!(synth.silent());
    }

    #[test]
    fn every_sound_stays_in_range() {
        let mut synth = Synth::new();
        for hit in [Hit::Pop(1.), Hit::Land(1.), Hit::GrindOn(Grind::Trucks), Hit::GrindOn(Grind::Deck), Hit::Catch, Hit::Step(1.), Hit::Bail, Hit::Joint(1.)] {
            synth.hit(hit);
        }
        for grind in [Grind::None, Grind::Trucks, Grind::Deck] {
            for surface in 1..=5 {
                synth.set(Rolling { speed: 12., wheels: 4, surface, grind, slide: true });
                let out = run(&mut synth, 0.1);
                assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 1.));
                assert!(loudness(&out) > 0.005, "{grind:?} on {surface}");
            }
        }
        synth.set(Rolling { speed: f32::NAN, wheels: 4, surface: 9, ..Default::default() });
        assert!(run(&mut synth, 0.05).iter().all(|s| s.is_finite()));
    }
}
