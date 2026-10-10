//! Extra decks from the player's own images (`skate-ped --import-decks
//! <folder>`): each image in the folder (and its subfolders) becomes a new
//! finish of Skate 3's deck, beside Skate's own (none is replaced).
//!
//! - A square image of 256 px or more is taken for a whole Skate deck
//!   texture as a Skate 3 texture mod has it (the grip side in the top
//!   half, the graphic in the bottom half, along the board).
//! - Any other image is a graphic: laid into the bottom half of a plain
//!   deck's texture (turned along the board when it stands upright, cropped
//!   to fill), the grip side kept as Skate's.
//!
//! The maker is the first known brand in the file's path (a folder named
//! after it, or the name), else "custom". Written to
//! `<assets>\private\boards\custom\` (`custom.json` and the textures),
//! which `skate_boards::Library` adds to the catalog.

use std::path::{Path, PathBuf};

/// Skate 3's deck model, the one nearly all finishes are for.
const DECK: &str = "f2c517ee8b8359da";

/// Makers known by name: Skate 3's own (its `ManufacturerID`s) and other
/// skate brands a downloaded graphic may be named after.
const BRANDS: &[&str] = &[
    "alienworkshop", "alien", "almost", "antihero", "anti_hero", "baker", "blacklabel", "blind", "chocolate", "city", "cliche", "control",
    "creature", "deathwish", "dgk", "element", "enjoi", "expedition", "flip", "girl", "habitat", "krooked", "mightyhealthy", "planb", "real",
    "santacruz", "shut", "skatemental", "stereo", "think", "zero", "zooyork", "birdhouse", "toymachine", "powell", "powellperalta", "primitive",
    "polar", "palace", "hockey", "fucking awesome", "fuckingawesome", "april", "sovrn", "girl", "lakai", "darkstar", "foundation", "zoo", "hook-ups",
    "hookups", "world industries", "worldindustries", "bloodwizard", "welcome", "magenta", "quasi", "scumco", "sk8mafia", "jart", "madness", "cruzade",
];

/// The maker in `path` (lower case, letters and digits only compared), or
/// "custom".
pub fn brand_of(path: &Path) -> String {
    let words: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect())
        .collect();
    // The deepest folder or the file's own name first.
    for word in words.iter().rev() {
        let mut found: Vec<&str> = BRANDS
            .iter()
            .map(|b| *b)
            .filter(|b| {
                let key: String = b.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
                !key.is_empty() && word.contains(&key)
            })
            .collect();
        // The longest name wins ("alienworkshop" over "alien").
        found.sort_by_key(|b| std::cmp::Reverse(b.len()));
        if let Some(b) = found.first() {
            return b.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        }
    }
    "custom".into()
}

/// A file's name as a finish's name: "my_cool-deck" → "My Cool Deck".
pub fn title_of(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    stem.split(|c: char| c == '_' || c == '-' || c == ' ' || c == '.')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// An RGBA image.
#[derive(Clone)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

impl Rgba {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixels[(y.min(self.height - 1) * self.width + x.min(self.width - 1)) as usize]
    }

    /// Bilinear sample at (u, v) in 0..1.
    fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let x = (u * self.width as f32 - 0.5).max(0.);
        let y = (v * self.height as f32 - 0.5).max(0.);
        let (x0, y0) = (x as u32, y as u32);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p = [self.at(x0, y0), self.at(x0 + 1, y0), self.at(x0, y0 + 1), self.at(x0 + 1, y0 + 1)];
        std::array::from_fn(|c| {
            let top = p[0][c] as f32 * (1. - fx) + p[1][c] as f32 * fx;
            let bottom = p[2][c] as f32 * (1. - fx) + p[3][c] as f32 * fx;
            (top * (1. - fy) + bottom * fy).round() as u8
        })
    }
}

/// The graphic's place in a deck texture: the bottom half, short of the
/// wood strip at the right edge (Skate's layout, along the board).
const GRAPHIC: [f32; 4] = [0.0, 0.5, 0.92, 1.0];

/// `picture` laid into `base` (a plain deck texture): turned along the
/// board if it stands upright, scaled to fill the graphic's place and
/// cropped in the middle. The grip half stays `base`'s.
pub fn lay_graphic(base: &Rgba, picture: &Rgba) -> Rgba {
    let mut out = base.clone();
    let [u0, v0, u1, v1] = GRAPHIC;
    let (x0, x1) = ((u0 * base.width as f32) as u32, (u1 * base.width as f32) as u32);
    let (y0, y1) = ((v0 * base.height as f32) as u32, (v1 * base.height as f32) as u32);
    let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let upright = picture.height > picture.width;
    // The picture's size along the place's width and height.
    let (pw, ph) = if upright { (picture.height as f32, picture.width as f32) } else { (picture.width as f32, picture.height as f32) };
    let scale = (w / pw).max(h / ph);
    let (cw, ch) = (w / scale / pw, h / scale / ph); // the shown share of each axis
    for y in y0..y1 {
        for x in x0..x1 {
            let s = (x - x0) as f32 / w;
            let t = (y - y0) as f32 / h;
            let a = 0.5 - cw / 2. + s * cw;
            let b = 0.5 - ch / 2. + t * ch;
            // Upright pictures are turned: the board's length runs up them.
            let (u, v) = if upright { (b, 1. - a) } else { (a, b) };
            let mut p = picture.sample(u, v);
            p[3] = 255;
            out.pixels[(y * base.width + x) as usize] = p;
        }
    }
    out
}

