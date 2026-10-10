//! Skate 3's object dropper in GTA V: GTA's own skate park props (ramps,
//! a quarter pipe, a half pipe, rails, funboxes) and benches, tables and
//! crates, dropped anywhere while skating. The world scan finds them like
//! any other ground, so they ride, grind and launch as they look.

use bevy::math::Vec3;
use skate_park::{Dropper, Event, Pad, ParkObject};

use super::natives as n;
use crate::coords::joaat;

/// The objects on offer: their names and the GTA models that make them
/// (the first one this game has).
const OBJECTS: [(&str, &[&str]); 12] = [
    ("Kicker", &["prop_skate_kickers"]),
    ("Launch ramp", &["prop_mp_ramp_01", "prop_skate_flatramp"]),
    ("Quarter pipe", &["prop_skate_quartpipe"]),
    ("Half pipe", &["prop_skate_halfpipe"]),
    ("Spine", &["prop_skate_spiner"]),
    ("Funbox", &["prop_skate_funbox"]),
    ("Flat ramp", &["prop_skate_flatramp"]),
    ("Rail", &["prop_skate_rail"]),
    ("Bench (ledge)", &["prop_bench_01a", "prop_bench_02"]),
    ("Picnic table", &["prop_picnictable_01"]),
    ("Crate", &["prop_box_wood02a", "prop_box_wood01a"]),
    ("Stunt ramp", &["prop_mp_ramp_02", "prop_mp_ramp_03"]),
];

/// What the dropper shows on the HUD.
pub struct View {
    pub open: bool,
    pub object: String,
    pub previous: String,
    pub next: String,
    pub placed: usize,
    pub max: usize,
}

