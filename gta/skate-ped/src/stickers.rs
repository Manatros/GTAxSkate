//! Stickers: the logos of Skate 3's graphics editor (the catalog's `Misc`
//! slot, `logo_*` and `team_logo_*`), stuck in the middle of a deck's
//! underside. Each is a see-through decal (`normal_spec_decal`, as the
//! scuffs) per deck shape: `sk3_k_<shape>_<id>`. The list goes to
//! `<assets>/private/boards/stickers.json` for the plugin's store.

use glam::{Vec2, Vec3};
use serde_json::{Value, json};
use skate_boards::{Board, Library, Part, PartRef, Texture};

use crate::rw;
use crate::weapon::{Frame, Laid, laid_in};

/// The sticker's size on the deck (m): along the board and across it, as
/// Skate's logo images are twice as wide as tall.
/// The archetype file of the sticker set (`skate3stickers.ytyp`).
pub const YTYP: &str = "skate3stickers";
pub const LENGTH: f32 = 0.26;
pub const WIDTH: f32 = 0.13;
/// Lifted off the deck so it draws over it (the scuffs lie at 1.5 mm; one
/// model serves every deck shape).
const LIFT: f32 = 0.003;

use skate_boards::stickers::sticker_model;

/// One of Skate's logos: its id (for models and saves), its name as a store
/// lists it, its group, and the texture's path in `createacharacter.big`.
pub struct Logo {
    pub id: String,
    pub title: String,
    pub group: &'static str,
    pub path: String,
}

/// The logos in Skate's catalog (`boards-catalog/catalog.json`).
pub fn logos(catalog: &Value) -> Vec<Logo> {
    let mut out: Vec<Logo> = Vec::new();
    let materials = &catalog["materials"];
    let Some(slots) = catalog["components"].as_array() else { return out };
    for slot in slots.iter().filter(|s| s["slot"] == "Misc") {
        for model in slot["models"].as_array().into_iter().flatten() {
            for instance in model["lods"][0]["material_instances"].as_array().into_iter().flatten().flat_map(|i| i.as_array().into_iter().flatten()) {
                let name = instance["name"].as_str().unwrap_or_default();
                let (group, stem) = if let Some(s) = name.strip_prefix("team_logo_") {
                    ("Team logos", s)
                } else if let Some(s) = name.strip_prefix("logo_") {
                    ("Brand logos", s)
                } else {
                    continue;
                };
                let stem = stem.trim_end_matches("_mat");
                let id: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect::<String>().to_ascii_lowercase();
                let Some(material) = instance["id"].as_str().map(|m| &materials[m]) else { continue };
                let Some(path) = material["textures"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|t| t["channel"] == "decal")
                    .and_then(|t| t["path"].as_str())
                else {
                    continue;
                };
                if id.is_empty() || out.iter().any(|l| l.id == id) {
                    continue;
                }
                out.push(Logo { id, title: title_of(stem), group, path: path.to_owned() });
            }
        }
    }
    out.sort_by(|a, b| (a.group, a.title.to_lowercase()).cmp(&(b.group, b.title.to_lowercase())));
    out
}

/// "dc_colour" -> "DC (colour)", "zero_bw2" -> "Zero (bw 2)".
pub fn title_of(stem: &str) -> String {
    let words: Vec<&str> = stem.split('_').filter(|w| !w.is_empty()).collect();
    let style = |w: &str| matches!(w.trim_end_matches(|c: char| c.is_ascii_digit()), "bw" | "colour" | "color" | "white" | "black" | "red" | "small" | "alt");
    let split = words.iter().position(|w| style(w)).unwrap_or(words.len()).max(1);
    let cap = |w: &str| {
        if w.len() <= 2 {
            w.to_ascii_uppercase()
        } else {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        }
    };
    let name = words[..split.min(words.len())].iter().map(|w| cap(w)).collect::<Vec<_>>().join(" ");
    let rest = words.get(split..).unwrap_or_default().join(" ");
    if rest.is_empty() { name } else { format!("{name} ({rest})") }
}

