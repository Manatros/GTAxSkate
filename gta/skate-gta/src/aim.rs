//! Aiming a gun while skating, and walking the way GTA's camera looks.
//!
//! The pose Skate gives the skater is bent toward the camera's aim the way a
//! body can bend: the spine turns at most [`TWIST_LIMIT`] from the hips (no
//! half turn at the waist), the right arm reaches toward the aim (two-bone
//! IK) and the hand holds the gun along it. An aim further round than the
//! spine can turn steers the board slowly that way.

use bevy_math::{Mat3, Mat4, Quat, Vec2, Vec3};

/// How far the spine turns from the hips, at most (degrees).
pub const TWIST_LIMIT: f32 = 75.;
/// The spine bones that share the turn, and their shares.
const SPINE: [(&str, f32); 3] = [("SKEL_Spine1", 0.3), ("SKEL_Spine2", 0.35), ("SKEL_Spine3", 0.35)];

/// `degrees` brought into -180..180.
pub fn wrap_degrees(degrees: f32) -> f32 {
    (degrees + 180.).rem_euclid(360.) - 180.
}

/// The turn about GTA's up (degrees, left positive) from horizontal
/// direction `from` to `to`.
pub fn turn_between(from: Vec3, to: Vec3) -> f32 {
    let (a, b) = (from.truncate(), to.truncate());
    if a.length_squared() < 1e-8 || b.length_squared() < 1e-8 {
        return 0.;
    }
    a.angle_to(b).to_degrees()
}

/// Steering (-1 left to 1 right, Skate's left stick x) that turns the board
/// toward an aim `turn` degrees round from the hips (left positive): none
/// while the spine can turn that far, then more the further round it is.
pub fn steer_toward(turn: f32) -> f32 {
    let beyond = turn.abs() - TWIST_LIMIT;
    if beyond <= 0. {
        return 0.;
    }
    -turn.signum() * (beyond / 45.).min(1.) * 0.7
}

/// A stick direction (x right, y ahead) meant relative to GTA's camera,
/// turned so Skate, which reads it relative to its own camera, goes the
/// same way. Yaws are GTA headings in degrees (left positive).
pub fn stick_for_camera(stick: [f32; 2], gta_camera_yaw: f32, skate_camera_yaw: f32) -> [f32; 2] {
    let turn = wrap_degrees(gta_camera_yaw - skate_camera_yaw).to_radians();
    let v = Vec2::from_angle(turn).rotate(Vec2::from_array(stick));
    [v.x, v.y]
}

/// The elbow for a two-bone limb reaching from `shoulder` toward `target`
/// with bone lengths `upper` and `lower`, bending toward `pole`; and where
/// the hand ends up (the target, or as near as the limb reaches).
pub fn two_bone(shoulder: Vec3, target: Vec3, upper: f32, lower: f32, pole: Vec3) -> (Vec3, Vec3) {
    let to = target - shoulder;
    let reach = (upper + lower) * 0.999;
    let distance = to.length().clamp((upper - lower).abs() + 1e-3, reach);
    let dir = to.try_normalize().unwrap_or(Vec3::Y);
    let hand = shoulder + dir * distance;
    // Law of cosines: how far along the reach the elbow sits, and how far
    // off it.
    let along = (upper * upper - lower * lower + distance * distance) / (2. * distance);
    let off = (upper * upper - along * along).max(0.).sqrt();
    let side = (pole - shoulder).reject_from(dir).try_normalize().unwrap_or_else(|| dir.any_orthonormal_vector());
    (shoulder + dir * along + side * off, hand)
}

/// Turns bone `root` and everything below it by `delta` (in world space).
/// Bones come parents first.
pub fn turn_subtree(world: &mut [Mat4], parents: &[Option<usize>], root: usize, delta: Mat4) {
    let mut inside = vec![false; world.len()];
    for i in root..world.len() {
        inside[i] = i == root || parents.get(i).copied().flatten().is_some_and(|p| inside[p]);
        if inside[i] {
            world[i] = delta * world[i];
        }
    }
}

/// `rotation` about point `at`.
fn about(at: Vec3, rotation: Quat) -> Mat4 {
    Mat4::from_translation(at) * Mat4::from_quat(rotation) * Mat4::from_translation(-at)
}

fn rotation_of(m: &Mat4) -> Quat {
    let (_, r, _) = m.to_scale_rotation_translation();
    r.normalize()
}

/// The rotation taking direction `forward` (with up `Z`) onto `to` (with
/// world up as near as it goes).
fn facing(forward: Vec3, to: Vec3) -> Quat {
    let frame = |f: Vec3| {
        let f = f.try_normalize().unwrap_or(Vec3::Y);
        let right = f.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        Mat3::from_cols(f, right, right.cross(f))
    };
    Quat::from_mat3(&(frame(to) * frame(forward).transpose())).normalize()
}

