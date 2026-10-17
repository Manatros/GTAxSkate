//! One wheel on its own for the store's part view (the tester, video 10-10 09-43
//! 08:44: the wheels alone, as Skate shows them, should be one wheel, not
//! the four floating where the board would be). `sk3_w1_<finish>`, every
//! library wheel and every painted one, a set of its own (`skate3wheel`):
//! a failing set must not take the board parts with it.

use glam::Vec3;
use skate_boards::{Board, Library, Part};

use crate::weapon::{Frame, Laid, laid_in, part_model, write_model_lit};

/// The archetype file of the set.
pub const YTYP: &str = "skate3wheel";
/// Where the wheel is put: the middle of the board's length (the models'
/// middle, 0.35 m from the grip end), on the board's centre line.
const MIDDLE: Vec3 = Vec3::new(0., 0., 0.35);

/// The model of one wheel of `material`.
pub fn wheel_model(material: &str) -> String {
    part_model(Part::Wheels, Some(material)).replacen("sk3_w_", "sk3_w1_", 1)
}

/// The triangles of one of the four wheels (the one at the far end and the
/// +Y side), moved to the board's middle.
pub fn one_wheel(laid: &Laid) -> Laid {
    let all: Vec<Vec3> = laid.iter().flat_map(|s| s.0.iter().copied()).collect();
    if all.is_empty() {
        return Vec::new();
    }
    let (lo, hi) = all.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let mid = (lo + hi) * 0.5;
    let mut out: Laid = Vec::new();
    let mut picked = Vec::new();
    for (positions, normals, uvs, indices, texture) in laid {
        let keep: Vec<u32> = indices
            .chunks_exact(3)
            .filter(|t| {
                let c = t.iter().map(|i| positions[*i as usize]).sum::<Vec3>() / 3.;
                c.z > mid.z && c.y > mid.y
            })
            .flatten()
            .copied()
            .collect();
        picked.extend(keep.iter().map(|i| positions[*i as usize]));
        out.push((positions.clone(), normals.clone(), uvs.clone(), keep, *texture));
    }
    if picked.is_empty() {
        return Vec::new();
    }
    let (plo, phi) = picked.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let shift = Vec3::new(0., MIDDLE.y, MIDDLE.z) - Vec3::new(0., (plo.y + phi.y) * 0.5, (plo.z + phi.z) * 0.5);
    for s in &mut out {
        for p in &mut s.0 {
            *p += shift;
        }
    }
    out.retain(|s| !s.3.is_empty());
    out
}

/// Writes every wheel finish and every painted wheel as one wheel; returns
/// the models with their bounds.
pub fn write(dir: &std::path::Path, library: &Library, defaults: &Board, frame: &Frame) -> Result<Vec<(String, Vec3, Vec3)>, String> {
    let mut out = Vec::new();
    for item in library.items(Part::Wheels) {
        let mut board = library.part_mesh(Part::Wheels, &item.part_ref(), 512)?;
        for texture in &mut board.textures {
            texture.make_opaque();
        }
        let laid = one_wheel(&laid_in(&board, defaults, frame));
        if laid.is_empty() {
            continue;
        }
        let normals = if board.textures.len() == 1 { vec![library.normal_map(&item.material, 512)] } else { Vec::new() };
        let shine = library.lighting(&item.material).map_or(24, crate::weapon::shine_of);
        let name = wheel_model(&item.material);
        let (lo, hi) = write_model_lit(dir, &name, &laid, &board.textures, false, shine, &normals)?;
        out.push((name, lo, hi));
    }
    if let Some(base) = library.items(Part::Wheels).iter().find(|i| i.label == skate_boards::paints::BASE) {
        let normals = vec![library.normal_map(&base.material, 512)];
        for paint in skate_boards::paints::catalog() {
            let mut board = library.part_mesh(Part::Wheels, &base.part_ref(), 512)?;
            for texture in &mut board.textures {
                texture.make_opaque();
                skate_boards::paints::tint(&mut texture.rgba, &paint);
            }
            let laid = one_wheel(&laid_in(&board, defaults, frame));
            let name = wheel_model(&paint.part_ref().material);
            let (lo, hi) = write_model_lit(dir, &name, &laid, &board.textures, false, paint.finish.shine(), &normals)?;
            out.push((name, lo, hi));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn one_of_four_wheels_is_kept_and_put_in_the_middle() {
        // Four small triangles, one at each wheel's place.
        let mut positions = Vec::new();
        for (y, z) in [(-0.1, 0.1), (0.1, 0.1), (-0.1, 0.6), (0.1, 0.6)] {
            positions.extend([Vec3::new(0., y, z), Vec3::new(0., y + 0.02, z), Vec3::new(0., y, z + 0.02)]);
        }
        let n = positions.len();
        let laid: Laid = vec![(positions, vec![Vec3::X; n], vec![Vec2::ZERO; n], (0..n as u32).collect(), 0)];
        let one = one_wheel(&laid);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].3.len(), 3);
        let c = one[0].3.iter().map(|i| one[0].0[*i as usize]).sum::<Vec3>() / 3.;
        assert!((c.z - 0.35).abs() < 0.01 && c.y.abs() < 0.01);
        assert_eq!(wheel_model("2C7F38"), "sk3_w1_2c7f38");
    }
}
