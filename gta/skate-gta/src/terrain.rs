//! The world around the skater, from ray probes, as Skate collision and
//! grind rails.
//!
//! GTA V gives scripts no collision meshes, only ray probes. Straight-down
//! probes on a world-aligned grid give the surface heights; where two
//! neighbouring probes disagree by more than a slope would, the step
//! between them is located by bisection to about a centimetre. The mesh is
//! a height field whose stepped cells split at those located edges into
//! flat tops and vertical walls, so curbs, ledges and walls stand where
//! they are rather than becoming ramps. The located edges whose high side
//! is walkable and drops away are lips; lips along a line become grind
//! rails, including diagonal and curved ledges. Probes that stand well
//! above all their neighbours are thin things (handrails, fence tops):
//! they are left out of the mesh and, where they line up, become rails.

use bevy::math::{Vec2, Vec3};
use std::collections::{HashMap, HashSet};

/// The first surface a downward probe met.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub z: f32,
    /// The surface normal's up component: 1 flat, 0 a wall.
    pub normal_z: f32,
    /// Skate's ground for the surface's material (see `crate::ground`); 0
    /// unknown.
    pub ground: u8,
}

pub trait Probe {
    /// The first surface straight down from `(x, y, top)` to `(x, y, bottom)`.
    fn down(&mut self, x: f32, y: f32, top: f32, bottom: f32) -> Option<Hit>;
    /// Where the line from `from` to `to` first meets something, if it does.
    fn across(&mut self, _from: Vec3, _to: Vec3) -> Option<Vec3> {
        None
    }
    /// Whether the last probe failed to run (GTA out of shape tests): no
    /// answer, not "nothing there". Cleared by asking.
    fn failed(&mut self) -> bool {
        false
    }
    /// Starts a wide probe: a capsule of `radius` swept from `from` to `to`
    /// (GTA answers those a frame later). Its id, or none if unsupported.
    fn wide_start(&mut self, _from: Vec3, _to: Vec3, _radius: f32) -> Option<i32> {
        None
    }
    /// A wide probe's answer: none while pending, then where it first
    /// touched something, if it did.
    fn wide_result(&mut self, _id: i32) -> Option<Option<Vec3>> {
        Some(None)
    }
}

/// A height change between neighbours bigger than this is a step, not a slope.
pub const STEP: f32 = 0.06;
/// On foot, a step up to this high is walked up as a slope: Skate's
/// walker climbed every stair on its hands.
pub const STAIR: f32 = 0.25;
/// A lip must drop at least this much to be grindable.
pub const MIN_DROP: f32 = 0.08;
/// Surfaces at least this flat can be ridden on and ground along.
pub const WALKABLE: f32 = 0.65;
/// A probe this far above all four neighbours hit something thin.
pub const THIN: f32 = 0.25;
/// Probes start this far above the skater's feet and end this far below.
pub const PROBE_UP: f32 = 2.2;
/// Rails higher than this above the ground beside them are out of an
/// ollie's reach: fence and wall tops. Skate's grind assist pulled the
/// skater up onto them (the maintainer: "magnetisiert", video 19-54 10:57).
pub const MAX_RAIL_HEIGHT: f32 = 1.3;
/// Handrails are followed from where a probe first met them, in steps of
/// this length, looking this far either side for the rail again.
const TRACE_STEP: f32 = 0.2;
const TRACE_SIDE: [f32; 9] = [0., 0.03, -0.03, 0.06, -0.06, 0.1, -0.1, 0.14, -0.14];
/// A traced rail ends after this many steps each way (12 m).
const TRACE_STEPS: usize = 60;
/// A rail stands at least this much above the ground either side of it
/// (train tracks 0.15 m, a handrail far more).
const RAIL_RISE: f32 = 0.07;
/// The wide probes' radius: the grid's spacing is 0.2 m, so together they
/// leave no gap a 4 cm rail could pass through.
const WIDE_RADIUS: f32 = 0.12;
/// Wide probes waiting for an answer at most: GTA has few shape tests,
/// and with hundreds waiting its ground probes failed.
const WIDE_PER_SCAN: usize = 6;
pub const PROBE_DOWN: f32 = 5.0;
/// Cells probed from a height this far from the feet are probed again.
const RESAMPLE_HEIGHT: f32 = 1.2;
/// A wall taller than [`PROBE_UP`] holds the probe's start, so the probe
/// finds the ground beyond it. A cell at about the feet's height is looked
/// at sideways from the feet at this height too; something right at the
/// cell makes it a wall up to the probes' start.
const WALL_LOOK: f32 = 1.0;
/// Cells within this height of the feet are looked at sideways.
const WALL_LEVEL: f32 = 0.5;
/// A wall cell is looked at again once the feet moved this far from where
/// it was seen.
const WALL_RESEE: f32 = 1.0;
/// Bisection steps locating a step: spacing / 2^5, 6 mm at 0.2 m.
const REFINE: u32 = 5;
const MIN_LIP_RAIL: f32 = 0.5;
const MIN_THIN_RAIL: f32 = 1.0;
/// Rails split where they turn more than about 35 degrees.
const MIN_TURN_COS: f32 = 0.82;
const SIMPLIFY: f32 = 0.02;

#[derive(Clone, Copy, Debug)]
struct Cell {
    hit: Option<Hit>,
    from_z: f32,
    /// Seen as a tall wall from these feet (see [`WALL_LOOK`]).
    wall_from: Option<Vec2>,
}

/// The skater's surroundings: GTA world coordinates, metres, Z up.
pub struct Terrain {
    spacing: f32,
    cells: HashMap<(i32, i32), Cell>,
    /// Where the step between a cell and its +X (0) or +Y (1) neighbour
    /// lies, as the fraction of the way from the cell.
    crossings: HashMap<(i32, i32, u8), f32>,
    revision: u64,
    scans: u32,
    /// Look at cells sideways for walls taller than the probes' start
    /// ([`WALL_LOOK`]): the config's `TallWalls`, about 1 ms a frame.
    tall_walls: bool,
    /// Points along handrails followed from a thin hit (see `trace`), and
    /// the hits still to follow.
    rail_points: Vec<Vec<Vec3>>,
    seeds: Vec<Vec3>,
    /// Cells still to look at with a wide probe (point and the line
    /// probe's ground there), and the wide probes waiting for an answer.
    wide_queue: std::collections::VecDeque<(Vec2, f32)>,
    wide_pending: Vec<(i32, Vec2, f32)>,
    /// Wide probes that touched something above the ground, to look at,
    /// and the touches already looked at that were no rail (a wall, a kerb).
    wide_touches: Vec<(Vec3, f32)>,
    looked: Vec<Vec3>,
}

/// Collision triangles (counterclockwise seen from outside) and grind rails,
/// in GTA world coordinates.
#[derive(Default)]
pub struct Mesh {
    pub triangles: Vec<[Vec3; 3]>,
    /// Skate's ground under each triangle, parallel to `triangles`.
    pub surfaces: Vec<u8>,
    pub rails: Vec<Vec<Vec3>>,
}

impl Mesh {
    fn push(&mut self, triangle: [Vec3; 3], ground: u8) {
        self.triangles.push(triangle);
        self.surfaces.push(ground);
    }
}

/// Whether neighbours `spacing` apart at heights `a` and `b` lie on one
/// slope (a ramp, a quarter pipe's curve) rather than either side of a
/// step: the surfaces' own tilt explains the difference in height.
fn sloped(a: (f32, f32), b: (f32, f32), spacing: f32) -> bool {
    let rise = |normal_z: f32| {
        let n = normal_z.clamp(0.05, 1.);
        (1. - n * n).sqrt() / n
    };
    // A flat top and floor either side of a step have no tilt to explain
    // it; both ends of a slope lean.
    let tilt = rise(a.1).min(rise(b.1));
    tilt > 0.08 && (a.0 - b.0).abs() <= spacing * rise(a.1).max(rise(b.1)) * 1.4 + 0.03
}

