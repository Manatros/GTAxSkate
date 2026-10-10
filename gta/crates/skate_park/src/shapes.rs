//! The dropper's objects as geometry: triangles wound outward with their
//! normals and materials, and the edges that grind as rails. Metres, x
//! right, y away from the one dropping it (ramps rise towards +y), z up,
//! centred on the footprint.

type V3 = [f32; 3];

/// What a surface looks like.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Material {
    Concrete,
    Wood,
    Metal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub points: [V3; 3],
    pub normal: V3,
    pub material: Material,
}

/// An object's triangles and grind rails.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shape {
    pub triangles: Vec<Triangle>,
    pub rails: Vec<Vec<V3>>,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn normalize(a: V3) -> V3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 1e-9 { [a[0] / l, a[1] / l, a[2] / l] } else { [0., 0., 1.] }
}

impl Shape {
    /// A triangle wound counter-clockwise seen from outside.
    fn tri(&mut self, a: V3, b: V3, c: V3, material: Material) {
        let normal = cross(sub(b, a), sub(c, a));
        if normal.iter().all(|v| v.abs() < 1e-9) {
            return;
        }
        self.triangles.push(Triangle { points: [a, b, c], normal: normalize(normal), material });
    }

    /// A quad a, b, c, d counter-clockwise seen from outside.
    fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, material: Material) {
        self.tri(a, b, c, material);
        self.tri(a, c, d, material);
    }

    /// A box from `lo` to `hi`.
    fn cuboid(&mut self, lo: V3, hi: V3, material: Material) {
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        let p = |x: f32, y: f32, z: f32| [x, y, z];
        self.quad(p(x0, y0, z1), p(x1, y0, z1), p(x1, y1, z1), p(x0, y1, z1), material); // top
        self.quad(p(x0, y1, z0), p(x1, y1, z0), p(x1, y0, z0), p(x0, y0, z0), material); // bottom
        self.quad(p(x0, y0, z0), p(x1, y0, z0), p(x1, y0, z1), p(x0, y0, z1), material); // front (-y)
        self.quad(p(x1, y1, z0), p(x0, y1, z0), p(x0, y1, z1), p(x1, y1, z1), material); // back (+y)
        self.quad(p(x0, y1, z0), p(x0, y0, z0), p(x0, y0, z1), p(x0, y1, z1), material); // left (-x)
        self.quad(p(x1, y0, z0), p(x1, y1, z0), p(x1, y1, z1), p(x1, y0, z1), material); // right (+x)
    }

    /// A ramp `width` wide over `y0`..`y1`, rising from 0 to `height` along
    /// +y, with its back wall and sides.
    fn wedge(&mut self, width: f32, y0: f32, y1: f32, height: f32, material: Material) {
        let (x0, x1) = (-width / 2., width / 2.);
        let p = |x: f32, y: f32, z: f32| [x, y, z];
        self.quad(p(x0, y0, 0.), p(x1, y0, 0.), p(x1, y1, height), p(x0, y1, height), material);
        self.quad(p(x1, y1, 0.), p(x0, y1, 0.), p(x0, y1, height), p(x1, y1, height), material);
        self.quad(p(x0, y1, 0.), p(x1, y1, 0.), p(x1, y0, 0.), p(x0, y0, 0.), material);
        self.tri(p(x0, y1, 0.), p(x0, y0, 0.), p(x0, y1, height), material);
        self.tri(p(x1, y0, 0.), p(x1, y1, 0.), p(x1, y1, height), material);
    }

    /// The edges along x at `y`, `z`, as one rail from `x0` to `x1`.
    fn rail_x(&mut self, x0: f32, x1: f32, y: f32, z: f32) {
        self.rails.push(vec![[x0, y, z], [x1, y, z]]);
    }

    /// Centres the footprint on the origin.
    fn centred(mut self) -> Self {
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for p in self.triangles.iter().flat_map(|t| t.points) {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
        let shift = |p: &mut V3| {
            p[0] -= c[0];
            p[1] -= c[1];
        };
        for t in &mut self.triangles {
            t.points.iter_mut().for_each(shift);
        }
        for rail in &mut self.rails {
            rail.iter_mut().for_each(shift);
        }
        self
    }
}

