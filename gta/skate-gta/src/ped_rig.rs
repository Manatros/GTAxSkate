//! Skate's pose on a GTA ped: SK8V's retarget (its `ped.rs`, Apache-2.0,
//! Copyright 2026 becaring), the best one we know (B86, A31).
//!
//! Each mapped GTA bone wears its Skate joint's full rotation, carried
//! through a bind fit that swings the GTA bind segment onto Skate's. GTA
//! keeps its own bone lengths (every bone rigid): the pelvis is placed so
//! the hip joints straddle Skate's, and two-bone limb solves put the GTA
//! ankles on Skate's (and the wrists, when the hand is in contact: grabs,
//! carrying, plants), bending in the plane Skate's limb bends in. GTA's
//! expression-driven roll bones take the swing/twist split their bind
//! position along the limb implies, and shirt/belt helpers under the legs
//! follow the pelvis.
//!
//! Taken over: the fit, the solve, the limbs, the rolls and the hand
//! contact. Left for later: SK8V's shoe soles, deck and finger contact,
//! which need the worn ped's and Skate's meshes.

use crate::coords::transform_from_skate;
use crate::rig::{GtaBone, ReferenceBone};
use bevy_math::{Mat3, Mat4, Quat, Vec3};

/// GTA bone, the Skate joint it wears, and the bind segment (GTA child,
/// Skate child) the fit swings together. `None`: the end of a chain, fitted
/// like the bone named in `TERMINAL_FIT`.
type Segment = Option<(&'static str, &'static str)>;
const MAP: [(&str, &str, Segment); 23] = [
    ("SKEL_Pelvis", "HIPS", None),
    ("SKEL_Spine0", "SPINE", Some(("SKEL_Spine1", "SPINE1"))),
    ("SKEL_Spine1", "SPINE1", Some(("SKEL_Spine2", "SPINE2"))),
    ("SKEL_Spine2", "SPINE2", Some(("SKEL_Spine3", "SPINE3"))),
    ("SKEL_Spine3", "SPINE3", Some(("SKEL_Neck_1", "NECK"))),
    ("SKEL_Neck_1", "NECK", Some(("SKEL_Head", "HEAD"))),
    ("SKEL_Head", "HEAD", None),
    ("SKEL_L_Clavicle", "LEFTSHOULDER", Some(("SKEL_L_UpperArm", "LEFTARM"))),
    ("SKEL_L_UpperArm", "LEFTARM", Some(("SKEL_L_Forearm", "LEFTFOREARM"))),
    ("SKEL_L_Forearm", "LEFTFOREARM", Some(("SKEL_L_Hand", "LEFTHAND"))),
    ("SKEL_L_Hand", "LEFTHAND", None),
    ("SKEL_R_Clavicle", "RIGHTSHOULDER", Some(("SKEL_R_UpperArm", "RIGHTARM"))),
    ("SKEL_R_UpperArm", "RIGHTARM", Some(("SKEL_R_Forearm", "RIGHTFOREARM"))),
    ("SKEL_R_Forearm", "RIGHTFOREARM", Some(("SKEL_R_Hand", "RIGHTHAND"))),
    ("SKEL_R_Hand", "RIGHTHAND", None),
    ("SKEL_L_Thigh", "LEFTUPLEG", Some(("SKEL_L_Calf", "LEFTLEG"))),
    ("SKEL_L_Calf", "LEFTLEG", Some(("SKEL_L_Foot", "LEFTFOOT"))),
    ("SKEL_L_Foot", "LEFTFOOT", Some(("SKEL_L_Toe0", "LEFTTOEBASE"))),
    ("SKEL_L_Toe0", "LEFTTOEBASE", None),
    ("SKEL_R_Thigh", "RIGHTUPLEG", Some(("SKEL_R_Calf", "RIGHTLEG"))),
    ("SKEL_R_Calf", "RIGHTLEG", Some(("SKEL_R_Foot", "RIGHTFOOT"))),
    ("SKEL_R_Foot", "RIGHTFOOT", Some(("SKEL_R_Toe0", "RIGHTTOEBASE"))),
    ("SKEL_R_Toe0", "RIGHTTOEBASE", None),
];
/// Chain ends take the fit of the bone before them, which keeps wrist and
/// neck seams attached.
const TERMINAL_FIT: [(&str, &str); 3] = [("SKEL_Head", "SKEL_Neck_1"), ("SKEL_L_Hand", "SKEL_L_Forearm"), ("SKEL_R_Hand", "SKEL_R_Forearm")];
/// The ankle->toe bones slope differently (GTA 18, Skate 29 degrees):
/// feet are fitted about the vertical only, so the soles stay flat.
const GROUND_FIT: [&str; 2] = ["SKEL_L_Foot", "SKEL_R_Foot"];
/// Toes end the leg chains and follow their foot's fit.
const TOE_FIT: [(&str, &str); 2] = [("SKEL_L_Toe0", "SKEL_L_Foot"), ("SKEL_R_Toe0", "SKEL_R_Foot")];
/// Two-bone limbs whose end joint is placed on Skate's: (upper, lower, end).
const LIMBS: [(&str, &str, &str); 4] = [
    ("SKEL_L_Thigh", "SKEL_L_Calf", "SKEL_L_Foot"),
    ("SKEL_R_Thigh", "SKEL_R_Calf", "SKEL_R_Foot"),
    ("SKEL_L_UpperArm", "SKEL_L_Forearm", "SKEL_L_Hand"),
    ("SKEL_R_UpperArm", "SKEL_R_Forearm", "SKEL_R_Hand"),
];
/// Roll bones GTA drives by expression: (roll, base, twisted, carries the
/// twisted bone's swing). Rigid FK left them behind (crotch torn 3-4 cm).
const ROLLS: [(&str, &str, &str, bool); 10] = [
    ("RB_L_ThighRoll", "SKEL_Pelvis", "SKEL_L_Thigh", true),
    ("RB_R_ThighRoll", "SKEL_Pelvis", "SKEL_R_Thigh", true),
    ("RB_L_ArmRoll", "SKEL_L_Clavicle", "SKEL_L_UpperArm", true),
    ("RB_R_ArmRoll", "SKEL_R_Clavicle", "SKEL_R_UpperArm", true),
    ("RB_L_ArmRoll_Vest", "SKEL_L_Clavicle", "SKEL_L_UpperArm", true),
    ("RB_R_ArmRoll_Vest", "SKEL_R_Clavicle", "SKEL_R_UpperArm", true),
    ("RB_L_ForeArmRoll", "SKEL_L_Forearm", "SKEL_L_Hand", false),
    ("RB_R_ForeArmRoll", "SKEL_R_Forearm", "SKEL_R_Hand", false),
    ("RB_Neck", "SKEL_Spine3", "SKEL_Neck_1", true),
    ("RB_Neck_1", "SKEL_Neck_1", "SKEL_Head", false),
];
/// Beyond reach a limb's end still meets its target up to this far (m):
/// the legs differ by a few millimetres.
const STRETCH: f32 = 0.03;

