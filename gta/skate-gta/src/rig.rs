//! The Skate skater's pose on a GTA ped's skeleton.
//!
//! The same retarget the mashup uses for its MW2 soldier: each mapped GTA
//! bone is first fitted onto the matching bone of the Skate reference
//! skater at bind pose (turned so the segment to its child lines up), then
//! follows that Skate bone's motion. Unmapped bones (fingers, face, roll
//! and helper bones) follow their nearest mapped ancestor.

use crate::coords::transform_from_skate;
use bevy_math::{Mat4, Quat, Vec3};
use serde::Deserialize;
use std::path::Path;

/// A bone of the Skate reference skater (`rig.json`), bind pose in Skate
/// space.
#[derive(Clone, Debug, Deserialize)]
pub struct ReferenceBone {
    pub name: String,
    pub bind: [f32; 16],
}

pub fn load_reference(path: &Path) -> Result<Vec<ReferenceBone>, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&data).map_err(|e| format!("{}: {e}", path.display()))
}

/// A bone of the GTA ped, bind pose in the ped's model space.
#[derive(Clone, Debug)]
pub struct GtaBone {
    pub name: String,
    pub parent: Option<usize>,
    pub bind: Mat4,
}

/// GTA bones from their parent-relative bind rotations and translations.
pub fn bind_pose(locals: &[(String, Option<usize>, Quat, Vec3)]) -> Vec<GtaBone> {
    let mut bones: Vec<GtaBone> = Vec::with_capacity(locals.len());
    for (index, (name, parent, rotation, translation)) in locals.iter().enumerate() {
        let local = Mat4::from_rotation_translation(rotation.normalize(), *translation);
        let parent = parent.filter(|&p| p < index);
        let bind = parent.map_or(local, |p| bones[p].bind * local);
        bones.push(GtaBone { name: name.clone(), parent, bind });
    }
    bones
}

/// GTA bone: its Skate bone, and the child pair that sets its direction.
fn mapping(name: &str) -> Option<(&'static str, Option<(&'static str, &'static str)>)> {
    Some(match name {
        "SKEL_ROOT" | "SKEL_Pelvis" => ("HIPS", Some(("SKEL_Spine0", "SPINE"))),
        "SKEL_Spine0" => ("SPINE", Some(("SKEL_Spine2", "SPINE1"))),
        "SKEL_Spine2" => ("SPINE1", Some(("SKEL_Spine3", "SPINE3"))),
        "SKEL_L_Clavicle" => ("LEFTSHOULDER", Some(("SKEL_L_UpperArm", "LEFTARM"))),
        "SKEL_L_UpperArm" => ("LEFTARM", Some(("SKEL_L_Forearm", "LEFTFOREARM"))),
        "SKEL_L_Forearm" => ("LEFTFOREARM", Some(("SKEL_L_Hand", "LEFTHAND"))),
        "SKEL_L_Hand" => ("LEFTHAND", None),
        "SKEL_R_Clavicle" => ("RIGHTSHOULDER", Some(("SKEL_R_UpperArm", "RIGHTARM"))),
        "SKEL_R_UpperArm" => ("RIGHTARM", Some(("SKEL_R_Forearm", "RIGHTFOREARM"))),
        "SKEL_R_Forearm" => ("RIGHTFOREARM", Some(("SKEL_R_Hand", "RIGHTHAND"))),
        "SKEL_R_Hand" => ("RIGHTHAND", None),
        "SKEL_L_Thigh" => ("LEFTUPLEG", Some(("SKEL_L_Calf", "LEFTLEG"))),
        "SKEL_L_Calf" => ("LEFTLEG", Some(("SKEL_L_Foot", "LEFTFOOT"))),
        "SKEL_L_Foot" => ("LEFTFOOT", Some(("SKEL_L_Toe0", "LEFTTOEBASE"))),
        "SKEL_L_Toe0" => ("LEFTTOEBASE", None),
        "SKEL_R_Thigh" => ("RIGHTUPLEG", Some(("SKEL_R_Calf", "RIGHTLEG"))),
        "SKEL_R_Calf" => ("RIGHTLEG", Some(("SKEL_R_Foot", "RIGHTFOOT"))),
        "SKEL_R_Foot" => ("RIGHTFOOT", Some(("SKEL_R_Toe0", "RIGHTTOEBASE"))),
        "SKEL_R_Toe0" => ("RIGHTTOEBASE", None),
        _ => return None,
    })
}

