//! Skate 3's damage model for the skater (its Hall of Meat), from Skate's
//! own tables (`scoring_wipeouts/default` in `db.big`: `BoneSettings`,
//! `DamageScalar`). Skate measures each bone's knock in a wipeout; the
//! damage goes to that bone and, a share of it, to its neighbours (a hand's
//! knock reaches the forearm). Each bone has six levels, Scrape to
//! BreakAll, each at its own damage and worth its own points; a level a bone
//! cannot have is left out (a skull scrapes, bruises and breaks).
//!
//! Here the knock is a bone's sudden change of speed in metres per second,
//! taken from Skate's pose frame to frame.

/// The levels, mildest first (Skate's `EDamageLevel`).
pub const LEVELS: [&str; 6] = ["Scrape", "Bruise", "Sprain", "Dislocate", "BreakPart", "BreakAll"];

/// Skate's own scale on all damage.
const DAMAGE_SCALAR: f32 = 0.75;

/// A knock smaller than this (m/s) is the animation's own movement; only
/// what is over it hurts (summed frame by frame, the whole knock made
/// ankles BreakAll in plain bails).
const KNOCK: f32 = 3.;

/// One frame's knock counts at most this (m/s): more is a jump in the pose,
/// not a fall.
const MAX_KNOCK: f32 = 20.;

/// A bone: its name in Skate's skeleton (`EBoneIndex`), the bones its damage
/// spreads to with their share, and its levels (points, damage; 0 = none).
pub struct Bone {
    pub name: &'static str,
    pub label: &'static str,
    spread: &'static [(usize, f32)],
    levels: [(u32, f32); 6],
}

const fn lv(a: (u32, f32), b: (u32, f32), c: (u32, f32), d: (u32, f32), e: (u32, f32), f: (u32, f32)) -> [(u32, f32); 6] {
    [a, b, c, d, e, f]
}
const NO: (u32, f32) = (0, 0.);

pub const BONES: [Bone; 25] = [
    Bone { name: "NECK1", label: "skull", spread: &[(23, 0.), (1, 0.5)], levels: lv((500, 7.), (1000, 14.), NO, NO, (5000, 21.), NO) },
    Bone { name: "NECK", label: "neck", spread: &[(0, 0.125), (10, 0.5)], levels: lv(NO, NO, NO, NO, NO, NO) },
    Bone { name: "LEFTHAND", label: "left hand", spread: &[(3, 0.25)], levels: lv((100, 6.), (250, 9.), (500, 12.), (750, 15.), NO, (1000, 18.)) },
    Bone { name: "LEFTFOREARM", label: "left forearm", spread: &[(4, 0.25), (2, 0.125)], levels: lv((100, 9.), (250, 12.), (500, 15.), NO, (1000, 21.), NO) },
    Bone { name: "LEFTARM", label: "left upper arm", spread: &[(5, 0.25), (3, 0.125)], levels: lv((100, 9.), (250, 12.), (500, 15.), (750, 18.), (1000, 21.), NO) },
    Bone { name: "LEFTSHOULDER", label: "left shoulder", spread: &[(4, 0.125), (10, 0.25)], levels: lv((100, 9.), (250, 12.), (500, 15.), (750, 18.), (1000, 21.), NO) },
    Bone { name: "RIGHTHAND", label: "right hand", spread: &[(7, 0.25)], levels: lv((100, 6.), (250, 9.), (500, 12.), (750, 15.), NO, (1000, 18.)) },
    Bone { name: "RIGHTFOREARM", label: "right forearm", spread: &[(8, 0.25), (6, 0.125)], levels: lv((100, 9.), (250, 12.), (500, 15.), NO, (1000, 21.), NO) },
    Bone { name: "RIGHTARM", label: "right upper arm", spread: &[(9, 0.25), (7, 0.125)], levels: lv((100, 9.), (250, 12.), (500, 15.), (750, 18.), (1000, 21.), NO) },
    Bone { name: "RIGHTSHOULDER", label: "right shoulder", spread: &[(10, 0.25), (8, 0.125)], levels: lv((100, 9.), (250, 12.), (500, 15.), (750, 18.), (1000, 21.), NO) },
    Bone { name: "SPINE3", label: "chest", spread: &[(5, 0.25), (9, 0.25), (1, 0.125), (11, 0.5)], levels: lv(NO, NO, NO, NO, NO, NO) },
    Bone { name: "SPINE2", label: "ribs", spread: &[(10, 0.), (12, 0.25)], levels: lv((250, 7.), (500, 10.), NO, NO, (1000, 15.), (2000, 25.)) },
    Bone { name: "SPINE1", label: "upper back", spread: &[(11, 0.25), (13, 0.25)], levels: lv((250, 7.), (500, 10.), NO, NO, (1000, 15.), (2000, 25.)) },
    Bone { name: "SPINE", label: "lower back", spread: &[(12, 0.25), (22, 0.25)], levels: lv((250, 7.), (500, 10.), NO, NO, (1000, 15.), (2000, 25.)) },
    Bone { name: "LEFTTOEBASE", label: "left toes", spread: &[(15, 0.25)], levels: lv((100, 7.), (250, 10.), NO, (500, 13.), (750, 16.), (1000, 19.)) },
    Bone { name: "LEFTFOOT", label: "left ankle", spread: &[(14, 0.125), (16, 0.25)], levels: lv((100, 7.), (250, 10.), NO, (500, 16.), (750, 19.), (1000, 22.)) },
    Bone { name: "LEFTLEG", label: "left shin", spread: &[(17, 0.25), (15, 0.125)], levels: lv((100, 10.), (250, 13.), (500, 16.), (750, 19.), (1000, 22.), NO) },
    Bone { name: "LEFTUPLEG", label: "left thigh", spread: &[(22, 0.25), (16, 0.125)], levels: lv((100, 10.), (250, 13.), (500, 16.), (750, 19.), (1000, 22.), NO) },
    Bone { name: "RIGHTTOEBASE", label: "right toes", spread: &[(19, 0.25)], levels: lv((100, 7.), (250, 10.), NO, (500, 13.), (750, 16.), (1000, 19.)) },
    Bone { name: "RIGHTFOOT", label: "right ankle", spread: &[(18, 0.125), (20, 0.25)], levels: lv((100, 7.), (250, 10.), (500, 13.), (750, 16.), NO, (1000, 22.)) },
    Bone { name: "RIGHTLEG", label: "right shin", spread: &[(21, 0.25), (19, 0.125)], levels: lv((100, 10.), (250, 13.), (500, 16.), (750, 19.), (1000, 22.), NO) },
    Bone { name: "RIGHTUPLEG", label: "right thigh", spread: &[(22, 0.25), (20, 0.125)], levels: lv((100, 10.), (250, 16.), (500, 16.), (750, 19.), (1000, 22.), NO) },
    Bone { name: "HIPS", label: "pelvis", spread: &[(17, 0.25), (21, 0.25), (13, 0.25), (24, 0.)], levels: lv((250, 7.), (500, 10.), NO, (750, 16.), (1000, 19.), (2000, 25.)) },
    Bone { name: "FACE", label: "face", spread: &[(0, 1.)], levels: lv((500, 6.), (1000, 12.), NO, NO, (5000, 20.), NO) },
    Bone { name: "GROIN", label: "groin", spread: &[(22, 1.)], levels: lv((1000, 3.), (2500, 5.), (5000, 7.), NO, (10000, 14.), NO) },
];

