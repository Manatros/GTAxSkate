//! Griptapes as their own models (`skate_boards::grips`): the deck's grip
//! side cut from its mesh and lifted 2 mm off it, textured with a griptape
//! made from Skate 3's own grain (its bolt holes too) in a colour, a
//! checkerboard or a deck's graphic. One model per deck shape and griptape:
//! the plugin lays it on whatever deck is ridden.

use glam::{Vec2, Vec3};
use skate_boards::grips::{self, GripLook};
use skate_boards::{Board, Library, Part, PartRef, Texture};

use crate::weapon::{Frame, Laid, laid_in};

/// The grip texture: the board's length across, its width down.
const WIDTH: u32 = 2048;
const HEIGHT: u32 = 512;
/// How far the grip lies off the deck (m).
const LIFT: f32 = 0.002;
/// Grain this bright is a bolt head.
const BOLT: f32 = 1.7;

/// Triangles facing this far toward -X (the deck's grip side in the weapon
/// frame) are the grip.
const GRIP_FACING: f32 = 0.7;

/// The faces of `laid` facing `side` (-1: the grip, +1: the graphic): their
/// corners, uvs and the uvs' box.
pub(crate) fn faces(laid: &Laid, side: f32) -> (Vec<[Vec3; 3]>, Vec<[Vec2; 3]>) {
    let (mut tris, mut uvs) = (Vec::new(), Vec::new());
    for (positions, normals, uv, indices, _) in laid {
        for t in indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
            // By the vertices' normals (some faces are wound the other way)
            // and by the texture's half: the grip's top, the graphic's bottom.
            let n = (normals[a] + normals[b] + normals[c]).normalize_or_zero();
            let v = (uv[a].y + uv[b].y + uv[c].y) / 3.;
            let half = if side < 0. { v < 0.5 } else { v > 0.5 };
            if n.x * side > GRIP_FACING && half {
                tris.push([positions[a], positions[b], positions[c]]);
                uvs.push([uv[a], uv[b], uv[c]]);
            }
        }
    }
    (tris, uvs)
}

fn uv_box(uvs: &[[Vec2; 3]]) -> (Vec2, Vec2) {
    uvs.iter().flatten().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)))
}

/// `region` (a uv box) of `texture`, resampled to `w` × `h`; `flip` mirrors
/// u and v.
fn crop(texture: &Texture, region: (Vec2, Vec2), w: u32, h: u32, flip: (bool, bool)) -> Texture {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut fx = (x as f32 + 0.5) / w as f32;
            let mut fy = (y as f32 + 0.5) / h as f32;
            if flip.0 {
                fx = 1. - fx;
            }
            if flip.1 {
                fy = 1. - fy;
            }
            let u = region.0.x + (region.1.x - region.0.x) * fx;
            let v = region.0.y + (region.1.y - region.0.y) * fy;
            rgba.extend(texture.sample([u, v]));
        }
    }
    Texture { width: w, height: h, rgba }
}

/// The grain as brightness about 1 (bolt holes bright), from a deck's
/// stored finish: the grip side's colour under its see-through.
fn grain(stored: &Texture, region: (Vec2, Vec2)) -> Vec<f32> {
    let cut = crop(stored, region, WIDTH, HEIGHT, (false, false));
    let light: Vec<f32> = cut.rgba.chunks_exact(4).map(|p| (f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2])) / 765.).collect();
    let mean = (light.iter().sum::<f32>() / light.len().max(1) as f32).max(0.05);
    let mut g: Vec<f32> = light.into_iter().map(|l| (l / mean).clamp(0.3, 2.5)).collect();
    // The deck's edge and a sliver of its graphic can lie in the box: a
    // column or row clearly brighter or darker than grain takes the grain of
    // one that is not.
    let (w, h) = (WIDTH as usize, HEIGHT as usize);
    let odd = |m: f32| !(0.75..=1.3).contains(&m);
    let column = |g: &[f32], x: usize| (0..h).map(|y| g[y * w + x]).sum::<f32>() / h as f32;
    let good_cols: Vec<usize> = (0..w).filter(|&x| !odd(column(&g, x))).collect();
    if !good_cols.is_empty() {
        for x in 0..w {
            if odd(column(&g, x)) {
                let from = good_cols[x % good_cols.len()];
                for y in 0..h {
                    g[y * w + x] = g[y * w + from];
                }
            }
        }
    }
    let row = |g: &[f32], y: usize| g[y * w..(y + 1) * w].iter().sum::<f32>() / w as f32;
    let good_rows: Vec<usize> = (0..h).filter(|&y| !odd(row(&g, y))).collect();
    if !good_rows.is_empty() {
        for y in 0..h {
            if odd(row(&g, y)) {
                let from = good_rows[y % good_rows.len()];
                let copy: Vec<f32> = g[from * w..(from + 1) * w].to_vec();
                g[y * w..(y + 1) * w].copy_from_slice(&copy);
            }
        }
    }
    // Skate's grain is a 512 px deck texture's: blurred at this size. Real
    // grip is grit about half a millimetre across (a texel here, 2048 along
    // 0.8 m): laid over it, the bolt heads left bare (the maintainer, video 10-10
    // 09-43 07:33: "zu low resolution").
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if g[i] <= BOLT {
                g[i] *= 1. + GRIT * (grit(x as u32, y as u32) - 0.5);
            }
        }
    }
    g
}