/// Bones that keep their place on the upper spine, as the mashup keeps its
/// soldier's: Skate's own upper spine, neck and head read badly on a body
/// built differently.
const RIGID: [&str; 3] = ["SKEL_Spine3", "SKEL_Neck_1", "SKEL_Head"];

/// The turn about GTA's up (radians) that lines the GTA ped's hips (left
/// thigh to right) up with the Skate skater's. Peds differ: a freemode
/// ped's bind already faces the way Skate's skater does, and a fixed 180°
/// turned every bone about its own axis (hips and chest facing backwards).
fn facing_turn(gta: &dyn Fn(&str) -> Option<Vec3>, skate: &dyn Fn(&str) -> Option<Vec3>) -> f32 {
    let across = |left: Option<Vec3>, right: Option<Vec3>| Some((right? - left?).with_z(0.)).filter(|v| v.length() > 1e-4);
    let (Some(g), Some(s)) = (
        across(gta("SKEL_L_Thigh"), gta("SKEL_R_Thigh")),
        across(skate("LEFTUPLEG"), skate("RIGHTUPLEG")),
    ) else {
        return std::f32::consts::PI;
    };
    s.y.atan2(s.x) - g.y.atan2(g.x)
}

#[derive(Clone, Copy, Debug)]
enum Plan {
    /// Follows Skate pose bone `pose`; `fit` takes the GTA bind onto the
    /// Skate reference bind.
    Mapped { pose: usize, fit: Mat4 },
    Parent(usize),
    /// A root with nothing to follow stays at the ped's origin.
    Still,
}

#[derive(Clone)]
pub struct Retarget {
    plans: Vec<Plan>,
    binds: Vec<Mat4>,
    mapped: usize,
    parents: Vec<Option<usize>>,
    /// Each bone's place in its parent's frame at bind pose: GTA's own
    /// bone lengths.
    offsets: Vec<Vec3>,
    /// The feet, which stay where Skate's are (on the board).
    feet: Vec<usize>,
    /// Forearm roll bones, their hand, and the roll's bind rotation in the
    /// hand's frame: they take half the hand's twist.
    rolls: Vec<(usize, usize, Quat)>,
    /// SK8V's retarget, when it fits this ped: used before the one here.
    better: Option<crate::ped_rig::PedRig>,
}