/// Whether an image is a whole Skate deck texture (a texture mod's).
pub fn is_deck_texture(image: &Rgba) -> bool {
    image.width == image.height && image.width >= 256
}

fn read_image(path: &Path) -> Result<Rgba, String> {
    let image = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?.to_rgba8();
    let (width, height) = image.dimensions();
    let pixels = image.pixels().map(|p| p.0).collect();
    Ok(Rgba { width, height, pixels })
}

fn write_png(path: &Path, image: &Rgba) -> Result<(), String> {
    let buffer: Vec<u8> = image.pixels.iter().flatten().copied().collect();
    image::save_buffer(path, &buffer, image.width, image.height, image::ExtendedColorType::Rgba8).map_err(|e| format!("{}: {e}", path.display()))
}

fn images_in(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            images_in(&path, out);
        } else if path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .is_some_and(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tga" | "dds" | "webp"))
        {
            out.push(path);
        }
    }
}

/// Imports the images in `folder` as decks into `assets`. Returns how many.
pub fn import_decks(assets: &Path, folder: &Path) -> Result<usize, String> {
    let library = skate_boards::Library::load(assets)?;
    // A plain deck to lay graphics into: Skate's unbranded white one.
    let plain = library
        .new_items(skate_boards::Part::Deck)
        .into_iter()
        .find(|i| i.asset == DECK && i.label.starts_with("White"))
        .or_else(|| library.new_items(skate_boards::Part::Deck).into_iter().find(|i| i.asset == DECK))
        .ok_or("no Skate deck in the library")?
        .material
        .clone();
    let base_path = library.texture_path(&plain).ok_or("the plain deck has no texture")?;
    let base = read_image(&base_path)?;

    let out_dir = skate_boards::custom_dir(assets);
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    images_in(folder, &mut files);
    let mut materials = serde_json::Map::new();
    let mut decks = Vec::new();
    for file in &files {
        let image = match read_image(file) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("  skipped {e}");
                continue;
            }
        };
        let relative = file.strip_prefix(folder).unwrap_or(file);
        let id = format!("c057{:012x}", fnv(&relative.to_string_lossy()) & 0xffff_ffff_ffff);
        let texture = if is_deck_texture(&image) {
            image
        } else {
            lay_graphic(&base, &image)
        };
        let png = out_dir.join(format!("{id}.png"));
        write_png(&png, &texture)?;
        let brand = brand_of(relative);
        let name = title_of(file);
        eprintln!("  {} -> {} ({})", relative.display(), name, skate_boards::brand_name(&brand));
        materials.insert(
            id.clone(),
            serde_json::json!({
                "name": name,
                "diffuse": format!("{}/{id}.png", skate_boards::CUSTOM_DIR),
                "alpha": false,
                "flags": { "ManufacturerID": brand, "Custom": "True" },
            }),
        );
        decks.push(id);
    }
    let json = serde_json::json!({ "model": DECK, "materials": materials, "decks": decks });
    std::fs::write(out_dir.join("custom.json"), serde_json::to_string_pretty(&json).unwrap()).map_err(|e| e.to_string())?;
    Ok(decks.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makers_come_from_the_folder_or_the_name() {
        assert_eq!(brand_of(Path::new("Baker/my deck.png")), "baker");
        assert_eq!(brand_of(Path::new("downloads/zoo_york_classic.dds")), "zooyork");
        assert_eq!(brand_of(Path::new("Alien Workshop/Trim.png")), "alienworkshop");
        assert_eq!(brand_of(Path::new("random/cat.png")), "custom");
        assert_eq!(skate_boards::brand_name(&brand_of(Path::new("random/cat.png"))), "Custom");
        assert_eq!(title_of(Path::new("x/my_cool-deck.png")), "My Cool Deck");
    }

    #[test]
    fn an_upright_graphic_is_laid_along_the_board_in_the_bottom_half() {
        let base = Rgba { width: 64, height: 64, pixels: vec![[200, 200, 200, 255]; 64 * 64] };
        // Upright: red top, blue bottom.
        let mut picture = Rgba { width: 10, height: 40, pixels: vec![[0, 0, 0, 255]; 400] };
        for y in 0..40 {
            for x in 0..10 {
                picture.pixels[(y * 10 + x) as usize] = if y < 20 { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
            }
        }
        let out = lay_graphic(&base, &picture);
        assert_eq!(out.at(10, 10), [200, 200, 200, 255], "the grip half stays");
        // Turned along the board: one end red, the other blue.
        let (left, right) = (out.at(2, 48), out.at(56, 48));
        assert!(left != right && (left[0] > 200 || left[2] > 200) && (right[0] > 200 || right[2] > 200), "{left:?} {right:?}");
        assert_eq!(out.at(62, 48), [200, 200, 200, 255], "the wood strip stays");
    }
}
