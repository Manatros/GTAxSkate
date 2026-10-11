//! The Skate 3 board as a GTA V weapon model (`w_me_skate3board`), laid out
//! as GTA's baseball bat is (`w_me_bat`, exported as the template): a three
//! bone weapon skeleton (Gun_Root, Gun_GripR, Gun_Main_Bone), the length
//! along +Z from the grip, every vertex on the root bone, a box for its
//! collision. The board is held by its tail, as a bat by its handle.

use std::fmt::Write;

use glam::{Mat3, Mat4, Vec2, Vec3};
use skate_boards::Board;

use crate::dds;
use crate::xml::{self, Geometry, TextureItem, Vertex};

pub const MODEL: &str = "w_me_skate3board";
/// Where the hand holds it: this far in from the tail along the length.
const GRIP_FROM_TAIL: f32 = 0.1;

fn f(v: f32) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.into() }
}

/// A board's vertices at rest (parts placed by the reference board's
/// joints, as `BoardLook::rest` does): per surface, positions and normals.
fn placed(board: &Board, reference: &Board) -> Vec<(Vec<Vec3>, Vec<Vec3>)> {
    let joint = |name: &str| {
        reference
            .joints
            .iter()
            .find(|j| j.name == name)
            .map(|j| Mat4::from_cols_array(&j.inverse_bind))
            .filter(|m| m.determinant().abs() > 1e-9)
            .map_or(Mat4::IDENTITY, |m| m.inverse())
    };
    let moves: Vec<Mat4> = board.joints.iter().map(|j| joint(&j.name) * Mat4::from_cols_array(&j.inverse_bind)).collect();
    board
        .surfaces
        .iter()
        .map(|s| {
            s.vertices
                .iter()
                .map(|v| {
                    let mut m = Mat4::ZERO;
                    let mut total = 0.;
                    for (j, w) in v.joints.iter().zip(v.weights) {
                        if w > 0. {
                            m += *moves.get(*j as usize).unwrap_or(&Mat4::IDENTITY) * w;
                            total += w;
                        }
                    }
                    let m = if total > 0. { m * (1. / total) } else { Mat4::IDENTITY };
                    (m.transform_point3(Vec3::from_array(v.position)), m.transform_vector3(Vec3::from_array(v.normal)).normalize_or_zero())
                })
                .unzip()
        })
        .collect()
}

/// The weapon's frame for a whole board: its turn (length onto +Z, width
/// +Y, deck +X), its centre and the grip.
pub struct Frame {
    turn: Mat3,
    centre: Vec3,
    grip: Vec3,
}