impl Retarget {
    /// The plan for `gta`, given the Skate reference skater and the names of
    /// the bones in Skate's poses. The GTA ped's facing is turned onto the
    /// reference skater's (found from their hips), then by `extra_degrees`.
    pub fn new(gta: &[GtaBone], reference: &[ReferenceBone], pose_names: &[String], extra_degrees: f32) -> Result<Self, String> {
        let source = |name: &str| {
            reference
                .iter()
                .find(|b| b.name == name)
                .map(|b| transform_from_skate(Mat4::from_cols_array(&b.bind), Vec3::ZERO))
        };
        let position = |name: &str| gta.iter().find(|b| b.name == name).map(|b| b.bind.w_axis.truncate());
        let facing = Quat::from_rotation_z(facing_turn(&position, &|n| source(n).map(|m| m.w_axis.truncate())) + extra_degrees.to_radians());
        let mut plans = Vec::with_capacity(gta.len());
        let mut mapped = 0;
        for bone in gta {
            let parent_plan = || bone.parent.map_or(Plan::Still, Plan::Parent);
            if RIGID.contains(&bone.name.as_str()) {
                plans.push(parent_plan());
                continue;
            }
            let plan = mapping(&bone.name).and_then(|(skate, child)| {
                let bind = source(skate)?;
                let pose = pose_names.iter().position(|n| n == skate)?;
                let from = bone.bind.w_axis.truncate();
                let to = bind.w_axis.truncate();
                let rotation = child
                    .and_then(|(gta_child, skate_child)| {
                        let g = position(gta_child)?;
                        let s = source(skate_child)?.w_axis.truncate();
                        Some(Quat::from_rotation_arc((facing * (g - from)).try_normalize()?, (s - to).try_normalize()?) * facing)
                    })
                    .or_else(|| {
                        // Ends of chains take their parent's alignment, so
                        // hands and toes stay on their wrists and feet.
                        let parent = &gta[bone.parent?];
                        let (parent_skate, _) = mapping(&parent.name)?;
                        let s = source(parent_skate)?.w_axis.truncate();
                        let p = parent.bind.w_axis.truncate();
                        Some(Quat::from_rotation_arc((facing * (from - p)).try_normalize()?, (to - s).try_normalize()?) * facing)
                    })
                    .unwrap_or(facing);
                let fit = Mat4::from_rotation_translation(rotation, to - rotation * from);
                Some(Plan::Mapped { pose, fit: bind.inverse() * fit })
            });
            if plan.is_some() {
                mapped += 1;
            }
            plans.push(plan.unwrap_or_else(parent_plan));
        }
        if mapped < 10 {
            return Err(format!("only {mapped} GTA bones matched the Skate skater; is this a human ped?"));
        }
        let index = |name: &str| gta.iter().position(|b| b.name == name);
        let rotation = |m: &Mat4| m.to_scale_rotation_translation().1.normalize();
        Ok(Self {
            plans,
            binds: gta.iter().map(|b| b.bind).collect(),
            mapped,
            parents: gta.iter().map(|b| b.parent).collect(),
            offsets: gta
                .iter()
                .map(|b| b.parent.map_or(Vec3::ZERO, |p| gta[p].bind.inverse().transform_point3(b.bind.w_axis.truncate())))
                .collect(),
            feet: ["SKEL_L_Foot", "SKEL_R_Foot"].into_iter().filter_map(index).collect(),
            rolls: [("RB_L_ForeArmRoll", "SKEL_L_Hand"), ("RB_R_ForeArmRoll", "SKEL_R_Hand")]
                .into_iter()
                .filter_map(|(roll, hand)| {
                    let (roll, hand) = (index(roll)?, index(hand)?);
                    Some((roll, hand, rotation(&gta[hand].bind).inverse() * rotation(&gta[roll].bind)))
                })
                .collect(),            better: None,
        })
    }

    /// How many GTA bones follow a Skate bone directly.
    /// Uses SK8V's retarget (`ped_rig`) for this ped when given.
    pub fn with_ped_rig(mut self, rig: Option<crate::ped_rig::PedRig>) -> Self {
        self.better = rig;
        self
    }

    /// Whether SK8V's retarget poses this ped.
    pub fn uses_ped_rig(&self) -> bool {
        self.better.is_some()
    }

    pub fn mapped(&self) -> usize {
        self.mapped
    }

    /// The Skate pose's bones (indices) this retarget reads.
    pub fn used(&self) -> Vec<usize> {
        let mut used: Vec<usize> = self
            .plans
            .iter()
            .filter_map(|plan| match plan {
                Plan::Mapped { pose, .. } => Some(*pose),
                _ => None,
            })
            .collect();
        if let Some(better) = &self.better {
            used.extend(better.used());
        }
        used.sort_unstable();
        used.dedup();
        used
    }

    /// GTA world matrices for every bone, from a Skate pose (Skate space,
    /// Skate's origin being GTA point `origin`). `still` places bones that
    /// follow nothing.
    pub fn pose(&self, skate: &[Mat4], origin: Vec3, still: Mat4) -> Vec<Mat4> {
        if let Some(world) = self.better.as_ref().and_then(|rig| rig.pose(skate, origin)) {
            return world;
        }
        let mut skins: Vec<Mat4> = Vec::with_capacity(self.plans.len());
        for plan in &self.plans {
            let skin = match *plan {
                Plan::Mapped { pose, fit } => skate.get(pose).map_or(still, |m| transform_from_skate(*m, origin) * fit),
                Plan::Parent(p) => skins.get(p).copied().unwrap_or(still),
                Plan::Still => still,
            };
            skins.push(skin);
        }
        let mut world: Vec<Mat4> = skins.iter().zip(&self.binds).map(|(skin, bind)| *skin * *bind).collect();
        self.keep_gta_proportions(&mut world);
        world
    }