/// The grip's specular level: sandpaper, barely a sheen.
const GRIP_SHINE: u8 = 16;

/// A normal map of the grain read as height (bright grit stands up): how
/// light falls on sandpaper.
fn grit_normals(grain: &[f32]) -> Texture {
    let (w, h) = (WIDTH as usize, HEIGHT as usize);
    let at = |x: usize, y: usize| grain.get(y.min(h - 1) * w + x.min(w - 1)).copied().unwrap_or(1.).min(BOLT);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let dx = at(x + 1, y) - at(x.saturating_sub(1), y);
            let dy = at(x, y + 1) - at(x, y.saturating_sub(1));
            let n = Vec3::new(-dx * BUMP, -dy * BUMP, 1.).normalize();
            let byte = |v: f32| ((v + 1.) * 127.5).round().clamp(0., 255.) as u8;
            rgba.extend_from_slice(&[byte(n.x), byte(n.y), byte(n.z), 255]);
        }
    }
    Texture { width: WIDTH, height: HEIGHT, rgba }
}

/// How steep the grit's bumps are in the normal map.
const BUMP: f32 = 1.6;

/// How strong the fine grit is (brightness, either way).
const GRIT: f32 = 0.45;

/// A texel's grit, 0 to 1: noise in clusters of about two texels.
fn grit(x: u32, y: u32) -> f32 {
    let hash = |x: u32, y: u32| {
        let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        (h & 0xFFFF) as f32 / 65535.
    };
    0.6 * hash(x, y) + 0.4 * hash(x / 2, y / 2)
}

