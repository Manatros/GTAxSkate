//! GTA V's skate spots: where they are (for the map) and their exact
//! collision. Probing the ground straight down makes a bowl's walls and a
//! ramp's curve into small steps the board catches on; in a park the
//! game's own collision triangles (`get-skate-parks.ps1` exports them from
//! the map's .ybn files and the ramp props' bounds, `<assets>\parks\<id>.tri`)
//! take the place of the probed ground, and their sharp top edges (coping,
//! ledges, rails) are rails to grind.

use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;
use std::path::Path;

pub struct Park {
    pub id: &'static str,
    pub name: &'static str,
    pub centre: Vec2,
    /// The exported collision's reach round the centre, metres.
    pub radius: f32,
}

/// The spots, found in the map's data (the skatepark models and where the
/// map places the `prop_skate_*` ramps).
pub const PARKS: &[Park] = &[
    Park { id: "vespucci", name: "Vespucci Beach Skatepark", centre: Vec2::new(-1374., -1396.), radius: 60. },
    Park { id: "hawick", name: "Hawick Skate Spot", centre: Vec2::new(280., -206.), radius: 30. },
    Park { id: "littleseoul", name: "Little Seoul Skate Spot", centre: Vec2::new(-950., -786.), radius: 30. },
    Park { id: "strawberry", name: "Strawberry Halfpipe", centre: Vec2::new(-266., -1571.), radius: 30. },
    Park { id: "lamesa", name: "La Mesa Rails", centre: Vec2::new(718., -1231.), radius: 30. },
];

/// Size of the cells that say where a park's collision is, metres.
const CELL: f32 = 1.;

/// One park's collision, in the world.
pub struct ParkMesh {
    pub triangles: Vec<[Vec3; 3]>,
    pub rails: Vec<Vec<Vec3>>,
    /// The cells its triangles cover, with the heights they span there:
    /// probed ground at those heights goes (ground well above or below,
    /// a street under a rooftop park, stays).
    covered: HashMap<(i32, i32), (f32, f32)>,
    /// The same for its steep faces (a bowl's walls, a ramp's face): where
    /// the park has its own, a probed wall is not wanted either.
    steep: HashMap<(i32, i32), (f32, f32)>,
}

/// Whether `p` is in the triangle `a b c` (either winding).
fn inside(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let (d1, d2, d3) = ((b - a).perp_dot(p - a), (c - b).perp_dot(p - b), (a - c).perp_dot(p - c));
    !((d1 < 0. || d2 < 0. || d3 < 0.) && (d1 > 0. || d2 > 0. || d3 > 0.))
}

fn cell(p: Vec2) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32)
}