/// The deck's underside, clipped to the sticker's rectangle in the middle,
/// with uvs across it (u along the board, v across).
pub fn underside(laid: &Laid) -> Laid {
    let (tris, _) = crate::grip::faces(laid, 1.);
    if tris.is_empty() {
        return Vec::new();
    }
    let (lo, hi) = tris.iter().flatten().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let centre = (lo + hi) * 0.5;
    let uv = |p: Vec3| Vec2::new((p.z - centre.z) / LENGTH + 0.5, (p.y - centre.y) / WIDTH + 0.5);
    let (mut positions, mut normals, mut uvs) = (Vec::new(), Vec::new(), Vec::new());
    for t in &tris {
        let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
        let n = if n.x < 0. { -n } else { n };
        let mut poly: Vec<Vec3> = t.to_vec();
        // Clipped to 0 <= u, v <= 1 (the sticker's texture would repeat).
        for (axis, keep_above) in [(0usize, true), (0, false), (1, true), (1, false)] {
            let inside = |p: Vec3| {
                let q = uv(p)[axis];
                if keep_above { q >= 0. } else { q <= 1. }
            };
            let mut next = Vec::new();
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                if inside(a) {
                    next.push(a);
                }
                if inside(a) != inside(b) {
                    let (qa, qb) = (uv(a)[axis], uv(b)[axis]);
                    let edge = if keep_above { 0. } else { 1. };
                    let s = (edge - qa) / (qb - qa);
                    next.push(a + (b - a) * s);
                }
            }
            poly = next;
            if poly.len() < 3 {
                break;
            }
        }
        if poly.len() < 3 {
            continue;
        }
        for i in 1..poly.len() - 1 {
            for p in [poly[0], poly[i], poly[i + 1]] {
                positions.push(p + Vec3::X * LIFT);
                normals.push(n);
                uvs.push(uv(p));
            }
        }
    }
    let indices: Vec<u32> = (0..positions.len() as u32).collect();
    if indices.is_empty() { Vec::new() } else { vec![(positions, normals, uvs, indices, 0)] }
}

/// Decodes every logo from Skate's `createacharacter.big` (in `skate3`, the
/// game's folder) and writes its sticker model for each deck shape.
pub fn write_stickers(dir: &std::path::Path, assets: &std::path::Path, skate3: &std::path::Path, library: &Library, defaults: &Board, frame: &Frame) -> Result<Vec<(String, Vec3, Vec3)>, String> {
    let catalog_path = assets.join("private/boards-catalog/catalog.json");
    let catalog: Value = serde_json::from_str(&std::fs::read_to_string(&catalog_path).map_err(|e| format!("cannot read {}: {e}", catalog_path.display()))?).map_err(|e| e.to_string())?;
    let logos = logos(&catalog);
    let archive = rw::Archive::open(&skate3.join("data/content/createacharacter.big"))?;
    // One underside for every deck: Skate's own deck shape (most of its
    // decks), else the default board's.
    let shape = match library.items(Part::Deck).first() {
        Some(item) => laid_in(&library.part_mesh(Part::Deck, &item.part_ref(), 256)?, defaults, frame),
        None => laid_in(&defaults.only(Part::Deck), defaults, frame),
    };
    let shapes: Vec<(Option<PartRef>, Laid)> = vec![(None, underside(&shape))];
    let mut out = Vec::new();
    let mut listed = Vec::new();
    for logo in &logos {
        let Ok((width, height, rgba)) = archive.read(&logo.path).and_then(|d| rw::rx2_image(&d)) else { continue };
        // A logo with no see-through pixel is a picture, not a sticker.
        if !rgba.chunks_exact(4).any(|p| p[3] < 128) {
            continue;
        }
        let texture = Texture { width, height, rgba };
        for (deck, surface) in &shapes {
            if surface.is_empty() {
                continue;
            }
            let name = sticker_model(deck.as_ref(), &logo.id);
            let (lo, hi) = crate::weapon::write_model_decal(dir, &name, surface, std::slice::from_ref(&texture))?;
            out.push((name, lo, hi));
        }
        listed.push(json!({ "id": logo.id, "title": logo.title, "group": logo.group }));
    }
    let index = assets.join("private/boards/stickers.json");
    std::fs::write(&index, serde_json::to_string_pretty(&json!({ "stickers": listed })).map_err(|e| e.to_string())?)
        .map_err(|e| format!("cannot write {}: {e}", index.display()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_names_read_as_brands() {
        assert_eq!(title_of("dc_colour"), "DC (colour)");
        assert_eq!(title_of("zero_bw2"), "Zero (bw2)");
        assert_eq!(title_of("creature"), "Creature");
        assert_eq!(title_of("alien_workshop_bw"), "Alien Workshop (bw)");
    }

    #[test]
    fn the_sticker_is_cut_to_its_rectangle_on_the_underside() {
        // A flat underside 0.8 m long, 0.2 m wide, facing +X.
        let p = vec![Vec3::new(0., -0.1, 0.), Vec3::new(0., 0.1, 0.), Vec3::new(0., 0.1, 0.8), Vec3::new(0., -0.1, 0.8)];
        let laid: Laid = vec![(p, vec![Vec3::X; 4], vec![Vec2::new(0.5, 0.9); 4], vec![0, 1, 2, 0, 2, 3], 0)];
        let cut = underside(&laid);
        assert_eq!(cut.len(), 1);
        let (positions, _, uvs, _, _) = &cut[0];
        assert!(uvs.iter().all(|q| (-1e-4..=1.0001).contains(&q.x) && (-1e-4..=1.0001).contains(&q.y)));
        let (lo, hi) = positions.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        assert!((hi.z - lo.z - LENGTH).abs() < 1e-4 && (hi.y - lo.y - WIDTH).abs() < 1e-4);
        assert!((lo.x - LIFT).abs() < 1e-6);
    }
}