/// Rotation (possibly scaled) + translation as a matrix.
fn rigid(r: Mat3, t: Vec3) -> Mat4 {
    Mat4::from_cols(r.x_axis.extend(0.0), r.y_axis.extend(0.0), r.z_axis.extend(0.0), t.extend(1.0))
}

/// Orthonormal frame with `u` first and `v` (orthogonalised) second.
fn frame(u: Vec3, v: Vec3) -> Option<Mat3> {
    let u = u.try_normalize()?;
    let w = u.cross(v).try_normalize()?;
    Some(Mat3::from_cols(u, w.cross(u), w))
}

/// The part of rotation `q` about unit `axis` (swing-twist split, `q =
/// swing * twist`), on the short arc.
fn twist(q: Quat, axis: Vec3) -> Quat {
    let p = axis * Vec3::new(q.x, q.y, q.z).dot(axis);
    let t = Quat::from_xyzw(p.x, p.y, p.z, q.w);
    let t = if t.length_squared() < 1e-12 { Quat::IDENTITY } else { t.normalize() };
    if t.w < 0.0 { -t } else { t }
}

/// A solved two-bone limb: new rotations of the upper and lower bones and
/// where the end lands.
struct Limb {
    upper: Mat3,
    lower: Mat3,
    end: Vec3,
}

