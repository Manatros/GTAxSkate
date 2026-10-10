//! Vehicles and people as things to skate on and into.
//!
//! GTA gives scripts no vehicle meshes, so a model's body is surveyed once
//! with ray probes straight down its own up axis over a grid of its
//! footprint: the heights make a roof, hood and trunk to ride on, walled
//! down to the wheels. Until a model is surveyed (and for one that cannot
//! be) its bounding box stands in. People are upright boxes. All of it goes
//! to Skate as collision where each one stands this frame; a vehicle the
//! skater stands on carries them along.

use bevy::math::{Mat4, Vec2, Vec3};

/// Spacing of the survey grid, metres.
pub const SPACING: f32 = 0.3;
/// The gap left under every vehicle, metres: a board (with its wheels about
/// 0.1 m high) rolls on underneath while its rider hippie jumps the car.
pub const UNDER: f32 = 0.2;
/// Probes per model, at most (a long truck is surveyed more coarsely).
const MAX_PROBES: usize = 500;

/// A model's body in its own space (x right, y forward, z up): its top
/// heights on a grid over the bounding box's footprint.
#[derive(Clone, Debug)]
pub struct Shape {
    pub min: Vec3,
    pub max: Vec3,
    nx: usize,
    ny: usize,
    step: Vec2,
    heights: Vec<Option<f32>>,
}

impl Shape {
    /// The bounding box as the body: a flat top at its height.
    pub fn boxed(min: Vec3, max: Vec3) -> Self {
        let size = (max - min).truncate().max(Vec2::splat(0.1));
        Self { min, max, nx: 2, ny: 2, step: size, heights: vec![Some(max.z); 4] }
    }

    fn at(&self, i: usize, j: usize) -> Option<f32> {
        self.heights.get(j * self.nx + i).copied().flatten()
    }

    fn point(&self, i: usize, j: usize, z: f32) -> Vec3 {
        Vec3::new(self.min.x + i as f32 * self.step.x, self.min.y + j as f32 * self.step.y, z)
    }

    /// The top's height above `p` (own space), if the body is there.
    pub fn top_at(&self, p: Vec2) -> Option<f32> {
        let u = (p.x - self.min.x) / self.step.x;
        let v = (p.y - self.min.y) / self.step.y;
        if u < -0.25 || v < -0.25 || u > (self.nx - 1) as f32 + 0.25 || v > (self.ny - 1) as f32 + 0.25 {
            return None;
        }
        let i = (u.round().max(0.) as usize).min(self.nx - 1);
        let j = (v.round().max(0.) as usize).min(self.ny - 1);
        self.at(i, j)
    }