/// The ground most of `corners` stand on (the first of a tie).
fn majority(corners: [u8; 4]) -> u8 {
    let count = |g: u8| corners.iter().filter(|c| **c == g).count();
    corners.iter().copied().max_by_key(|g| (count(*g), std::cmp::Reverse(corners.iter().position(|c| c == g)))).unwrap_or(0)
}

/// The cells `ring` steps from the centre, as offsets.
fn ring_cells(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    let side = (-ring..=ring).flat_map(move |a| [(a, -ring), (a, ring)]);
    let ends = (-ring + 1..ring).flat_map(move |b| [(-ring, b), (ring, b)]);
    let centre = (ring == 0).then_some((0, 0));
    centre.into_iter().chain(side.filter(move |_| ring > 0)).chain(ends)
}

impl Terrain {
    pub fn new(spacing: f32) -> Self {
        Self {
            spacing,
            cells: HashMap::new(),
            crossings: HashMap::new(),
            revision: 0,
            scans: 0,
            tall_walls: false,
            rail_points: Vec::new(),
            seeds: Vec::new(),
            wide_queue: std::collections::VecDeque::new(),
            wide_pending: Vec::new(),
            wide_touches: Vec::new(),
            looked: Vec::new(),
        }
    }

    pub fn with_tall_walls(mut self, on: bool) -> Self {
        self.tall_walls = on;
        self
    }

