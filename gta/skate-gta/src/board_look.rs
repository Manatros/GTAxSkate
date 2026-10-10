//! Skate 3's board, drawn in GTA V.
//!
//! A script may draw triangles in the world: textured ones
//! (`DRAW_TEXTURED_POLY`) from the board textures the skater pack carries
//! (`skate_boards::gta_texture`), or, without the pack, flat-coloured ones
//! (`DRAW_POLY`): then the meshes are cut into small triangles wherever
//! the texture has detail (the deck's graphic), each coloured from it.
//! Either is skinned to the Skate pose every frame like the mashup skins
//! its board.

use bevy::math::{Mat4, Vec3, Vec4};
use skate_boards::Board;

use crate::coords::from_skate;

#[derive(Clone, Debug)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}

/// A drawn triangle: three points, each a blend of a source triangle's
/// corners, and its colour.
#[derive(Clone, Debug)]
struct Piece {
    corners: [u32; 3],
    bary: [[f32; 3]; 3],
    color: [u8; 3],
    /// Texture coordinates of its three points, and its texture (an index
    /// into `BoardLook::textures`).
    uv: [[f32; 2]; 3],
    texture: u16,
}

/// One triangle to draw this frame, in GTA world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Poly {
    pub points: [Vec3; 3],
    pub rgba: [u8; 4],
    /// For the textured board: texture coordinates, the texture (index into
    /// [`BoardLook::textures`]) and the light on it (0 to 1).
    pub uv: [[f32; 2]; 3],
    pub texture: u16,
    pub shade: f32,
}

pub struct BoardLook {
    joints: Vec<String>,
    /// Per joint: the converter's basis turn and the joint's inverse bind.
    inverse: Vec<Mat4>,
    vertices: Vec<Vertex>,
    pieces: Vec<Piece>,
    /// Per texture: its dictionary and name in GTA.
    textures: Vec<(std::ffi::CString, std::ffi::CString)>,
}

/// The lattice of a triangle cut `n` times along each side.
fn lattice(n: u32) -> Vec<[[f32; 3]; 3]> {
    let point = |i: u32, j: u32| {
        let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
        [1. - u - v, u, v]
    };
    let mut out = Vec::new();
    for i in 0..n {
        for j in 0..n - i {
            out.push([point(i, j), point(i + 1, j), point(i, j + 1)]);
            if j + 1 < n - i {
                out.push([point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)]);
            }
        }
    }
    out
}

fn blend<const N: usize>(corners: [[f32; N]; 3], bary: [f32; 3]) -> [f32; N] {
    std::array::from_fn(|k| corners[0][k] * bary[0] + corners[1][k] * bary[1] + corners[2][k] * bary[2])
}

impl BoardLook {
    /// Cuts `board` so that the texture shows at about `cells` triangles
    /// across its longest side, using at most `budget` triangles.
    pub fn new(board: &Board, cells: u32, budget: usize) -> Self {
        Self::build(board, cells, budget, true)
    }

    /// The board in its own triangles, for drawing it textured.
    pub fn textured(board: &Board) -> Self {
        Self::build(board, 1, usize::MAX, false)
    }

    /// The textures it is drawn with: (dictionary, name) each.
    pub fn textures(&self) -> &[(std::ffi::CString, std::ffi::CString)] {
        &self.textures
    }