    /// Closes holes in the top: probes through the windows or the open top
    /// of a convertible, or onto the driver, meet no body, and a walker
    /// climbing over the door's edge stood inside the car. A missed cell
    /// with body on both sides of it (along its row or its column) takes the
    /// higher of the two nearest heights, so nothing steps down into it.
    pub fn fill_holes(&mut self) {
        let before = self.heights.clone();
        let at = |i: usize, j: usize| before[j * self.nx + i];
        for j in 0..self.ny {
            for i in 0..self.nx {
                if at(i, j).is_some() {
                    continue;
                }
                let left = (0..i).rev().find_map(|k| at(k, j));
                let right = (i + 1..self.nx).find_map(|k| at(k, j));
                let back = (0..j).rev().find_map(|k| at(i, k));
                let front = (j + 1..self.ny).find_map(|k| at(i, k));
                let across = left.zip(right).map(|(a, b)| a.max(b));
                let along = back.zip(front).map(|(a, b)| a.max(b));
                if let Some(fill) = match (across, along) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (a, b) => a.or(b),
                } {
                    self.heights[j * self.nx + i] = Some(fill);
                }
            }
        }
    }

    /// Collision triangles (counterclockwise seen from outside) in own space:
    /// the top, and walls down to [UNDER] above the bottom of the box.
    pub fn triangles(&self) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        let present = |i: isize, j: isize| -> bool {
            i >= 0
                && j >= 0
                && (i as usize) + 1 < self.nx
                && (j as usize) + 1 < self.ny
                && [(0, 0), (1, 0), (0, 1), (1, 1)].iter().all(|(a, b)| self.at(i as usize + a, j as usize + b).is_some())
        };
        let bottom = self.min.z + UNDER;
        for j in 0..self.ny.saturating_sub(1) {
            for i in 0..self.nx.saturating_sub(1) {
                if !present(i as isize, j as isize) {
                    continue;
                }
                let h = |a: usize, b: usize| self.at(i + a, j + b).unwrap_or(self.max.z);
                let (p00, p10, p11, p01) =
                    (self.point(i, j, h(0, 0)), self.point(i + 1, j, h(1, 0)), self.point(i + 1, j + 1, h(1, 1)), self.point(i, j + 1, h(0, 1)));
                out.push([p00, p10, p11]);
                out.push([p00, p11, p01]);
                // Walls where the body ends, facing out.
                let (ii, jj) = (i as isize, j as isize);
                for (neighbour, a, b, facing) in [
                    ((ii, jj - 1), p00, p10, Vec2::NEG_Y),
                    ((ii + 1, jj), p10, p11, Vec2::X),
                    ((ii, jj + 1), p11, p01, Vec2::Y),
                    ((ii - 1, jj), p01, p00, Vec2::NEG_X),
                ] {
                    if present(neighbour.0, neighbour.1) {
                        continue;
                    }
                    let (a0, b0) = (Vec3::new(a.x, a.y, bottom), Vec3::new(b.x, b.y, bottom));
                    let quad = [a0, b0, b, a];
                    let normal = (quad[1] - quad[0]).cross(quad[2] - quad[0]);
                    if normal.truncate().dot(facing) >= 0. {
                        out.push([quad[0], quad[1], quad[2]]);
                        out.push([quad[0], quad[2], quad[3]]);
                    } else {
                        out.push([quad[0], quad[2], quad[1]]);
                        out.push([quad[0], quad[3], quad[2]]);
                    }
                }
            }
        }
        out
    }

    /// Grind rails along the top's edges (own space): down each side over
    /// hood, roof and trunk, and across the front and the back, split where
    /// the top steps more than a hand's height.
    pub fn rails(&self) -> Vec<Vec<Vec3>> {
        const SPLIT: f32 = 0.15;
        const MIN_LENGTH: f32 = 0.8;
        let mut rails = Vec::new();
        let mut finish = |rail: &mut Vec<Vec3>| {
            let length: f32 = rail.windows(2).map(|w| w[0].distance(w[1])).sum();
            if length >= MIN_LENGTH {
                rails.push(std::mem::take(rail));
            }
            rail.clear();
        };
        // Each edge: the outermost top point of every row (or column).
        let edges: [(usize, usize, bool, bool); 4] = [
            (self.ny, self.nx, false, false), // left side, along the length
            (self.ny, self.nx, false, true),  // right side
            (self.nx, self.ny, true, false),  // back, across
            (self.nx, self.ny, true, true),   // front
        ];
        for (rows, cells, across, far) in edges {
            let mut rail = Vec::new();
            for row in 0..rows {
                let mut order: Vec<usize> = (0..cells).collect();
                if far {
                    order.reverse();
                }
                let edge = order.into_iter().find_map(|cell| {
                    let (i, j) = if across { (row, cell) } else { (cell, row) };
                    self.at(i, j).map(|h| self.point(i, j, h))
                });
                match edge {
                    Some(p) if rail.last().is_none_or(|q: &Vec3| (q.z - p.z).abs() <= SPLIT) => rail.push(p),
                    Some(p) => {
                        finish(&mut rail);
                        rail.push(p);
                    }
                    None => finish(&mut rail),
                }
            }
            finish(&mut rail);
        }
        rails
    }
}