    /// Changes whenever a probe adds or replaces a cell.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.crossings.clear();
        self.revision += 1;
    }

    fn point(&self, (i, j): (i32, i32)) -> Vec2 {
        Vec2::new(i as f32, j as f32) * self.spacing
    }

    fn cell_of(&self, p: Vec3) -> (i32, i32) {
        ((p.x / self.spacing).round() as i32, (p.y / self.spacing).round() as i32)
    }

    /// Probes the cells within `radius` of `feet` that are missing or were
    /// probed from another height, nearest first, while `budget` allows.
    /// Returns how many probes it made.
    pub fn scan(&mut self, probe: &mut impl Probe, feet: Vec3, radius: f32, mut budget: impl FnMut() -> bool) -> usize {
        let centre = self.cell_of(feet);
        let reach = (radius / self.spacing).ceil() as i32;
        let reach2 = (radius / self.spacing).powi(2);
        let mut probes = 0;
        'rings: for ring in 0..=reach {
            for (di, dj) in ring_cells(ring) {
                if (di * di + dj * dj) as f32 > reach2 {
                    continue;
                }
                let key = (centre.0 + di, centre.1 + dj);
                if self.cells.get(&key).is_some_and(|c| {
                    (c.from_z - feet.z).abs() <= RESAMPLE_HEIGHT
                        && c.wall_from.is_none_or(|w| w.distance(feet.truncate()) <= WALL_RESEE)
                }) {
                    continue;
                }
                if !budget() {
                    break 'rings;
                }
                let p = self.point(key);
                let mut hit = probe.down(p.x, p.y, feet.z + PROBE_UP, feet.z - PROBE_DOWN);
                probes += 1;
                // A probe GTA could not run is asked again next time: taken
                // for "no ground", it left holes the skater fell through.
                if probe.failed() {
                    continue;
                }
                let mut wall_from = None;
                if self.tall_walls
                    && let Some(h) = hit
                    && (h.z - feet.z).abs() <= WALL_LEVEL
                    && p.distance(feet.truncate()) > self.spacing
                {
                    probes += 1;
                    let eye = feet + Vec3::Z * WALL_LOOK;
                    if probe
                        .across(eye, p.extend(feet.z + WALL_LOOK))
                        .is_some_and(|at| at.truncate().distance(p) <= self.spacing)
                    {
                        hit = Some(Hit { z: feet.z + PROBE_UP, normal_z: 1., ground: h.ground });
                        wall_from = Some(feet.truncate());
                    }
                }
                if let Some(h) = hit {
                    self.wide_queue.push_back((p, h.z));
                }
                self.cells.insert(key, Cell { hit, from_z: feet.z, wall_from });
                self.revision += 1;
                for edge in [(key.0, key.1, 0), (key.0, key.1, 1), (key.0 - 1, key.1, 0), (key.0, key.1 - 1, 1)] {
                    self.crossings.remove(&edge);
                    probes += self.refine(probe, edge);
                }
                for (di, dj) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if let Some(seed) = self.thin_between((key.0 + di, key.1 + dj)) {
                        self.seeds.push(seed);
                    }
                }
            }
        }
        probes += self.wide_probes(probe);
        // Follow the handrails met, as long as the budget allows.
        while let Some(seed) = self.seeds.pop() {
            if self.rail_points.iter().flatten().any(|p| p.distance(seed) < 0.25) {
                continue;
            }
            if !budget() {
                self.seeds.push(seed);
                break;
            }
            probes += self.trace(probe, seed);
        }
        self.scans += 1;
        if self.scans % 64 == 0 {
            let keep = (2.5 * radius / self.spacing).powi(2);
            self.cells.retain(|&(i, j), _| {
                let (di, dj) = (i - centre.0, j - centre.1);
                ((di * di + dj * dj) as f32) <= keep
            });
            let cells = &self.cells;
            self.crossings.retain(|&(i, j, _), _| cells.contains_key(&(i, j)));
            let (feet2, keep_m) = (feet.truncate(), 2.5 * radius);
            self.rail_points.retain(|r| r.iter().any(|p| p.truncate().distance(feet2) <= keep_m));
            self.looked.retain(|p| p.truncate().distance(feet2) <= keep_m);
        }
        probes
    }

    /// Wide probes find what the grid of line probes passes by: a rail
    /// between two probes. Answers come a frame later: one standing above
    /// the line probe's ground is looked at closely (a ring of line probes
    /// round it) and, met, followed as a rail.
    fn wide_probes<P: Probe>(&mut self, probe: &mut P) -> usize {
        let mut probes = 0;
        let mut still = Vec::new();
        for (id, p, ground) in std::mem::take(&mut self.wide_pending) {
            match probe.wide_result(id) {
                None => still.push((id, p, ground)),
                Some(Some(touch)) if touch.z > ground + RAIL_RISE && touch.z < ground + MAX_RAIL_HEIGHT + 0.2 => {
                    self.wide_touches.push((touch, ground));
                }
                Some(_) => {}
            }
        }
        self.wide_pending = still;
        // A few looked at closely per scan (a kerb or a wall touches many).
        for _ in 0..6 {
            let Some((touch, ground)) = self.wide_touches.pop() else { break };
            {
                {
                    if self.rail_points.iter().flatten().chain(&self.looked).any(|q| q.distance(touch) < 0.3) {
                        continue;
                    }
                    let mut found = false;
                    // Round the touch: the rail crosses the ring twice.
                    for k in 0..24 {
                        let d = Vec2::from_angle(k as f32 * std::f32::consts::TAU / 24.);
                        let q = touch.truncate() + d * WIDE_RADIUS;
                        probes += 1;
                        if let Some(h) = probe.down(q.x, q.y, touch.z + 0.2, ground + 0.02)
                            && h.z > ground + RAIL_RISE
                        {
                            self.seeds.push(q.extend(h.z));
                            found = true;
                            break;
                        }
                    }
                    // Nothing thin there (a wall, a kerb): remembered, so it
                    // is not looked at again.
                    if !found {
                        self.looked.push(touch);
                    }
                }
            }
        }
        while self.wide_pending.len() < WIDE_PER_SCAN * 2 {
            let Some((p, ground)) = self.wide_queue.pop_front() else { break };
            let top = ground + MAX_RAIL_HEIGHT + 0.3;
            // The capsule's bottom stops a rail's least height above the
            // ground: the ground itself is not met.
            match probe.wide_start(p.extend(top), p.extend(ground + RAIL_RISE + WIDE_RADIUS), WIDE_RADIUS) {
                Some(id) => self.wide_pending.push((id, p, ground)),
                None => {
                    self.wide_queue.clear();
                    break;
                }
            }
        }
        probes
    }

    /// A cell standing above both neighbours on one axis by more than
    /// [`THIN`], within reach of the ground: a thin thing (a handrail, also
    /// over stairs, where the other axis climbs). Its hit point.
    fn thin_between(&self, key: (i32, i32)) -> Option<Vec3> {
        let z = self.cells.get(&key)?.hit?.z;
        let at = |di: i32, dj: i32| self.cells.get(&(key.0 + di, key.1 + dj)).map(|c| c.hit.map_or(f32::MIN, |h| h.z));
        let low_both = |a: Option<f32>, b: Option<f32>| match (a, b) {
            (Some(a), Some(b)) => z - a > RAIL_RISE && z - b > RAIL_RISE && a.max(b) > z - MAX_RAIL_HEIGHT,
            _ => false,
        };
        (low_both(at(1, 0), at(-1, 0)) || low_both(at(0, 1), at(0, -1))).then(|| self.point(key).extend(z))
    }

    /// Follows a handrail from `seed`, a point on it: finds its direction
    /// on a ring round the seed, then steps along it both ways, looking a
    /// little to either side for it again, while both sides stay well below
    /// (a rail, not a ledge). Returns the probes made.
    fn trace(&mut self, probe: &mut impl Probe, seed: Vec3) -> usize {
        let mut probes = 0;
        let mut down = |x: f32, y: f32, top: f32, bottom: f32| {
            probes += 1;
            probe.down(x, y, top, bottom)
        };
        // The direction: a ring hit at about the seed's height.
        let c = seed.truncate();
        let mut dir = None;
        for k in 0..32 {
            let d = Vec2::from_angle(k as f32 * std::f32::consts::TAU / 32.);
            if let Some(h) = down(c.x + d.x * 0.25, c.y + d.y * 0.25, seed.z + 0.25, seed.z - 0.25)
                && (h.z - seed.z).abs() < 0.22
            {
                dir = Some(d);
                break;
            }
        }
        let mut ways: [Vec<Vec3>; 2] = [Vec::new(), Vec::new()];
        if let Some(dir) = dir {
            for (w, way) in [dir, -dir].into_iter().enumerate() {
                let (mut at, mut z, mut d) = (c, seed.z, way);
                for _ in 0..TRACE_STEPS {
                    let ahead = at + d * TRACE_STEP;
                    let side = d.perp();
                    let window = TRACE_STEP * 0.85 + 0.05;
                    let mut found = None;
                    for &o in &TRACE_SIDE {
                        let q = ahead + side * o;
                        let Some(hit) = down(q.x, q.y, z + window, z - window) else { continue };
                        if (hit.z - z).abs() > window {
                            continue;
                        }
                        // Both sides well below: a rail, not a ledge or a wall.
                        let below = |g: Option<Hit>| g.map_or(f32::MIN, |h| h.z);
                        let (l, r) = (q + side * 0.2, q - side * 0.2);
                        let a = below(down(l.x, l.y, hit.z + 0.05, hit.z - MAX_RAIL_HEIGHT - 2.));
                        let b = below(down(r.x, r.y, hit.z + 0.05, hit.z - MAX_RAIL_HEIGHT - 2.));
                        if hit.z - a > RAIL_RISE && hit.z - b > RAIL_RISE && a.max(b) > hit.z - MAX_RAIL_HEIGHT {
                            found = Some((q, hit.z));
                        }
                        break;
                    }
                    let Some((q, hz)) = found else { break };
                    d = (d * 0.7 + (q - at).normalize_or_zero() * 0.3).normalize_or_zero();
                    at = q;
                    z = hz;
                    ways[w].push(q.extend(hz));
                }
            }
        }
        // In order along the rail; kept even when short, so the seed is not
        // followed again.
        let [ahead, behind] = ways;
        let mut rail: Vec<Vec3> = behind.into_iter().rev().collect();
        rail.push(seed);
        rail.extend(ahead);
        self.rail_points.push(rail);
        self.revision += 1;
        probes
    }

    fn neighbour((i, j, d): (i32, i32, u8)) -> (i32, i32) {
        if d == 0 { (i + 1, j) } else { (i, j + 1) }
    }

    /// Locates the step on `edge`, if it has one. Returns the probes made.
    fn refine(&mut self, probe: &mut impl Probe, edge: (i32, i32, u8)) -> usize {
        let a = (edge.0, edge.1);
        let b = Self::neighbour(edge);
        let (Some(ca), Some(cb)) = (self.cells.get(&a).copied(), self.cells.get(&b).copied()) else {
            return 0;
        };
        let (Some(ha), Some(hb)) = (ca.hit, cb.hit) else {
            return 0;
        };
        if (ha.z - hb.z).abs() <= STEP
            || (ca.from_z - cb.from_z).abs() > RESAMPLE_HEIGHT
            || sloped((ha.z, ha.normal_z), (hb.z, hb.normal_z), self.spacing)
        {
            return 0;
        }
        let (pa, pb) = (self.point(a), self.point(b));
        // A tall wall's face, from the open side (probes down inside it find
        // the ground beyond).
        if ca.wall_from.is_some() != cb.wall_from.is_some() {
            let (open, wall, flip) = if ca.wall_from.is_some() { (pb, pa, true) } else { (pa, pb, false) };
            let z = ca.from_z.max(cb.from_z) + WALL_LOOK;
            let along = probe
                .across(open.extend(z), wall.extend(z))
                .map_or(1., |at| (at.truncate().distance(open) / self.spacing).clamp(0., 1.));
            self.crossings.insert(edge, if flip { 1. - along } else { along });
            return 1;
        }
        let (top, bottom) = (ca.from_z + PROBE_UP, ca.from_z - PROBE_DOWN);
        let (mut lo, mut hi) = (0f32, 1f32);
        for _ in 0..REFINE {
            let mid = 0.5 * (lo + hi);
            let q = pa.lerp(pb, mid);
            match probe.down(q.x, q.y, top, bottom) {
                Some(h) if (h.z - ha.z).abs() < (h.z - hb.z).abs() => lo = mid,
                _ => hi = mid,
            }
        }
        self.crossings.insert(edge, 0.5 * (lo + hi));
        REFINE as usize
    }

    /// The collision and rails within `radius` of `centre`.
    pub fn build(&self, centre: Vec3, radius: f32) -> Mesh {
        self.build_for(centre, radius, false)
    }

    /// As [`Self::build`]; `on_foot` makes steps up to [`STAIR`] high
    /// slopes to walk up (on the board they stay steps to ollie and grind).
    pub fn build_for(&self, centre: Vec3, radius: f32, on_foot: bool) -> Mesh {
        let s = self.spacing;
        let (ci, cj) = self.cell_of(centre);
        let reach = (radius / s).ceil() as i32 + 1;
        let raw = |key: (i32, i32)| self.cells.get(&key).and_then(|c| c.hit);

        // Mesh heights, with thin things taken out.
        let mut heights: HashMap<(i32, i32), (f32, f32, u8)> = HashMap::new();
        let mut thin: HashSet<(i32, i32)> = HashSet::new();
        let mut thin_points = Vec::new();
        for i in ci - reach - 1..=ci + reach + 1 {
            for j in cj - reach - 1..=cj + reach + 1 {
                let Some(hit) = raw((i, j)) else { continue };
                let around: Option<Vec<f32>> = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|(di, dj)| raw((i + di, j + dj)).map(|h| h.z))
                    .collect();
                let mut z = hit.z;
                if let Some(around) = around {
                    let (low, high) = around.iter().fold((f32::MAX, f32::MIN), |(l, h), &v| (l.min(v), h.max(v)));
                    if hit.z - high > THIN && high - low < 0.3 {
                        thin.insert((i, j));
                        let p = self.point((i, j));
                        // Fence and wall tops stay out of the mesh but are
                        // no rails: out of an ollie's reach.
                        if hit.z - low <= MAX_RAIL_HEIGHT {
                            thin_points.push(Vec3::new(p.x, p.y, hit.z));
                        }
                        z = around.iter().sum::<f32>() / 4.;
                    }
                }
                heights.insert((i, j), (z, hit.normal_z, hit.ground));
            }
        }

        // The cells on a followed rail are no ground either (riding over
        // train tracks caught on them): the ground either side of it there.
        // Only followed rails, only low ones (tracks: a handrail is above
        // the probes' ground anyway), only where the ground both sides of
        // the cell lies lower by about the rail (taking a cell down beside
        // a drop dug holes: the skater fell through, 10-09 22:13).
        for rail in self.rail_points.iter().filter(|r| length(r) >= MIN_THIN_RAIL) {
            for p in rail {
                let key = self.cell_of(*p);
                for k in [key, (key.0 + 1, key.1), (key.0 - 1, key.1), (key.0, key.1 + 1), (key.0, key.1 - 1)] {
                    let Some(&(z, n, g)) = heights.get(&k) else { continue };
                    if (z - p.z).abs() > 0.06 {
                        continue;
                    }
                    let side = |di: i32, dj: i32| heights.get(&(k.0 + di, k.1 + dj)).map(|h| h.0);
                    let pair = |a: Option<f32>, b: Option<f32>| match (a, b) {
                        (Some(a), Some(b)) if p.z - a > RAIL_RISE && p.z - b > RAIL_RISE && p.z - a.min(b) <= THIN => Some(a.max(b)),
                        _ => None,
                    };
                    if let Some(low) = pair(side(1, 0), side(-1, 0)).or(pair(side(0, 1), side(0, -1))) {
                        heights.insert(k, (low, n, g));
                        thin.insert(k);
                    }
                }
            }
        }

        let mut mesh = Mesh::default();
        let crossing = |edge: (i32, i32, u8), h0: f32, h1: f32| -> f32 {
            let b = Self::neighbour(edge);
            if (h0 - h1).abs() <= STEP || thin.contains(&(edge.0, edge.1)) || thin.contains(&b) {
                return 0.5;
            }
            self.crossings.get(&edge).copied().unwrap_or(0.5).clamp(0.02, 0.98)
        };
        let r2 = (radius / s).powi(2);
        for i in ci - reach..ci + reach {
            for j in cj - reach..cj + reach {
                let (di, dj) = (i as f32 + 0.5 - ci as f32, j as f32 + 0.5 - cj as f32);
                if di * di + dj * dj > r2 {
                    continue;
                }
                let corner = |a: i32, b: i32| heights.get(&(i + a, j + b)).map(|h| h.0);
                let (Some(h00), Some(h10), Some(h01), Some(h11)) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1)) else {
                    continue;
                };
                let ground = |a: i32, b: i32| heights.get(&(i + a, j + b)).map_or(0, |h| h.2);
                let (g00, g10, g01, g11) = (ground(0, 0), ground(1, 0), ground(0, 1), ground(1, 1));
                let base = self.point((i, j));
                let at = |u: f32, v: f32, z: f32| Vec3::new(base.x + u * s, base.y + v * s, z);
                let low = h00.min(h10).min(h01).min(h11);
                let high = h00.max(h10).max(h01).max(h11);
                // A ramp's or a pipe's surface: one slope across the cell,
                // ridden up rather than met as stairs.
                let corner_hit = |a: i32, b: i32| heights.get(&(i + a, j + b)).map(|h| (h.0, h.1));
                let smooth = [((0, 0), (1, 0)), ((0, 1), (1, 1)), ((0, 0), (0, 1)), ((1, 0), (1, 1))].iter().all(|(p, q)| {
                    match (corner_hit(p.0, p.1), corner_hit(q.0, q.1)) {
                        (Some(a), Some(b)) => (a.0 - b.0).abs() <= STEP || sloped(a, b, s),
                        _ => false,
                    }
                });
                if high - low <= STEP || smooth || (on_foot && high - low <= STAIR) {
                    let (a, b, c, d) = (at(0., 0., h00), at(1., 0., h10), at(1., 1., h11), at(0., 1., h01));
                    let g = majority([g00, g10, g11, g01]);
                    mesh.push([a, b, c], g);
                    mesh.push([a, c, d], g);
                    continue;
                }
                // A stepped cell: four flat regions meeting at the located
                // edges, with walls between them.
                let xb = crossing((i, j, 0), h00, h10);
                let xt = crossing((i, j + 1, 0), h01, h11);
                let yl = crossing((i, j, 1), h00, h01);
                let yr = crossing((i + 1, j, 1), h10, h11);
                let denom = 1. - (xt - xb) * (yr - yl);
                let mx = ((xb + (xt - xb) * yl) / denom).clamp(0.02, 0.98);
                let my = (yl + (yr - yl) * mx).clamp(0.02, 0.98);
                let m = Vec2::new(mx, my);
                let regions: [(f32, [Vec2; 4]); 4] = [
                    (h00, [Vec2::new(0., 0.), Vec2::new(xb, 0.), m, Vec2::new(0., yl)]),
                    (h10, [Vec2::new(xb, 0.), Vec2::new(1., 0.), Vec2::new(1., yr), m]),
                    (h11, [m, Vec2::new(1., yr), Vec2::new(1., 1.), Vec2::new(xt, 1.)]),
                    (h01, [Vec2::new(0., yl), m, Vec2::new(xt, 1.), Vec2::new(0., 1.)]),
                ];
                let grounds = [g00, g10, g11, g01];
                for ((z, poly), g) in regions.iter().zip(grounds) {
                    let p: Vec<Vec3> = poly.iter().map(|q| at(q.x, q.y, *z)).collect();
                    mesh.push([p[0], p[1], p[2]], g);
                    mesh.push([p[0], p[2], p[3]], g);
                }
                let centroid = |r: usize| regions[r].1.iter().copied().sum::<Vec2>() / 4.;
                for (ra, rb, from, to) in [
                    (0, 1, Vec2::new(xb, 0.), m),
                    (3, 2, m, Vec2::new(xt, 1.)),
                    (0, 3, Vec2::new(0., yl), m),
                    (1, 2, m, Vec2::new(1., yr)),
                ] {
                    let (za, zb) = (regions[ra].0, regions[rb].0);
                    if (za - zb).abs() < 1e-3 {
                        continue;
                    }
                    let (lower, higher) = if za < zb { (ra, rb) } else { (rb, ra) };
                    let (lo, hi) = (za.min(zb), za.max(zb));
                    let (p, q) = (at(from.x, from.y, lo), at(to.x, to.y, lo));
                    let toward_low = (base + centroid(lower) * s) - p.truncate();
                    wall(&mut mesh, p, q, hi - lo, toward_low, grounds[higher]);
                }
            }
        }

        // Lips: located steps whose high side is walkable and drops away.
        let mut lips = Vec::new();
        for (&edge, &t) in &self.crossings {
            let a = (edge.0, edge.1);
            let b = Self::neighbour(edge);
            if thin.contains(&a) || thin.contains(&b) {
                continue;
            }
            let (Some(ha), Some(hb)) = (raw(a), raw(b)) else { continue };
            let (pa, pb) = (self.point(a), self.point(b));
            let p = pa.lerp(pb, t);
            let (dx, dy) = ((p.x - centre.x) / s, (p.y - centre.y) / s);
            if dx * dx + dy * dy > r2 || (ha.z - hb.z).abs() < MIN_DROP {
                continue;
            }
            let (top, out) = if ha.z > hb.z { (ha, pb - pa) } else { (hb, pa - pb) };
            if top.normal_z < WALKABLE {
                continue;
            }
            // A high drop's edge is a rail only from up there (a ledge along
            // a drop), not to be pulled up onto from below.
            if (ha.z - hb.z).abs() > MAX_RAIL_HEIGHT && (top.z - centre.z).abs() > 0.6 {
                continue;
            }
            lips.push(Lip {
                p: Vec3::new(p.x, p.y, top.z),
                out: out.normalize(),
            });
        }
        mesh.rails = lip_rails(&lips, 1.6 * s);
        // Followed handrails as they are; grid hits on them are not joined
        // a second time.
        let reach = radius + s;
        let traced: Vec<&Vec<Vec3>> = self.rail_points.iter().filter(|r| length(r) >= MIN_THIN_RAIL).collect();
        thin_points.retain(|p| !traced.iter().any(|r| r.iter().any(|q| q.distance(*p) < 0.3)));
        mesh.rails.extend(thin_rails(&thin_points, 2.0));
        for rail in traced {
            let near: Vec<Vec3> = rail.iter().copied().filter(|p| p.truncate().distance(centre.truncate()) <= reach).collect();
            if length(&near) >= MIN_THIN_RAIL {
                mesh.rails.push(near);
            }
        }
        mesh
    }
}