impl Frame {
    pub fn of(board: &Board, reference: &Board) -> Self {
        // Its extent along each axis: the longest is the length, the
        // shortest goes through the deck.
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in placed(board, reference).iter().flat_map(|(p, _)| p) {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let size = hi - lo;
        let mut order = [0usize, 1, 2];
        order.sort_by(|a, b| size[*b].total_cmp(&size[*a]));
        let unit = |i: usize| Vec3::from_array(std::array::from_fn(|k| if k == i { 1. } else { 0. }));
        let (long, wide, thin) = (unit(order[0]), unit(order[1]), unit(order[2]));
        // Rows of the turn: board axes onto weapon Z, Y and X, kept a rotation.
        let mut turn = Mat3::from_cols(thin, wide, long).transpose();
        if turn.determinant() < 0. {
            turn = Mat3::from_cols(-thin, wide, long).transpose();
        }
        let centre = (lo + hi) * 0.5;
        let tail = turn * (lo - centre);
        let grip = Vec3::new(0., 0., tail.z.min((turn * (hi - centre)).z) + GRIP_FROM_TAIL);
        Self { turn, centre, grip }
    }
}

/// Per surface: positions, normals, uvs, triangles and its texture's index.
pub type Laid = Vec<(Vec<Vec3>, Vec<Vec3>, Vec<Vec2>, Vec<u32>, usize)>;

/// The board's vertices at rest, turned so its length is +Z, its width +Y
/// and its deck faces +X, the grip at the origin.
pub fn laid_out(board: &Board, reference: &Board) -> Laid {
    laid_in(board, reference, &Frame::of(board, reference))
}

/// `board` (a whole board or one part of it) in `frame`: every part laid
/// out in the same frame fits together again.
pub fn laid_in(board: &Board, reference: &Board, frame: &Frame) -> Laid {
    board
        .surfaces
        .iter()
        .zip(placed(board, reference))
        .map(|(s, (positions, normals))| {
            let positions = positions.iter().map(|p| frame.turn * (*p - frame.centre) - frame.grip).collect();
            let normals = normals.iter().map(|n| (frame.turn * *n).normalize_or_zero()).collect();
            let uvs = s.vertices.iter().map(|v| Vec2::from_array(v.uv)).collect();
            (positions, normals, uvs, s.indices.clone(), s.texture)
        })
        .collect()
}

/// A deck's finish laid on the griptape, as Skate 3 draws it: the finish is
/// see-through where the grip shows (most of the top), and the grip under
/// it is dark. `alpha` is the finish as stored, its see-through-ness kept
/// (the part's texture lost it, made opaque). A grip print of its own (a
/// worn deck's, a pro model's) shows over the dark grip as far as it is
/// solid, so decks differ on top too.
pub fn lay_on_grip(texture: &mut skate_boards::Texture, alpha: &skate_boards::Texture) -> bool {
    if alpha.width != texture.width || alpha.height != texture.height || alpha.rgba.len() != texture.rgba.len() {
        return false;
    }
    // The top half is the grip side; under the graphic's see-through bits
    // (the bottom half) Skate shows the deck's wood, left as stored.
    let half = (texture.width * texture.height / 2) as usize;
    for (p, a) in texture.rgba.chunks_exact_mut(4).zip(alpha.rgba.chunks_exact(4)).take(half) {
        let solid = f32::from(a[3]) / 255.;
        for c in p.iter_mut().take(3) {
            let v = f32::from(*c);
            *c = (v * solid + v * GRIP * (1. - solid)).round() as u8;
        }
        p[3] = 255;
    }
    true
}

/// The texture columns (u, 0 to 1) the deck's edges, its plies, are mapped
/// to: a strip running the texture's whole height. None without edge faces.
pub fn edge_columns(laid: &Laid) -> Option<(f32, f32)> {
    let mut range: Option<(f32, f32)> = None;
    for (_, normals, uvs, indices, _) in laid {
        for t in indices.chunks_exact(3) {
            let n = t.iter().map(|i| normals[*i as usize]).sum::<Vec3>().normalize_or_zero();
            if n.x.abs() < 0.5 {
                for i in t {
                    let u = uvs[*i as usize].x;
                    range = Some(range.map_or((u, u), |(a, b)| (a.min(u), b.max(u))));
                }
            }
        }
    }
    range
}

/// The plies' strip in the grip half takes the wood of the same strip in
/// the graphic half: darkened as grip, the deck's edge was half dark, half
/// wood (the maintainer, video 10-10 10-22 07:16: "Two-Tone am Rand", "Holz falsch").
pub fn keep_wood(texture: &mut skate_boards::Texture, edges: Option<(f32, f32)>) {
    let Some((from, to)) = edges else { return };
    let (w, h) = (texture.width as usize, texture.height as usize);
    let x0 = ((from * w as f32).floor() as usize).saturating_sub(2);
    let x1 = (((to * w as f32).ceil() as usize) + 2).min(w);
    for y in 0..h / 2 {
        for x in x0..x1 {
            let (top, bottom) = ((y * w + x) * 4, ((y + h / 2) * w + x) * 4);
            let wood: [u8; 4] = texture.rgba[bottom..bottom + 4].try_into().unwrap_or([0; 4]);
            texture.rgba[top..top + 4].copy_from_slice(&wood);
        }
    }
}

/// The grip's darkness: its grey grain times this.
const GRIP: f32 = 0.16;

/// Without the stored finish (the default board's): grey in the top half
/// turns grip black; coloured grip art and the plies' edge keep their colour.
pub fn darken_grip(texture: &mut skate_boards::Texture) {
    let (w, h) = (texture.width as usize, texture.height as usize);
    for y in 0..h / 2 {
        for x in 0..w {
            let at = (y * w + x) * 4;
            let Some(p) = texture.rgba.get_mut(at..at + 3) else { return };
            let (lo, hi) = (p.iter().min().copied().unwrap_or(0), p.iter().max().copied().unwrap_or(0));
            if hi - lo < 24 && lo > 120 {
                for c in p.iter_mut() {
                    *c = (f32::from(*c) * GRIP).round() as u8;
                }
            }
        }
    }
}

/// The texture names: one per board texture, and the flat normal and dark
/// specular maps every surface shares.
pub fn texture_name(index: usize) -> String {
    format!("skate3board_{index}")
}
const NORMAL: &str = "skate3board_n";
const SPECULAR: &str = "skate3board_s";

/// Writes `w_me_skate3board.ydr.xml` and `.ytd.xml` (with their DDS files
/// in folders of the same names) into `dir`.
pub fn write(dir: &std::path::Path, board: &Board, reference: &Board) -> Result<(), String> {
    write_model(dir, MODEL, &laid_out(board, reference), &board.textures, true).map(|_| ())
}

/// Every board part in the library, each finish its own model laid out in
/// the default board's weapon frame (`sk3_<d|t|w>_<finish>`, and
/// `sk3_<d|t|w>_default` for the default board's): the plugin puts a
/// board together from three of them, props (`skate3parts.ytyp.xml`: GTA
/// holds only 115 weapon models). Returns the model names.
pub fn write_parts(dir: &std::path::Path, library: &skate_boards::Library, defaults: &Board, progress: impl Fn(usize, usize)) -> Result<Vec<String>, String> {
    use skate_boards::Part;
    let frame = Frame::of(defaults, defaults);
    let mut jobs: Vec<(String, Part, Option<skate_boards::PartRef>)> = Vec::new();
    for part in Part::ALL {
        jobs.push((part_model(part, None), part, None));
        for item in library.items(part) {
            jobs.push((part_model(part, Some(&item.material)), part, Some(item.part_ref())));
        }
    }
    let mut names = Vec::new();
    for (i, (name, part, wanted)) in jobs.iter().enumerate() {
        progress(i + 1, jobs.len());
        let mut board = match wanted {
            Some(wanted) => library.part_mesh(*part, wanted, 1024)?,
            None => defaults.only(*part),
        };
        if *part == Part::Deck {
            // The finish as stored, with its see-through grip.
            let stored = wanted
                .as_ref()
                .and_then(|w| library.texture_path(&w.material))
                .and_then(|path| std::fs::read(path).ok())
                .and_then(|bytes| skate_boards::decode_png(&bytes).ok())
                .map(|t| t.shrink(1024));
            let edges = edge_columns(&laid_in(&board, defaults, &frame));
            for texture in &mut board.textures {
                if !stored.as_ref().is_some_and(|s| lay_on_grip(texture, s)) {
                    darken_grip(texture);
                }
                keep_wood(texture, edges);
            }
        }
        for texture in &mut board.textures {
            texture.make_opaque();
        }
        // Decks uncompressed: DXT's two colours a block gave their graphics
        // two-toned edges. Trucks and wheels (small, plain) compressed.
        // Skate's normal map and shine for the finish (a finish is one
        // texture; the default board's parts keep the flat map).
        let lighting = wanted.as_ref().and_then(|w| library.lighting(&w.material));
        let normals = match wanted {
            Some(w) if board.textures.len() == 1 => vec![library.normal_map(&w.material, 1024)],
            _ => Vec::new(),
        };
        let shine = lighting.map_or(24, shine_of);
        let (lo, hi) = write_model_lit(dir, name, &laid_in(&board, defaults, &frame), &board.textures, *part == Part::Deck, shine, &normals)?;
        names.push((name.clone(), lo, hi));
    }
    // Griptapes, apart from the decks (`crate::grip`).
    names.extend(crate::grip::write_grips(dir, library, defaults, &frame)?);
    // Scuffs laid on a deck's underside as it wears (`crate::grip::write_scuffs`).
    names.extend(crate::grip::write_scuffs(dir, library, defaults, &frame)?);
    // Painted wheels: Skate's plain white wheel in every colour and finish.
    let base = library
        .items(Part::Wheels)
        .iter()
        .find(|i| i.label == skate_boards::paints::BASE)
        .or_else(|| library.items(Part::Wheels).first())
        .ok_or("no wheel to paint")?
        .part_ref();
    // Painted trucks: the plainest truck, split into its baseplate (on the
    // deck) and its hanger, each in every paint.
    if let Some(truck) = library.items(Part::Trucks).iter().find(|i| i.label == skate_boards::paints::TRUCK_BASE) {
        use skate_boards::paints::TruckPart;
        let board = library.part_mesh(Part::Trucks, &truck.part_ref(), 1024)?;
        let normals = vec![library.normal_map(&truck.material, 1024)];
        let laid = laid_in(&board, defaults, &frame);
        let deck_side = laid.iter().flat_map(|s| s.0.iter().map(|p| p.x)).fold(f32::MAX, f32::min);
        for side in [TruckPart::Base, TruckPart::Hanger] {
            let part: Laid = laid
                .iter()
                .map(|(positions, normals, uvs, indices, texture)| {
                    let keep: Vec<u32> = indices
                        .chunks_exact(3)
                        .filter(|t| {
                            let x = t.iter().map(|i| positions[*i as usize].x).sum::<f32>() / 3.;
                            (x < deck_side + TruckPart::BASE_DEPTH) == (side == TruckPart::Base)
                        })
                        .flatten()
                        .copied()
                        .collect();
                    (positions.clone(), normals.clone(), uvs.clone(), keep, *texture)
                })
                .collect();
            for paint in skate_boards::paints::catalog() {
                let mut textures = board.textures.clone();
                for texture in &mut textures {
                    texture.make_opaque();
                    skate_boards::paints::tint(&mut texture.rgba, &paint);
                }
                let name = side.model(&paint.id);
                let (lo, hi) = write_model_lit(dir, &name, &part, &textures, false, paint.finish.shine(), &normals)?;
                names.push((name, lo, hi));
            }
        }
    }
    // One-colour decks: Skate's plain white deck in every paint, its grip
    // half as the other decks' (`lay_on_grip`).
    if let Some(deck) = library.items(Part::Deck).iter().find(|i| i.label == skate_boards::paints::DECK_BASE) {
        let base = library.part_mesh(Part::Deck, &deck.part_ref(), 1024)?;
        let stored = library
            .texture_path(&deck.material)
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| skate_boards::decode_png(&bytes).ok())
            .map(|t| t.shrink(1024));
        let normals = vec![library.normal_map(&deck.material, 1024)];
        let laid = laid_in(&base, defaults, &frame);
        let edges = edge_columns(&laid_in(&base, defaults, &frame));
        for paint in skate_boards::paints::catalog() {
            let mut textures = base.textures.clone();
            for texture in &mut textures {
                if !stored.as_ref().is_some_and(|s| lay_on_grip(texture, s)) {
                    darken_grip(texture);
                }
                keep_wood(texture, edges);
                texture.make_opaque();
                skate_boards::paints::tint(&mut texture.rgba, &paint);
            }
            let name = part_model(Part::Deck, Some(&paint.part_ref().material));
            // Compressed: plain colours, and 42 uncompressed decks would add
            // about 200 MB to the pack.
            let (lo, hi) = write_model_lit(dir, &name, &laid, &textures, false, paint.finish.shine(), &normals)?;
            names.push((name, lo, hi));
        }
    }
    let wheel_normals = vec![library.normal_map(&base.material, 1024)];
    for paint in skate_boards::paints::catalog() {
        let mut board = library.part_mesh(Part::Wheels, &base, 1024)?;
        for texture in &mut board.textures {
            texture.make_opaque();
            skate_boards::paints::tint(&mut texture.rgba, &paint);
        }
        let name = part_model(Part::Wheels, Some(&paint.part_ref().material));
        let (lo, hi) = write_model_lit(dir, &name, &laid_in(&board, defaults, &frame), &board.textures, false, paint.finish.shine(), &wheel_normals)?;
        names.push((name, lo, hi));
    }
    std::fs::write(dir.join(format!("{PARTS_YTYP}.ytyp.xml")), ytyp(PARTS_YTYP, &names)).map_err(|e| e.to_string())?;
    Ok(names.into_iter().map(|(name, ..)| name).collect())
}