/// A model's survey, a few probes a frame.
#[derive(Clone, Debug)]
pub struct Survey {
    shape: Shape,
    next: usize,
    hits: usize,
}

impl Survey {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        let size = (max - min).truncate().max(Vec2::splat(0.2));
        let mut spacing = SPACING;
        while ((size.x / spacing).ceil() as usize + 1) * ((size.y / spacing).ceil() as usize + 1) > MAX_PROBES {
            spacing *= 1.25;
        }
        let nx = (size.x / spacing).ceil() as usize + 1;
        let ny = (size.y / spacing).ceil() as usize + 1;
        let step = Vec2::new(size.x / (nx - 1) as f32, size.y / (ny - 1) as f32);
        Self { shape: Shape { min, max, nx, ny, step, heights: vec![None; nx * ny] }, next: 0, hits: 0 }
    }

    pub fn done(&self) -> bool {
        self.next >= self.shape.heights.len()
    }

    /// The next probe: from above the box to its bottom, in own space.
    pub fn next_probe(&self) -> Option<(Vec3, Vec3)> {
        (!self.done()).then(|| {
            let (i, j) = (self.next % self.shape.nx, self.next / self.shape.nx);
            // Just inside the edges, so the outermost probes meet the body.
            let p = self.shape.point(i, j, 0.);
            let inset = |v: f32, lo: f32, hi: f32| v.clamp(lo + 0.03, hi - 0.03);
            let (x, y) = (inset(p.x, self.shape.min.x, self.shape.max.x), inset(p.y, self.shape.min.y, self.shape.max.y));
            (Vec3::new(x, y, self.shape.max.z + 0.5), Vec3::new(x, y, self.shape.min.z))
        })
    }

    /// The last probe's hit height (own space), or none.
    pub fn record(&mut self, height: Option<f32>) {
        if let Some(slot) = self.shape.heights.get_mut(self.next) {
            // Something a hand's width above the bottom: the wheels' lower
            // edges are not a roof.
            *slot = height.filter(|z| *z > self.shape.min.z + 0.15).map(|z| z.min(self.shape.max.z));
            if slot.is_some() {
                self.hits += 1;
            }
        }
        self.next += 1;
    }

    /// The surveyed body, or the box when the probes met too little of it.
    pub fn finish(mut self) -> Shape {
        if self.hits * 4 < self.shape.heights.len() {
            Shape::boxed(self.shape.min, self.shape.max)
        } else {
            self.shape.fill_holes();
            self.shape
        }
    }
}

/// A person as an upright box (own space, feet at 0).
pub fn person_shape() -> Shape {
    Shape::boxed(Vec3::new(-0.25, -0.2, -1.0), Vec3::new(0.25, 0.2, 0.85))
}

/// Whether world point `p` is within `margin` of the body at `transform`.
pub fn touches(shape: &Shape, transform: Mat4, p: Vec3, margin: f32) -> bool {
    let q = transform.inverse().transform_point3(p);
    q.x > shape.min.x - margin
        && q.x < shape.max.x + margin
        && q.y > shape.min.y - margin
        && q.y < shape.max.y + margin
        && q.z > shape.min.z - margin
        && q.z < shape.top_at(q.truncate()).unwrap_or(shape.max.z) + margin
}

/// Whether a vehicle body hits world point `p` (a point of the skater):
/// inside the surveyed body, not just its bounding box (GTA's box takes in
/// mirrors and rounded corners, and riding past a car counted as being run
/// over), a little in from its sides.
pub fn hits(shape: &Shape, transform: Mat4, p: Vec3) -> bool {
    const IN: f32 = 0.1;
    let q = transform.inverse().transform_point3(p);
    q.x > shape.min.x + IN
        && q.x < shape.max.x - IN
        && q.y > shape.min.y + IN
        && q.y < shape.max.y - IN
        && q.z > shape.min.z + UNDER
        && shape.top_at(q.truncate()).is_some_and(|top| q.z < top)
}

