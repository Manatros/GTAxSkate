//! The vehicles and people around the skater, from GTA (see
//! `crate::vehicles`): their bodies as Skate collision, a vehicle the
//! skater stands on carrying them, vehicles running them over, and people
//! getting out of the way, getting knocked over and having a word.

use super::natives as n;
use super::shv;
use crate::vehicles::{self, Shape, Survey};
use bevy::math::{Mat4, Vec3};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// One vehicle or person near the skater this frame.
pub struct Body {
    pub handle: i32,
    pub person: bool,
    model: u32,
    pub transform: Mat4,
    pub velocity: Vec3,
}

/// A vehicle running into the skater.
pub struct Impact {
    pub velocity: Vec3,
    /// How fast it hit, relative to the skater, m/s.
    pub speed: f32,
}

pub struct Traffic {
    /// Objects pushed just now (not again for a moment).
    pushed: HashMap<i32, Instant>,
    shapes: HashMap<u32, Shape>,
    surveys: HashMap<u32, Survey>,
    boxes: HashMap<u32, Shape>,
    person: Shape,
    pub bodies: Vec<Body>,
    vehicles: Vec<i32>,
    peds: Vec<i32>,
    listed: Option<Instant>,
    /// The bodies as last sent to Skate.
    sent: Vec<(i32, Mat4)>,
    /// The vehicle under the skater and where it was last frame.
    carrier: Option<(i32, Mat4)>,
    /// Speed carried on after leaving a moving vehicle (m/s), and since when.
    drift: Vec3,
    /// When each person last reacted.
    reacted: HashMap<i32, Instant>,
    /// The skater goes faster than a person can stand: people are no
    /// collision then, they get knocked over (as collision, the skater
    /// crashed into them, lost the speed and lay there, the maintainer: video 19-54
    /// 06:23, 19:43). Also when this last changed was sent.
    fast: bool,
    fast_sent: bool,
    /// The pelvis last frame, to test the whole way since for people.
    last_pelvis: Option<Vec3>,
    seed: u32,
}

/// Vehicles and people this much beyond the scan radius are listed.
const LIST_MARGIN: f32 = 12.;
const MAX_VEHICLES: usize = 10;
const MAX_PEOPLE: usize = 14;
/// A vehicle this fast (m/s), hitting the skater this much faster than they
/// move, runs them over.
const RUN_OVER_SPEED: f32 = 3.;
const RUN_OVER_RELATIVE: f32 = 4.;
/// The skater this fast (m/s) knocks a person over.
const KNOCK_SPEED: f32 = 2.5;

impl Traffic {
    pub fn new() -> Self {
        Self {
            pushed: HashMap::new(),
            shapes: HashMap::new(),
            surveys: HashMap::new(),
            boxes: HashMap::new(),
            person: vehicles::person_shape(),
            bodies: Vec::new(),
            vehicles: Vec::new(),
            peds: Vec::new(),
            listed: None,
            sent: Vec::new(),
            carrier: None,
            drift: Vec3::ZERO,
            reacted: HashMap::new(),
            fast: false,
            fast_sent: false,
            last_pelvis: None,
            seed: 0x9E37_79B9,
        }
    }

    /// Starts over (skate mode came on): nothing carried, nothing sent.
    pub fn reset(&mut self) {
        self.bodies.clear();
        self.sent.clear();
        self.carrier = None;
        self.drift = Vec3::ZERO;
        self.listed = None;
    }