/// The model of a board part in a finish (none: the default board's).
pub fn part_model(part: skate_boards::Part, material: Option<&str>) -> String {
    let letter = ["d", "t", "w"][part.index()];
    format!("sk3_{letter}_{}", material.map_or("default".to_owned(), |m| m.to_ascii_lowercase()))
}

/// One model: `<name>.ydr.xml`, `<name>.ytd.xml` and its DDS files in the
/// folder `<name>`. `raw`: the graphics uncompressed, else DXT.
pub(crate) fn write_model(dir: &std::path::Path, name: &str, surfaces: &Laid, textures: &[skate_boards::Texture], raw: bool) -> Result<(Vec3, Vec3), String> {
    write_model_shine(dir, name, surfaces, textures, raw, 24)
}

/// As [`write_model`], with the shine of its specular map (0 to 255).
pub(crate) fn write_model_shine(dir: &std::path::Path, name: &str, surfaces: &Laid, textures: &[skate_boards::Texture], raw: bool, shine: u8) -> Result<(Vec3, Vec3), String> {
    write_model_full(dir, name, surfaces, textures, raw, shine, false, &[])
}

/// As [`write_model_shine`], with Skate's own normal map per texture
/// (`normals[t]` for `textures[t]`; none: flat).
pub(crate) fn write_model_lit(dir: &std::path::Path, name: &str, surfaces: &Laid, textures: &[skate_boards::Texture], raw: bool, shine: u8, normals: &[Option<skate_boards::Texture>]) -> Result<(Vec3, Vec3), String> {
    write_model_full(dir, name, surfaces, textures, raw, shine, false, normals)
}