/// How fast a vehicle at `centre` moving at `vehicle` closes in on a skater
/// at `p` moving at `skater` (m/s; negative when they draw apart). Passing
/// alongside closes nothing.
pub fn closing_speed(centre: Vec3, vehicle: Vec3, p: Vec3, skater: Vec3) -> f32 {
    let toward = (p - centre).truncate().normalize_or_zero();
    (vehicle - skater).truncate().dot(toward)
}

/// Whether the feet at `p` (world) stand on the body's top.
pub fn stands_on(shape: &Shape, transform: Mat4, p: Vec3) -> bool {
    let q = transform.inverse().transform_point3(p);
    shape.top_at(q.truncate()).is_some_and(|top| q.z > top - 0.35 && q.z < top + 0.5)
}

/// How far a point riding on a body moved when the body went from `before`
/// to `after`.
pub fn carried(before: Mat4, after: Mat4, p: Vec3) -> Vec3 {
    after.transform_point3(before.inverse().transform_point3(p)) - p
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Quat;

    /// A car-like body: 2 m wide, 4.5 m long, roof 1.5 m in the middle,
    /// hood and trunk at 1 m.
    fn survey_car() -> Shape {
        let (min, max) = (Vec3::new(-1., -2.25, 0.), Vec3::new(1., 2.25, 1.5));
        let mut survey = Survey::new(min, max);
        while let Some((top, _)) = survey.next_probe() {
            let z = if top.y.abs() < 1. { 1.5 } else { 1.0 };
            survey.record(Some(z));
        }
        survey.finish()
    }

    #[test]
    fn a_surveyed_car_has_its_roof_and_hood() {
        let car = survey_car();
        assert_eq!(car.top_at(Vec2::new(0., 0.)), Some(1.5));
        assert_eq!(car.top_at(Vec2::new(0., 2.)), Some(1.0));
        assert_eq!(car.top_at(Vec2::new(0., 3.)), None, "off the car");
        let triangles = car.triangles();
        assert!(triangles.len() > 100);
        // Every triangle faces away from the car's middle (or up).
        let centre = Vec3::new(0., 0., 0.5);
        for t in &triangles {
            let normal = (t[1] - t[0]).cross(t[2] - t[0]);
            let mid = (t[0] + t[1] + t[2]) / 3.;
            assert!(normal.dot(mid - centre) > -1e-4, "{t:?}");
        }
    }

    #[test]
    fn riding_past_a_car_is_not_being_hit_by_it() {
        let car = survey_car();
        let at = Mat4::from_translation(Vec3::new(5., 5., 0.));
        // Hip height, 15 cm beside the car's side: past it, not hit.
        assert!(!hits(&car, at, Vec3::new(5. + 1.15, 5., 0.95)));
        assert!(!hits(&car, at, Vec3::new(5. + 0.95, 5., 0.95)), "within the box's rounded edge");
        assert!(hits(&car, at, Vec3::new(5. + 0.5, 5., 0.95)), "inside the body");
        assert!(!hits(&car, at, Vec3::new(5. + 0.5, 5., 1.7)), "above the roof (a hippie jump)");
        // A car overtaking alongside closes nothing; one driving at the
        // skater does.
        let (car_at, skater) = (Vec3::new(5., 5., 0.), Vec3::new(6.2, 5., 0.9));
        assert!(closing_speed(car_at, Vec3::new(0., 12., 0.), skater, Vec3::new(0., 5., 0.)).abs() < 0.01);
        assert!(closing_speed(car_at, Vec3::new(8., 0., 0.), skater, Vec3::ZERO) > 7.);
    }

    #[test]
    fn a_car_has_rails_along_its_top_edges() {
        let car = survey_car();
        let rails = car.rails();
        // Each side splits at the step from hood or trunk (1 m) to roof
        // (1.5 m); front and back run across.
        assert!(rails.len() >= 4, "{} rails", rails.len());
        for rail in &rails {
            assert!(rail.iter().all(|p| p.z == 1.0 || p.z == 1.5), "on the top: {rail:?}");
            let step = rail.windows(2).map(|w| (w[0].z - w[1].z).abs()).fold(0., f32::max);
            assert!(step <= 0.15, "no rail climbs the windshield");
        }
        let roof_side = rails.iter().find(|r| r.iter().all(|p| p.z == 1.5 && p.x < 0.)).expect("the roof's left edge");
        let length = roof_side.first().unwrap().distance(*roof_side.last().unwrap());
        assert!(length > 1.2, "roof rail {length} m");
    }

    #[test]
    fn a_board_fits_under_a_car_but_its_rider_does_not() {
        let car = survey_car();
        let lowest = car.triangles().iter().flatten().map(|p| p.z).fold(f32::MAX, f32::min);
        assert!((lowest - UNDER).abs() < 1e-5, "walls end {lowest} m up");
        assert!(lowest > 0.12, "a board with its wheels rolls under");
        assert!(lowest < 0.3, "a skater's legs do not");
    }

    #[test]
    fn a_car_open_over_its_seats_is_closed_over() {
        // A convertible: the probes over the cabin (x -0.5..0.5, y -0.5..0.5)
        // meet only the driver, the body round it is 1.0 high.
        let mut survey = Survey::new(Vec3::new(-1., -2., 0.), Vec3::new(1., 2., 1.4));
        while let Some((top, _)) = survey.next_probe() {
            let cabin = top.x.abs() < 0.6 && top.y.abs() < 0.6;
            survey.record((!cabin).then_some(1.0));
        }
        let car = survey.finish();
        assert_eq!(car.top_at(Vec2::new(0., 0.)), Some(1.0), "no hole to stand in");
        assert!(car.triangles().iter().flatten().all(|p| p.z <= 1.0 + 1e-4), "no walls down into the cabin");
    }

    #[test]
    fn a_missed_survey_falls_back_to_the_box() {
        let mut survey = Survey::new(Vec3::new(-1., -2., 0.), Vec3::new(1., 2., 1.4));
        while survey.next_probe().is_some() {
            survey.record(None);
        }
        let shape = survey.finish();
        assert_eq!(shape.top_at(Vec2::ZERO), Some(1.4));
        assert_eq!(shape.triangles().len(), 2 + 8, "a top and four walls");
    }

    #[test]
    fn long_vehicles_stay_within_the_probe_budget() {
        let survey = Survey::new(Vec3::new(-1.5, -9., 0.), Vec3::new(1.5, 9., 4.));
        assert!(survey.shape.heights.len() <= MAX_PROBES);
    }

    #[test]
    fn contacts_and_standing() {
        let car = survey_car();
        let at = Mat4::from_rotation_translation(Quat::from_rotation_z(0.5), Vec3::new(10., 20., 0.));
        let on_roof = at.transform_point3(Vec3::new(0., 0., 1.55));
        assert!(stands_on(&car, at, on_roof));
        assert!(!stands_on(&car, at, at.transform_point3(Vec3::new(0., 0., 0.2))));
        assert!(touches(&car, at, at.transform_point3(Vec3::new(1.2, 0., 0.9)), 0.3));
        assert!(!touches(&car, at, at.transform_point3(Vec3::new(2.5, 0., 0.9)), 0.3));
    }

    #[test]
    fn riding_on_a_turning_car_moves_with_it() {
        let before = Mat4::from_translation(Vec3::new(0., 0., 0.));
        let after = Mat4::from_rotation_translation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Vec3::new(1., 0., 0.));
        let moved = carried(before, after, Vec3::new(0., 2., 1.5));
        assert!((moved - Vec3::new(-1., -2., 0.)).length() < 1e-4, "{moved}");
    }
}