    /// Takes only the turn of each bone from Skate's pose and builds the
    /// places again from GTA's own bone lengths, then sets the feet where
    /// Skate's are. Placed on Skate's joints, the mapped bones had Skate's
    /// proportions while the bones between them kept GTA's: shoulders
    /// hunched off the neck and the skin stretched at the waist.
    fn keep_gta_proportions(&self, world: &mut [Mat4]) {
        let feet = |world: &[Mat4]| {
            let sum: Vec3 = self.feet.iter().filter_map(|&f| world.get(f)).map(|m| m.w_axis.truncate()).sum();
            sum / self.feet.len().max(1) as f32
        };
        let skates_feet = feet(world);
        for i in 0..world.len() {
            let Some(p) = self.parents.get(i).copied().flatten().filter(|&p| p < i) else { continue };
            let (_, turn, _) = world[i].to_scale_rotation_translation();
            let (_, parent_turn, parent_at) = world[p].to_scale_rotation_translation();
            let at = parent_at + parent_turn * self.offsets[i];
            world[i] = Mat4::from_rotation_translation(turn.normalize(), at);
        }
        for &(roll, hand, in_hand) in &self.rolls {
            let (_, turn, at) = world[roll].to_scale_rotation_translation();
            let with_hand = world[hand].to_scale_rotation_translation().1 * in_hand;
            world[roll] = Mat4::from_rotation_translation(turn.slerp(with_hand, 0.5).normalize(), at);
        }
        if !self.feet.is_empty() {
            let shift = Mat4::from_translation(skates_feet - feet(world));
            for m in world.iter_mut() {
                *m = shift * *m;
            }
        }
    }
}

/// One bone `t` of the way from `a` to `b`. A bone GTA was not posing
/// (hair and face bones can hold a zero or stale matrix) is no place to
/// blend from: the head and hair burst apart for a moment as skating began.
/// Such a bone, or one more than a metre from where it goes, is `b` at once.
pub fn blend_bone(a: &Mat4, b: &Mat4, t: f32) -> Mat4 {
    let usable = a.is_finite() && (a.determinant() - 1.).abs() < 0.2 && a.w_axis.truncate().distance(b.w_axis.truncate()) < 1.;
    if !usable || t >= 1. {
        return *b;
    }
    let (sa, ra, ta) = a.to_scale_rotation_translation();
    let (sb, rb, tb) = b.to_scale_rotation_translation();
    Mat4::from_scale_rotation_translation(sa.lerp(sb, t), ra.slerp(rb, t), ta.lerp(tb, t))
}

#[cfg(test)]
mod tests {
    #[test]
    fn blending_skips_bones_gta_was_not_posing() {
        let to = Mat4::from_translation(Vec3::new(0., 0., 1.7));
        assert_eq!(blend_bone(&Mat4::ZERO, &to, 0.3), to, "a zero matrix");
        let far = Mat4::from_translation(Vec3::new(5., 0., 0.));
        assert_eq!(blend_bone(&far, &to, 0.3), to, "a stale one far away");
        let near = Mat4::from_translation(Vec3::new(0., 0., 1.5));
        let half = blend_bone(&near, &to, 0.5);
        assert!((half.w_axis.z - 1.6).abs() < 1e-5, "a real one blends");
    }

    use super::*;
    use crate::coords::{basis, to_skate};