/// Two-bone solve keeping both bone lengths, bending about `bend`; the end
/// follows the target up to `stretch` beyond (or inside) reach, then falls
/// short rather than tearing the mesh.
#[allow(clippy::too_many_arguments)]
fn solve_limb(hip: Vec3, knee: Vec3, ankle: Vec3, target: Vec3, bend: Vec3, upper: Mat3, lower: Mat3, stretch: f32) -> Option<Limb> {
    let (a, b) = (hip.distance(knee), knee.distance(ankle));
    let reach = target - hip;
    let distance = reach.length();
    if !(distance.is_finite() && a > 1e-6 && b > 1e-6 && distance > 1e-6) {
        return None;
    }
    let u = reach / distance;
    let d = distance.clamp((a - b).abs() + 1e-5, a + b - 1e-5);
    let along = (a * a - b * b + d * d) / (2.0 * d);
    let height = (a * a - along * along).max(0.0).sqrt();
    let side = u.cross(bend).try_normalize()?;
    let new_knee = hip + u * along + side * height;
    let end = hip + u * (d + (distance - d).clamp(-stretch, stretch));
    let turn = |from: Vec3, to: Vec3| -> Option<Mat3> { Some(frame(to, bend)? * frame(from, bend)?.transpose()) };
    Some(Limb { upper: turn(knee - hip, new_knee - hip)? * upper, lower: turn(ankle - knee, end - new_knee)? * lower, end })
}

/// A GTA roll bone and how its expression is reproduced.
#[derive(Clone, Debug)]
struct Roll {
    bone: usize,
    base: usize,
    twisted: usize,
    swing: bool,
    /// Twist share: the roll's position along its segment in bind.
    share: f32,
    /// Bind direction of that segment (GTA model space).
    axis: Vec3,
}

/// The GTA ped's rig fitted to Skate's skater, ready to pose.
#[derive(Clone, Debug)]
pub struct PedRig {
    parents: Vec<Option<usize>>,
    local: Vec<Mat4>,
    bind: Vec<Mat4>,
    order: Vec<usize>,
    /// Per source: the Skate pose bone it reads (index into the pose).
    source_pose: Vec<Option<usize>>,
    /// Per GTA bone: the source it wears.
    mapped: Vec<Option<usize>>,
    pelvis: usize,
    hip_sources: [usize; 2],
    truck_sources: [usize; 2],
    body_sources: Vec<usize>,
    limbs: Vec<[usize; 3]>,
    rolls: Vec<Roll>,
    followers: Vec<Option<(usize, Mat4)>>,
    /// Per GTA bone: Skate bind -> GTA bind rotation.
    offset: Vec<Option<Mat3>>,
    /// Skate bind joint positions (GTA axes), by source.
    source_bind: Vec<Option<Vec3>>,
    /// Inverse Skate bind (GTA axes), by source.
    source_inverse: Vec<Option<Mat4>>,
    /// Midpoint of the GTA hip joints in the pelvis bind frame.
    hips: Vec3,
    /// Per limb: hinge axis in the upper bone's bind frame.
    hinge: Vec<Vec3>,
}