/// A griptape's texture.
pub fn design(look: &GripLook, grain: &[f32], print: Option<&Texture>) -> Texture {
    let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    // A grip's grain shows (it is sandpaper); the bolt heads through it
    // stay bare metal whatever its colour.
    let tone = |c: [u8; 3], g: f32| {
        if g > BOLT {
            let m = (150. + 40. * (g - BOLT)).min(215.) as u8;
            return [m, m, m.saturating_add(4)];
        }
        c.map(|v| (f32::from(v) * (0.38 + 0.62 * g)).round().clamp(0., 255.) as u8)
    };
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let g = grain.get((y * WIDTH + x) as usize).copied().unwrap_or(1.);
            let c = match look {
                GripLook::Solid(c) => tone(*c, g),
                GripLook::Checker(a, b, across) => {
                    let square = (HEIGHT / (*across).max(1)).max(1);
                    tone(if (x / square + y / square) % 2 == 0 { *a } else { *b }, g)
                }
                GripLook::America => {
                    // Seven red and six white stripes along the board; the
                    // blue field with its stars at the nose end, across four
                    // stripes' worth of the width.
                    let stripe = (y * 13 / HEIGHT) as usize;
                    let (fw, fh) = (WIDTH * 2 / 5, HEIGHT * 7 / 13);
                    if x >= WIDTH - fw && y < fh {
                        let (sx, sy) = (WIDTH - 1 - x, y);
                        let (cell_w, cell_h) = (fw / 10, fh / 6);
                        let (cx, cy) = (sx % cell_w, sy % cell_h);
                        let row = sy / cell_h;
                        let offset = if row % 2 == 0 { 0 } else { cell_w / 2 };
                        let dx = (cx + cell_w - offset) % cell_w;
                        let star = dx.abs_diff(cell_w / 2) * dx.abs_diff(cell_w / 2) + cy.abs_diff(cell_h / 2) * cy.abs_diff(cell_h / 2) <= 6;
                        tone(if star { [235, 235, 230] } else { [35, 50, 120] }, g)
                    } else {
                        tone(if stripe % 2 == 0 { [180, 30, 40] } else { [235, 235, 230] }, g)
                    }
                }
                GripLook::Print(_) => {
                    let base = tone([38, 38, 40], g);
                    match print {
                        Some(p) if g <= BOLT => {
                            let at = ((y * p.width + x) * 4) as usize;
                            let px = &p.rgba[at..at + 4];
                            // The graphic as the deck's underside shows it
                            // (on its wood), printed on the grip; off the
                            // deck's outline (no graphic there), plain grip.
                            if px == [0, 0, 0, 0] {
                                base
                            } else {
                                // As the deck's underside shows it: solid.
                                [0, 1, 2].map(|i| (f32::from(px[i]) * (0.6 + 0.4 * g) * 0.88 + f32::from(base[i]) * 0.12).round().clamp(0., 255.) as u8)
                            }
                        }
                        _ => base,
                    }
                }
            };
            rgba.extend([c[0], c[1], c[2], 255]);
        }
    }
    Texture { width: WIDTH, height: HEIGHT, rgba }
}

/// A deck's stored finish (its see-through kept).
fn stored(library: &Library, material: &str) -> Option<Texture> {
    let bytes = std::fs::read(library.texture_path(material)?).ok()?;
    skate_boards::decode_png(&bytes).ok()
}

/// Every griptape on every deck shape: `sk3_g_<shape>_<id>`. Returns each
/// model's name and box.
pub fn write_grips(dir: &std::path::Path, library: &Library, defaults: &Board, frame: &Frame) -> Result<Vec<(String, Vec3, Vec3)>, String> {
    let decks = library.items(Part::Deck);
    // The deck shapes: one finish of each library deck model, and the
    // default skater's deck.
    let mut shapes: Vec<(Option<PartRef>, Board)> = vec![(None, defaults.only(Part::Deck))];
    let mut seen = Vec::new();
    for item in decks {
        let key = grips::deck_key(Some(&item.part_ref()));
        if !seen.contains(&key) {
            seen.push(key);
            shapes.push((Some(item.part_ref()), library.part_mesh(Part::Deck, &item.part_ref(), 1024)?));
        }
    }
    // The grain: a library deck's grip side.
    let (grain_source, grain_region) = decks
        .first()
        .and_then(|item| {
            let board = library.part_mesh(Part::Deck, &item.part_ref(), 1024).ok()?;
            let (_, uvs) = faces(&laid_in(&board, defaults, frame), -1.);
            Some((stored(library, &item.material)?, uv_box(&uvs)))
        })
        .ok_or("no deck to take the grip's grain from")?;
    let grain = grain(&grain_source, grain_region);
    let bumps = grit_normals(&grain);
    let catalog = grips::catalog(Some(library));
    let mut out = Vec::new();
    for (deck, board) in &shapes {
        let laid = laid_in(board, defaults, frame);
        let (tris, uvs) = faces(&laid, -1.);
        if tris.is_empty() {
            continue;
        }
        let region = uv_box(&uvs);
        let size = (region.1 - region.0).max(Vec2::splat(1e-6));
        let mut grip_faces = Vec::new();
        // The grip's own mesh, lifted off the deck, its uvs over the whole
        // grip texture.
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut mapped = Vec::new();
        for (t, uv) in tris.iter().zip(&uvs) {
            let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            for (p, q) in t.iter().zip(uv) {
                positions.push(*p - Vec3::X * LIFT);
                normals.push(n);
                mapped.push((*q - region.0) / size);
            }
            grip_faces.push((*t, uv.map(|q| (q - region.0) / size)));
        }
        let indices: Vec<u32> = (0..positions.len() as u32).collect();
        let surface: Laid = vec![(positions, normals, mapped, indices, 0)];
        for grip in &catalog {
            let print = match &grip.look {
                GripLook::Print(source) => print_for(library, source, defaults, frame, &grip_faces),
                _ => None,
            };
            let texture = design(&grip.look, &grain, print.as_ref());
            let name = grips::grip_model(deck.as_ref(), &grip.id);
            // Compressed: 2048 px uncompressed would add ~300 MB to the pack.
            // Its grit as a normal map and a dull sheen: flat it looked
            // painted on (the maintainer, video 10-10 10-22 07:08).
            let normals = [Some(bumps.clone())];
            let (lo, hi) = crate::weapon::write_model_lit(dir, &name, &surface, std::slice::from_ref(&texture), false, GRIP_SHINE, &normals)?;
            out.push((name, lo, hi));
        }
    }
    Ok(out)
}