/// One wipeout's injuries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Injuries {
    damage: [f32; 25],
}

impl Injuries {
    /// A knock of `knock` m/s on bone `bone`: Skate's damage there and its
    /// share on the neighbours.
    pub fn knock(&mut self, bone: usize, knock: f32) {
        if knock < KNOCK || bone >= BONES.len() {
            return;
        }
        let damage = (knock - KNOCK) * DAMAGE_SCALAR;
        self.damage[bone] += damage;
        for &(other, share) in BONES[bone].spread {
            self.damage[other] += damage * share;
        }
    }

    /// The level a bone reached (index into [`LEVELS`]).
    pub fn level(&self, bone: usize) -> Option<usize> {
        BONES[bone].levels.iter().enumerate().filter(|(_, (_, at))| *at > 0. && self.damage[bone] >= *at).map(|(i, _)| i).last()
    }

    /// Skate's points for the wipeout: each bone's reached level.
    pub fn points(&self) -> u32 {
        (0..BONES.len()).filter_map(|b| self.level(b).map(|l| BONES[b].levels[l].0)).sum()
    }

    /// The worst injuries, worst first: (bone label, level name).
    pub fn worst(&self, count: usize) -> Vec<(&'static str, &'static str)> {
        let mut hurt: Vec<(usize, usize)> = (0..BONES.len()).filter_map(|b| self.level(b).map(|l| (l, b))).collect();
        hurt.sort_by(|a, b| b.cmp(a));
        hurt.into_iter().take(count).map(|(l, b)| (BONES[b].label, LEVELS[l])).collect()
    }

    /// A leg broken (BreakPart or worse on shin, thigh or ankle).
    pub fn leg_broken(&self) -> bool {
        [15, 16, 17, 19, 20, 21].iter().any(|&b| self.level(b).is_some_and(|l| l >= 4))
    }

    pub fn any(&self) -> bool {
        (0..BONES.len()).any(|b| self.level(b).is_some())
    }
}

/// Follows the bones through a wipeout: their speeds frame to frame, and
/// the knocks where a speed changes suddenly.
#[derive(Default)]
pub struct Tracker {
    /// Skate skeleton index per Hall of Meat bone.
    map: Vec<Option<usize>>,
    last: Vec<Option<[f32; 3]>>,
    speed: Vec<Option<[f32; 3]>>,
    pub injuries: Injuries,
}