    fn random(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    pub fn shape(&self, body: &Body) -> &Shape {
        if body.person {
            return &self.person;
        }
        self.shapes.get(&body.model).or_else(|| self.boxes.get(&body.model)).unwrap_or(&self.person)
    }

    /// Finds the vehicles and people within `radius` of `centre` (skipping
    /// `skip`: the player, a co-op stand-in) and surveys new vehicle models
    /// within `budget`.
    pub fn update(&mut self, skip: &[i32], centre: Vec3, radius: f32, budget: Duration) {
        let started = Instant::now();
        if self.listed.is_none_or(|at| at.elapsed() > Duration::from_millis(250)) {
            self.listed = Some(Instant::now());
            let near = |h: &i32| n::entity_exists(*h) && n::entity_coords(*h).distance(centre) < radius + LIST_MARGIN;
            self.vehicles = shv::all_vehicles().into_iter().filter(near).collect();
            self.peds = shv::all_peds().into_iter().filter(|h| !skip.contains(h)).filter(near).collect();
        }
        let mut bodies = Vec::new();
        for &handle in &self.vehicles {
            if !n::entity_exists(handle) {
                continue;
            }
            let position = n::entity_coords(handle);
            if position.distance(centre) > radius + 4. {
                continue;
            }
            let model = n::entity_model(handle);
            if !self.boxes.contains_key(&model) {
                let (min, max) = n::model_dimensions(model);
                if !(max - min).is_finite() || (max - min).min_element() <= 0.05 || n::is_train(model) && (max - min).length() > 40. {
                    continue;
                }
                self.boxes.insert(model, Shape::boxed(min, max));
                self.surveys.insert(model, Survey::new(min, max));
            }
            let transform = Mat4::from_rotation_translation(n::entity_quaternion(handle), position);
            bodies.push(Body { handle, person: false, model, transform, velocity: n::entity_velocity(handle) });
        }
        bodies.sort_by(|a, b| a.transform.w_axis.truncate().distance(centre).total_cmp(&b.transform.w_axis.truncate().distance(centre)));
        bodies.truncate(MAX_VEHICLES);
        let mut people = Vec::new();
        for &handle in &self.peds {
            if !n::entity_exists(handle) || n::is_dead_or_dying(handle) || n::in_vehicle(handle) || n::is_ragdoll(handle) {
                continue;
            }
            let position = n::entity_coords(handle);
            if position.distance(centre) > radius + 2. {
                continue;
            }
            let transform = Mat4::from_rotation_translation(n::entity_quaternion(handle), position);
            people.push(Body { handle, person: true, model: 0, transform, velocity: n::entity_velocity(handle) });
        }
        people.sort_by(|a, b| a.transform.w_axis.truncate().distance(centre).total_cmp(&b.transform.w_axis.truncate().distance(centre)));
        people.truncate(MAX_PEOPLE);
        bodies.extend(people);
        self.bodies = bodies;

        // Survey the nearest vehicles' models first, a few probes a frame.
        let order: Vec<(i32, u32, Mat4)> = self.bodies.iter().filter(|b| !b.person).map(|b| (b.handle, b.model, b.transform)).collect();
        for (handle, model, transform) in order {
            let Some(survey) = self.surveys.get_mut(&model) else { continue };
            while started.elapsed() < budget {
                let Some((top, bottom)) = survey.next_probe() else { break };
                let (a, b) = (transform.transform_point3(top), transform.transform_point3(bottom));
                let height = n::probe_line(a, b, 2, 0)
                    .filter(|(_, entity)| *entity == handle)
                    .map(|(at, _)| transform.inverse().transform_point3(at).z);
                survey.record(height);
            }
            if survey.done()
                && let Some(survey) = self.surveys.remove(&model)
            {
                self.shapes.insert(model, survey.finish());
            }
            if started.elapsed() >= budget {
                break;
            }
        }
    }

    /// Every body's collision, in the world.
    pub fn triangles(&self) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        for body in self.bodies.iter().filter(|b| !(b.person && self.fast)) {
            let shape = self.shape(body);
            out.extend(shape.triangles().into_iter().map(|t| t.map(|p| body.transform.transform_point3(p))));
        }
        out
    }

    /// Grind rails along the vehicles' top edges, where each stands now.
    pub fn rails(&self) -> Vec<Vec<Vec3>> {
        let mut out = Vec::new();
        for body in self.bodies.iter().filter(|b| !b.person) {
            let shape = self.shape(body);
            out.extend(shape.rails().into_iter().map(|r| r.into_iter().map(|p| body.transform.transform_point3(p)).collect::<Vec<_>>()));
        }
        out
    }

    /// Whether the bodies moved (or came or went) since last sent.
    pub fn changed(&self) -> bool {
        if self.sent.len() != self.bodies.len() || self.fast != self.fast_sent {
            return true;
        }
        self.bodies.iter().zip(&self.sent).any(|(body, (handle, at))| {
            body.handle != *handle
                || body.transform.w_axis.truncate().distance(at.w_axis.truncate()) > 0.03
                || body.transform.y_axis.truncate().dot(at.y_axis.truncate()) < 0.9995
        })
    }

    pub fn mark_sent(&mut self) {
        self.sent = self.bodies.iter().map(|b| (b.handle, b.transform)).collect();
        self.fast_sent = self.fast;
    }