    fn build(board: &Board, cells: u32, budget: usize, cut: bool) -> Self {
        let rb = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let mut look = Self {
            joints: board.joints.iter().map(|j| j.name.clone()).collect(),
            inverse: board.joints.iter().map(|j| rb * Mat4::from_cols_array(&j.inverse_bind)).collect(),
            vertices: Vec::new(),
            pieces: Vec::new(),
            textures: Vec::new(),
        };
        let mut texture_of = std::collections::HashMap::new();
        let surface_textures: Vec<u16> = board
            .surfaces
            .iter()
            .map(|surface| {
                let (dict, name) = skate_boards::gta_texture(surface.part, &surface.material);
                *texture_of.entry((dict.clone(), name.clone())).or_insert_with(|| {
                    look.textures.push((
                        std::ffi::CString::new(dict).unwrap_or_default(),
                        std::ffi::CString::new(name).unwrap_or_default(),
                    ));
                    (look.textures.len() - 1) as u16
                })
            })
            .collect();
        let mut cells = cells.max(1) as f32;
        loop {
            look.vertices.clear();
            look.pieces.clear();
            for (surface, &texture_index) in board.surfaces.iter().zip(&surface_textures) {
                let texture = &board.textures[surface.texture];
                let size = texture.width.max(texture.height).max(1) as f32;
                let cell = size / cells;
                let base = look.vertices.len() as u32;
                look.vertices.extend(surface.vertices.iter().map(|v| Vertex {
                    position: Vec3::from_array(v.position),
                    normal: Vec3::from_array(v.normal),
                    joints: v.joints.map(|j| j as u16),
                    weights: v.weights,
                }));
                for triangle in surface.indices.chunks_exact(3) {
                    let uv = [0, 1, 2].map(|k| surface.vertices[triangle[k] as usize].uv);
                    let texels = |a: [f32; 2], b: [f32; 2]| {
                        ((a[0] - b[0]) * texture.width as f32).hypot((a[1] - b[1]) * texture.height as f32)
                    };
                    let longest = texels(uv[0], uv[1]).max(texels(uv[1], uv[2])).max(texels(uv[2], uv[0]));
                    let mut n = if cut { ((longest / cell).ceil() as u32).clamp(1, 24) } else { 1 };
                    if n > 1 {
                        // A plain area needs no cuts.
                        let samples: Vec<[u8; 4]> = lattice(n.min(6))
                            .iter()
                            .flatten()
                            .map(|b| texture.sample(blend(uv, *b)))
                            .collect();
                        let spread = (0..3)
                            .map(|c| {
                                let (lo, hi) = samples
                                    .iter()
                                    .fold((255u8, 0u8), |(lo, hi), s| (lo.min(s[c]), hi.max(s[c])));
                                hi - lo
                            })
                            .max()
                            .unwrap_or(0);
                        if spread < 14 {
                            n = 1;
                        }
                    }
                    for bary in lattice(n) {
                        let centre = [0, 1, 2].map(|k| (bary[0][k] + bary[1][k] + bary[2][k]) / 3.);
                        let mut sum = [0u32; 3];
                        let mut count = 0;
                        for corner in [None, Some(0), Some(1), Some(2)] {
                            let at = match corner {
                                None => centre,
                                Some(c) => std::array::from_fn(|k| (centre[k] + bary[c][k]) * 0.5),
                            };
                            let s = texture.sample(blend(uv, at));
                            for c in 0..3 {
                                sum[c] += u32::from(s[c]);
                            }
                            count += 1;
                        }
                        look.pieces.push(Piece {
                            corners: [base + triangle[0], base + triangle[1], base + triangle[2]],
                            bary,
                            color: sum.map(|s| (s / count) as u8),
                            uv: bary.map(|b| blend(uv, b)),
                            texture: texture_index,
                        });
                    }
                }
            }
            if look.pieces.len() <= budget || cells <= 2. {
                return look;
            }
            cells = (cells * 0.8).max(2.);
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.pieces.len()
    }

    /// Each joint's bone in a Skate pose with bones `names`, or none when
    /// the pose lacks one.
    pub fn bind(&self, names: &[String]) -> Option<Vec<usize>> {
        self.joints.iter().map(|joint| names.iter().position(|n| n == joint)).collect()
    }

    /// The triangles facing `eye`, farthest first, for a Skate pose whose
    /// origin is GTA point `origin`, lit by `light` (0 to 1).
    pub fn polys(&self, bones: &[Mat4], bound: &[usize], origin: Vec3, eye: Vec3, light: f32) -> Vec<Poly> {
        let skinned: Vec<(Vec3, Vec3)> = self
            .skin(bones, bound)
            .into_iter()
            .map(|(position, normal)| (from_skate(position) + origin, from_skate(normal)))
            .collect();
        let mut out: Vec<(f32, Poly)> = Vec::with_capacity(self.pieces.len() / 2);
        for piece in &self.pieces {
            let corner = piece.corners.map(|c| skinned[c as usize]);
            let at = |b: [f32; 3]| corner[0].0 * b[0] + corner[1].0 * b[1] + corner[2].0 * b[2];
            let points = piece.bary.map(at);
            let mut normal = (points[1] - points[0]).cross(points[2] - points[0]);
            let hint = corner[0].1 + corner[1].1 + corner[2].1;
            if normal.dot(hint) < 0. {
                normal = -normal;
            }
            let normal = normal.normalize_or_zero();
            let centre = (points[0] + points[1] + points[2]) / 3.;
            if normal == Vec3::ZERO || normal.dot(eye - centre) <= 0. {
                continue;
            }
            let shade = light.clamp(0., 1.) * (0.45 + 0.55 * normal.z.max(0.));
            let rgba = [
                (f32::from(piece.color[0]) * shade) as u8,
                (f32::from(piece.color[1]) * shade) as u8,
                (f32::from(piece.color[2]) * shade) as u8,
                255,
            ];
            out.push((centre.distance_squared(eye), Poly { points, rgba, uv: piece.uv, texture: piece.texture, shade }));
        }
        out.sort_by(|a, b| b.0.total_cmp(&a.0));
        out.into_iter().map(|(_, poly)| poly).collect()
    }
}

/// A board at rest, for showing it on its own (the stores' preview):
/// bones for [`BoardLook::polys`] that hold it as the reference board's
/// bind pose does, and its centre and axes in Skate's space.
pub struct Rest {
    pub bones: Vec<Mat4>,
    pub bound: Vec<usize>,
    pub centre: Vec3,
    /// Its longest, middle and thinnest directions: along the deck, across
    /// it, and through it.
    pub axes: [Vec3; 3],
}

impl BoardLook {
    /// The board at rest, its parts placed by the bones of `reference`
    /// (the default skater's board), so parts from different models line
    /// up.
    pub fn rest(&self, reference: &Board) -> Rest {
        let rb = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let bones: Vec<Mat4> = self
            .joints
            .iter()
            .map(|name| {
                let world = reference
                    .joints
                    .iter()
                    .find(|j| j.name == *name)
                    .map(|j| Mat4::from_cols_array(&j.inverse_bind))
                    .filter(|m| m.determinant().abs() > 1e-9)
                    .map_or(Mat4::IDENTITY, |m| m.inverse());
                world * rb.inverse()
            })
            .collect();
        let bound: Vec<usize> = (0..bones.len()).collect();
        let points = self.skin(&bones, &bound);
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        for (p, _) in &points {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let size = (hi - lo).max(Vec3::ZERO);
        let mut order = [0usize, 1, 2];
        order.sort_by(|a, b| size[*b].total_cmp(&size[*a]));
        let unit = |i: usize| {
            let mut v = Vec3::ZERO;
            v[i] = 1.;
            v
        };
        Rest {
            bones,
            bound,
            centre: if lo.is_finite() && hi.is_finite() { (lo + hi) * 0.5 } else { Vec3::ZERO },
            axes: order.map(unit),
        }
    }

    /// Each vertex skinned to `bones`, in Skate space, with its normal.
    fn skin(&self, bones: &[Mat4], bound: &[usize]) -> Vec<(Vec3, Vec3)> {
        let matrices: Vec<Mat4> = bound
            .iter()
            .zip(&self.inverse)
            .map(|(&bone, inverse)| bones.get(bone).copied().unwrap_or(Mat4::IDENTITY) * *inverse)
            .collect();
        self.vertices
            .iter()
            .map(|v| {
                let mut position = Vec3::ZERO;
                let mut normal = Vec3::ZERO;
                for k in 0..4 {
                    if v.weights[k] == 0. {
                        continue;
                    }
                    let m = matrices.get(usize::from(v.joints[k])).copied().unwrap_or(Mat4::IDENTITY);
                    position += m.transform_point3(v.position) * v.weights[k];
                    normal += m.transform_vector3(v.normal) * v.weights[k];
                }
                (position, normal)
            })
            .collect()
    }
}

/// The turn that shows a board at rest beside the camera: its length
/// along `right`, its face turned `angle` radians over from facing
/// `toward` (all Skate-space directions), centred on the origin.
pub fn showcase(rest: &Rest, right: Vec3, up: Vec3, toward: Vec3, angle: f32) -> Vec<Mat4> {
    use bevy::math::{Mat3, Quat};
    let [long, wide, thin] = rest.axes;
    let roll = Quat::from_axis_angle(right.normalize_or(Vec3::X), angle);
    let target = Mat3::from_cols(right, roll * up, roll * toward);
    let mut source = Mat3::from_cols(long, wide, thin);
    if source.determinant() * target.determinant() < 0. {
        source = Mat3::from_cols(long, -wide, thin);
    }
    let turn = Mat4::from_mat3(target * source.transpose()) * Mat4::from_translation(-rest.centre);
    rest.bones.iter().map(|b| turn * *b).collect()
}

/// Daylight for the board's colours by the hour of the game's clock.
pub fn daylight(hour: i32) -> f32 {
    match hour {
        7..=18 => 1.0,
        6 | 19 => 0.75,
        5 | 20 => 0.55,
        _ => 0.4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate_boards::{Joint, Part, Surface, Texture, Vertex as BoardVertex};

    fn identity() -> [f32; 16] {
        Mat4::IDENTITY.to_cols_array()
    }

    /// A unit square on the converter's XY plane, facing +Z, with a texture
    /// that is plain on the left half and striped on the right.
    fn square(striped: bool) -> Board {
        let v = |x: f32, y: f32| BoardVertex {
            position: [x, y, 0.],
            normal: [0., 0., 1.],
            uv: [x, y],
            joints: [0; 4],
            weights: [1., 0., 0., 0.],
        };
        let mut rgba = Vec::new();
        for _y in 0..64 {
            for x in 0..64 {
                let c = if striped && x % 8 < 4 { 250 } else { 10 };
                rgba.extend([c, c, c, 255]);
            }
        }
        Board {
            joints: vec![Joint { name: "DECK".into(), inverse_bind: identity() }],
            surfaces: vec![Surface {
                part: Part::Deck,
                material: "m".into(),
                texture: 0,
                vertices: vec![v(0., 0.), v(1., 0.), v(1., 1.), v(0., 1.)],
                indices: vec![0, 1, 2, 0, 2, 3],
            }],
            textures: vec![Texture { width: 64, height: 64, rgba }],
        }
    }

    #[test]
    fn the_textured_board_keeps_its_own_triangles_and_coordinates() {
        let mut board = square(true);
        board.surfaces[0].material = "iw4l_skate/board/00ABCDEF".into();
        let look = BoardLook::textured(&board);
        assert_eq!(look.triangle_count(), 2, "no cuts");
        assert_eq!(look.textures().len(), 1);
        assert_eq!(look.textures()[0].0.to_str().unwrap(), "skate3_board_deck");
        assert_eq!(look.textures()[0].1.to_str().unwrap(), "00abcdef");
        let corners: Vec<[f32; 2]> = look.pieces.iter().flat_map(|p| p.uv).collect();
        assert!(corners.contains(&[1., 1.]) && corners.contains(&[0., 0.]));
        // The flat look still cuts the striped texture.
        assert!(BoardLook::new(&board, 16, 10_000).triangle_count() > 2);
    }

    #[test]
    fn lattices_cover_the_triangle() {
        for n in 1..6 {
            let pieces = lattice(n);
            assert_eq!(pieces.len() as u32, n * n);
            for piece in pieces {
                for b in piece {
                    assert!((b.iter().sum::<f32>() - 1.).abs() < 1e-5 && b.iter().all(|&x| x >= -1e-6));
                }
            }
        }
    }

    #[test]
    fn detail_goes_where_the_texture_has_it() {
        assert_eq!(BoardLook::new(&square(false), 16, 5000).triangle_count(), 2, "a plain texture is not cut");
        let striped = BoardLook::new(&square(true), 16, 5000);
        assert!(striped.triangle_count() > 100, "{}", striped.triangle_count());
        assert!(BoardLook::new(&square(true), 16, 60).triangle_count() <= 60, "kept within the budget");
    }

    #[test]
    fn faces_away_from_the_eye_are_left_out() {
        let look = BoardLook::new(&square(false), 4, 100);
        let bound = look.bind(&["ROOT".into(), "DECK".into()]).unwrap();
        assert_eq!(bound, vec![1]);
        assert!(look.bind(&["ROOT".into()]).is_none());
        let bones = vec![Mat4::IDENTITY; 2];
        // The converter's +Z turns to Skate's up (rb), GTA's +Z: the square
        // lies on the ground facing the sky.
        let facing = look.polys(&bones, &bound, Vec3::ZERO, Vec3::new(0.5, 0.5, 5.), 1.);
        assert_eq!(facing.len(), 2);
        assert!(facing.iter().flat_map(|p| p.points).all(|p| p.z.abs() < 1e-5));
        let away = look.polys(&bones, &bound, Vec3::ZERO, Vec3::new(0.5, 0.5, -5.), 1.);
        assert!(away.is_empty());
        let origin = Vec3::new(100., 200., 30.);
        let moved = look.polys(&bones, &bound, origin, origin + Vec3::new(0.5, 0.5, 5.), 1.);
        assert!(moved[0].points.iter().all(|p| p.distance(origin) < 2.));
        assert_eq!(moved[0].rgba[3], 255);
    }

    #[test]
    fn a_board_at_rest_shows_beside_the_camera() {
        let board = square(true);
        let look = BoardLook::new(&board, 8, 500);
        let rest = look.rest(&board);
        // A unit square in the converter's XY plane, as the converter has it.
        assert!((rest.centre - Vec3::new(0.5, 0.5, 0.)).length() < 1e-4, "{:?}", rest.centre);
        assert_eq!(rest.axes[2], Vec3::Z, "the thin axis last");
        // Shown with its length along GTA's +X and its face to the camera
        // on GTA's -Y, it stands up in the XZ plane around the origin.
        let (right, up, toward) = (Vec3::X, Vec3::Y, Vec3::Z);
        let bones = showcase(&rest, right, up, toward, 0.);
        let eye = Vec3::new(0., -3., 0.);
        let polys = look.polys(&bones, &rest.bound, Vec3::ZERO, eye, 1.);
        assert!(!polys.is_empty());
        for p in polys.iter().flat_map(|p| p.points) {
            assert!(p.y.abs() < 1e-4 && p.x.abs() <= 0.5001 && p.z.abs() <= 0.5001, "{p:?}");
        }
        // Edge on after a quarter turn: it lies flat in the XY plane.
        let edge = showcase(&rest, right, up, toward, std::f32::consts::FRAC_PI_2);
        let above = Vec3::new(0., 0., 3.);
        for p in look.polys(&edge, &rest.bound, Vec3::ZERO, above, 1.).iter().flat_map(|p| p.points) {
            assert!(p.z.abs() < 1e-4, "{p:?}");
        }
    }

    #[test]
    fn night_dims_the_board() {
        assert!(daylight(12) > daylight(21));
        assert!(daylight(3) > 0.);
    }
}
