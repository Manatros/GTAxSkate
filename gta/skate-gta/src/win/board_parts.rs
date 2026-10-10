//! The board as GTA models, part by part: the skate3weapon pack carries
//! every deck, truck and wheel finish of the board library as its own model
//! (`sk3_<d|t|w>_<finish>`, `skate-ped --weapon` writes them), all laid out
//! in the weapon model's frame (length along Z from the grip, deck facing
//! -X). Three of them make any board, lit and filtered by GTA: in the
//! store, in the hand and under the feet.

use super::natives as n;
use crate::coords::joaat;
use bevy::math::{Mat3, Mat4, Quat};
use skate_boards::{BoardChoice, Part};

/// The model of `part` on `choice` (unchosen: the default board's).
pub fn model_name(part: Part, choice: &BoardChoice) -> String {
    let letter = ["d", "t", "w"][part.index()];
    match choice.get(part) {
        Some(wanted) => format!("sk3_{letter}_{}", wanted.material.to_ascii_lowercase()),
        None => format!("sk3_{letter}_default"),
    }
}

/// The pack has the part models.
pub fn available() -> bool {
    n::model_valid(joaat("sk3_d_default"))
}

/// Where the parts go: placed each frame, or attached to an entity (the
/// weapon in the hand) at its origin.
#[derive(Clone, Copy, PartialEq)]
pub enum Place {
    At(Mat4),
    On(i32),
}

/// Deck, trucks, wheels, a griptape laid on the deck, and painted trucks'
/// baseplate and hanger (in place of the trucks).
const SLOTS: usize = 7;
const GRIP: usize = 3;
const TRUCK_BASE: usize = 4;
const TRUCK_HANGER: usize = 5;
/// The deck's scuffs as it wears.
const SCUFF: usize = 6;

/// What is laid over or swapped into the catalog board.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    pub grip: Option<String>,
    pub truck_base: Option<String>,
    pub truck_hanger: Option<String>,
    /// The deck's scuff stage (1 to 3), none while new.
    pub scuff: Option<u8>,
}

impl Extras {
    pub fn painted_trucks(&self) -> bool {
        self.truck_base.is_some() || self.truck_hanger.is_some()
    }
}

/// Unpainted half of a painted truck: plain silver.
const TRUCK_PLAIN: &str = "greyg";

#[derive(Default)]
pub struct Parts {
    objects: [i32; SLOTS],
    models: [u32; SLOTS],
    attached_to: [i32; SLOTS],
}

impl Parts {
    /// Shows `choice` with griptape `grip` (none: the deck's own) at
    /// `place`; true once every part is out (a part not loaded yet is asked
    /// for and shows a frame or two later).
    pub fn show(&mut self, choice: &BoardChoice, extras: &Extras, place: Place) -> bool {
        use skate_boards::paints::TruckPart;
        let mut all = true;
        let painted = extras.painted_trucks();
        for i in 0..SLOTS {
            let wanted = match i {
                GRIP => extras.grip.as_ref().map(|id| skate_boards::grips::grip_model(choice.get(Part::Deck), id)),
                TRUCK_BASE if painted => Some(TruckPart::Base.model(extras.truck_base.as_deref().unwrap_or(TRUCK_PLAIN))),
                TRUCK_HANGER if painted => Some(TruckPart::Hanger.model(extras.truck_hanger.as_deref().unwrap_or(TRUCK_PLAIN))),
                TRUCK_BASE | TRUCK_HANGER => None,
                SCUFF => extras.scuff.map(|stage| format!("sk3_s_{}_{stage}", skate_boards::grips::deck_key(choice.get(Part::Deck)))),
                _ if i == Part::Trucks.index() && painted => None,
                _ => Some(String::new()),
            };
            let hash = if i >= GRIP || wanted.is_none() {
                let Some(hash) = wanted.map(|name| joaat(&name)).filter(|h| n::model_valid(*h)) else {
                    if self.objects[i] != 0 {
                        n::delete_object(&mut self.objects[i]);
                    }
                    self.models[i] = 0;
                    self.attached_to[i] = 0;
                    continue;
                };
                hash
            } else {
                let part = Part::from_index(i).unwrap_or(Part::Deck);
                let hash = joaat(&model_name(part, choice));
                if n::model_valid(hash) { hash } else { joaat(&model_name(part, &BoardChoice::default())) }
            };
            if self.models[i] != hash || !n::entity_exists(self.objects[i]) {
                if self.objects[i] != 0 {
                    n::delete_object(&mut self.objects[i]);
                }
                self.models[i] = 0;
                self.attached_to[i] = 0;
                n::request_model(hash);
                if !n::model_loaded(hash) {
                    all = false;
                    continue;
                }
                let at = match place {
                    Place::At(m) => m.w_axis.truncate(),
                    Place::On(entity) => n::entity_coords(entity),
                };
                let object = n::create_object(hash, at);
                n::release_model(hash);
                if object == 0 {
                    all = false;
                    continue;
                }
                n::set_collision(object, false);
                self.objects[i] = object;
                self.models[i] = hash;
            }
            let object = self.objects[i];
            match place {
                Place::At(m) => {
                    if self.attached_to[i] != 0 {
                        n::detach(object);
                        self.attached_to[i] = 0;
                    }
                    n::freeze(object, true);
                    n::set_coords_no_offset(object, m.w_axis.truncate());
                    n::set_quaternion(object, Quat::from_mat3(&Mat3::from_mat4(m)).normalize());
                }
                Place::On(entity) => {
                    if self.attached_to[i] != entity {
                        n::attach(object, entity);
                        self.attached_to[i] = entity;
                    }
                }
            }
        }
        all
    }

    pub fn shown(&self) -> bool {
        self.objects.iter().any(|o| *o != 0)
    }

    pub fn clear(&mut self) {
        for object in &mut self.objects {
            if *object != 0 {
                n::delete_object(object);
            }
        }
        *self = Self::default();
    }
}
