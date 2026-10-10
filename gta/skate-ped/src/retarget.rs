//! Rebuilds Skate 3 meshes on the GTA V ped skeleton.
//!
//! The skater is turned Z-up, faced along +Y and scaled to the ped's height
//! with its hips on the ped's pelvis. Then every body bone is moved rigidly
//! onto its GTA counterpart: its pivot onto the GTA joint and its direction
//! (towards its child) onto the GTA bone's, so limbs held differently in the
//! two bind poses (a T pose and an A pose) line up. A vertex follows its
//! bones' moves blended by its weights and is skinned to the GTA bones.

use std::collections::HashMap;

use glam::{Quat, Vec3};

use crate::skate::{Rig, Vertex};
use crate::skeleton::Skeleton;

/// Skate bone, GTA bone, and the child pair that gives each its direction.
const BODY: [(&str, &str, Option<(&str, &str)>); 22] = [
    ("HIPS", "SKEL_Pelvis", None),
    ("SPINE", "SKEL_Spine0", Some(("SPINE1", "SKEL_Spine1"))),
    ("SPINE1", "SKEL_Spine1", Some(("SPINE3", "SKEL_Spine3"))),
    ("SPINE3", "SKEL_Spine3", Some(("NECK", "SKEL_Neck_1"))),
    ("NECK", "SKEL_Neck_1", Some(("HEAD", "SKEL_Head"))),
    ("HEAD", "SKEL_Head", None),
    ("LEFTSHOULDER", "SKEL_L_Clavicle", Some(("LEFTARM", "SKEL_L_UpperArm"))),
    ("LEFTARM", "SKEL_L_UpperArm", Some(("LEFTFOREARM", "SKEL_L_Forearm"))),
    ("LEFTFOREARM", "SKEL_L_Forearm", Some(("LEFTHAND", "SKEL_L_Hand"))),
    ("LEFTHAND", "SKEL_L_Hand", None),
    ("RIGHTSHOULDER", "SKEL_R_Clavicle", Some(("RIGHTARM", "SKEL_R_UpperArm"))),
    ("RIGHTARM", "SKEL_R_UpperArm", Some(("RIGHTFOREARM", "SKEL_R_Forearm"))),
    ("RIGHTFOREARM", "SKEL_R_Forearm", Some(("RIGHTHAND", "SKEL_R_Hand"))),
    ("RIGHTHAND", "SKEL_R_Hand", None),
    ("LEFTUPLEG", "SKEL_L_Thigh", Some(("LEFTLEG", "SKEL_L_Calf"))),
    ("LEFTLEG", "SKEL_L_Calf", Some(("LEFTFOOT", "SKEL_L_Foot"))),
    ("LEFTFOOT", "SKEL_L_Foot", Some(("LEFTTOEBASE", "SKEL_L_Toe0"))),
    ("LEFTTOEBASE", "SKEL_L_Toe0", None),
    ("RIGHTUPLEG", "SKEL_R_Thigh", Some(("RIGHTLEG", "SKEL_R_Calf"))),
    ("RIGHTLEG", "SKEL_R_Calf", Some(("RIGHTFOOT", "SKEL_R_Foot"))),
    ("RIGHTFOOT", "SKEL_R_Foot", Some(("RIGHTTOEBASE", "SKEL_R_Toe0"))),
    ("RIGHTTOEBASE", "SKEL_R_Toe0", None),
];

/// GTA's finger bones for a Skate finger joint's name (THUMB/INDEX/MIDDLE/
/// RING/PINKY and its 1-3 segment), on the hand's side.
fn finger(name: &str) -> Option<String> {
    let upper = name.to_ascii_uppercase();
    let side = if upper.starts_with("LEFT") { 'L' } else if upper.starts_with("RIGHT") { 'R' } else { return None };
    let digit = [("THUMB", 0), ("INDEX", 1), ("MIDDLE", 2), ("RING", 3), ("PINKY", 4)]
        .iter()
        .find(|(word, _)| upper.contains(word))?
        .1;
    let segment = upper.chars().rev().find(|c| c.is_ascii_digit()).and_then(|c| c.to_digit(10)).unwrap_or(1).clamp(1, 3) - 1;
    Some(format!("SKEL_{side}_Finger{digit}{segment}"))
}

/// One Skate bone's move onto the GTA skeleton.
#[derive(Clone, Copy, Debug)]
struct Move {
    gta: usize,
    from: Vec3,
    to: Vec3,
    rotation: Quat,
}

pub struct Retarget {
    /// Skate space to the GTA frame: Z up, facing +Y, scaled, hips on the
    /// pelvis.
    turn: Quat,
    scale: f32,
    offset: Vec3,
    moves: HashMap<String, Move>,
    rig: Rig,
}