/// A rail `length` long along y, its top at `z0` at the near end and `z1`
/// at the far one, on posts.
fn rail(length: f32, z0: f32, z1: f32) -> Shape {
    use Material::Metal;
    let mut s = Shape::default();
    const BAR: f32 = 0.025;
    let (y0, y1) = (0., length);
    let slope = |y: f32| z0 + (z1 - z0) * (y - y0) / length;
    // The bar in pieces, so a sloped one follows its slope.
    const PIECES: usize = 6;
    for i in 0..PIECES {
        let a = y0 + length * i as f32 / PIECES as f32;
        let b = y0 + length * (i + 1) as f32 / PIECES as f32;
        let (za, zb) = (slope(a), slope(b));
        let p = |x: f32, y: f32, z: f32| [x, y, z];
        s.quad(p(-BAR, a, za), p(BAR, a, za), p(BAR, b, zb), p(-BAR, b, zb), Metal);
        s.quad(p(BAR, a, za - 2. * BAR), p(-BAR, a, za - 2. * BAR), p(-BAR, b, zb - 2. * BAR), p(BAR, b, zb - 2. * BAR), Metal);
        s.quad(p(-BAR, a, za - 2. * BAR), p(-BAR, a, za), p(-BAR, b, zb), p(-BAR, b, zb - 2. * BAR), Metal);
        s.quad(p(BAR, a, za), p(BAR, a, za - 2. * BAR), p(BAR, b, zb - 2. * BAR), p(BAR, b, zb), Metal);
    }
    s.quad([-BAR, y0, z0 - 2. * BAR], [BAR, y0, z0 - 2. * BAR], [BAR, y0, z0], [-BAR, y0, z0], Metal);
    s.quad([BAR, y1, z1 - 2. * BAR], [-BAR, y1, z1 - 2. * BAR], [-BAR, y1, z1], [BAR, y1, z1], Metal);
    for y in [y0 + 0.2, y1 - 0.2] {
        s.cuboid([-0.03, y - 0.03, 0.], [0.03, y + 0.03, slope(y) - 2. * BAR], Metal);
    }
    s.rails.push(vec![[0., y0, z0], [0., y1, z1]]);
    s
}