    /// A small GTA-like skeleton, facing +Y, Z up, in metres.
    fn gta() -> Vec<GtaBone> {
        let locals: Vec<(&str, Option<usize>, Vec3)> = vec![
            ("SKEL_ROOT", None, Vec3::ZERO),
            ("SKEL_Pelvis", Some(0), Vec3::new(0., 0., 0.)),
            ("SKEL_Spine0", Some(1), Vec3::new(0., 0., 0.1)),
            ("SKEL_Spine2", Some(2), Vec3::new(0., 0., 0.2)),
            ("SKEL_Spine3", Some(3), Vec3::new(0., 0., 0.15)),
            ("SKEL_Head", Some(4), Vec3::new(0., 0., 0.3)),
            ("SKEL_L_Clavicle", Some(4), Vec3::new(-0.05, 0., 0.1)),
            ("SKEL_L_UpperArm", Some(6), Vec3::new(-0.15, 0., 0.)),
            ("SKEL_L_Forearm", Some(7), Vec3::new(-0.28, 0., 0.)),
            ("SKEL_L_Hand", Some(8), Vec3::new(-0.25, 0., 0.)),
            ("SKEL_L_Finger00", Some(9), Vec3::new(-0.05, 0.02, 0.)),
            ("SKEL_L_Thigh", Some(1), Vec3::new(-0.1, 0., -0.05)),
            ("SKEL_L_Calf", Some(11), Vec3::new(0., 0., -0.45)),
            ("SKEL_L_Foot", Some(12), Vec3::new(0., 0., -0.42)),
            ("SKEL_L_Toe0", Some(13), Vec3::new(0., 0.15, -0.05)),
            ("SKEL_R_Thigh", Some(1), Vec3::new(0.1, 0., -0.05)),
            ("SKEL_R_Calf", Some(15), Vec3::new(0., 0., -0.45)),
            ("SKEL_R_Foot", Some(16), Vec3::new(0., 0., -0.42)),
        ];
        let locals: Vec<_> = locals.into_iter().map(|(n, p, t)| (n.to_owned(), p, Quat::IDENTITY, t)).collect();
        bind_pose(&locals)
    }

    /// A Skate reference skater: the same body, a bit taller, facing GTA -Y
    /// as Skate's skater does, as Skate-space binds.
    fn reference(gta: &[GtaBone]) -> Vec<ReferenceBone> {
        reference_turned(gta, std::f32::consts::PI)
    }

    /// The reference skater, turned `turn` radians from the GTA ped.
    fn reference_turned(gta: &[GtaBone], turn: f32) -> Vec<ReferenceBone> {
        let names = [
            ("SKEL_Pelvis", "HIPS"),
            ("SKEL_Spine0", "SPINE"),
            ("SKEL_Spine2", "SPINE1"),
            ("SKEL_Spine3", "SPINE3"),
            ("SKEL_L_Clavicle", "LEFTSHOULDER"),
            ("SKEL_L_UpperArm", "LEFTARM"),
            ("SKEL_L_Forearm", "LEFTFOREARM"),
            ("SKEL_L_Hand", "LEFTHAND"),
            ("SKEL_L_Thigh", "LEFTUPLEG"),
            ("SKEL_L_Calf", "LEFTLEG"),
            ("SKEL_L_Foot", "LEFTFOOT"),
            ("SKEL_L_Toe0", "LEFTTOEBASE"),
            ("SKEL_R_Thigh", "RIGHTUPLEG"),
            ("SKEL_R_Calf", "RIGHTLEG"),
            ("SKEL_R_Foot", "RIGHTFOOT"),
        ];
        let turn = Mat4::from_rotation_z(turn);
        let b = basis();
        names
            .iter()
            .map(|(g, s)| {
                let bone = gta.iter().find(|b| b.name == *g).unwrap();
                let gta_space = turn * Mat4::from_scale(Vec3::splat(1.1)) * bone.bind;
                let mut skate = b.inverse() * gta_space * b;
                skate.w_axis = to_skate(gta_space.w_axis.truncate()).extend(1.);
                ReferenceBone { name: s.to_string(), bind: skate.to_cols_array() }
            })
            .collect()
    }