    /// How far the skater is carried this frame: along with the vehicle
    /// under their feet, or on with the speed they left a moving one with.
    pub fn carry(&mut self, feet: Vec3, grounded: bool, dt: f32) -> Vec3 {
        let under = self
            .bodies
            .iter()
            .filter(|b| !b.person)
            .find(|b| vehicles::stands_on(self.shape(b), b.transform, feet))
            .map(|b| (b.handle, b.transform, b.velocity));
        match (under, self.carrier) {
            (Some((handle, now, velocity)), Some((carrier, before))) if handle == carrier => {
                self.carrier = Some((handle, now));
                self.drift = velocity;
                vehicles::carried(before, now, feet)
            }
            (Some((handle, now, velocity)), _) => {
                // On it this frame: from the next frame it carries.
                self.carrier = Some((handle, now));
                self.drift = velocity;
                Vec3::ZERO
            }
            (None, _) => {
                self.carrier = None;
                if self.drift.length_squared() < 0.01 {
                    self.drift = Vec3::ZERO;
                    return Vec3::ZERO;
                }
                // In the air the speed carries on; on the ground it is lost
                // quickly, as a landing's push would.
                let moved = self.drift * dt;
                if grounded {
                    self.drift *= (-dt / 0.3).exp();
                }
                self.drift.z = 0.;
                moved
            }
        }
    }

    /// The skater is on a vehicle's top.
    pub fn carrying(&self) -> bool {
        self.carrier.is_some()
    }

    /// A vehicle hitting the skater (whose body is at `points`, moving at
    /// `velocity`), if one does.
    pub fn run_over(&self, points: &[Vec3], velocity: Vec3) -> Option<Impact> {
        self.bodies
            .iter()
            .filter(|b| !b.person && Some(b.handle) != self.carrier.map(|c| c.0))
            .filter(|b| b.velocity.length() > RUN_OVER_SPEED)
            .filter_map(|b| {
                let hit = points.iter().find(|p| vehicles::hits(self.shape(b), b.transform, **p))?;
                let closing = vehicles::closing_speed(b.transform.w_axis.truncate(), b.velocity, *hit, velocity);
                (closing > RUN_OVER_RELATIVE).then(|| Impact { velocity: b.velocity, speed: closing })
            })
            .next()
    }

    /// People near the skater (at `pelvis`, moving at `velocity`) react:
    /// knocked over when hit, out of the way and a word when nearly hit.
    /// Returns how many were knocked over.
    /// Doors, bins, cones and the like in the skater's way get a push
    /// (they rode straight through doors): a line ahead along the travel at
    /// the knees and the hips, as long as the next moment's travel. Locked
    /// doors and the map's fixed things do not move. Returns how many moved.
    pub fn push_objects(&mut self, player: i32, feet: Vec3, velocity: Vec3) -> usize {
        let speed = velocity.truncate().length();
        if speed < 1. {
            return 0;
        }
        let ahead = velocity.truncate().normalize_or_zero().extend(0.);
        let reach = 0.6 + speed * 0.12;
        let mut pushed = 0;
        self.pushed.retain(|_, at| at.elapsed() < Duration::from_millis(400));
        for height in [0.5, 1.0] {
            let from = feet + Vec3::Z * height;
            let Some((_, entity)) = n::probe_line(from, from + ahead * reach, 16, player) else { continue };
            if !n::is_object(entity) || self.pushed.contains_key(&entity) || n::is_mission_entity(entity) {
                continue;
            }
            super::crash::did("an object was pushed");
            // The skater's momentum (about 80 kg) on the object's weight,
            // guessed from its size: a cone flies, a compressor barely
            // moves (the maintainer: video 19-54 22:12).
            let (min, max) = n::model_dimensions(n::entity_model(entity));
            let size = (max - min).abs();
            let mass = (size.x * size.y * size.z * 150.).clamp(5., 5000.);
            let share = (60. / mass).min(0.8);
            n::push(entity, ahead * (speed * share) + Vec3::Z * (0.3 * share / 0.8));
            self.pushed.insert(entity, Instant::now());
            pushed += 1;
        }
        pushed
    }