impl ParkMesh {
    /// Reads `SKPK`, a triangle count and the triangles (9 little-endian
    /// f32 each); none when the file is not one.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.get(0..4)? != b"SKPK" {
            return None;
        }
        let count = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
        let body = bytes.get(8..8 + count * 36)?;
        let floats: Vec<f32> = body.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let triangles: Vec<[Vec3; 3]> = floats
            .chunks_exact(9)
            .map(|t| [Vec3::new(t[0], t[1], t[2]), Vec3::new(t[3], t[4], t[5]), Vec3::new(t[6], t[7], t[8])])
            // Slivers out: their normals are noise to Skate's physics.
            .filter(|t| t.iter().all(|p| p.is_finite()) && (t[1] - t[0]).cross(t[2] - t[0]).length_squared() > 1e-6)
            .collect();
        Some(Self::new(triangles))
    }

    pub fn new(triangles: Vec<[Vec3; 3]>) -> Self {
        // The cells a triangle lies over (seen from above): ones its corners
        // are in, and ones whose middle it covers. Not its whole bounding
        // box: a long slanting triangle's box is mostly empty, and the
        // probed ground dropped there left holes to fall through.
        let mut covered: HashMap<(i32, i32), (f32, f32)> = HashMap::new();
        let mut steep: HashMap<(i32, i32), (f32, f32)> = HashMap::new();
        for t in &triangles {
            let (z0, z1) = (t[0].z.min(t[1].z).min(t[2].z), t[0].z.max(t[1].z).max(t[2].z));
            let up = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero().z.abs();
            if up < 0.7 {
                // Every cell along it, its height span.
                let (a, b) = (t[0].min(t[1]).min(t[2]).truncate(), t[0].max(t[1]).max(t[2]).truncate());
                let (lo, hi) = (cell(a), cell(b));
                for i in lo.0..=hi.0 {
                    for j in lo.1..=hi.1 {
                        let span = steep.entry((i, j)).or_insert((z0, z1));
                        *span = (span.0.min(z0), span.1.max(z1));
                    }
                }
            }
            let mut cover = |c: (i32, i32)| {
                let span = covered.entry(c).or_insert((z0, z1));
                *span = (span.0.min(z0), span.1.max(z1));
            };
            let [a, b, c] = t.map(|p| p.truncate());
            if (b - a).perp_dot(c - a).abs() < 1e-4 {
                continue; // a wall: nothing under it to replace
            }
            for p in [a, b, c] {
                cover(cell(p));
            }
            let (lo, hi) = (cell(a.min(b).min(c)), cell(a.max(b).max(c)));
            for i in lo.0..=hi.0 {
                for j in lo.1..=hi.1 {
                    let middle = Vec2::new((i as f32 + 0.5) * CELL, (j as f32 + 0.5) * CELL);
                    if inside(middle, a, b, c) {
                        cover((i, j));
                    }
                }
            }
        }
        let rails = rails_of(&triangles);
        Self { triangles, rails, covered, steep }
    }

    pub fn load(path: &Path) -> Option<Self> {
        Self::parse(&std::fs::read(path).ok()?)
    }

    /// Whether the park's collision is at `p`, within a metre of its
    /// height (the probed ground there is not wanted).
    pub fn covers(&self, p: Vec3) -> bool {
        self.covered.get(&cell(p.truncate())).is_some_and(|(lo, hi)| p.z >= lo - 1. && p.z <= hi + 1.)
    }

    /// Whether the park has steep faces of its own at `p` (within a metre of
    /// their height): a probed wall there is the park's, already sent.
    pub fn has_wall(&self, p: Vec3) -> bool {
        self.steep.get(&cell(p.truncate())).is_some_and(|(lo, hi)| p.z >= lo - 1. && p.z <= hi + 1.)
    }

    /// Its triangles within `radius` of `centre` (on the ground plane).
    pub fn near(&self, centre: Vec3, radius: f32) -> impl Iterator<Item = &[Vec3; 3]> {
        let c = centre.truncate();
        self.triangles.iter().filter(move |t| {
            let lo = t[0].min(t[1]).min(t[2]).truncate();
            let hi = t[0].max(t[1]).max(t[2]).truncate();
            c.clamp(lo, hi).distance(c) <= radius
        })
    }

    /// Its rails with a point within `radius` of `centre`.
    pub fn rails_near(&self, centre: Vec3, radius: f32) -> impl Iterator<Item = &Vec<Vec3>> {
        let c = centre.truncate();
        self.rails.iter().filter(move |r| r.iter().any(|p| p.truncate().distance(c) <= radius))
    }
}

/// Every park's collision found in `dir` (`<id>.tri`).
pub fn load_all(dir: &Path) -> Vec<(&'static Park, ParkMesh)> {
    PARKS.iter().filter_map(|park| ParkMesh::load(&dir.join(format!("{}.tri", park.id))).map(|mesh| (park, mesh))).collect()
}