impl PedRig {
    /// Fits the GTA ped `gta` (bind in model space) to the Skate reference
    /// skater (`rig.json`); `pose_names` are the bones of Skate's poses.
    pub fn new(gta: &[GtaBone], reference: &[ReferenceBone], pose_names: &[String]) -> Result<Self, String> {
        let names: Vec<&str> = gta.iter().map(|b| b.name.as_str()).collect();
        let idx = |n: &str| names.iter().position(|x| *x == n);
        if let Some(missing) = MAP.iter().flat_map(|m| std::iter::once(m.0).chain(m.2.map(|s| s.0))).find(|n| idx(n).is_none()) {
            return Err(format!("not a human skeleton (no {missing})"));
        }
        let parents: Vec<Option<usize>> = gta.iter().map(|b| b.parent).collect();
        let bind: Vec<Mat4> = gta.iter().map(|b| b.bind).collect();
        let local: Vec<Mat4> = (0..gta.len()).map(|i| parents[i].map_or(bind[i], |p| bind[p].inverse() * bind[i])).collect();
        // Parents before children.
        let mut order = Vec::with_capacity(gta.len());
        let mut placed = vec![false; gta.len()];
        while order.len() < gta.len() {
            let before = order.len();
            for i in 0..gta.len() {
                if !placed[i] && parents[i].is_none_or(|p| placed[p]) {
                    placed[i] = true;
                    order.push(i);
                }
            }
            if order.len() == before {
                return Err("the skeleton has a parent cycle".into());
            }
        }
        let pelvis = idx("SKEL_Pelvis").ok_or("no SKEL_Pelvis")?;

        let mut sources: Vec<&'static str> = Vec::new();
        let mut src = |n: &'static str| -> usize {
            if let Some(i) = sources.iter().position(|s| *s == n) {
                return i;
            }
            sources.push(n);
            sources.len() - 1
        };
        let mut mapped = vec![None; gta.len()];
        for (bone, source, _) in MAP {
            mapped[idx(bone).unwrap()] = Some(src(source));
        }
        let mut fit_from: Vec<usize> = (0..gta.len()).collect();
        for (bone, from) in TERMINAL_FIT.iter().chain(TOE_FIT.iter()) {
            if let (Some(b), Some(f)) = (idx(bone), idx(from)) {
                fit_from[b] = f;
            }
        }
        let hip_sources = [src("LEFTUPLEG"), src("RIGHTUPLEG")];
        let truck_sources = [src("TRUCK_FRONT"), src("TRUCK_BACK")];
        let body_sources: Vec<usize> = ["HIPS", "SPINE3", "HEAD", "LEFTLEG", "RIGHTLEG", "LEFTFOOT", "RIGHTFOOT", "LEFTTOEBASE", "RIGHTTOEBASE"]
            .iter()
            .map(|n| src(n))
            .collect();
        let thighs = [idx("SKEL_L_Thigh").unwrap(), idx("SKEL_R_Thigh").unwrap()];
        let limbs: Vec<[usize; 3]> = LIMBS.iter().map(|l| [idx(l.0).unwrap(), idx(l.1).unwrap(), idx(l.2).unwrap()]).collect();
        // Shirt/belt/bum helpers under the legs follow the pelvis: following
        // the thigh dragged the jacket hem with every knee lift.
        let under_leg = |mut i: usize| {
            while let Some(p) = parents[i] {
                if names[p].starts_with("SKEL_") && (names[p].contains("Thigh") || names[p].contains("Calf")) {
                    return true;
                }
                i = p;
            }
            false
        };
        let helper = |n: &str| ["Shirt", "Belt", "Bum"].iter().any(|k| n.contains(k));
        let followers: Vec<Option<(usize, Mat4)>> = (0..gta.len())
            .map(|i| {
                (!names[i].starts_with("SKEL_") && helper(names[i]) && parents[i].is_some_and(|p| !helper(names[p])) && under_leg(i))
                    .then(|| (pelvis, bind[pelvis].inverse() * bind[i]))
            })
            .collect();
        let segment_end = |bone: usize| MAP.iter().find(|m| m.0 == names[bone]).and_then(|m| m.2).and_then(|s| idx(s.0));
        let rolls: Vec<Roll> = ROLLS
            .iter()
            .filter_map(|&(roll, base, twisted, swing)| {
                let (bone, base, twisted) = (idx(roll)?, idx(base)?, idx(twisted)?);
                let (from, to) = if swing { (twisted, segment_end(twisted)?) } else { (base, twisted) };
                let p = |i: usize| bind[i].w_axis.truncate();
                let axis = (p(to) - p(from)).try_normalize()?;
                let share = ((p(bone) - p(from)).dot(axis) / p(to).distance(p(from))).clamp(0.0, 1.0);
                Some(Roll { bone, base, twisted, swing, share, axis })
            })
            .collect();

        // Skate's bind in GTA axes, and where each source is in its poses.
        let source_bind_world: Vec<Option<Mat4>> = sources
            .iter()
            .map(|n| reference.iter().find(|b| b.name == *n).map(|b| transform_from_skate(Mat4::from_cols_array(&b.bind), Vec3::ZERO)))
            .collect();
        let source_bind: Vec<Option<Vec3>> = source_bind_world.iter().map(|m| m.map(|m| m.w_axis.truncate())).collect();
        let source_inverse: Vec<Option<Mat4>> = source_bind_world.iter().map(|m| m.map(|m| m.inverse())).collect();
        let source_pose: Vec<Option<usize>> = sources.iter().map(|n| pose_names.iter().position(|p| p == n)).collect();