/// Object `kind`'s shape (see [`crate::KINDS`]); an unknown kind is a
/// crate.
pub fn shape(kind: usize) -> Shape {
    use Material::*;
    let mut s = Shape::default();
    match kind {
        // Kicker: a short, steep little launch.
        0 => s.wedge(1.2, 0., 1.3, 0.45, Wood),
        // Launch ramp: longer and higher, with a flat deck at the top.
        1 => {
            s.wedge(1.8, 0., 2.4, 1.0, Wood);
            s.cuboid([-0.9, 2.4, 0.], [0.9, 3.0, 1.0], Wood);
        }
        // Quarter pipe: a transition up to vertical, a deck, coping.
        2 => {
            const R: f32 = 1.8;
            const W: f32 = 3.0;
            const STEPS: usize = 12;
            const DECK: f32 = 1.0;
            let (x0, x1) = (-W / 2., W / 2.);
            let profile: Vec<(f32, f32)> = (0..=STEPS)
                .map(|i| {
                    let a = (i as f32 / STEPS as f32) * 88f32.to_radians();
                    (R * a.sin(), R * (1. - a.cos()))
                })
                .collect();
            let top = profile[STEPS];
            for w in profile.windows(2) {
                let ((ya, za), (yb, zb)) = (w[0], w[1]);
                s.quad([x0, ya, za], [x1, ya, za], [x1, yb, zb], [x0, yb, zb], Concrete);
            }
            let back = top.0 + DECK;
            s.quad([x0, top.0, top.1], [x1, top.0, top.1], [x1, back, top.1], [x0, back, top.1], Concrete);
            s.quad([x1, back, 0.], [x0, back, 0.], [x0, back, top.1], [x1, back, top.1], Concrete);
            s.quad([x0, back, 0.], [x1, back, 0.], [x1, 0., 0.], [x0, 0., 0.], Concrete);
            // The sides, fanned from the bottom of the back wall.
            let corner = (back, 0.);
            let mut outline = profile.clone();
            outline.push((back, top.1));
            for w in outline.windows(2) {
                let ((ya, za), (yb, zb)) = (w[0], w[1]);
                s.tri([x0, corner.0, corner.1], [x0, ya, za], [x0, yb, zb], Concrete);
                s.tri([x1, corner.0, corner.1], [x1, yb, zb], [x1, ya, za], Concrete);
            }
            s.rail_x(x0, x1, top.0, top.1);
        }
        // Bank: a long, tall wedge to carve and air off.
        3 => s.wedge(2.4, 0., 2.2, 1.2, Concrete),
        4 => s = rail(3.5, 0.35, 0.35),
        // Down rail: from waist height down to a low end, ridden downhill
        // (towards -y), so it starts high at the far end.
        5 => s = rail(3.5, 0.35, 1.0),
        // Ledge: a concrete block with metal-edged top corners.
        6 => {
            s.cuboid([-0.3, 0., 0.], [0.3, 3.5, 0.45], Concrete);
            s.rails.push(vec![[-0.3, 0., 0.45], [-0.3, 3.5, 0.45]]);
            s.rails.push(vec![[0.3, 0., 0.45], [0.3, 3.5, 0.45]]);
        }
        // Manual pad: low and wide; its edges grind too.
        7 => {
            s.cuboid([-0.7, 0., 0.], [0.7, 3.0, 0.2], Concrete);
            s.rails.push(vec![[-0.7, 0., 0.2], [-0.7, 3.0, 0.2]]);
            s.rails.push(vec![[0.7, 0., 0.2], [0.7, 3.0, 0.2]]);
        }
        // Funbox: banks up to a flat top and down again, ledges on its
        // sides.
        8 => {
            const H: f32 = 0.6;
            const W: f32 = 2.4;
            s.wedge(W, 0., 1.2, H, Wood);
            s.cuboid([-W / 2., 1.2, 0.], [W / 2., 3.0, H], Wood);
            // The far bank, the near one turned round.
            let mut far = Shape::default();
            far.wedge(W, 0., 1.2, H, Wood);
            for t in &mut far.triangles {
                for p in &mut t.points {
                    p[0] = -p[0];
                    p[1] = 4.2 - p[1];
                }
                t.normal = [-t.normal[0], -t.normal[1], t.normal[2]];
            }
            s.triangles.extend(far.triangles);
            s.rails.push(vec![[-W / 2., 1.2, H], [-W / 2., 3.0, H]]);
            s.rails.push(vec![[W / 2., 1.2, H], [W / 2., 3.0, H]]);
        }
        // Crate: a wooden box to ollie onto or over.
        _ => {
            s.cuboid([-0.6, 0., 0.], [0.6, 1.2, 0.7], Wood);
            for (y, x0, x1) in [(0., -0.6, 0.6), (1.2, -0.6, 0.6)] {
                s.rail_x(x0, x1, y, 0.7);
            }
        }
    }
    s.centred()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_object_is_solid_and_outward() {
        for kind in 0..crate::KINDS.len() {
            let s = shape(kind);
            assert!(s.triangles.len() >= 8, "{}", crate::KINDS[kind]);
            assert!(!s.rails.is_empty() || matches!(kind, 0 | 1 | 3), "{} grinds", crate::KINDS[kind]);
            let mut lo = [f32::INFINITY; 3];
            let mut hi = [f32::NEG_INFINITY; 3];
            let mut centre = [0f32; 3];
            let n = (s.triangles.len() * 3) as f32;
            for p in s.triangles.iter().flat_map(|t| t.points) {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                    centre[k] += p[k] / n;
                }
            }
            assert!(lo[2].abs() < 1e-5, "{} sits on the ground", crate::KINDS[kind]);
            assert!((lo[0] + hi[0]).abs() < 1e-4 && (lo[1] + hi[1]).abs() < 1e-4, "centred");
            // Wound outward, the enclosed volume (divergence theorem) is
            // positive and within the bounding box's.
            let volume: f32 = s
                .triangles
                .iter()
                .map(|t| {
                    let [a, b, c] = t.points;
                    let bc = cross(b, c);
                    (a[0] * bc[0] + a[1] * bc[1] + a[2] * bc[2]) / 6.
                })
                .sum();
            let bounds = (hi[0] - lo[0]) * (hi[1] - lo[1]) * (hi[2] - lo[2]);
            assert!(volume > 0. && volume <= bounds * 1.01, "{}: {volume} of {bounds}", crate::KINDS[kind]);
            assert!(centre.iter().all(|v| v.is_finite()));
            for rail in &s.rails {
                assert!(rail.len() >= 2 && rail.iter().all(|p| p.iter().all(|v| v.is_finite())));
            }
        }
    }
}