/// The specular map's level (0 to 255) for Skate's metallic and roughness:
/// a deck's wood (0, 0.72) stays as dull as before (23), a wheel's
/// urethane (0, 0.48) shines a little, a truck's metal (0.65, 0.48) more.
pub fn shine_of(lighting: &skate_boards::Lighting) -> u8 {
    let smooth = 1. - lighting.roughness.clamp(0., 1.);
    let level = 0.06 + 0.4 * smooth * smooth + 0.5 * lighting.metallic.clamp(0., 1.) * smooth;
    (level.clamp(0., 1.) * 255.).round() as u8
}

/// As [`write_model`], see-through where its texture is (a decal laid on
/// another model: GTA's `normal_spec_decal` shader).
pub(crate) fn write_model_decal(dir: &std::path::Path, name: &str, surfaces: &Laid, textures: &[skate_boards::Texture]) -> Result<(Vec3, Vec3), String> {
    write_model_full(dir, name, surfaces, textures, true, 24, true, &[])
}

#[allow(clippy::too_many_arguments)]
/// `image` twice as large, bilinear (normal maps stay smooth).
fn double_smooth(image: &dds::Image) -> dds::Image {
    let (w, h) = (image.width as usize, image.height as usize);
    let at = |x: usize, y: usize, c: usize| f32::from(image.rgba[(y.min(h - 1) * w + x.min(w - 1)) * 4 + c]);
    let mut rgba = vec![0u8; w * h * 16];
    for y in 0..h * 2 {
        for x in 0..w * 2 {
            let (fx, fy) = (((x as f32 - 0.5) / 2.0).max(0.0), ((y as f32 - 0.5) / 2.0).max(0.0));
            let (x0, y0) = (fx as usize, fy as usize);
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            for c in 0..4 {
                let v = at(x0, y0, c) * (1. - tx) * (1. - ty) + at(x0 + 1, y0, c) * tx * (1. - ty)
                    + at(x0, y0 + 1, c) * (1. - tx) * ty + at(x0 + 1, y0 + 1, c) * tx * ty;
                rgba[(y * w * 2 + x) * 4 + c] = v.round().clamp(0., 255.) as u8;
            }
        }
    }
    dds::Image { width: image.width * 2, height: image.height * 2, rgba }
}