/// Rails along the sharp top edges: an edge two triangles share where the
/// surface folds down by more than 50 degrees (coping, a ledge's lip, a
/// rail's top), with a walkable face on one side and running within 25
/// degrees of level; edges meeting in line are joined, and only rails at
/// least 0.8 m long are kept.
pub fn rails_of(triangles: &[[Vec3; 3]]) -> Vec<Vec<Vec3>> {
    const MIN_FOLD: f32 = 0.64; // cos 50
    const MAX_SLOPE: f32 = 0.42; // sin 25
    const MIN_LENGTH: f32 = 0.8;
    let key = |p: Vec3| ((p.x * 50.).round() as i64, (p.y * 50.).round() as i64, (p.z * 50.).round() as i64);
    let mut edges: HashMap<((i64, i64, i64), (i64, i64, i64)), Vec<usize>> = HashMap::new();
    let normals: Vec<Vec3> = triangles.iter().map(|t| (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero()).collect();
    for (i, t) in triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (key(t[k]), key(t[(k + 1) % 3]));
            edges.entry(if a < b { (a, b) } else { (b, a) }).or_default().push(i);
        }
    }
    let mut pieces: Vec<(Vec3, Vec3)> = Vec::new();
    for (&(ka, kb), sharing) in &edges {
        let [i, j] = sharing[..] else { continue };
        let (ni, nj) = (normals[i], normals[j]);
        if ni.dot(nj) > MIN_FOLD || ni.z.max(nj.z) < 0.7 {
            continue;
        }
        let point = |k: (i64, i64, i64)| Vec3::new(k.0 as f32, k.1 as f32, k.2 as f32) / 50.;
        let (a, b) = (point(ka), point(kb));
        let along = b - a;
        if along.length() < 0.05 || (along.z / along.length()).abs() > MAX_SLOPE {
            continue;
        }
        // Convex: each triangle lies below the other's plane.
        let middle = (a + b) * 0.5;
        let centre = |t: usize| (triangles[t][0] + triangles[t][1] + triangles[t][2]) / 3.;
        if (centre(j) - middle).dot(ni) > 0.01 || (centre(i) - middle).dot(nj) > 0.01 {
            continue;
        }
        pieces.push((a, b));
    }
    // Join pieces end to end while they run on in line.
    let mut rails: Vec<Vec<Vec3>> = Vec::new();
    let mut used = vec![false; pieces.len()];
    let near = |p: Vec3, q: Vec3| p.distance(q) < 0.03;
    for start in 0..pieces.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut rail = vec![pieces[start].0, pieces[start].1];
        for forward in [true, false] {
            loop {
                let (end, before) = if forward { (rail[rail.len() - 1], rail[rail.len() - 2]) } else { (rail[0], rail[1]) };
                let heading = (end - before).normalize_or_zero();
                let next = (0..pieces.len()).find(|&k| {
                    !used[k] && {
                        let (a, b) = pieces[k];
                        let other = if near(a, end) { Some(b) } else if near(b, end) { Some(a) } else { None };
                        other.is_some_and(|o| (o - end).normalize_or_zero().dot(heading) > 0.9)
                    }
                });
                let Some(k) = next else { break };
                used[k] = true;
                let (a, b) = pieces[k];
                let other = if near(a, end) { b } else { a };
                if forward {
                    rail.push(other);
                } else {
                    rail.insert(0, other);
                }
            }
        }
        let length: f32 = rail.windows(2).map(|w| w[0].distance(w[1])).sum();
        if length >= MIN_LENGTH {
            rails.push(rail);
        }
    }
    rails
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ledge 2 m long, 0.5 m high and 1 m deep on flat ground: its top,
    /// its front face, and the ground before it.
    fn ledge() -> Vec<[Vec3; 3]> {
        let p = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        vec![
            // top (y 0..1, z 0.5)
            [p(0., 0., 0.5), p(2., 0., 0.5), p(2., 1., 0.5)],
            [p(0., 0., 0.5), p(2., 1., 0.5), p(0., 1., 0.5)],
            // front face (y 0, z 0..0.5), facing -y
            [p(0., 0., 0.), p(2., 0., 0.), p(2., 0., 0.5)],
            [p(0., 0., 0.), p(2., 0., 0.5), p(0., 0., 0.5)],
            // ground before it
            [p(0., -2., 0.), p(2., -2., 0.), p(2., 0., 0.)],
            [p(0., -2., 0.), p(2., 0., 0.), p(0., 0., 0.)],
        ]
    }

    #[test]
    fn a_ledges_lip_is_a_rail_and_its_foot_is_not() {
        let rails = rails_of(&ledge());
        assert_eq!(rails.len(), 1, "{rails:?}");
        let rail = &rails[0];
        assert!(rail.iter().all(|p| (p.z - 0.5).abs() < 0.02 && p.y.abs() < 0.02), "along the top front edge: {rail:?}");
        let length: f32 = rail.windows(2).map(|w| w[0].distance(w[1])).sum();
        assert!((length - 2.).abs() < 0.05);
    }

    #[test]
    fn park_files_read_back_and_cover_their_ground() {
        let mut bytes = b"SKPK".to_vec();
        let triangles = ledge();
        bytes.extend((triangles.len() as u32).to_le_bytes());
        for t in &triangles {
            for p in t {
                for v in p.to_array() {
                    bytes.extend(v.to_le_bytes());
                }
            }
        }
        let mesh = ParkMesh::parse(&bytes).unwrap();
        assert_eq!(mesh.triangles.len(), 6);
        assert!(mesh.covers(Vec3::new(1., 0.5, 0.5)) && mesh.covers(Vec3::new(1., -1.5, 0.)));
        assert!(!mesh.covers(Vec3::new(10., 10., 0.)));
        assert!(!mesh.covers(Vec3::new(1., -1.5, -8.)), "a street 8 m under the park keeps its ground");
        assert_eq!(mesh.near(Vec3::new(1., 0.5, 0.), 1.).count(), 6);
        assert_eq!(mesh.near(Vec3::new(20., 0., 0.), 5.).count(), 0);
        assert!(ParkMesh::parse(b"nope").is_none());
    }

    #[test]
    fn a_slanting_triangle_covers_only_the_ground_under_it() {
        // Corner to corner of a 10 m square: half the square is not under it.
        let mesh = ParkMesh::new(vec![[Vec3::new(0., 0., 0.), Vec3::new(10., 10., 0.), Vec3::new(10., 9., 0.)]]);
        assert!(mesh.covers(Vec3::new(9.5, 9.2, 0.)) && mesh.covers(Vec3::new(5.5, 5.3, 0.)));
        assert!(!mesh.covers(Vec3::new(1.5, 8.5, 0.)), "the box's empty corner keeps its probed ground");
    }

    #[test]
    fn a_bowls_wall_is_the_parks_and_a_building_beside_it_is_not() {
        // A bowl's side near its top: steep, from (0,0,0) up to (1,0,3) along x.
        let p = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        let mesh = ParkMesh::new(vec![[p(0., 0., 0.), p(1., 0., 3.), p(1., 2., 3.)], [p(0., 0., 0.), p(1., 2., 3.), p(0., 2., 0.)], [p(3., 0., 2.), p(9., 0., 2.), p(9., 6., 2.)]]);
        assert!(mesh.has_wall(p(0.5, 1., 1.5)), "the bowl's side");
        assert!(!mesh.has_wall(p(8.5, 0.5, 2.5)), "a flat roof: a building's wall there stays");
    }

    #[test]
    fn every_park_has_its_own_file_name() {
        let mut ids: Vec<&str> = PARKS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PARKS.len());
    }

    #[test]
    #[ignore = "reads the exported parks"]
    fn real_parks_load() {
        let dir = Path::new(r"E:\Games\2010-Rust-Rewrite-Mashup\skate-data\assets\parks");
        for (park, mesh) in load_all(dir) {
            let length: f32 = mesh.rails.iter().flat_map(|r| r.windows(2)).map(|w| w[0].distance(w[1])).sum();
            println!("{}: {} triangles, {} rails ({length:.0} m)", park.id, mesh.triangles.len(), mesh.rails.len());
        }
    }
}
