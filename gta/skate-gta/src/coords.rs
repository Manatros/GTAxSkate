//! GTA V and Skate 3 spaces.
//!
//! GTA V: metres, Z up, a ped facing +Y at heading 0, headings in degrees
//! turning left. Skate 3: metres, Y up. Skate runs around a local origin,
//! the place skate mode was switched on, to keep its numbers small.

use bevy::math::{Mat3, Mat4, Quat, Vec3, Vec4};

pub fn to_skate(gta: Vec3) -> Vec3 {
    Vec3::new(gta.x, gta.z, -gta.y)
}

pub fn from_skate(skate: Vec3) -> Vec3 {
    Vec3::new(skate.x, -skate.z, skate.y)
}

/// The rotation taking Skate directions to GTA ones.
pub fn basis() -> Mat4 {
    Mat4::from_cols(Vec4::X, Vec4::Z, -Vec4::Y, Vec4::W)
}

/// A Skate transform as the GTA world transform, Skate's origin being GTA
/// point `origin`.
pub fn transform_from_skate(skate: Mat4, origin: Vec3) -> Mat4 {
    let b = basis();
    let mut gta = b * skate * b.inverse();
    gta.w_axis = (from_skate(skate.w_axis.truncate()) + origin).extend(1.);
    gta
}

/// The Skate heading (radians) of a GTA heading (degrees): Skate faces
/// GTA's +Y at pi.
pub fn skate_heading(gta_heading_degrees: f32) -> f32 {
    gta_heading_degrees.to_radians() + std::f32::consts::PI
}

/// The GTA heading (degrees) facing horizontal direction `forward`.
pub fn gta_heading(forward: Vec3) -> f32 {
    (-forward.x).atan2(forward.y).to_degrees()
}

/// A GTA ped's transform at `position` facing `heading` degrees.
pub fn entity_transform(position: Vec3, heading: f32) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_z(heading.to_radians()), position)
}

/// GTA camera rotation (pitch, roll, yaw in degrees, rotation order 2) that
/// looks along `forward`. Skate's camera does not roll.
pub fn camera_rotation(forward: Vec3) -> [f32; 3] {
    let f = forward.normalize_or(Vec3::Y);
    [f.z.clamp(-1., 1.).asin().to_degrees(), 0., gta_heading(f)]
}

/// A rotation's quaternion for `SET_ENTITY_QUATERNION`, after removing any
/// scale from its axes.
pub fn rotation_quaternion(m: Mat4) -> Quat {
    let axes = Mat3::from_cols(
        m.x_axis.truncate().normalize_or(Vec3::X),
        m.y_axis.truncate().normalize_or(Vec3::Y),
        m.z_axis.truncate().normalize_or(Vec3::Z),
    );
    Quat::from_mat3(&axes).normalize()
}

/// GTA's `GET_HASH_KEY` (Jenkins one-at-a-time over lowercase bytes).
pub fn joaat(name: &str) -> u32 {
    let mut hash: u32 = 0;
    for byte in name.bytes() {
        hash = hash.wrapping_add(u32::from(byte.to_ascii_lowercase()));
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash.wrapping_add(hash << 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn spaces_roundtrip_and_keep_up() {
        let p = Vec3::new(1., 2., 3.);
        assert!(close(from_skate(to_skate(p)), p));
        assert!(close(to_skate(Vec3::Z), Vec3::Y), "GTA up is Skate up");
        let b = basis();
        assert!(close(b.transform_vector3(Vec3::new(4., 5., 6.)), from_skate(Vec3::new(4., 5., 6.))));
        assert!((b.determinant() - 1.).abs() < 1e-6, "a rotation keeps triangle winding");
    }

    #[test]
    fn transforms_land_at_origin_offsets() {
        let origin = Vec3::new(-1200., 300., 40.);
        let skate = Mat4::from_translation(Vec3::new(1., 2., 3.));
        let gta = transform_from_skate(skate, origin);
        assert!(close(gta.w_axis.truncate(), origin + from_skate(Vec3::new(1., 2., 3.))));
        let turned = transform_from_skate(Mat4::from_rotation_y(0.7), Vec3::ZERO);
        assert!(close(turned.transform_vector3(Vec3::Z), Vec3::Z), "turns about Skate up are turns about GTA up");
    }

    #[test]
    fn headings_and_cameras() {
        assert!(gta_heading(Vec3::Y).abs() < 1e-4);
        assert!((gta_heading(-Vec3::X) - 90.).abs() < 1e-3, "heading turns left");
        let e = entity_transform(Vec3::ZERO, 90.);
        assert!(close(e.transform_vector3(Vec3::Y), -Vec3::X), "heading 90 faces -X");
        let [pitch, roll, yaw] = camera_rotation(Vec3::new(0., 1., 1.));
        assert!((pitch - 45.).abs() < 1e-3 && roll == 0. && yaw.abs() < 1e-3);
        // Skate faces GTA +Y at heading pi: its forward (-Z at 0 heading,
        // turned by the heading about Y) maps to GTA +Y.
        let skate_forward = Quat::from_rotation_y(skate_heading(0.)) * Vec3::NEG_Z;
        assert!(close(from_skate(skate_forward), Vec3::NEG_Y) || close(from_skate(skate_forward), Vec3::Y));
    }

    #[test]
    fn hashes_match_gta() {
        assert_eq!(joaat("adder"), 0xB779A091);
        assert_eq!(joaat("ADDER"), 0xB779A091);
    }

    #[test]
    fn quaternions_ignore_scale() {
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(2.), Quat::from_rotation_z(0.5), Vec3::ONE);
        assert!(rotation_quaternion(m).angle_between(Quat::from_rotation_z(0.5)) < 1e-4);
    }
}