    #[test]
    fn at_bind_pose_the_ped_takes_the_skaters_shape() {
        let gta = gta();
        let reference = reference(&gta);
        let names: Vec<String> = reference.iter().map(|b| b.name.clone()).collect();
        let retarget = Retarget::new(&gta, &reference, &names, 0.).unwrap();
        assert_eq!(retarget.mapped(), 15);
        let origin = Vec3::new(100., -50., 10.);
        let pose: Vec<Mat4> = reference.iter().map(|b| Mat4::from_cols_array(&b.bind)).collect();
        let world = retarget.pose(&pose, origin, Mat4::IDENTITY);
        // The skater is a tenth bigger: the ped keeps its own bone lengths
        // (no stretched skin), its joints turned as the skater's.
        for (i, bone) in gta.iter().enumerate() {
            let Some(p) = bone.parent else { continue };
            let want = bone.bind.w_axis.truncate().distance(gta[p].bind.w_axis.truncate());
            let got = world[i].w_axis.truncate().distance(world[p].w_axis.truncate());
            assert!((got - want).abs() < 1e-3, "{}: {got} vs {want}", bone.name);
        }
        let at = |name: &str| world[gta.iter().position(|b| b.name == name).unwrap()].w_axis.truncate();
        let skate_at = |name: &str| {
            let bone = reference.iter().find(|b| b.name == name).unwrap();
            transform_from_skate(Mat4::from_cols_array(&bone.bind), origin).w_axis.truncate()
        };
        // Its feet stand where the skater's do: on the board.
        let feet = (at("SKEL_L_Foot") + at("SKEL_R_Foot")) / 2.;
        let skates_feet = (skate_at("LEFTFOOT") + skate_at("RIGHTFOOT")) / 2.;
        assert!(feet.distance(skates_feet) < 1e-3, "{feet} vs {skates_feet}");
        let shin = (at("SKEL_L_Foot") - at("SKEL_L_Calf")).normalize();
        let skates_shin = (skate_at("LEFTFOOT") - skate_at("LEFTLEG")).normalize();
        assert!(shin.dot(skates_shin) > 0.999, "the shin points as the skater's");
        // The thigh-to-calf segment points the way the skater's does.
        let down = (at("SKEL_L_Calf") - at("SKEL_L_Thigh")).normalize();
        assert!(down.z < -0.99, "legs hang down: {down}");
        // A finger rides along with its hand, turned as the skater is.
        let finger = at("SKEL_L_Finger00") - at("SKEL_L_Hand");
        assert!(finger.distance(Vec3::new(0.05, -0.02, 0.)) < 1e-3, "{finger}");
    }

    #[test]
    fn a_ped_already_facing_like_the_skater_is_not_turned_round() {
        // GTA's freemode ped's bind faces the way Skate's skater does: a
        // fixed half turn showed hips and chest facing backwards.
        let gta = gta();
        let reference = reference_turned(&gta, 0.);
        let names: Vec<String> = reference.iter().map(|b| b.name.clone()).collect();
        let retarget = Retarget::new(&gta, &reference, &names, 0.).unwrap();
        let pose: Vec<Mat4> = reference.iter().map(|b| Mat4::from_cols_array(&b.bind)).collect();
        let world = retarget.pose(&pose, Vec3::ZERO, Mat4::IDENTITY);
        let at = |name: &str| world[gta.iter().position(|b| b.name == name).unwrap()];
        let finger = at("SKEL_L_Finger00").w_axis - at("SKEL_L_Hand").w_axis;
        assert!(finger.truncate().distance(Vec3::new(-0.05, 0.02, 0.)) < 1e-3, "{finger}");
        // The pelvis keeps its bind facing: its forward stays forward.
        let forward = at("SKEL_Pelvis").transform_vector3(Vec3::Y);
        assert!(forward.y > 0.99, "pelvis forward {forward}");
    }

    #[test]
    fn moving_a_skate_bone_moves_its_gta_bone() {
        let gta = gta();
        let reference = reference(&gta);
        let names: Vec<String> = reference.iter().map(|b| b.name.clone()).collect();
        let retarget = Retarget::new(&gta, &reference, &names, 0.).unwrap();
        let mut pose: Vec<Mat4> = reference.iter().map(|b| Mat4::from_cols_array(&b.bind)).collect();
        let before = retarget.pose(&pose, Vec3::ZERO, Mat4::IDENTITY);
        // The whole skater a metre up (Skate's Y): the feet lead the body.
        for m in pose.iter_mut() {
            *m = Mat4::from_translation(Vec3::new(0., 1., 0.)) * *m;
        }
        let after = retarget.pose(&pose, Vec3::ZERO, Mat4::IDENTITY);
        let pelvis = gta.iter().position(|b| b.name == "SKEL_Pelvis").unwrap();
        let moved = after[pelvis].w_axis - before[pelvis].w_axis;
        assert!((moved.truncate() - Vec3::Z).length() < 1e-4, "Skate up is GTA up: {moved}");
    }

    #[test]
    fn a_skeleton_with_nothing_in_common_is_refused() {
        let other = bind_pose(&[("root".into(), None, Quat::IDENTITY, Vec3::ZERO)]);
        assert!(Retarget::new(&other, &[], &[], 0.).is_err());
    }
}