/// The bones aiming moves, found by name in a ped's skeleton.
pub struct AimRig {
    parents: Vec<Option<usize>>,
    pelvis: usize,
    spine: Vec<(usize, f32)>,
    upper_arm: usize,
    forearm: usize,
    hand: usize,
    /// Bind rotations (model space) of the pelvis and the hand.
    pelvis_bind: Quat,
    hand_bind: Quat,
    /// The way the ped faces in its bind pose (model space): toes ahead.
    forward: Vec3,
}

impl AimRig {
    /// From the ped's bones (name, parent, bind rotation and translation
    /// relative to the parent), as [`crate::rig::bind_pose`] reads them.
    pub fn new(bones: &[(String, Option<usize>, Quat, Vec3)]) -> Option<Self> {
        let index = |name: &str| bones.iter().position(|b| b.0 == name);
        let binds = crate::rig::bind_pose(bones);
        let position = |i: usize| binds[i].bind.w_axis.truncate();
        let (foot, toe) = (index("SKEL_R_Foot")?, index("SKEL_R_Toe0")?);
        let forward = (position(toe) - position(foot)).with_z(0.).try_normalize().unwrap_or(Vec3::NEG_Y);
        let pelvis = index("SKEL_Pelvis")?;
        let hand = index("SKEL_R_Hand")?;
        Some(Self {
            parents: bones.iter().map(|b| b.1).collect(),
            pelvis,
            spine: SPINE.iter().filter_map(|(name, share)| Some((index(name)?, *share))).collect(),
            upper_arm: index("SKEL_R_UpperArm")?,
            forearm: index("SKEL_R_Forearm")?,
            hand,
            pelvis_bind: rotation_of(&binds[pelvis].bind),
            hand_bind: rotation_of(&binds[hand].bind),
            forward,
        })
    }

    /// The way the hips face in world (horizontal), in pose `world`.
    pub fn hips_facing(&self, world: &[Mat4]) -> Vec3 {
        let model_to_world = rotation_of(&world[self.pelvis]) * self.pelvis_bind.inverse();
        (model_to_world * self.forward).with_z(0.).try_normalize().unwrap_or(Vec3::Y)
    }