/// A vertical wall quad on segment `p`-`q` (at its foot height), `height`
/// tall, facing horizontal direction `facing`.
/// The wall takes the ground of the step's top.
fn wall(out: &mut Mesh, p: Vec3, q: Vec3, height: f32, facing: Vec2, ground: u8) {
    let up = Vec3::Z * height;
    let quad = [p, q, q + up, p + up];
    let normal = (quad[1] - quad[0]).cross(quad[2] - quad[0]);
    if normal.truncate().dot(facing) >= 0. {
        out.push([quad[0], quad[1], quad[2]], ground);
        out.push([quad[0], quad[2], quad[3]], ground);
    } else {
        out.push([quad[0], quad[2], quad[1]], ground);
        out.push([quad[0], quad[3], quad[2]], ground);
    }
}

#[derive(Clone, Copy, Debug)]
struct Lip {
    p: Vec3,
    /// Horizontal direction from the high side to the drop.
    out: Vec2,
}

fn grid_of(points: impl Iterator<Item = Vec3>, cell: f32) -> HashMap<(i32, i32), Vec<usize>> {
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (index, p) in points.enumerate() {
        grid.entry(((p.x / cell).floor() as i32, (p.y / cell).floor() as i32)).or_default().push(index);
    }
    grid
}

fn near(grid: &HashMap<(i32, i32), Vec<usize>>, p: Vec3, cell: f32) -> impl Iterator<Item = usize> + '_ {
    let (x, y) = ((p.x / cell).floor() as i32, (p.y / cell).floor() as i32);
    (x - 1..=x + 1)
        .flat_map(move |i| (y - 1..=y + 1).map(move |j| (i, j)))
        .filter_map(|key| grid.get(&key))
        .flatten()
        .copied()
}