impl Tracker {
    /// For a skeleton with these bone names.
    pub fn new(names: &[String]) -> Self {
        let map = BONES.iter().map(|b| names.iter().position(|n| n.eq_ignore_ascii_case(b.name))).collect();
        Self { map, last: vec![None; 25], speed: vec![None; 25], injuries: Injuries::default() }
    }

    /// One frame of a wipeout: the bones' positions (Skate's skeleton, in
    /// metres) after `dt` seconds.
    pub fn frame(&mut self, positions: &[[f32; 3]], dt: f32) {
        if dt <= 0. {
            return;
        }
        // The whole body jumped (a reset, a retry): no knock, start over.
        let moved: Vec<f32> = (0..BONES.len())
            .filter_map(|b| {
                let at = self.map.get(b).copied().flatten().and_then(|i| positions.get(i))?;
                let last = self.last[b]?;
                Some(((at[0] - last[0]).powi(2) + (at[1] - last[1]).powi(2) + (at[2] - last[2]).powi(2)).sqrt())
            })
            .collect();
        if !moved.is_empty() && moved.iter().sum::<f32>() / moved.len() as f32 > 30. * dt.max(1. / 60.) {
            self.last.iter_mut().for_each(|l| *l = None);
            self.speed.iter_mut().for_each(|s| *s = None);
        }
        for b in 0..BONES.len() {
            let Some(at) = self.map.get(b).copied().flatten().and_then(|i| positions.get(i)).copied() else { continue };
            if let Some(last) = self.last[b] {
                let v = [(at[0] - last[0]) / dt, (at[1] - last[1]) / dt, (at[2] - last[2]) / dt];
                if let Some(u) = self.speed[b] {
                    let change = ((v[0] - u[0]).powi(2) + (v[1] - u[1]).powi(2) + (v[2] - u[2]).powi(2)).sqrt();
                    self.injuries.knock(b, change.min(MAX_KNOCK));
                }
                self.speed[b] = Some(v);
            }
            self.last[b] = Some(at);
        }
    }

    /// The wipeout is over: its injuries, and a fresh start.
    pub fn finish(&mut self) -> Injuries {
        self.last.iter_mut().for_each(|l| *l = None);
        self.speed.iter_mut().for_each(|s| *s = None);
        std::mem::take(&mut self.injuries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hard_knock_breaks_the_bone_and_hurts_its_neighbours() {
        let mut hurt = Injuries::default();
        hurt.knock(16, 4.); // left shin: no more than the animation, nothing
        assert!(!hurt.any());
        hurt.knock(16, 33.); // (33 - 3) × 0.75 = 22.5, past BreakPart (22)
        assert_eq!(hurt.level(16), Some(4));
        assert!(hurt.leg_broken());
        // A quarter reached the thigh (5.6): not enough for a scrape (10).
        assert_eq!(hurt.level(17), None);
        assert_eq!(hurt.worst(1), [("left shin", "BreakPart")]);
        // The ankle got an eighth (2.8): no scrape either.
        assert_eq!(hurt.points(), 1000);
    }

    #[test]
    fn the_skull_skips_the_levels_it_cannot_have() {
        let mut hurt = Injuries::default();
        hurt.knock(0, 23.); // 15 damage: bruise (14), not BreakPart (21)
        assert_eq!(hurt.level(0), Some(1));
        hurt.knock(0, 11.); // 6 more: 21
        assert_eq!(hurt.level(0), Some(4));
    }

    #[test]
    fn a_falling_bone_that_stops_dead_is_knocked() {
        let names: Vec<String> = BONES.iter().map(|b| b.name.to_string()).collect();
        let mut t = Tracker::new(&names);
        let mut at = vec![[0., 0., 5.]; 25];
        for _ in 0..10 {
            for p in &mut at {
                p[2] -= 0.4; // 12 m/s down at 30 fps
            }
            t.frame(&at, 1. / 30.);
        }
        // Stops dead: a 12 m/s knock on every bone, 6.75 damage each.
        t.frame(&at, 1. / 30.);
        let hurt = t.finish();
        assert!(hurt.any());
        assert_eq!(hurt.level(0), Some(1), "skull bruised: 6.75 damage, the face's 6.75 and a little of the neck's");
    }

    #[test]
    fn a_jump_of_the_whole_body_is_no_knock() {
        let names: Vec<String> = BONES.iter().map(|b| b.name.to_string()).collect();
        let mut t = Tracker::new(&names);
        let mut at = vec![[0., 0., 1.]; 25];
        for _ in 0..5 {
            t.frame(&at, 1. / 60.);
        }
        // Put back 5 m away (a retry), then still.
        for p in &mut at {
            p[0] += 5.;
        }
        for _ in 0..5 {
            t.frame(&at, 1. / 60.);
        }
        assert!(!t.finish().any());
    }
}