/// Fills `grid` (w × h cells over `area`, a box on the plane the points
/// are given in) with each triangle's values, interpolated.
fn rasterize<const N: usize>(grid: &mut [Option<[f32; N]>], w: usize, h: usize, area: (Vec2, Vec2), tris: &[([Vec2; 3], [[f32; N]; 3])]) {
    let size = (area.1 - area.0).max(Vec2::splat(1e-6));
    for (corners, values) in tris {
        let p = corners.map(|c| (c - area.0) / size * Vec2::new(w as f32, h as f32));
        let (lo, hi) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
        let det = (p[1] - p[0]).perp_dot(p[2] - p[0]);
        if det.abs() < 1e-9 {
            continue;
        }
        for y in (lo.y.floor().max(0.) as usize)..(hi.y.ceil().min(h as f32) as usize) {
            for x in (lo.x.floor().max(0.) as usize)..(hi.x.ceil().min(w as f32) as usize) {
                let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let b1 = (q - p[0]).perp_dot(p[2] - p[0]) / det;
                let b2 = (p[1] - p[0]).perp_dot(q - p[0]) / det;
                let b0 = 1. - b1 - b2;
                if b0 < -0.01 || b1 < -0.01 || b2 < -0.01 {
                    continue;
                }
                grid[y * w + x] = Some(std::array::from_fn(|k| values[0][k] * b0 + values[1][k] * b1 + values[2][k] * b2));
            }
        }
    }
}