    /// Bends pose `world` (each bone in world space) to aim at `target`.
    /// Returns how far round from the hips the aim is (degrees, left
    /// positive) and where the gun hand is.
    pub fn aim(&self, world: &mut [Mat4], target: Vec3) -> (f32, Vec3) {
        if world.len() < self.parents.len() {
            return (0., Vec3::ZERO);
        }
        let at = |world: &[Mat4], i: usize| world[i].w_axis.truncate();
        let hips = self.hips_facing(world);
        let turn = turn_between(hips, target - at(world, self.pelvis));
        let twist = turn.clamp(-TWIST_LIMIT, TWIST_LIMIT).to_radians();
        for &(bone, share) in &self.spine {
            let delta = about(at(world, bone), Quat::from_rotation_z(twist * share));
            turn_subtree(world, &self.parents, bone, delta);
        }

        // The arm toward the aim, the elbow down and out.
        let (shoulder, elbow, hand) = (at(world, self.upper_arm), at(world, self.forearm), at(world, self.hand));
        let (upper, lower) = (elbow.distance(shoulder), hand.distance(elbow));
        let aim = (target - shoulder).try_normalize().unwrap_or(hips);
        let pole = shoulder - Vec3::Z + aim.cross(Vec3::Z) * 0.3;
        let (new_elbow, new_hand) = two_bone(shoulder, shoulder + aim * (upper + lower) * 0.92, upper, lower, pole);
        let turn_upper = Quat::from_rotation_arc((elbow - shoulder).normalize(), (new_elbow - shoulder).normalize());
        turn_subtree(world, &self.parents, self.upper_arm, about(shoulder, turn_upper));
        let (elbow, hand) = (at(world, self.forearm), at(world, self.hand));
        let turn_lower = Quat::from_rotation_arc((hand - elbow).normalize(), (new_hand - elbow).normalize());
        turn_subtree(world, &self.parents, self.forearm, about(elbow, turn_lower));

        // The hand holds the gun along the aim: its bind pose holds it the
        // way the ped faces.
        let hand = at(world, self.hand);
        let gun_aim = (target - hand).try_normalize().unwrap_or(aim);
        let wanted = facing(self.forward, gun_aim) * self.hand_bind;
        let now = rotation_of(&world[self.hand]);
        turn_subtree(world, &self.parents, self.hand, about(hand, (wanted * now.inverse()).normalize()));
        (turn, hand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small ped facing -Y like GTA's freemode one (its right is -X):
    /// pelvis, spine, the right arm stretched out, a leg.
    fn bones() -> Vec<(String, Option<usize>, Quat, Vec3)> {
        let b = |n: &str, p: Option<usize>, t: Vec3| (n.to_owned(), p, Quat::IDENTITY, t);
        vec![
            b("SKEL_ROOT", None, Vec3::ZERO),
            b("SKEL_Pelvis", Some(0), Vec3::ZERO),
            b("SKEL_Spine1", Some(1), Vec3::new(0., 0., 0.2)),
            b("SKEL_Spine2", Some(2), Vec3::new(0., 0., 0.15)),
            b("SKEL_Spine3", Some(3), Vec3::new(0., 0., 0.15)),
            b("SKEL_R_UpperArm", Some(4), Vec3::new(-0.2, 0., 0.1)),
            b("SKEL_R_Forearm", Some(5), Vec3::new(-0.28, 0., 0.)),
            b("SKEL_R_Hand", Some(6), Vec3::new(-0.25, 0., 0.)),
            b("SKEL_R_Finger00", Some(7), Vec3::new(-0.05, -0.02, 0.)),
            b("SKEL_R_Thigh", Some(1), Vec3::new(-0.1, 0., -0.05)),
            b("SKEL_R_Foot", Some(9), Vec3::new(0., 0., -0.85)),
            b("SKEL_R_Toe0", Some(10), Vec3::new(0., -0.15, -0.05)),
        ]
    }

    fn world(bones: &[(String, Option<usize>, Quat, Vec3)]) -> Vec<Mat4> {
        crate::rig::bind_pose(bones).iter().map(|b| Mat4::from_translation(Vec3::new(10., 20., 1.)) * b.bind).collect()
    }

    #[test]
    fn the_hand_points_the_gun_at_the_target_ahead() {
        let bones = bones();
        let rig = AimRig::new(&bones).unwrap();
        let mut pose = world(&bones);
        assert!(rig.hips_facing(&pose).distance(Vec3::NEG_Y) < 1e-4);
        let target = Vec3::new(10., 0., 1.5);
        let (turn, hand) = rig.aim(&mut pose, target);
        assert!(turn.abs() < 10., "straight ahead: {turn}");
        // The gun (the bind's forward, in the hand's frame) points at it.
        let gun = (rotation_of(&pose[7]) * rig.hand_bind.inverse()) * rig.forward;
        let want = (target - hand).normalize();
        assert!(gun.dot(want) > 0.99, "gun {gun} vs {want}");
        // The finger came along with the hand.
        assert!(pose[8].w_axis.truncate().distance(hand) < 0.1);
    }

    #[test]
    fn the_spine_turns_at_most_its_limit_and_the_hips_stay() {
        let bones = bones();
        let rig = AimRig::new(&bones).unwrap();
        let mut pose = world(&bones);
        let pelvis_before = pose[1];
        // Straight behind the ped.
        let (turn, _) = rig.aim(&mut pose, Vec3::new(10., 40., 1.5));
        assert!(turn.abs() > 170., "{turn}");
        assert_eq!(pose[1], pelvis_before, "the hips do not turn");
        let chest = rotation_of(&pose[4]) * Vec3::NEG_Y;
        let twist = turn_between(Vec3::NEG_Y, chest).abs();
        assert!((twist - TWIST_LIMIT).abs() < 1., "chest turned {twist}");
        assert!(steer_toward(turn).abs() > 0.5, "the board turns that way");
    }

    #[test]
    fn steering_only_beyond_the_spines_reach_and_toward_the_aim() {
        assert_eq!(steer_toward(40.), 0.);
        assert!(steer_toward(120.) < 0., "aim to the left steers left");
        assert!(steer_toward(-120.) > 0.);
    }

    #[test]
    fn two_bones_reach_the_target_and_keep_their_lengths() {
        let shoulder = Vec3::new(0., 0., 1.5);
        let target = Vec3::new(0.3, 0.3, 1.4);
        let (elbow, hand) = two_bone(shoulder, target, 0.28, 0.25, shoulder - Vec3::Z);
        assert!(hand.distance(target) < 1e-4);
        assert!((elbow.distance(shoulder) - 0.28).abs() < 1e-4);
        assert!((hand.distance(elbow) - 0.25).abs() < 1e-4);
        assert!(elbow.z < shoulder.z, "the elbow bends down");
    }

    #[test]
    fn walking_follows_gtas_camera() {
        // GTA's camera looks 90 degrees left of Skate's: "ahead" on the
        // keys must be Skate's "left".
        let [x, y] = stick_for_camera([0., 1.], 90., 0.);
        assert!((x + 1.).abs() < 1e-5 && y.abs() < 1e-5, "{x} {y}");
        assert_eq!(stick_for_camera([0.5, 0.5], 30., 30.), [0.5, 0.5]);
    }
}