    pub fn people(&mut self, player: i32, pelvis: Vec3, velocity: Vec3) -> usize {
        let speed = velocity.length();
        // A little either way of the knock speed, so it does not flicker.
        self.fast = if self.fast { speed > KNOCK_SPEED - 0.5 } else { speed > KNOCK_SPEED };
        let from = self.last_pelvis.replace(pelvis).filter(|p| p.distance(pelvis) < 3.).unwrap_or(pelvis);
        let mut knocked = 0;
        let now = Instant::now();
        self.reacted.retain(|_, at| at.elapsed() < Duration::from_secs(20));
        let people: Vec<(i32, Mat4)> = self.bodies.iter().filter(|b| b.person).map(|b| (b.handle, b.transform)).collect();
        for (handle, transform) in people {
            let at = transform.w_axis.truncate();
            let mission = n::is_mission_entity(handle);
            // Anywhere along the way since last frame, feet to head.
            let hit = (0..=4).any(|k| {
                let p = from.lerp(pelvis, k as f32 / 4.);
                [-0.7, 0., 0.6].iter().any(|dz| vehicles::touches(&self.person, transform, p + Vec3::Z * *dz, 0.3))
            });
            if speed > KNOCK_SPEED && hit {
                if mission {
                    continue;
                }
                let push = velocity * 0.9 + Vec3::Z * (1. + 0.15 * speed);
                super::crash::did("a person was knocked over");
                n::knock_down(handle, 2500 + (speed * 150.) as i32, push);
                // Hurt as by a blow with the board, more the faster, but
                // never killed (the maintainer, video 10-10 09:29 02:42: speed squared,
                // up to 150, killed people just skated into).
                let health = n::health(handle);
                let hurt = vehicles::knock_damage(speed).min(health - vehicles::SURVIVE);
                if hurt > 0 {
                    n::damage(handle, hurt);
                }
                let line = ["GENERIC_CURSE_HIGH", "GENERIC_SHOCKED_HIGH", "GENERIC_FRIGHTENED_HIGH"][self.random() as usize % 3];
                n::speak(handle, line);
                self.reacted.insert(handle, now);
                knocked += 1;
                continue;
            }
            // Nearly hit: coming at them within a few metres.
            let to = at - pelvis;
            let distance = to.truncate().length();
            let coming = speed > 3.5 && velocity.truncate().normalize_or_zero().dot(to.truncate().normalize_or_zero()) > 0.6;
            let recent = self.reacted.get(&handle).is_some_and(|t| t.elapsed() < Duration::from_secs(6));
            if recent || mission || !n::is_human(handle) || n::is_player(handle) {
                continue;
            }
            if coming && distance < 3.5 {
                // A step to the side, away from the skater's line.
                let side = velocity.truncate().perp().normalize_or_zero();
                let sign = if side.dot(to.truncate()) >= 0. { 1. } else { -1. };
                let step = at + (side * sign * 1.6).extend(0.);
                super::crash::did("a person stepped aside");
                n::step_to(handle, step);
                n::look_at(handle, player, 2500);
                let line = ["GENERIC_SHOCKED_HIGH", "GENERIC_INSULT_HIGH", "GENERIC_FRIGHTENED_MED", "GENERIC_CURSE_MED"][self.random() as usize % 4];
                n::speak(handle, line);
                self.reacted.insert(handle, now);
            } else if speed > 2. && distance < 6. && self.random() % 90 == 0 {
                // Someone watches the skater go by.
                super::crash::did("a person looked at the skater");
                n::look_at(handle, player, 2000);
                self.reacted.insert(handle, now);
            }
        }
        knocked
    }

    /// A trick landed: someone close by has something to say about it.
    pub fn cheer(&mut self, pelvis: Vec3) {
        if self.random() % 3 != 0 {
            return;
        }
        let near: Vec<i32> = self
            .bodies
            .iter()
            .filter(|b| b.person && b.transform.w_axis.truncate().distance(pelvis) < 10.)
            .map(|b| b.handle)
            .collect();
        if near.is_empty() {
            return;
        }
        let handle = near[self.random() as usize % near.len()];
        if n::is_mission_entity(handle) || !n::is_human(handle) {
            return;
        }
        let line = ["GENERIC_HOWS_IT_GOING", "GENERIC_HI", "CHAT_RESP", "GENERIC_WHATEVER"][self.random() as usize % 4];
        super::crash::did("a person spoke about a landing");
        n::speak(handle, line);
    }
}