pub struct GtaDropper {
    dropper: Dropper,
    /// The objects this game has: name and model.
    available: Option<Vec<(&'static str, u32)>>,
    /// The ghost: its object, kind and model.
    ghost: Option<(i32, usize)>,
    /// Dropped objects and their GTA objects.
    spawned: Vec<(ParkObject, i32)>,
}

impl Default for GtaDropper {
    fn default() -> Self {
        Self { dropper: Dropper::default(), available: None, ghost: None, spawned: Vec::new() }
    }
}

/// Seconds a model may take to load before it is given up on.
const LOAD_WAIT: std::time::Duration = std::time::Duration::from_millis(1500);

pub(super) fn spawn(model: u32, at: Vec3, heading: f32, ghost: bool) -> i32 {
    n::request_model(model);
    let deadline = std::time::Instant::now() + LOAD_WAIT;
    while !n::model_loaded(model) && std::time::Instant::now() < deadline {
        super::shv::wait(0);
    }
    if !n::model_loaded(model) {
        return 0;
    }
    let object = n::create_object(model, at);
    n::release_model(model);
    if object != 0 {
        n::set_heading(object, heading);
        n::keep_entity(object);
        if ghost {
            n::set_collision(object, false);
            n::set_alpha(object, 140);
        } else {
            n::freeze(object, true);
        }
    }
    object
}

impl GtaDropper {
    fn available(&mut self) -> &[(&'static str, u32)] {
        self.available.get_or_insert_with(|| {
            OBJECTS
                .iter()
                .filter_map(|(name, models)| {
                    models.iter().map(|m| joaat(m)).find(|hash| n::model_in_cdimage(*hash)).map(|hash| (*name, hash))
                })
                .collect()
        })
    }

    pub fn open(&self) -> bool {
        self.dropper.open
    }

    /// The objects dropped (GTA metres).
    pub fn placed(&self) -> &[ParkObject] {
        &self.dropper.placed
    }

    /// The GTA model of object `kind`, when this game has it.
    pub fn model(&mut self, kind: usize) -> Option<u32> {
        self.available().get(kind).map(|(_, model)| *model)
    }

    /// Opens or closes the dropper ahead of the skater at `feet`.
    pub fn toggle(&mut self, feet: Vec3, camera_forward: Vec3) {
        if self.dropper.open {
            self.close();
            return;
        }
        let count = self.available().len();
        if count == 0 {
            n::notify("Skate 3: this game has none of the dropper's objects.");
            return;
        }
        self.dropper.kinds = count;
        self.dropper.kind = self.dropper.kind.min(count - 1);
        self.dropper.open(feet.to_array(), heading_of(camera_forward));
    }

    /// Closes the dropper; the ghost goes.
    pub fn close(&mut self) {
        self.dropper.open = false;
        if let Some((mut ghost, _)) = self.ghost.take() {
            n::delete_object(&mut ghost);
        }
    }

    /// One frame of the dropper; true when the dropped objects changed.
    pub fn tick(&mut self, pad: &skate_input::Pad, camera_forward: Vec3, feet: Vec3, dt: f32) -> bool {
        if !self.dropper.open {
            return false;
        }
        let pad = Pad {
            buttons: pad.buttons,
            left: pad.left,
            right: pad.right,
            triggers: pad.triggers.map(|t| f32::from(t) / 255.),
        };
        let mut changed = false;
        for event in self.dropper.update(pad, heading_of(camera_forward), feet.to_array(), dt) {
            match event {
                Event::Changed => changed = true,
                Event::Full => n::notify("Skate 3: that's the most objects; pick one up first (X)."),
                Event::Closed => self.close(),
                Event::Opened => {}
            }
        }
        if changed {
            self.sync();
        }
        self.place_ghost();
        changed
    }

    /// The ghost where the dropper has it, the kind it shows.
    fn place_ghost(&mut self) {
        if !self.dropper.open {
            return;
        }
        let kind = self.dropper.kind;
        let Some(&(_, model)) = self.available().get(kind) else { return };
        let ghost = self.dropper.ghost();
        let at = Vec3::from_array(ghost.at);
        let heading = ghost.yaw.to_degrees();
        match self.ghost {
            Some((object, shown)) if shown == kind && n::entity_exists(object) => {
                n::set_coords_no_offset(object, at);
                n::set_heading(object, heading);
            }
            _ => {
                if let Some((mut old, _)) = self.ghost.take() {
                    n::delete_object(&mut old);
                }
                let object = spawn(model, at, heading, true);
                if object != 0 {
                    self.ghost = Some((object, kind));
                }
            }
        }
    }

    /// The GTA objects for what the dropper holds: new ones made, picked
    /// up ones removed.
    fn sync(&mut self) {
        let wanted = self.dropper.placed.clone();
        let mut kept = Vec::new();
        for (object, mut handle) in std::mem::take(&mut self.spawned) {
            if wanted.contains(&object) && n::entity_exists(handle) {
                kept.push((object, handle));
            } else {
                n::delete_object(&mut handle);
            }
        }
        for object in wanted {
            if kept.iter().any(|(o, _)| *o == object) {
                continue;
            }
            let Some(&(_, model)) = self.available().get(usize::from(object.kind)) else { continue };
            let handle = spawn(model, Vec3::from_array(object.at), object.yaw.to_degrees(), false);
            if handle != 0 {
                if self.dropper.lift.abs() < 0.01 {
                    n::place_on_ground(handle);
                }
                kept.push((object, handle));
            }
        }
        self.spawned = kept;
    }

    pub fn view(&mut self) -> View {
        let open = self.dropper.open;
        let kind = self.dropper.kind;
        let placed = self.dropper.placed.len();
        let available = self.available();
        let n = available.len().max(1);
        let name = |i: usize| available.get(i % n).map_or("", |(name, _)| name).to_owned();
        let object = name(kind);
        let previous = name(kind + n - 1);
        let next = name(kind + 1);
        View { open, object, previous, next, placed, max: skate_park::MAX_OBJECTS }
    }
}

/// The dropper's heading (radians, 0 along GTA's +Y, turning left) of a
/// camera looking along `forward`.
fn heading_of(forward: Vec3) -> f32 {
    (-forward.x).atan2(forward.y)
}