/// A deck's graphic on the grip texture, each grip texel taking the
/// graphic from under it on the board (mirrored, so it reads from above as
/// it does on the underside). `grip` is the grip's faces with
/// their texture places (0..1 over the grip texture).
fn print_for(library: &Library, source: &PartRef, defaults: &Board, frame: &Frame, grip: &[([Vec3; 3], [Vec2; 3])]) -> Option<Texture> {
    let board = library.part_mesh(Part::Deck, source, 1024).ok()?;
    let (tris, uvs) = faces(&laid_in(&board, defaults, frame), 1.);
    if tris.is_empty() {
        return None;
    }
    let graphic = stored(library, &source.material)?;
    // The graphic's uv over the board's plane (z along, y across).
    let (gw, gh) = (1024usize, 256usize);
    let plane = |p: Vec3| Vec2::new(p.z, p.y);
    let area = tris.iter().flatten().chain(grip.iter().flat_map(|(t, _)| t)).fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| {
        let q = plane(*p);
        (lo.min(q), hi.max(q))
    });
    let mut under = vec![None; gw * gh];
    let bottom: Vec<([Vec2; 3], [[f32; 2]; 3])> = tris.iter().zip(&uvs).map(|(t, uv)| (t.map(plane), uv.map(|u| [u.x, u.y]))).collect();
    rasterize(&mut under, gw, gh, area, &bottom);
    // Each grip texel's place on the board's plane.
    let (w, h) = (WIDTH as usize, HEIGHT as usize);
    let mut place = vec![None; w * h];
    // Mirrored along the board: the graphic then reads on the grip as it
    // does on the underside.
    let (z0, z1) = (area.0.x, area.1.x);
    let top: Vec<([Vec2; 3], [[f32; 2]; 3])> = grip.iter().map(|(t, uv)| (*uv, t.map(|p| [z0 + z1 - p.z, p.y]))).collect();
    rasterize(&mut place, w, h, (Vec2::ZERO, Vec2::ONE), &top);
    let size = (area.1 - area.0).max(Vec2::splat(1e-6));
    let mut rgba = Vec::with_capacity(w * h * 4);
    for spot in place {
        let px = spot
            .and_then(|[z, y]| {
                let cell = ((Vec2::new(z, y) - area.0) / size * Vec2::new(gw as f32, gh as f32)).floor();
                let (cx, cy) = (cell.x as isize, cell.y as isize);
                (cx >= 0 && cy >= 0 && (cx as usize) < gw && (cy as usize) < gh).then(|| under[cy as usize * gw + cx as usize]).flatten()
            })
            .map_or([0, 0, 0, 0], |[u, v]| graphic.sample([u, v]));
        rgba.extend(px);
    }
    Some(Texture { width: WIDTH, height: HEIGHT, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_fills_its_cells_with_its_values() {
        // Half the unit square, the value running 0..1 along x.
        let mut grid = vec![None; 10 * 10];
        let tri = ([Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(0., 1.)], [[0.], [1.], [0.]]);
        rasterize(&mut grid, 10, 10, (Vec2::ZERO, Vec2::ONE), &[tri]);
        let filled = grid.iter().filter(|c| c.is_some()).count();
        assert!((40..=60).contains(&filled), "about half the cells: {filled}");
        let v = grid[2 * 10 + 5].unwrap()[0];
        assert!((v - 0.55).abs() < 0.02, "interpolated: {v}");
        assert!(grid[9 * 10 + 9].is_none(), "outside stays empty");
    }

    #[test]
    fn a_solid_grip_keeps_the_grain_and_a_checker_alternates() {
        let mut grain = vec![1.; (WIDTH * HEIGHT) as usize];
        grain[0] = 1.4;
        grain[2] = 2.2;
        let red = design(&GripLook::Solid([200, 0, 0]), &grain, None);
        assert!(red.rgba[0] > red.rgba[4], "a bright grain spot stays brighter");
        assert_eq!(red.rgba[1], 0);
        let bolt = &red.rgba[8..11];
        assert!(bolt[0] > 140 && bolt[0] == bolt[1], "a bolt head stays bare metal: {bolt:?}");
        let checker = design(&GripLook::Checker([255, 255, 255], [0, 0, 0], 4), &grain, None);
        let square = (HEIGHT / 4) as usize;
        let at = |x: usize, y: usize| checker.rgba[(y * WIDTH as usize + x) * 4];
        assert!(at(1, 1) > 200 && at(square + 1, 1) < 30 && at(square + 1, square + 1) > 200);
    }
}

/// Scuff stages laid over a deck's graphic as it wears: light, worn,
/// battered (`sk3_s_<shape>_<1..3>`, see-through but for the scratches).
pub const SCUFF_STAGES: u8 = 3;

/// The scuff model of `stage` on `deck`.
pub fn scuff_model(deck: Option<&PartRef>, stage: u8) -> String {
    format!("sk3_s_{}_{stage}", grips::deck_key(deck))
}

/// Scratches into the bare maple, denser at the nose and tail (where a
/// board meets curbs and the ground), and chips round its edge. `place`
/// gives each texel's board point (z along, y across), none off the deck.
fn scuffs(place: &[Option<[f32; 2]>], w: usize, h: usize, stage: u8, seed: u32) -> Texture {
    let mut rgba = vec![0u8; w * h * 4];
    let (z0, z1) = place.iter().flatten().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
    let (mid, half) = ((z0 + z1) / 2., ((z1 - z0) / 2.).max(1e-3));
    let mut s = seed | 1;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f32 / u32::MAX as f32
    };
    let paint = |x: usize, y: usize, c: [u8; 3], a: u8, rgba: &mut Vec<u8>| {
        let at = (y * w + x) * 4;
        if place[y * w + x].is_some() && rgba[at + 3] < a {
            rgba[at..at + 4].copy_from_slice(&[c[0], c[1], c[2], a]);
        }
    };
    let count = [0, 90, 260, 700][stage as usize];
    let mut made = 0;
    let mut tries = 0;
    while made < count && tries < count * 40 {
        tries += 1;
        let (x, y) = ((rnd() * w as f32) as usize % w, (rnd() * h as f32) as usize % h);
        let Some([z, _]) = place[y * w + x] else { continue };
        let end = ((z - mid).abs() / half).clamp(0., 1.);
        if rnd() > 0.15 + 0.85 * end.powi(3) {
            continue;
        }
        made += 1;
        // Mostly along the board (the texture's long axis), a little askew.
        let len = 4. + rnd() * (10. + 14. * stage as f32);
        let angle = (rnd() - 0.5) * 0.9 + if rnd() < 0.2 { std::f32::consts::FRAC_PI_2 } else { 0. };
        let wood = [222 - (rnd() * 30.) as u8, 196 - (rnd() * 30.) as u8, 150 - (rnd() * 30.) as u8];
        let alpha = 150 + (rnd() * 105.) as u8;
        for k in 0..len as usize {
            let px = x as f32 + angle.cos() * k as f32;
            let py = y as f32 + angle.sin() * k as f32 * 0.5;
            if px >= 0. && py >= 0. && (px as usize) < w && (py as usize) < h {
                paint(px as usize, py as usize, wood, alpha, &mut rgba);
            }
        }
    }
    // Chips round the edge: texels next to the outline.
    let chip = [0., 0.03, 0.09, 0.22][stage as usize];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let edge = place[y * w + x].is_some() && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| place[(y as isize + dy) as usize * w + (x as isize + dx) as usize].is_none());
            if edge && rnd() < chip {
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1)] {
                    paint((x as isize + dx) as usize, (y as isize + dy) as usize, [214, 186, 140], 235, &mut rgba);
                }
            }
        }
    }
    Texture { width: w as u32, height: h as u32, rgba }
}