/// Joins points into polylines through mutual neighbours: each point names
/// at most one neighbour on either side, and a link stands when both ends
/// name each other.
fn walk(count: usize, picks: &[[Option<usize>; 2]], position: impl Fn(usize) -> Vec3) -> Vec<Vec<Vec3>> {
    let mut links: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (a, pick) in picks.iter().enumerate() {
        for &b in pick.iter().flatten() {
            if a < b && picks[b].contains(&Some(a)) {
                links[a].push(b);
                links[b].push(a);
            }
        }
    }
    let mut seen = vec![false; count];
    let mut lines = Vec::new();
    // Open chains from their ends first, then the closed loops left over.
    for pass in 0..2 {
        for start in 0..count {
            if seen[start] || (pass == 0 && links[start].len() != 1) || links[start].is_empty() {
                continue;
            }
            let mut line = vec![position(start)];
            seen[start] = true;
            let mut at = start;
            while let Some(&next) = links[at].iter().find(|&&n| !seen[n]) {
                seen[next] = true;
                line.push(position(next));
                at = next;
            }
            lines.push(line);
        }
    }
    lines
}

/// Splits a polyline where it turns sharply.
fn split_turns(line: Vec<Vec3>) -> Vec<Vec<Vec3>> {
    let mut out = Vec::new();
    let mut current = vec![line[0]];
    for w in line.windows(3) {
        current.push(w[1]);
        let (d0, d1) = ((w[1] - w[0]).normalize_or_zero(), (w[2] - w[1]).normalize_or_zero());
        if d0.dot(d1) < MIN_TURN_COS {
            out.push(std::mem::replace(&mut current, vec![w[1]]));
        }
    }
    current.push(*line.last().unwrap());
    if line.len() == 1 {
        current.pop();
    }
    out.push(current);
    out
}

/// Douglas-Peucker with tolerance `tolerance`.
fn simplify(points: &[Vec3], tolerance: f32) -> Vec<Vec3> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let (a, b) = (points[0], *points.last().unwrap());
    let ab = b - a;
    let (index, distance) = points[1..points.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let t = if ab.length_squared() > 0. { ((*p - a).dot(ab) / ab.length_squared()).clamp(0., 1.) } else { 0. };
            (i + 1, (*p - (a + ab * t)).length())
        })
        .fold((0, 0f32), |best, item| if item.1 > best.1 { item } else { best });
    if distance <= tolerance {
        return vec![a, b];
    }
    let mut left = simplify(&points[..=index], tolerance);
    left.pop();
    left.extend(simplify(&points[index..], tolerance));
    left
}