fn write_model_full(dir: &std::path::Path, name: &str, surfaces: &Laid, textures: &[skate_boards::Texture], raw: bool, shine: u8, decal: bool, normals: &[Option<skate_boards::Texture>]) -> Result<(Vec3, Vec3), String> {
    let mut used: Vec<usize> = surfaces.iter().map(|s| s.4).collect();
    used.sort_unstable();
    used.dedup();
    // One geometry per texture, each with its shader.
    let mut geometries: Vec<(usize, Geometry)> = Vec::new();
    for (positions, normals, uvs, indices, texture) in surfaces {
        let tangents = xml::tangents(positions, normals, uvs, indices);
        let slot = used.iter().position(|t| t == texture).unwrap_or(0);
        if !geometries.iter().any(|(s, _)| *s == slot) {
            geometries.push((slot, Geometry { palette: vec![0], ..Default::default() }));
        }
        let geometry = &mut geometries.iter_mut().find(|(s, _)| *s == slot).unwrap().1;
        let base = geometry.vertices.len() as u32;
        if base as usize + positions.len() > 65535 {
            return Err("the board has too many vertices for one geometry".into());
        }
        for i in 0..positions.len() {
            geometry.vertices.push(Vertex {
                position: positions[i],
                normal: normals[i],
                uv: uvs[i],
                tangent: tangents[i],
                bones: [0; 4],
                weights: [255, 0, 0, 0],
            });
        }
        geometry.indices.extend(indices.iter().map(|i| (base + i) as u16));
    }
    // Textures: the board's, Skate's normal maps (else a flat one), a
    // specular map.
    let folder = dir.join(name);
    let texture_of = |t: usize| if name == MODEL { texture_name(t) } else { format!("{name}_{t}") };
    let mut images = Vec::new();
    let mut bumps = Vec::new();
    for &t in &used {
        let texture = textures.get(t).ok_or("a surface without its texture")?;
        let image = dds::Image { width: texture.width, height: texture.height, rgba: texture.rgba.clone() }.power_of_two(2048);
        images.push((texture_of(t), image, raw, "DIFFUSE"));
        match normals.get(t).and_then(Option::as_ref) {
            Some(normal) => {
                let bump = format!("{}_n", texture_of(t));
                let mut image = dds::Image { width: normal.width, height: normal.height, rgba: normal.rgba.clone() }.power_of_two(2048);
                // Small ones (a wheel's 64 px) uncompressed: DXT's 4 px
                // blocks broke their curves up.
                let mut small = image.width * image.height <= 128 * 128;
                // A shiny paint's tight highlight showed every texel of the
                // normal map (pixelated metallic, the maintainer 10-11): smoothly
                // enlarged to 256 px and uncompressed.
                if shine >= 100 {
                    while image.width < 256 || image.height < 256 {
                        image = double_smooth(&image);
                    }
                    small = true;
                }
                images.push((bump.clone(), image, small, "NORMAL"));
                bumps.push(bump);
            }
            None => bumps.push(NORMAL.into()),
        }
    }
    if bumps.iter().any(|b| b == NORMAL) {
        images.push((NORMAL.into(), dds::Image::solid(4, [128, 128, 255, 255]), false, "NORMAL"));
    }
    images.push((SPECULAR.into(), dds::Image::solid(4, [shine, shine, shine, 255]), false, "SPECULAR"));
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let mut encoded = Vec::new();
    for (texture, image, raw, usage) in &images {
        // The weapon's graphics uncompressed (no block artefacts), the
        // parts' (hundreds of them) and the flat helper maps compressed.
        let (bytes, levels, format) = if *raw { dds::encode_raw(image) } else { dds::encode(image) };
        std::fs::write(folder.join(format!("{texture}.dds")), &bytes).map_err(|e| e.to_string())?;
        encoded.push((texture.clone(), image.width, image.height, levels, format, *usage));
    }
    let items: Vec<TextureItem> = encoded
        .iter()
        .map(|(texture, width, height, levels, format, usage)| TextureItem { name: texture, usage, width: *width, height: *height, levels: *levels, format })
        .collect();
    std::fs::write(dir.join(format!("{name}.ytd.xml")), xml::ytd(&items)).map_err(|e| e.to_string())?;
    let shaders: Vec<String> = used.iter().map(|t| texture_of(*t)).collect();
    std::fs::write(dir.join(format!("{name}.ydr.xml")), ydr_with(name, &shaders, &bumps, &geometries, name == MODEL, decal, spec_params(shine))).map_err(|e| e.to_string())?;
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in surfaces.iter().flat_map(|s| &s.0) {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    Ok((lo, hi))
}

/// GTA's `normal_spec` light for a shine (0 to 255, the specular map's
/// level: Matte 12, Gloss 110, Metallic 230, wood about 23): how strong the
/// highlight is, how tight (falloff) and how much more at grazing angles
/// (fresnel). All parts once had 0.3 / 20 / 0.5, and Matte, Gloss and
/// Metallic looked alike (the maintainer, 10-10).
pub fn spec_params(shine: u8) -> [f32; 3] {
    let s = f32::from(shine) / 255.;
    [0.05 + 1.45 * s, 3. + 300. * s * s, 0.2 + 0.7 * s]
}

/// The props' archetypes (`skate3parts.ytyp.xml`): each model its own txd,
/// a dynamic prop as GTA's own crates are.
pub fn ytyp(name: &str, models: &[(String, Vec3, Vec3)]) -> String {
    let mut out = format!("{}<CMapTypes>\n <extensions />\n <archetypes>\n", xml::HEADER);
    for (model, lo, hi) in models {
        let centre = (*lo + *hi) * 0.5;
        let radius = (*hi - *lo).length() * 0.5;
        let _ = write!(
            out,
            concat!(
                "  <Item type=\"CBaseArchetypeDef\">\n   <lodDist value=\"100\" />\n   <flags value=\"537001984\" />\n   <specialAttribute value=\"0\" />\n",
                "   <bbMin x=\"{}\" y=\"{}\" z=\"{}\" />\n   <bbMax x=\"{}\" y=\"{}\" z=\"{}\" />\n   <bsCentre x=\"{}\" y=\"{}\" z=\"{}\" />\n   <bsRadius value=\"{}\" />\n",
                "   <hdTextureDist value=\"15\" />\n   <name>{}</name>\n   <textureDictionary>{}</textureDictionary>\n   <clipDictionary />\n   <drawableDictionary />\n   <physicsDictionary />\n",
                "   <assetType>ASSET_TYPE_DRAWABLE</assetType>\n   <assetName>{}</assetName>\n   <extensions />\n  </Item>\n"
            ),
            f(lo.x), f(lo.y), f(lo.z), f(hi.x), f(hi.y), f(hi.z), f(centre.x), f(centre.y), f(centre.z), f(radius), model, model, model
        );
    }
    let _ = write!(out, " </archetypes>\n <name>{name}</name>\n <dependencies />\n <compositeEntityTypes itemType=\"CCompositeEntityType\" />\n</CMapTypes>\n");
    out
}

/// The props' archetype file's name.
pub const PARTS_YTYP: &str = "skate3parts";

/// The weapon drawable (`.ydr.xml`), its textures in the `.ytd` of the same
/// name (the weapon archetype's txd).
#[cfg(test)]
pub fn ydr(name: &str, diffuse: &[String], geometries: &[(usize, Geometry)], skinned: bool) -> String {
    ydr_with(name, diffuse, &[], geometries, skinned, false, spec_params(24))
}

/// As [`ydr`]; `bumps`: each shader's normal map (missing: the flat one);
/// `decal`: see-through by its texture's alpha, drawn over what it lies on.
pub fn ydr_with(name: &str, diffuse: &[String], bumps: &[String], geometries: &[(usize, Geometry)], skinned: bool, decal: bool, spec: [f32; 3]) -> String {
    let (shader, bucket) = if decal { ("normal_spec_decal", 2) } else { ("normal_spec", 0) };
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in geometries.iter().flat_map(|(_, g)| &g.vertices) {
        lo = lo.min(v.position);
        hi = hi.max(v.position);
    }
    if lo.x > hi.x {
        (lo, hi) = (Vec3::ZERO, Vec3::ZERO);
    }
    let centre = (lo + hi) * 0.5;
    let radius = (hi - lo).length() * 0.5;
    let mut out = format!("{}<Drawable>\n <Name>{name}</Name>\n", xml::HEADER);
    let _ = write!(
        out,
        " <BoundingSphereCenter x=\"{}\" y=\"{}\" z=\"{}\" />\n <BoundingSphereRadius value=\"{}\" />\n <BoundingBoxMin x=\"{}\" y=\"{}\" z=\"{}\" />\n <BoundingBoxMax x=\"{}\" y=\"{}\" z=\"{}\" />\n",
        f(centre.x), f(centre.y), f(centre.z), f(radius), f(lo.x), f(lo.y), f(lo.z), f(hi.x), f(hi.y), f(hi.z)
    );
    for lod in ["High", "Med", "Low", "Vlow"] {
        let _ = writeln!(out, " <LodDist{lod} value=\"9998\" />");
    }
    out.push_str(" <FlagsHigh value=\"1\" />\n <FlagsMed value=\"0\" />\n <FlagsLow value=\"0\" />\n <FlagsVlow value=\"0\" />\n");
    out.push_str(" <ShaderGroup>\n  <Shaders>\n");
    for (i, texture) in diffuse.iter().enumerate() {
        let bump = bumps.get(i).map_or(NORMAL, String::as_str);
        let _ = write!(out, "   <Item>\n    <Name>{shader}</Name>\n    <FileName>{shader}.sps</FileName>\n    <RenderBucket value=\"{bucket}\" />\n    <Parameters>\n");
        for (param, name) in [("DiffuseSampler", texture.as_str()), ("BumpSampler", bump), ("SpecSampler", SPECULAR)] {
            let _ = write!(out, "     <Item name=\"{param}\" type=\"Texture\">\n      <Name>{name}</Name>\n     </Item>\n");
        }
        for (param, x) in [("bumpiness", 1.), ("specularIntensityMult", spec[0]), ("specularFalloffMult", spec[1]), ("specularFresnel", spec[2])] {
            let _ = writeln!(out, "     <Item name=\"{param}\" type=\"Vector\" x=\"{}\" y=\"0\" z=\"0\" w=\"0\" />", f(x));
        }
        out.push_str("    </Parameters>\n   </Item>\n");
    }
    out.push_str("  </Shaders>\n </ShaderGroup>\n");
    // The bat's skeleton: the root the hand attaches to, and its grip bones.
    // A prop has none.
    if skinned {
    out.push_str(concat!(
        " <Skeleton>\n  <Unknown1C value=\"16777216\" />\n  <Unknown50 value=\"3237619933\" />\n  <Unknown54 value=\"1757459591\" />\n  <Unknown58 value=\"3710988181\" />\n  <Bones>\n",
        "   <Item>\n    <Name>Gun_Root</Name>\n    <Tag value=\"0\" />\n    <Index value=\"0\" />\n    <ParentIndex value=\"-1\" />\n    <SiblingIndex value=\"-1\" />\n    <Flags>RotX, RotY, RotZ, TransX, TransY, TransZ, Unk0</Flags>\n    <Translation x=\"0\" y=\"0\" z=\"0\" />\n    <Rotation x=\"0\" y=\"0\" z=\"0\" w=\"1\" />\n    <Scale x=\"1\" y=\"1\" z=\"1\" />\n    <TransformUnk x=\"0\" y=\"4\" z=\"-3\" w=\"0\" />\n   </Item>\n",
        "   <Item>\n    <Name>Gun_GripR</Name>\n    <Tag value=\"18308\" />\n    <Index value=\"1\" />\n    <ParentIndex value=\"0\" />\n    <SiblingIndex value=\"-1\" />\n    <Flags>RotX, RotY, RotZ, TransX, TransY, TransZ, Unk0</Flags>\n    <Translation x=\"-0.045597\" y=\"0.002018\" z=\"0.012259\" />\n    <Rotation x=\"0.6189591\" y=\"-0.106119931\" z=\"0.0177541487\" w=\"0.778018653\" />\n    <Scale x=\"1\" y=\"1\" z=\"1\" />\n    <TransformUnk x=\"0\" y=\"2.421252\" z=\"-1.421252\" w=\"0\" />\n   </Item>\n",
        "   <Item>\n    <Name>Gun_Main_Bone</Name>\n    <Tag value=\"3360\" />\n    <Index value=\"2\" />\n    <ParentIndex value=\"1\" />\n    <SiblingIndex value=\"-1\" />\n    <Flags>RotX, RotY, RotZ, TransX, TransY, TransZ</Flags>\n    <Translation x=\"0.042456\" y=\"-0.019481\" z=\"-0.007166\" />\n    <Rotation x=\"-0.6189593\" y=\"0.106119826\" z=\"-0.0177538656\" w=\"0.7780185\" />\n    <Scale x=\"1\" y=\"1\" z=\"1\" />\n    <TransformUnk x=\"0\" y=\"2.421251\" z=\"-1.421251\" w=\"0\" />\n   </Item>\n",
        "  </Bones>\n </Skeleton>\n",
    ));
    }
    let _ = write!(
        out,
        " <DrawableModelsHigh>\n  <Item>\n   <RenderMask value=\"255\" />\n   <Flags value=\"{}\" />\n   <HasSkin value=\"{}\" />\n   <BoneIndex value=\"0\" />\n   <Unknown1 value=\"{}\" />\n   <Geometries>\n",
        u8::from(skinned),
        u8::from(skinned),
        if skinned { 3 } else { 0 }
    );
    for (shader, g) in geometries {
        let (mut glo, mut ghi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &g.vertices {
            glo = glo.min(v.position);
            ghi = ghi.max(v.position);
        }
        let _ = write!(
            out,
            "    <Item>\n     <ShaderIndex value=\"{shader}\" />\n     <BoundingBoxMin x=\"{}\" y=\"{}\" z=\"{}\" w=\"0\" />\n     <BoundingBoxMax x=\"{}\" y=\"{}\" z=\"{}\" w=\"0\" />\n{}     <VertexBuffer>\n      <Flags value=\"0\" />\n      <Layout type=\"GTAV1\">\n       <Position />\n{}       <Normal />\n       <Colour0 />\n       <TexCoord0 />\n       <Tangent />\n      </Layout>\n      <Data>\n",
            f(glo.x), f(glo.y), f(glo.z), f(ghi.x), f(ghi.y), f(ghi.z),
            if skinned { "     <BoneIDs>0, 1, 2</BoneIDs>\n" } else { "" },
            if skinned { "       <BlendWeights />\n       <BlendIndices />\n" } else { "" },
        );
        for v in &g.vertices {
            let skin = if skinned {
                format!(
                    "{} {} {} {}   {} {} {} {}   ",
                    v.weights[0], v.weights[1], v.weights[2], v.weights[3], v.bones[0], v.bones[1], v.bones[2], v.bones[3]
                )
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "       {} {} {}   {skin}{} {} {}   255 255 255 255   {} {}   {} {} {} {}",
                f(v.position.x), f(v.position.y), f(v.position.z),
                f(v.normal.x), f(v.normal.y), f(v.normal.z),
                f(v.uv.x), f(v.uv.y),
                f(v.tangent.x), f(v.tangent.y), f(v.tangent.z), f(v.tangent.w),
            );
        }
        out.push_str("      </Data>\n     </VertexBuffer>\n     <IndexBuffer>\n      <Data>");
        for (i, chunk) in g.indices.chunks(24).enumerate() {
            if i > 0 {
                out.push_str("\n       ");
            }
            let line: Vec<String> = chunk.iter().map(|x| x.to_string()).collect();
            out.push_str(&line.join(" "));
        }
        out.push_str("</Data>\n     </IndexBuffer>\n    </Item>\n");
    }
    out.push_str("   </Geometries>\n  </Item>\n </DrawableModelsHigh>\n");
    // Its collision: one box round it (as the bat's).
    let half = (hi - lo) * 0.5;
    let volume = (hi - lo).x * (hi - lo).y * (hi - lo).z;
    let _ = write!(
        out,
        concat!(
            " <Bounds type=\"Composite\">\n  <BoxMin x=\"{lx}\" y=\"{ly}\" z=\"{lz}\" />\n  <BoxMax x=\"{hx}\" y=\"{hy}\" z=\"{hz}\" />\n  <BoxCenter x=\"{cx}\" y=\"{cy}\" z=\"{cz}\" />\n  <SphereCenter x=\"{cx}\" y=\"{cy}\" z=\"{cz}\" />\n  <SphereRadius value=\"{r}\" />\n  <Margin value=\"0\" />\n  <Volume value=\"{v}\" />\n  <Inertia x=\"0.05\" y=\"0.05\" z=\"0.001\" />\n",
            "  <MaterialIndex value=\"0\" />\n  <MaterialColourIndex value=\"0\" />\n  <ProceduralID value=\"0\" />\n  <RoomID value=\"0\" />\n  <PedDensity value=\"0\" />\n  <UnkFlags value=\"0\" />\n  <PolyFlags value=\"0\" />\n  <UnkType value=\"1\" />\n  <Children>\n   <Item type=\"Box\">\n",
            "    <BoxMin x=\"-{bx}\" y=\"-{by}\" z=\"-{bz}\" />\n    <BoxMax x=\"{bx}\" y=\"{by}\" z=\"{bz}\" />\n    <BoxCenter x=\"0\" y=\"0\" z=\"0\" />\n    <SphereCenter x=\"0\" y=\"0\" z=\"0\" />\n    <SphereRadius value=\"{r}\" />\n    <Margin value=\"0.005\" />\n    <Volume value=\"{v}\" />\n    <Inertia x=\"0.05\" y=\"0.05\" z=\"0.001\" />\n",
            "    <MaterialIndex value=\"58\" />\n    <MaterialColourIndex value=\"0\" />\n    <ProceduralID value=\"0\" />\n    <RoomID value=\"0\" />\n    <PedDensity value=\"0\" />\n    <UnkFlags value=\"0\" />\n    <PolyFlags value=\"0\" />\n    <UnkType value=\"1\" />\n",
            "    <CompositeTransform>\n     1 0 0 0\n     0 1 0 0\n     0 0 1 0\n     {cx} {cy} {cz} 1\n    </CompositeTransform>\n",
            "    <CompositeFlags1>MAP_WEAPON, MAP_DYNAMIC, MAP_ANIMAL, MAP_COVER, MAP_VEHICLE</CompositeFlags1>\n",
            "    <CompositeFlags2>VEHICLE_NOT_BVH, VEHICLE_BVH, PED, RAGDOLL, ANIMAL, ANIMAL_RAGDOLL, OBJECT, PLANT, PROJECTILE, EXPLOSION, FORKLIFT_FORKS, TEST_WEAPON, TEST_CAMERA, TEST_AI, TEST_SCRIPT, TEST_VEHICLE_WHEEL, GLASS</CompositeFlags2>\n",
            "   </Item>\n  </Children>\n </Bounds>\n <Lights />\n</Drawable>\n"
        ),
        lx = f(lo.x), ly = f(lo.y), lz = f(lo.z), hx = f(hi.x), hy = f(hi.y), hz = f(hi.z),
        cx = f(centre.x), cy = f(centre.y), cz = f(centre.z), r = f(radius), v = f(volume),
        bx = f(half.x), by = f(half.y), bz = f(half.z),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate_boards::{Joint, Part, Surface, Texture, Vertex as BoardVertex};

    /// A deck 0.8 long (Skate's Z), 0.2 wide (X), 0.02 thick (Y).
    fn deck() -> Board {
        let v = |x: f32, y: f32, z: f32| BoardVertex { position: [x, y, z], normal: [0., 1., 0.], uv: [0., 0.], joints: [0; 4], weights: [1., 0., 0., 0.] };
        Board {
            joints: vec![Joint { name: "deck".into(), inverse_bind: Mat4::IDENTITY.to_cols_array() }],
            surfaces: vec![Surface {
                part: Part::Deck,
                material: "deck".into(),
                texture: 0,
                vertices: vec![v(-0.1, 0., -0.4), v(0.1, 0., -0.4), v(0.1, 0.02, 0.4), v(-0.1, 0.02, 0.4)],
                indices: vec![0, 1, 2, 2, 3, 0],
            }],
            textures: vec![Texture { width: 4, height: 4, rgba: vec![200; 64] }],
        }
    }

    #[test]
    fn the_board_lies_along_the_bats_length_held_by_its_tail() {
        let board = deck();
        let surfaces = laid_out(&board, &board);
        let positions = &surfaces[0].0;
        let (lo, hi) = positions.iter().fold((Vec3::splat(9.), Vec3::splat(-9.)), |(l, h), p| (l.min(*p), h.max(*p)));
        let size = hi - lo;
        assert!((size.z - 0.8).abs() < 1e-4, "length along Z: {size}");
        assert!((size.y - 0.2).abs() < 1e-4, "width along Y: {size}");
        assert!((lo.z + GRIP_FROM_TAIL).abs() < 1e-4, "the grip is {GRIP_FROM_TAIL} m in from the tail: {lo}");
        assert!(surfaces[0].1[0].x.abs() > 0.99, "the deck faces X");
    }

    #[test]
    fn writes_the_weapon_model_and_its_textures() {
        let dir = std::env::temp_dir().join("skate-ped-weapon-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let board = deck();
        write(&dir, &board, &board).unwrap();
        let ydr = std::fs::read_to_string(dir.join(format!("{MODEL}.ydr.xml"))).unwrap();
        assert!(ydr.contains("<Name>Gun_Root</Name>") && ydr.contains("normal_spec") && ydr.contains("skate3board_0"));
        let ytd = std::fs::read_to_string(dir.join(format!("{MODEL}.ytd.xml"))).unwrap();
        assert!(ytd.contains("skate3board_n") && ytd.contains("skate3board_s"));
        assert!(dir.join(MODEL).join("skate3board_0.dds").is_file());
    }

    #[test]
    fn the_finish_lies_on_dark_grip_by_its_see_through_ness() {
        // Grain under a see-through pixel, a solid print, a half-solid one.
        // Top row (grip side): grain under a see-through pixel, a solid
        // print, a half-solid one; bottom row (the graphic): see-through.
        let mut texture = Texture { width: 3, height: 2, rgba: vec![200, 200, 200, 255, 220, 30, 30, 255, 100, 100, 100, 255, 90, 90, 90, 255, 90, 90, 90, 255, 90, 90, 90, 255] };
        let stored = Texture { width: 3, height: 2, rgba: vec![200, 200, 200, 0, 220, 30, 30, 255, 100, 100, 100, 128, 90, 90, 90, 0, 90, 90, 90, 0, 90, 90, 90, 0] };
        assert!(lay_on_grip(&mut texture, &stored));
        assert_eq!(&texture.rgba[0..4], &[32, 32, 32, 255], "dark grip");
        assert_eq!(&texture.rgba[4..8], &[220, 30, 30, 255], "a solid print kept");
        assert!(texture.rgba[8] > 32 && texture.rgba[8] < 100, "half way: {}", texture.rgba[8]);
        assert_eq!(&texture.rgba[12..15], &[90, 90, 90], "the graphic's side left as stored");
        let mut other = Texture { width: 1, height: 1, rgba: vec![1, 2, 3, 255] };
        assert!(!lay_on_grip(&mut other, &stored), "sizes must match");
    }

    #[test]
    fn grip_turns_dark_and_the_graphic_stays() {
        // Top half: grey grain and a red grip print; bottom half: grey graphic.
        let mut texture = Texture { width: 2, height: 2, rgba: vec![200, 205, 198, 255, 220, 30, 30, 255, 200, 200, 200, 255, 200, 200, 200, 255] };
        darken_grip(&mut texture);
        assert!(texture.rgba[0] < 40, "grey grip darkened: {:?}", &texture.rgba[0..3]);
        assert_eq!(&texture.rgba[4..7], &[220, 30, 30], "coloured grip art kept");
        assert_eq!(&texture.rgba[8..11], &[200, 200, 200], "the graphic's half kept");
    }

    #[test]
    fn parts_share_the_whole_boards_frame() {
        let board = deck();
        let frame = Frame::of(&board, &board);
        assert_eq!(laid_in(&board, &board, &frame)[0].0, laid_out(&board, &board)[0].0);
        assert_eq!(part_model(Part::Wheels, Some("2C7F38170002056F")), "sk3_w_2c7f38170002056f");
        assert_eq!(part_model(Part::Deck, None), "sk3_d_default");
    }

    #[test]
    fn the_plies_strip_stays_wood_in_the_grip_half() {
        // 8 x 4: top half dark grip, bottom half light; the plies at u 0.75.
        let mut rgba = Vec::new();
        for y in 0..4 {
            for _ in 0..8 {
                rgba.extend_from_slice(if y < 2 { &[20, 20, 20, 255] } else { &[200, 170, 120, 255] });
            }
        }
        let mut t = skate_boards::Texture { width: 8, height: 4, rgba };
        keep_wood(&mut t, Some((0.75, 0.75)));
        let px = |x: usize, y: usize| t.rgba[(y * 8 + x) * 4];
        assert_eq!(px(6, 0), 200, "the strip took the wood");
        assert_eq!(px(1, 0), 20, "the grip elsewhere stays");
        assert_eq!(keep_wood_noop(), ());
    }

    fn keep_wood_noop() {
        let mut t = skate_boards::Texture { width: 2, height: 2, rgba: vec![1; 16] };
        keep_wood(&mut t, None);
        assert_eq!(t.rgba, vec![1; 16]);
    }

    #[test]
    fn matte_gloss_and_metallic_take_light_differently() {
        use skate_boards::paints::Finish;
        let [matte, gloss, metal] = [Finish::Matte, Finish::Gloss, Finish::Metallic].map(|f| spec_params(f.shine()));
        // Stronger, tighter and more at grazing angles, in that order.
        for i in 0..3 {
            assert!(matte[i] < gloss[i] && gloss[i] < metal[i]);
        }
        assert!(metal[1] > 10. * matte[1]);
    }

    #[test]
    fn parts_are_plain_props_in_one_ytyp() {
        let board = deck();
        let laid = laid_out(&board, &board);
        let mut g = Geometry { palette: vec![0], ..Default::default() };
        g.vertices.push(Vertex { position: laid[0].0[0], normal: Vec3::X, uv: Vec2::ZERO, tangent: glam::Vec4::X, bones: [0; 4], weights: [255, 0, 0, 0] });
        let prop = ydr("sk3_d_x", &["sk3_d_x_0".into()], &[(0, g)], false);
        assert!(!prop.contains("<Skeleton>") && !prop.contains("BlendWeights") && prop.contains("<HasSkin value=\"0\" />"));
        let types = ytyp(PARTS_YTYP, &[("sk3_d_x".into(), Vec3::splat(-1.), Vec3::ONE)]);
        assert!(types.contains("<name>sk3_d_x</name>") && types.contains("<textureDictionary>sk3_d_x</textureDictionary>") && types.contains("<name>skate3parts</name>"));
    }
}