/// The scuff models for every deck shape: the graphic's faces lifted off
/// the underside by a hair, see-through but for the wear.
pub fn write_scuffs(dir: &std::path::Path, library: &Library, defaults: &Board, frame: &Frame) -> Result<Vec<(String, Vec3, Vec3)>, String> {
    let mut shapes: Vec<(Option<PartRef>, Board)> = vec![(None, defaults.only(Part::Deck))];
    let mut seen = Vec::new();
    for item in library.items(Part::Deck) {
        let key = grips::deck_key(Some(&item.part_ref()));
        if !seen.contains(&key) {
            seen.push(key);
            shapes.push((Some(item.part_ref()), library.part_mesh(Part::Deck, &item.part_ref(), 256)?));
        }
    }
    let (w, h) = (512usize, 256usize);
    let mut out = Vec::new();
    for (deck, board) in &shapes {
        let laid = laid_in(board, defaults, frame);
        let (tris, uvs) = faces(&laid, 1.);
        if tris.is_empty() {
            continue;
        }
        let region = uv_box(&uvs);
        let size = (region.1 - region.0).max(Vec2::splat(1e-6));
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut mapped = Vec::new();
        let mut top = Vec::new();
        for (t, uv) in tris.iter().zip(&uvs) {
            let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            let m = uv.map(|q| (q - region.0) / size);
            for (p, q) in t.iter().zip(&m) {
                positions.push(*p + Vec3::X * 0.0015);
                normals.push(n);
                mapped.push(*q);
            }
            top.push((m, t.map(|p| [p.z, p.y])));
        }
        let mut place = vec![None; w * h];
        rasterize(&mut place, w, h, (Vec2::ZERO, Vec2::ONE), &top);
        let indices: Vec<u32> = (0..positions.len() as u32).collect();
        let surface: Laid = vec![(positions, normals, mapped, indices, 0)];
        for stage in 1..=SCUFF_STAGES {
            let texture = scuffs(&place, w, h, stage, 0x9e37 + u32::from(stage) * 7919);
            let name = scuff_model(deck.as_ref(), stage);
            let (lo, hi) = crate::weapon::write_model_decal(dir, &name, &surface, std::slice::from_ref(&texture))?;
            out.push((name, lo, hi));
        }
    }
    Ok(out)
}