/// Skate's Y-up axes to Z-up ones.
fn z_up(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

impl Retarget {
    pub fn new(rig: &Rig, skeleton: &Skeleton) -> Result<Self, String> {
        let s = |name: &str| rig.position(name).map(z_up).ok_or_else(|| format!("the skater has no {name} joint"));
        let g = |name: &str| skeleton.position(name).ok_or_else(|| format!("the ped skeleton has no {name}"));
        // Face the ped's way: the hips' left-to-right line onto the ped's.
        let s_side = s("RIGHTUPLEG")? - s("LEFTUPLEG")?;
        let g_side = g("SKEL_R_Thigh")? - g("SKEL_L_Thigh")?;
        let yaw = g_side.y.atan2(g_side.x) - s_side.y.atan2(s_side.x);
        let turn = Quat::from_rotation_z(yaw);
        let s_height = s("HEAD")?.z - s("LEFTFOOT")?.z;
        let g_height = g("SKEL_Head")?.z - g("SKEL_L_Foot")?.z;
        if s_height <= 0.01 || g_height <= 0.01 {
            return Err("cannot measure the skeletons' heights".into());
        }
        let scale = g_height / s_height;
        let offset = g("SKEL_Pelvis")? - turn * (s("HIPS")? * scale);
        let mut retarget = Self { turn, scale, offset, moves: HashMap::new(), rig: rig.clone() };
        let mut add = |skate: &str, gta: &str, pair: Option<(&str, &str)>, parent: Option<Quat>| -> Option<Quat> {
            let gi = skeleton.find(gta)?;
            let from = retarget.place(rig.position(skate)?);
            let to = skeleton.bones[gi].position;
            let rotation = match pair {
                Some((sc, gc)) => {
                    let sd = (retarget.place(rig.position(sc)?) - from).try_normalize();
                    let gd = (skeleton.position(gc)? - to).try_normalize();
                    match (sd, gd) {
                        (Some(a), Some(b)) => Quat::from_rotation_arc(a, b),
                        _ => parent.unwrap_or(Quat::IDENTITY),
                    }
                }
                None => parent.unwrap_or(Quat::IDENTITY),
            };
            retarget.moves.insert(skate.to_owned(), Move { gta: gi, from, to, rotation });
            Some(rotation)
        };
        let mut rotations: HashMap<&str, Quat> = HashMap::new();
        for (skate, gta, pair) in BODY {
            // A terminal bone keeps its parent's turn.
            let parent = rig
                .joints
                .get(skate)
                .and_then(|j| j.parent.as_deref())
                .and_then(|p| rotations.get(p).copied());
            if let Some(r) = add(skate, gta, pair, parent) {
                rotations.insert(skate, r);
            }
        }
        if !retarget.moves.contains_key("HIPS") {
            return Err("the ped skeleton has no SKEL_Pelvis".into());
        }
        // Fingers, when both skeletons have them, follow their hand's turn.
        let names: Vec<String> = rig.joints.keys().cloned().collect();
        for name in names {
            if retarget.moves.contains_key(&name) {
                continue;
            }
            let Some(gta) = finger(&name) else { continue };
            let Some(gi) = skeleton.find(&gta) else { continue };
            let hand = if name.to_ascii_uppercase().starts_with("LEFT") { "LEFTHAND" } else { "RIGHTHAND" };
            let Some(hand_move) = retarget.moves.get(hand).copied() else { continue };
            let Some(at) = rig.position(&name) else { continue };
            let from = retarget.place(at);
            let to = skeleton.bones[gi].position;
            retarget.moves.insert(name, Move { gta: gi, from, to, rotation: hand_move.rotation });
        }
        Ok(retarget)
    }

    /// A Skate-space point in the GTA frame, before the per-bone moves.
    fn place(&self, p: Vec3) -> Vec3 {
        self.turn * (z_up(p) * self.scale) + self.offset
    }

    /// The move of a Skate joint: its own, or its nearest moved ancestor's.
    fn move_of(&self, joint: &str) -> Option<&Move> {
        let mut at = Some(joint.to_owned());
        let mut guard = 0;
        while let Some(name) = at {
            if let Some(m) = self.moves.get(&name) {
                return Some(m);
            }
            at = self.rig.joints.get(&name).and_then(|j| j.parent.clone());
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        self.moves.get("HIPS")
    }

    /// A Skate vertex on the GTA skeleton: position, normal, and up to four
    /// (GTA bone index, weight) summing to 1.
    pub fn vertex(&self, v: &Vertex) -> (Vec3, Vec3, Vec<(usize, f32)>) {
        let placed = self.place(v.position);
        let normal = self.turn * z_up(v.normal);
        let mut position = Vec3::ZERO;
        let mut turned = Vec3::ZERO;
        let mut weights: Vec<(usize, f32)> = Vec::new();
        for (joint, w) in &v.influences {
            let Some(m) = self.move_of(joint) else { continue };
            position += (m.to + m.rotation * (placed - m.from)) * *w;
            turned += (m.rotation * normal) * *w;
            match weights.iter_mut().find(|(b, _)| *b == m.gta) {
                Some(entry) => entry.1 += *w,
                None => weights.push((m.gta, *w)),
            }
        }
        if weights.is_empty() {
            return (placed, normal.normalize_or_zero(), vec![(self.moves["HIPS"].gta, 1.)]);
        }
        weights.sort_by(|a, b| b.1.total_cmp(&a.1));
        weights.truncate(4);
        let total: f32 = weights.iter().map(|w| w.1).sum();
        for w in &mut weights {
            w.1 /= total;
        }
        (position, turned.normalize_or(Vec3::Z), weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skate::Joint;
    use crate::skeleton::{Skeleton, TEST_SKELETON};

    /// A skater: Y up, facing +Z (left leg at +X), twice the ped's size.
    fn rig() -> Rig {
        let mut rig = Rig::default();
        let mut add = |name: &str, p: [f32; 3], parent: Option<&str>| {
            rig.joints.insert(name.into(), Joint { position: Vec3::from(p), parent: parent.map(Into::into) });
        };
        add("HIPS", [0., 2., 0.], None);
        add("LEFTUPLEG", [0.2, 2., 0.], Some("HIPS"));
        add("LEFTLEG", [0.2, 1., 0.], Some("LEFTUPLEG"));
        add("LEFTFOOT", [0.2, 0.2, 0.], Some("LEFTLEG"));
        add("RIGHTUPLEG", [-0.2, 2., 0.], Some("HIPS"));
        add("SPINE", [0., 2.4, 0.], Some("HIPS"));
        add("NECK", [0., 3.2, 0.], Some("SPINE"));
        add("HEAD", [0., 3.6, 0.], Some("NECK"));
        add("LEFTEYE", [0.05, 3.7, 0.1], Some("HEAD"));
        rig
    }

    #[test]
    fn the_skater_stands_on_the_ped_skeleton() {
        let skeleton = Skeleton::from_xml(TEST_SKELETON).unwrap();
        let r = Retarget::new(&rig(), &skeleton).unwrap();
        // Hips on the pelvis, at the ped's scale.
        let hips = Vertex { position: Vec3::new(0., 2., 0.), normal: Vec3::Y, uv: Default::default(), influences: vec![("HIPS".into(), 1.)] };
        let (p, n, w) = r.vertex(&hips);
        assert!((p - Vec3::new(0., 0., 1.)).length() < 1e-4, "{p}");
        assert!((n - Vec3::Z).length() < 1e-4, "up stays up: {n}");
        assert_eq!(w, [(skeleton.find("SKEL_Pelvis").unwrap(), 1.)]);
        // The left knee lands on the left calf bone: the ped's left is -X.
        let knee = Vertex { position: Vec3::new(0.2, 1., 0.), normal: Vec3::Z, uv: Default::default(), influences: vec![("LEFTLEG".into(), 1.)] };
        let (p, _, w) = r.vertex(&knee);
        assert!((p - skeleton.position("SKEL_L_Calf").unwrap()).length() < 0.02, "{p}");
        assert_eq!(w[0].0, skeleton.find("SKEL_L_Calf").unwrap());
        // An eye follows the head.
        let eye = Vertex { position: Vec3::new(0.05, 3.7, 0.1), normal: Vec3::Z, uv: Default::default(), influences: vec![("LEFTEYE".into(), 1.)] };
        assert_eq!(r.vertex(&eye).2[0].0, skeleton.find("SKEL_Head").unwrap());
        // Two bones' weights merge onto one GTA bone when they map to it.
        let split = Vertex {
            position: Vec3::new(0., 3.7, 0.),
            normal: Vec3::Z,
            uv: Default::default(),
            influences: vec![("HEAD".into(), 0.5), ("LEFTEYE".into(), 0.5)],
        };
        assert_eq!(r.vertex(&split).2.len(), 1);
        assert_eq!(finger("LEFTHANDINDEX2").as_deref(), Some("SKEL_L_Finger11"));
        assert_eq!(finger("RIGHTHANDTHUMB1").as_deref(), Some("SKEL_R_Finger00"));
        assert_eq!(finger("SPINE"), None);
    }
}