        // The fit (SK8V's `calibrate`).
        let source = |n: &str| sources.iter().position(|s| *s == n).and_then(|i| source_bind[i]);
        let need = |n: &str| source(n).ok_or_else(|| format!("Skate's bind has no {n}"));
        let g = |n: &str| idx(n).map(|i| bind[i].w_axis.truncate()).ok_or_else(|| format!("no {n}"));
        let facing = frame(need("SPINE3")? - need("HIPS")?, need("LEFTUPLEG")? - need("RIGHTUPLEG")?)
            .zip(frame(g("SKEL_Spine3")? - g("SKEL_Pelvis")?, g("SKEL_L_Thigh")? - g("SKEL_R_Thigh")?))
            .map(|(s, t)| s * t.transpose())
            .ok_or("degenerate pelvis frames")?;
        let horizontal = |v: Vec3| Vec3::new(v.x, v.y, 0.0).try_normalize();
        let mut fit_rotation = vec![facing; gta.len()];
        for (bone, skate, segment) in MAP {
            let Some((child, skate_child)) = segment else { continue };
            let (from, to) = (facing * (g(child)? - g(bone)?), need(skate_child)? - need(skate)?);
            let swing = if GROUND_FIT.contains(&bone) {
                horizontal(from).zip(horizontal(to)).map(|(a, b)| Quat::from_rotation_arc(a, b))
            } else {
                from.try_normalize().zip(to.try_normalize()).map(|(a, b)| Quat::from_rotation_arc(a, b))
            };
            fit_rotation[idx(bone).unwrap()] = Mat3::from_quat(swing.unwrap_or(Quat::IDENTITY)) * facing;
        }
        let offset = (0..gta.len()).map(|i| mapped[i].map(|_| fit_rotation[fit_from[i]] * Mat3::from_mat4(bind[i]))).collect();
        let p = |i: usize| bind[i].w_axis.truncate();
        let hips = Mat3::from_mat4(bind[pelvis]).transpose() * ((p(thighs[0]) + p(thighs[1])) * 0.5 - p(pelvis));
        let hinge = limbs
            .iter()
            .map(|&[upper, lower, end]| {
                let axis = (p(lower) - p(upper)).cross(p(end) - p(lower)).try_normalize().unwrap_or(Mat3::from_mat4(bind[upper]) * Vec3::Z);
                Mat3::from_mat4(bind[upper]).transpose() * axis
            })
            .collect();
        Ok(Self {
            parents,
            local,
            bind,
            order,
            source_pose,
            mapped,
            pelvis,
            hip_sources,
            truck_sources,
            body_sources,
            limbs,
            rolls,
            followers,
            offset,
            source_bind,
            source_inverse,
            hips,
            hinge,
        })
    }

    /// The Skate pose's bones (indices) this retarget reads.
    pub fn used(&self) -> Vec<usize> {
        self.source_pose.iter().flatten().copied().collect()
    }

    /// GTA world matrices for every bone from a Skate pose (Skate space,
    /// Skate's origin being GTA point `origin`); none when the pose lacks
    /// the hips or comes out non-finite.
    pub fn pose(&self, skate: &[Mat4], origin: Vec3) -> Option<Vec<Mat4>> {
        // Skate's skin transform (bind -> posed) of each source, GTA axes.
        let skin: Vec<Option<Mat4>> = self
            .source_pose
            .iter()
            .zip(&self.source_inverse)
            .map(|(pose, inverse)| Some(transform_from_skate(*skate.get((*pose)?)?, origin) * (*inverse)?))
            .collect();
        self.solve(&skin)
    }

    fn solve(&self, skate: &[Option<Mat4>]) -> Option<Vec<Mat4>> {
        let delta = |j: usize| skate.get(j).copied().flatten();
        let target = |j: usize| Some(delta(j)?.transform_point3(self.source_bind.get(j).copied().flatten()?));
        let n = self.local.len();
        let mut rot: Vec<Option<Mat3>> = (0..n).map(|i| Some(Mat3::from_mat4(delta(self.mapped[i]?)?) * self.offset[i]?)).collect();
        let mut pos: Vec<Option<Vec3>> = vec![None; n];
        // GTA's pelvis joint sits above its hips: place it so the GTA hip
        // joints straddle Skate's.
        let pelvis_rot = rot[self.pelvis]?;
        let hips = (target(self.hip_sources[0])? + target(self.hip_sources[1])?) * 0.5;
        let pelvis = rigid(pelvis_rot, hips - pelvis_rot * self.hips);
        let mut world = self.fk(pelvis, &rot, &pos);
        // A hand in contact beyond the GTA arm's reach dips the clavicle
        // toward it (up to 25 degrees) instead of locking the elbow.
        let mut dipped = false;
        for &[upper, lower, end] in &self.limbs[2..] {
            let (Some(j), Some(clavicle)) = (self.mapped[end], self.parents[upper]) else { continue };
            let (Some(goal), Some(r)) = (target(j), rot[clavicle]) else { continue };
            let p = |i: usize| world[i].w_axis.truncate();
            let full = p(upper).distance(p(lower)) + p(lower).distance(p(end));
            let excess = (goal.distance(p(upper)) - 0.98 * full) * self.hand_contact(goal, &target);
            let arm = p(upper) - p(clavicle);
            let Some(axis) = arm.cross(goal - p(clavicle)).try_normalize() else { continue };
            if excess > 0.0 && arm.length() > 1e-3 {
                let angle = (excess / arm.length()).min(25f32.to_radians());
                rot[clavicle] = Some(Mat3::from_axis_angle(axis, angle) * r);
                dipped = true;
            }
        }
        if dipped {
            world = self.fk(pelvis, &rot, &pos);
        }
        for (k, &[upper, lower, end]) in self.limbs.iter().enumerate() {
            let (Some(j), Some(r_upper), Some(r_lower)) = (self.mapped[end], rot[upper], rot[lower]) else { continue };
            let Some(mut goal) = target(j) else { continue };
            let p = |i: usize| world[i].w_axis.truncate();
            let (hip, knee, ankle) = (p(upper), p(lower), p(end));
            // Feet always meet Skate's; a hand only in contact (GTA's
            // shoulders sit ~7 cm above Skate's: pinning a free hand
            // straightened the elbow and amplified every swing).
            if k >= 2 {
                goal = ankle.lerp(goal, self.hand_contact(goal, &target));
            }
            // The chain's own hinge, easing to the bind hinge as the limb
            // straightens, kept on the bind hinge's side (no flips).
            let fallback = (r_upper * self.hinge[k]).normalize();
            let chain = (knee - hip).cross(ankle - knee);
            let chain = if chain.dot(fallback) < 0.0 { -chain } else { chain };
            let bent = chain.length() / (hip.distance(knee) * knee.distance(ankle)).max(1e-9);
            let w = ((bent - 0.02) / 0.08).clamp(0.0, 1.0);
            let bend = (fallback * (1.0 - w) + chain.normalize_or_zero() * w).try_normalize().unwrap_or(fallback);
            if let Some(l) = solve_limb(hip, knee, ankle, goal, bend, r_upper, r_lower, STRETCH) {
                rot[upper] = Some(l.upper);
                rot[lower] = Some(l.lower);
                pos[end] = Some(l.end);
            }
        }
        let world = self.fk(pelvis, &rot, &pos);
        for r in &self.rolls {
            rot[r.bone] = Some(self.roll(r, &world));
        }
        let world = self.fk(pelvis, &rot, &pos);
        world.iter().all(|m| m.is_finite()).then_some(world)
    }

    /// How much Skate's hand at `hand` is in contact (0..1): near the deck
    /// (grabs, carrying the board) or at the body's lowest level (plants,
    /// bails), faded over a band so the arm never snaps.
    fn hand_contact(&self, hand: Vec3, target: &dyn Fn(usize) -> Option<Vec3>) -> f32 {
        let fade = |d: f32, near: f32, far: f32| 1.0 - ((d - near) / (far - near)).clamp(0.0, 1.0);
        let board = match (target(self.truck_sources[0]), target(self.truck_sources[1])) {
            (Some(a), Some(b)) => {
                let axis = (a - b).normalize_or_zero();
                let t = (hand - b).dot(axis).clamp(-0.2, a.distance(b) + 0.2);
                fade(hand.distance(b + axis * t) - 0.11, 0.08, 0.2)
            }
            _ => 0.0,
        };
        let lowest = self.body_sources.iter().filter_map(|&j| target(j)).map(|p| p.z).fold(f32::INFINITY, f32::min);
        let ground = if lowest.is_finite() { fade(hand.z - lowest, 0.15, 0.35) } else { 0.0 };
        board.max(ground)
    }

    /// Forward kinematics from the pelvis with rotation/position overrides;
    /// every other bone keeps its GTA bind offset from its parent (or leader).
    fn fk(&self, pelvis: Mat4, rot: &[Option<Mat3>], pos: &[Option<Vec3>]) -> Vec<Mat4> {
        let mut world = vec![Mat4::IDENTITY; self.local.len()];
        let mut done = vec![false; self.local.len()];
        world[self.pelvis] = pelvis;
        done[self.pelvis] = true;
        if let Some(root) = self.parents[self.pelvis] {
            world[root] = pelvis * self.local[self.pelvis].inverse();
            done[root] = true;
        }
        for &i in &self.order {
            if done[i] {
                continue;
            }
            done[i] = true;
            if let Some((leader, relative)) = self.followers[i] {
                world[i] = world[leader] * relative;
                continue;
            }
            let fk = self.parents[i].map_or(Mat4::IDENTITY, |p| world[p]) * self.local[i];
            let at = pos[i].unwrap_or(fk.w_axis.truncate());
            world[i] = match rot[i] {
                Some(r) => rigid(r, at),
                None if pos[i].is_some() => rigid(Mat3::from_mat4(fk), at),
                None => fk,
            };
        }
        world
    }

    /// A roll bone's rotation: its base's change from bind carried rigidly,
    /// then the twisted bone's swing (for rolls on the twisted bone) and its
    /// share of the twist about the segment.
    fn roll(&self, r: &Roll, world: &[Mat4]) -> Mat3 {
        let rot = |i: usize| Mat3::from_mat4(world[i]);
        let bind = |i: usize| Mat3::from_mat4(self.bind[i]);
        let carried = rot(r.base) * bind(r.base).transpose();
        let delta = Quat::from_mat3(&(rot(r.twisted) * (carried * bind(r.twisted)).transpose())).normalize();
        let turn = twist(delta, (carried * r.axis).normalize());
        let share = Quat::IDENTITY.slerp(turn, r.share);
        let q = if r.swing { delta * turn.inverse() * share } else { share };
        Mat3::from_quat(q.normalize()) * carried * bind(r.bone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stick figure with GTA's bone names, standing in GTA axes (Z up,
    /// facing -Y, left +X), and Skate's names at the same joints.
    fn figure() -> (Vec<GtaBone>, Vec<ReferenceBone>, Vec<String>) {
        let joints: [(&str, &str, Option<&str>, [f32; 3]); 23] = [
            ("SKEL_Pelvis", "HIPS", None, [0., 0., 1.0]),
            ("SKEL_Spine0", "SPINE", Some("SKEL_Pelvis"), [0., 0., 1.1]),
            ("SKEL_Spine1", "SPINE1", Some("SKEL_Spine0"), [0., 0., 1.2]),
            ("SKEL_Spine2", "SPINE2", Some("SKEL_Spine1"), [0., 0., 1.3]),
            ("SKEL_Spine3", "SPINE3", Some("SKEL_Spine2"), [0., 0., 1.4]),
            ("SKEL_Neck_1", "NECK", Some("SKEL_Spine3"), [0., 0., 1.55]),
            ("SKEL_Head", "HEAD", Some("SKEL_Neck_1"), [0., 0., 1.65]),
            ("SKEL_L_Clavicle", "LEFTSHOULDER", Some("SKEL_Spine3"), [0.05, 0., 1.5]),
            ("SKEL_L_UpperArm", "LEFTARM", Some("SKEL_L_Clavicle"), [0.2, 0., 1.5]),
            ("SKEL_L_Forearm", "LEFTFOREARM", Some("SKEL_L_UpperArm"), [0.2, 0.02, 1.22]),
            ("SKEL_L_Hand", "LEFTHAND", Some("SKEL_L_Forearm"), [0.2, 0., 0.95]),
            ("SKEL_R_Clavicle", "RIGHTSHOULDER", Some("SKEL_Spine3"), [-0.05, 0., 1.5]),
            ("SKEL_R_UpperArm", "RIGHTARM", Some("SKEL_R_Clavicle"), [-0.2, 0., 1.5]),
            ("SKEL_R_Forearm", "RIGHTFOREARM", Some("SKEL_R_UpperArm"), [-0.2, 0.02, 1.22]),
            ("SKEL_R_Hand", "RIGHTHAND", Some("SKEL_R_Forearm"), [-0.2, 0., 0.95]),
            ("SKEL_L_Thigh", "LEFTUPLEG", Some("SKEL_Pelvis"), [0.1, 0., 0.95]),
            ("SKEL_L_Calf", "LEFTLEG", Some("SKEL_L_Thigh"), [0.1, -0.03, 0.5]),
            ("SKEL_L_Foot", "LEFTFOOT", Some("SKEL_L_Calf"), [0.1, 0., 0.1]),
            ("SKEL_L_Toe0", "LEFTTOEBASE", Some("SKEL_L_Foot"), [0.1, -0.12, 0.03]),
            ("SKEL_R_Thigh", "RIGHTUPLEG", Some("SKEL_Pelvis"), [-0.1, 0., 0.95]),
            ("SKEL_R_Calf", "RIGHTLEG", Some("SKEL_R_Thigh"), [-0.1, -0.03, 0.5]),
            ("SKEL_R_Foot", "RIGHTFOOT", Some("SKEL_R_Calf"), [-0.1, 0., 0.1]),
            ("SKEL_R_Toe0", "RIGHTTOEBASE", Some("SKEL_R_Foot"), [-0.1, -0.12, 0.03]),
        ];
        let names: Vec<&str> = joints.iter().map(|j| j.0).collect();
        let gta: Vec<GtaBone> = joints
            .iter()
            .map(|(name, _, parent, at)| GtaBone {
                name: (*name).into(),
                parent: parent.and_then(|p| names.iter().position(|n| *n == p)),
                bind: Mat4::from_translation(Vec3::from(*at)),
            })
            .collect();
        // Skate's bind at the same places, in Skate space.
        let reference: Vec<ReferenceBone> = joints
            .iter()
            .map(|(_, skate, _, at)| ReferenceBone {
                name: (*skate).into(),
                bind: Mat4::from_translation(crate::coords::to_skate(Vec3::from(*at))).to_cols_array(),
            })
            .collect();
        let pose_names = joints.iter().map(|j| j.1.to_string()).collect();
        (gta, reference, pose_names)
    }

    #[test]
    fn skate_in_bind_poses_gta_in_bind() {
        let (gta, reference, names) = figure();
        let rig = PedRig::new(&gta, &reference, &names).unwrap();
        let pose: Vec<Mat4> = reference.iter().map(|b| Mat4::from_cols_array(&b.bind)).collect();
        let origin = Vec3::new(100., -50., 30.);
        let world = rig.pose(&pose, origin).unwrap();
        for (bone, posed) in gta.iter().zip(&world) {
            let want = bone.bind.w_axis.truncate() + origin;
            assert!(posed.w_axis.truncate().distance(want) < 1e-3, "{}: {} vs {want}", bone.name, posed.w_axis.truncate());
        }
    }

    #[test]
    fn feet_follow_skate_and_bones_keep_gta_lengths() {
        let (gta, reference, names) = figure();
        let rig = PedRig::new(&gta, &reference, &names).unwrap();
        // The whole skater crouches 0.2 m: hips and everything above down,
        // the feet stay. Skate space: Y up.
        let pose: Vec<Mat4> = reference
            .iter()
            .map(|b| {
                let m = Mat4::from_cols_array(&b.bind);
                let low = ["LEFTFOOT", "RIGHTFOOT", "LEFTTOEBASE", "RIGHTTOEBASE"].contains(&b.name.as_str());
                if low { m } else { Mat4::from_translation(Vec3::new(0., -0.2, 0.)) * m }
            })
            .collect();
        let world = rig.pose(&pose, Vec3::ZERO).unwrap();
        let at = |n: &str| world[gta.iter().position(|b| b.name == n).unwrap()].w_axis.truncate();
        assert!(at("SKEL_L_Foot").distance(Vec3::new(0.1, 0., 0.1)) < 0.01, "foot stays: {}", at("SKEL_L_Foot"));
        assert!((at("SKEL_Pelvis").z - 0.8).abs() < 0.01, "hips down: {}", at("SKEL_Pelvis"));
        let thigh = at("SKEL_L_Thigh").distance(at("SKEL_L_Calf"));
        let bind_thigh = Vec3::new(0.1, 0., 0.95).distance(Vec3::new(0.1, -0.03, 0.5));
        assert!((thigh - bind_thigh).abs() < 1e-3, "rigid thigh");
        assert!(at("SKEL_L_Calf").y < -0.03, "the knee bends forward: {}", at("SKEL_L_Calf"));
    }
}