fn length(line: &[Vec3]) -> f32 {
    line.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn finish(lines: Vec<Vec<Vec3>>, min_length: f32, min_points: usize) -> Vec<Vec<Vec3>> {
    lines
        .into_iter()
        .filter(|line| line.len() >= 2)
        .flat_map(split_turns)
        .filter(|line| line.len() >= min_points.max(2) && length(line) >= min_length)
        .map(|line| simplify(&line, SIMPLIFY))
        .collect()
}

/// Lips along one edge, joined into rails.
fn lip_rails(lips: &[Lip], gap: f32) -> Vec<Vec<Vec3>> {
    let grid = grid_of(lips.iter().map(|l| l.p), gap);
    let picks: Vec<[Option<usize>; 2]> = lips
        .iter()
        .enumerate()
        .map(|(i, lip)| {
            let tangent = Vec2::new(-lip.out.y, lip.out.x);
            let mut best = [None::<(f32, usize)>; 2];
            for j in near(&grid, lip.p, gap) {
                let other = &lips[j];
                let d = other.p - lip.p;
                let flat = d.truncate().length();
                if j == i || flat < 1e-4 || flat > gap || d.z.abs() > 0.7 * flat + 0.03 || lip.out.dot(other.out) < 0.5 {
                    continue;
                }
                let along = d.truncate().dot(tangent) / flat;
                if along.abs() < 0.6 {
                    continue;
                }
                let side = usize::from(along < 0.);
                if best[side].is_none_or(|(bd, _)| flat < bd) {
                    best[side] = Some((flat, j));
                }
            }
            best.map(|b| b.map(|(_, j)| j))
        })
        .collect();
    finish(walk(lips.len(), &picks, |i| lips[i].p), MIN_LIP_RAIL, 2)
}

/// Thin things (handrails, fence tops) that line up, as rails.
fn thin_rails(points: &[Vec3], gap: f32) -> Vec<Vec<Vec3>> {
    let grid = grid_of(points.iter().copied(), gap);
    let picks: Vec<[Option<usize>; 2]> = points
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let candidates: Vec<(f32, usize)> = near(&grid, p, gap)
                .filter(|&j| j != i)
                .filter_map(|j| {
                    let d = points[j] - p;
                    let flat = d.truncate().length();
                    (flat > 1e-4 && flat <= gap && d.z.abs() <= 0.7 * flat + 0.05).then_some((flat, j))
                })
                .collect();
            let first = candidates.iter().copied().min_by(|a, b| a.0.total_cmp(&b.0));
            let second = first.and_then(|(_, f)| {
                let away = (points[f] - p).truncate().normalize_or_zero();
                candidates
                    .iter()
                    .copied()
                    .filter(|&(_, j)| (points[j] - p).truncate().normalize_or_zero().dot(away) < -0.9)
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            });
            [first.map(|f| f.1), second.map(|s| s.1)]
        })
        .collect();
    finish(walk(points.len(), &picks, |i| points[i]), MIN_THIN_RAIL, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ground at 0 and boxes: (centre, half size, turn about Z, top).
    struct Scene {
        boxes: Vec<(Vec2, Vec2, f32, f32)>,
        probes: usize,
        wide: Vec<Option<Vec3>>,
    }

    impl Scene {
        fn height(&self, x: f32, y: f32) -> f32 {
            self.boxes
                .iter()
                .filter(|(c, half, turn, _)| {
                    let local = Vec2::from_angle(-turn).rotate(Vec2::new(x, y) - *c);
                    local.x.abs() <= half.x && local.y.abs() <= half.y
                })
                .map(|b| b.3)
                .fold(0., f32::max)
        }
    }

    impl Probe for Scene {
        fn down(&mut self, x: f32, y: f32, top: f32, bottom: f32) -> Option<Hit> {
            self.probes += 1;
            let z = self.height(x, y);
            (z <= top && z >= bottom).then_some(Hit { z, normal_z: 1., ground: if x < 0. { 3 } else { 1 } })
        }

        /// The highest box top within the radius, as a touch point.
        fn wide_start(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<i32> {
            let mut best: Option<Vec3> = None;
            for i in -6..=6 {
                for j in -6..=6 {
                    let o = Vec2::new(i as f32, j as f32) * (radius / 6.);
                    if o.length() > radius {
                        continue;
                    }
                    let q = from.truncate() + o;
                    let z = self.height(q.x, q.y);
                    if z <= from.z && z >= to.z - radius && best.is_none_or(|b| z > b.z) {
                        best = Some(q.extend(z));
                    }
                }
            }
            self.wide.push(best);
            Some(self.wide.len() as i32 - 1)
        }

        fn wide_result(&mut self, id: i32) -> Option<Option<Vec3>> {
            self.wide.get(id as usize).copied()
        }
    }

    /// Flat ground with a 3 m wall across x 1.5..1.8 and a deck overhead
    /// (3 m up) at x < -1, as GTA probes them: a probe starting inside the
    /// wall finds the ground beyond it.
    struct TallWall;

    impl Probe for TallWall {
        fn down(&mut self, x: f32, _y: f32, top: f32, _bottom: f32) -> Option<Hit> {
            let z = if (1.5..=1.8).contains(&x) && top >= 3. { 3. } else { 0. };
            Some(Hit { z, normal_z: 1., ground: 1 })
        }
        fn across(&mut self, from: Vec3, to: Vec3) -> Option<Vec3> {
            if from.z >= 3. {
                return None;
            }
            [1.5f32, 1.8].into_iter().find_map(|face| {
                let t = (face - from.x) / (to.x - from.x);
                (to.x != from.x && (0. ..=1.).contains(&t)).then(|| from.lerp(to, t))
            })
        }
    }

    #[test]
    fn a_wall_taller_than_the_probes_start_still_stands() {
        let mut terrain = Terrain::new(0.2).with_tall_walls(true);
        terrain.scan(&mut TallWall, Vec3::ZERO, 4., || true);
        let mesh = terrain.build(Vec3::ZERO, 4.);
        assert!(mesh_height(&mesh, 1.6, 0.).is_some_and(|z| z > 2.), "the wall has a top");
        assert!(mesh_height(&mesh, 1.0, 0.).is_some_and(|z| z.abs() < 0.01), "open ground before it");
        assert!(mesh_height(&mesh, -2.0, 0.).is_some_and(|z| z.abs() < 0.01), "ridden under the deck");
        // Its face stands where the wall begins, not inside it.
        let faces: Vec<f32> = mesh
            .triangles
            .iter()
            .filter(|t| (t[1] - t[0]).cross(t[2] - t[0]).normalize().z.abs() < 0.01)
            .filter(|t| t.iter().all(|p| p.y.abs() < 0.5))
            .flat_map(|t| t.iter().map(|p| p.x))
            .collect();
        assert!(faces.iter().any(|x| (x - 1.5).abs() < 0.03), "a face at x 1.5: {faces:?}");
    }

    fn scanned(scene: &mut Scene, radius: f32) -> (Terrain, Mesh) {
        let mut terrain = Terrain::new(0.2);
        terrain.scan(scene, Vec3::ZERO, radius, || true);
        let mesh = terrain.build(Vec3::ZERO, radius);
        (terrain, mesh)
    }

    /// The highest mesh surface straight down at (x, y).
    fn mesh_height(mesh: &Mesh, x: f32, y: f32) -> Option<f32> {
        mesh.triangles
            .iter()
            .filter_map(|t| {
                let n = (t[1] - t[0]).cross(t[2] - t[0]);
                if n.z.abs() < 1e-6 {
                    return None;
                }
                let p = Vec2::new(x, y);
                let side = |a: Vec3, b: Vec3| (b.truncate() - a.truncate()).perp_dot(p - a.truncate());
                let (s0, s1, s2) = (side(t[0], t[1]), side(t[1], t[2]), side(t[2], t[0]));
                let inside = (s0 >= -1e-6 && s1 >= -1e-6 && s2 >= -1e-6) || (s0 <= 1e-6 && s1 <= 1e-6 && s2 <= 1e-6);
                inside.then(|| t[0].z - (n.x * (x - t[0].x) + n.y * (y - t[0].y)) / n.z)
            })
            .reduce(f32::max)
    }

    fn rail_near(mesh: &Mesh, a: Vec3, b: Vec3, tolerance: f32) -> Option<&Vec<Vec3>> {
        mesh.rails.iter().find(|rail| {
            let ab = b - a;
            rail.iter().all(|p| {
                let t = ((*p - a).dot(ab) / ab.length_squared()).clamp(0., 1.);
                (*p - (a + ab * t)).length() < tolerance
            }) && length(rail) > 0.6 * ab.length()
        })
    }

    #[test]
    fn each_triangle_keeps_the_ground_it_was_probed_on() {
        // Grass (3) where x < 0, concrete (1) elsewhere.
        let mut scene = Scene { boxes: vec![], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 3.);
        assert_eq!(mesh.surfaces.len(), mesh.triangles.len());
        for (t, g) in mesh.triangles.iter().zip(&mesh.surfaces) {
            let x = (t[0].x + t[1].x + t[2].x) / 3.;
            if x < -0.2 {
                assert_eq!(*g, 3, "grass at {x}");
            } else if x > 0.2 {
                assert_eq!(*g, 1, "concrete at {x}");
            }
        }
        assert_eq!(majority([1, 3, 3, 1]), 1);
        assert_eq!(majority([2, 3, 3, 1]), 3);
    }

    /// A 30 degree kicker 2 m long rising along +x from x = 1.
    struct Kicker;

    impl Probe for Kicker {
        fn down(&mut self, x: f32, _y: f32, _top: f32, _bottom: f32) -> Option<Hit> {
            let tan = 30f32.to_radians().tan();
            let (z, normal_z) = if (1.0..3.0).contains(&x) { ((x - 1.) * tan, 30f32.to_radians().cos()) } else { (0., 1.) };
            Some(Hit { z, normal_z, ground: 1 })
        }
    }

    #[test]
    fn a_ramp_is_ridden_up_not_met_as_stairs() {
        let mut terrain = Terrain::new(0.2);
        terrain.scan(&mut Kicker, Vec3::ZERO, 4., || true);
        let mesh = terrain.build(Vec3::ZERO, 4.);
        let walls = mesh
            .triangles
            .iter()
            .filter(|t| {
                let mid = (t[0] + t[1] + t[2]) / 3.;
                let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize();
                mid.x > 1.2 && mid.x < 2.8 && n.z.abs() < 0.2
            })
            .count();
        assert_eq!(walls, 0, "no stair walls on the ramp's face");
        let h = mesh_height(&mesh, 2.0, 0.1).unwrap();
        assert!((h - 30f32.to_radians().tan()).abs() < 0.05, "{h}");
    }

    /// Stairs up along +x from x = 1: 0.17 m rises every 0.3 m, six of them.
    struct Stairs;

    impl Probe for Stairs {
        fn down(&mut self, x: f32, _y: f32, _top: f32, _bottom: f32) -> Option<Hit> {
            let steps = ((x - 1.) / 0.3).floor().clamp(-1., 5.) + 1.;
            Some(Hit { z: steps * 0.17, normal_z: 1., ground: 1 })
        }
    }

    #[test]
    fn on_foot_stairs_are_a_slope_and_on_the_board_steps() {
        let mut terrain = Terrain::new(0.2);
        terrain.scan(&mut Stairs, Vec3::ZERO, 4., || true);
        let walls = |mesh: &Mesh| {
            mesh.triangles
                .iter()
                .filter(|t| {
                    let mid = (t[0] + t[1] + t[2]) / 3.;
                    let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize();
                    mid.x > 1.0 && mid.x < 2.8 && mid.y.abs() < 2. && n.z.abs() < 0.2
                })
                .count()
        };
        assert!(walls(&terrain.build_for(Vec3::ZERO, 4., false)) > 0, "on the board: steps");
        assert_eq!(walls(&terrain.build_for(Vec3::ZERO, 4., true)), 0, "on foot: no step to climb");
    }

    #[test]
    fn flat_ground_is_flat_with_no_rails() {
        let mut scene = Scene { boxes: vec![], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 3.);
        assert!(!mesh.triangles.is_empty());
        assert!(mesh.triangles.iter().all(|t| t.iter().all(|p| p.z == 0.)));
        assert!(mesh.triangles.iter().all(|t| (t[1] - t[0]).cross(t[2] - t[0]).z > 0.), "tops face up");
        assert!(mesh.rails.is_empty());
    }

    #[test]
    fn a_ledge_has_its_edges_in_place_and_grindable() {
        // A 3 x 4 m ledge, 0.5 m tall, its edges between probe rows.
        let mut scene = Scene { boxes: vec![(Vec2::new(2.53, 0.07), Vec2::new(1.5, 2.), 0., 0.5)], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 6.);
        // The mesh matches the world away from the edges...
        for (x, y) in [(2.5, 0.), (0.5, 0.), (3.9, 1.9), (1.10, -1.85), (4.1, 0.), (2.5, 2.2)] {
            let want = scene.height(x, y);
            let got = mesh_height(&mesh, x, y).expect("covered");
            assert!((got - want).abs() < 0.01, "at ({x}, {y}): {got} vs {want}");
        }
        // ...and close to them, within the bisection's centimetre.
        for (x, want) in [(1.045, 0.5), (1.015, 0.)] {
            let got = mesh_height(&mesh, x, 0.).unwrap();
            assert!((got - want).abs() < 0.01, "x {x}: {got} vs {want}");
        }
        // The front edge (x = 1.03) is one rail along its length, at the top.
        let rail = rail_near(&mesh, Vec3::new(1.03, -1.93, 0.5), Vec3::new(1.03, 2.07, 0.5), 0.02)
            .unwrap_or_else(|| panic!("no front rail in {:?}", mesh.rails));
        assert!(length(rail) > 3.5);
        assert!(rail.len() <= 4, "a straight edge simplifies to few points: {rail:?}");
        assert_eq!(mesh.rails.len(), 4, "one rail per side: {:?}", mesh.rails);
    }

    #[test]
    fn walls_face_away_from_the_ledge() {
        let mut scene = Scene { boxes: vec![(Vec2::new(2.53, 0.07), Vec2::new(1.5, 2.), 0., 0.5)], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 6.);
        let centre = Vec3::new(2.53, 0.07, 0.25);
        let walls: Vec<_> = mesh.triangles.iter().filter(|t| (t[1] - t[0]).cross(t[2] - t[0]).normalize().z.abs() < 0.01).collect();
        assert!(!walls.is_empty());
        for t in walls {
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            let mid = (t[0] + t[1] + t[2]) / 3.;
            assert!(n.dot(mid - centre) > 0., "wall at {mid:?} faces in");
        }
    }

    #[test]
    fn diagonal_and_low_edges_are_rails_too() {
        // A ledge turned 30 degrees, and a 10 cm curb.
        let turn = 30f32.to_radians();
        let mut scene = Scene {
            boxes: vec![
                (Vec2::new(0.4, 2.6), Vec2::new(1.6, 0.5), turn, 0.45),
                (Vec2::new(0.0, -2.0), Vec2::new(3.0, 0.4), 0., 0.10),
            ],
            probes: 0,
            wide: vec![],
        };
        let (_, mesh) = scanned(&mut scene, 6.);
        let corner = |u: f32, v: f32| {
            let p = Vec2::new(0.4, 2.6) + Vec2::from_angle(turn).rotate(Vec2::new(u, v));
            Vec3::new(p.x, p.y, 0.45)
        };
        assert!(
            rail_near(&mesh, corner(-1.6, -0.5), corner(1.6, -0.5), 0.03).is_some(),
            "no rail along the turned ledge's long side in {:?}",
            mesh.rails
        );
        assert!(
            rail_near(&mesh, Vec3::new(-3., -1.6, 0.1), Vec3::new(3., -1.6, 0.1), 0.02).is_some(),
            "no rail along the curb in {:?}",
            mesh.rails
        );
    }

    #[test]
    fn handrails_are_left_out_of_the_mesh_and_ground_where_they_line_up() {
        // A 6 cm handrail, 1 m up, running diagonally.
        let turn = 25f32.to_radians();
        let mut scene = Scene { boxes: vec![(Vec2::new(0., 1.), Vec2::new(3., 0.03), turn, 1.0)], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 6.);
        assert!(mesh.triangles.iter().all(|t| t.iter().all(|p| p.z < 0.5)), "the rail is not in the mesh");
        let end = |u: f32| {
            let p = Vec2::new(0., 1.) + Vec2::from_angle(turn).rotate(Vec2::new(u, 0.));
            Vec3::new(p.x, p.y, 1.0)
        };
        assert!(rail_near(&mesh, end(-3.), end(3.), 0.06).is_some(), "no handrail rail in {:?}", mesh.rails);
    }

    #[test]
    fn a_rail_between_the_probe_rows_is_found_by_the_wide_probes() {
        // A 4 cm handrail 0.9 m up, along x, exactly between two rows of
        // line probes (y 0.8 and 1.0).
        let mut scene = Scene { boxes: vec![(Vec2::new(0., 0.9), Vec2::new(3., 0.02), 0., 0.9)], probes: 0, wide: vec![] };
        let mut terrain = Terrain::new(0.2);
        // A few wide probes a frame: many frames.
        for _ in 0..400 {
            terrain.scan(&mut scene, Vec3::ZERO, 5., || true);
        }
        let mesh = terrain.build(Vec3::ZERO, 5.);
        assert!(rail_near(&mesh, Vec3::new(-3., 0.9, 0.9), Vec3::new(3., 0.9, 0.9), 0.06).is_some(), "no rail: {:?}", mesh.rails);
    }

    #[test]
    fn train_tracks_are_rails() {
        // A 7 cm rail, 0.15 m high, along x, between the probe rows.
        let mut scene = Scene { boxes: vec![(Vec2::new(0., 0.5), Vec2::new(3., 0.035), 0., 0.15)], probes: 0, wide: vec![] };
        let mut terrain = Terrain::new(0.2);
        // A few wide probes a frame: many frames.
        for _ in 0..400 {
            terrain.scan(&mut scene, Vec3::ZERO, 5., || true);
        }
        let mesh = terrain.build(Vec3::ZERO, 5.);
        // Ridden over, not caught on: no wall of it in the mesh.
        assert!(mesh.triangles.iter().all(|t| t.iter().all(|p| p.z < 0.05)), "the track is in the mesh");
        assert!(rail_near(&mesh, Vec3::new(-3., 0.5, 0.15), Vec3::new(3., 0.5, 0.15), 0.06).is_some(), "no rail: {:?}", mesh.rails);
    }

    #[test]
    fn a_drop_beside_a_kerb_digs_no_hole() {
        // A street at 10 m with a pit (ground at 0) beside it, and a 15 cm
        // kerb along the pit's edge.
        let mut scene = Scene {
            boxes: vec![
                (Vec2::new(0., 2.), Vec2::new(5., 2.), 0., 10.),
                (Vec2::new(0., 0.05), Vec2::new(5., 0.05), 0., 10.15),
            ],
            probes: 0,
            wide: vec![],
        };
        let mut terrain = Terrain::new(0.2);
        let feet = Vec3::new(0., 2., 10.);
        for _ in 0..400 {
            terrain.scan(&mut scene, feet, 5., || true);
        }
        let mesh = terrain.build(feet, 5.);
        for (x, y) in [(0., 1.), (1., 0.4), (-2., 0.6), (0., 2.)] {
            let h = mesh_height(&mesh, x, y).unwrap_or(f32::MIN);
            assert!(h > 9.9, "the street at ({x}, {y}) stays at 10 m: {h}");
        }
    }

    #[test]
    fn fence_tops_out_of_reach_are_no_rails() {
        // A 4 cm fence, 2 m high: kept out of the mesh, but no rail to be
        // pulled up onto.
        let mut scene = Scene { boxes: vec![(Vec2::new(0., 1.), Vec2::new(3., 0.02), 0., 2.0)], probes: 0, wide: vec![] };
        let (_, mesh) = scanned(&mut scene, 6.);
        assert!(mesh.rails.iter().all(|r| r.iter().all(|p| p.z < 1.5)), "a rail on the fence: {:?}", mesh.rails);
    }

    #[test]
    fn a_handrail_down_stairs_is_followed_its_whole_length() {
        // Ten 0.15 m steps down along +x, 0.4 m each, and over them a 4 cm
        // handrail 0.9 m above the flight, at a slant to the probe grid, in
        // short pieces going down with the stairs.
        let mut boxes = Vec::new();
        for k in 0..10 {
            let x = k as f32 * 0.4;
            boxes.push((Vec2::new(x + 0.2, 0.), Vec2::new(0.2, 2.), 0., 1.5 - 0.15 * k as f32));
        }
        let turn = 12f32.to_radians();
        for piece in 0..40 {
            let u = 0.05 + 0.1 * piece as f32;
            let at = Vec2::new(0., 0.3) + Vec2::from_angle(turn).rotate(Vec2::new(u, 0.));
            boxes.push((at, Vec2::new(0.05, 0.02), turn, 1.5 - 0.375 * at.x + 0.9));
        }
        let mut scene = Scene { boxes, probes: 0, wide: vec![] };
        let mut terrain = Terrain::new(0.2);
        terrain.scan(&mut scene, Vec3::new(2., 0., 0.8), 5., || true);
        let mesh = terrain.build(Vec3::new(2., 0., 0.8), 5.);
        let long = mesh.rails.iter().map(|r| length(r)).fold(0., f32::max);
        assert!(long > 3., "rail along the stairs only {long} m: {:?}", mesh.rails);
    }

    #[test]
    fn scanning_resumes_within_its_budget_and_forgets_nothing_nearby() {
        let mut scene = Scene { boxes: vec![], probes: 0, wide: vec![] };
        let mut terrain = Terrain::new(0.2);
        let mut left = 100;
        let first = terrain.scan(&mut scene, Vec3::ZERO, 3., || {
            left -= 1;
            left >= 0
        });
        assert_eq!(scene.probes, 100, "one cell per budget check");
        assert!(first >= 100, "sideways looks count too");
        let rest = terrain.scan(&mut scene, Vec3::ZERO, 3., || true);
        assert!(rest > 0);
        assert_eq!(terrain.scan(&mut scene, Vec3::ZERO, 3., || true), 0, "everything is known");
        // Climbing far up probes again from the new height.
        assert!(terrain.scan(&mut scene, Vec3::new(0., 0., 2.), 3., || true) > 0);
    }

    #[test]
    fn rings_cover_each_cell_once() {
        let mut seen = HashSet::new();
        for ring in 0..=4 {
            for cell in ring_cells(ring) {
                assert!(seen.insert(cell), "{cell:?} twice");
                assert_eq!(cell.0.abs().max(cell.1.abs()), ring);
            }
        }
        assert_eq!(seen.len(), 81);
    }
}

#[cfg(test)]
mod timing {
    use super::*;
    struct Bumpy;
    impl Probe for Bumpy {
        fn down(&mut self, x: f32, y: f32, _: f32, _: f32) -> Option<Hit> {
            let z = if ((x * 0.7).floor() + (y * 0.9).floor()) as i32 % 3 == 0 { 0.4 } else { 0.02 * x };
            Some(Hit { z, normal_z: 1., ground: 0 })
        }
    }
    #[test]
    #[ignore = "timing measurement"]
    fn build_time() {
        let mut t = Terrain::new(0.2);
        let s = std::time::Instant::now();
        let probes = t.scan(&mut Bumpy, Vec3::ZERO, 9., || true);
        let scanned = s.elapsed();
        let s = std::time::Instant::now();
        let mesh = t.build(Vec3::ZERO, 9.);
        println!("probes {probes} scan {scanned:?} build {:?} tris {} rails {}", s.elapsed(), mesh.triangles.len(), mesh.rails.len());
    }
}
