//! Builds your own Skate 3 skater as a GTA V add-on ped: every head, hair
//! style, top, pair of pants and shoes of Skate 3's Create-a-Skater, from
//! your converted Skate 3 data, rebuilt on GTA's ped skeleton as CodeWalker
//! XML. Import the XML with CodeWalker into a DLC pack; the Skate 3 for GTA V
//! plugin then plays the skater as a fourth character with a Skate-style
//! editor.
//!
//!     skate-ped --assets <skate-data\assets> --skeleton <mp_m_freemode_01.yft.xml> --out <folder>
//!               [--converter <iw4l-skate-convert.exe> --game <Skate 3 folder>]
//!               [--texture-size 512] [--max-per-slot 0]
//!
//! Nothing from Skate 3 or GTA V ships with it: both come from the player's
//! own games.

mod dds;
mod import;
mod weapon;
mod grip;
mod retarget;
mod rw;
mod stickers;
mod wheel_one;
mod skate;
mod skeleton;
mod xml;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use glam::Vec3;
use serde_json::{Value, json};

pub const PED: &str = "skate3_skater";
const DEVICE: &str = "dlc_skate3";
const PACK: &str = "skate3";

/// A GTA ped component for a Create-a-Skater slot, and its label.
fn component_of(slot: &str) -> Option<(u8, &'static str)> {
    let s = slot.to_ascii_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| s.contains(x));
    Some(if has(&["facialhair", "beard", "moustache", "mustache", "goatee"]) {
        (10, "Facial hair")
    } else if has(&["rostral", "head", "face"]) && !has(&["headwear", "headgear"]) {
        (0, "Head")
    } else if has(&["hat", "cap", "helmet", "beanie", "headwear", "headgear"]) {
        (1, "Hat")
    } else if has(&["hair"]) {
        (2, "Hair")
    } else if has(&["glove", "hand", "wrist", "watch"]) {
        (5, "Gloves")
    } else if has(&["arm", "skin", "body"]) {
        (3, "Arms")
    } else if has(&["pant", "short", "leg", "lower", "trouser", "jean"]) {
        (4, "Pants")
    } else if has(&["feet", "foot", "shoe", "sneaker", "boot"]) {
        (6, "Shoes")
    } else if has(&["organ", "eye", "teeth", "mouth"]) {
        (7, "Eyes")
    } else if has(&["inner", "undershirt"]) {
        (8, "Undershirt")
    } else if has(&["glass", "eyewear", "goggle", "shade"]) {
        (9, "Glasses")
    } else if has(&["torso", "shirt", "jacket", "hood", "top", "coat", "upper"]) {
        (11, "Top")
    } else {
        return None;
    })
}

struct Options {
    assets: PathBuf,
    skeleton: PathBuf,
    out: PathBuf,
    converter: Option<PathBuf>,
    game: Option<PathBuf>,
    texture_size: u32,
    max_per_slot: usize,
    /// Only the board textures (for a skater made before they were packed).
    boards_only: bool,
    /// Only the board as a GTA weapon model (`weapon`).
    weapon: bool,
    /// The GTA plugin's board file (`SkateGTA-boards.txt`): the weapon model
    /// is the board ridden there.
    boards: Option<PathBuf>,
    /// A folder of the player's own deck images to add (see `import`).
    import_decks: Option<PathBuf>,
    /// Prepares the board library from the Skate 3 folder (`--game`) with
    /// the Skate 3 Rust Engine's own tools (`--engine`, its source) run by
    /// `--python`: no 2010 Rust Rewrite Mashup needed (A29).
    prepare_boards: bool,
    engine: Option<PathBuf>,
    python: Option<PathBuf>,
}

fn usage() -> String {
    "usage: skate-ped --assets <skate-data\\assets> --skeleton <mp_m_freemode_01.yft.xml> --out <folder> [--converter <exe> --game <Skate 3 folder>] [--texture-size 512] [--max-per-slot 0] [--boards-only] [--weapon [--boards <SkateGTA-boards.txt>]] [--import-decks <folder of images>]\n       skate-ped --assets <folder> --prepare-boards --game <Skate 3 folder> --engine <skate-3-rust-engine source> --python <python.exe>".into()
}

fn options() -> Result<Options, String> {
    let mut o = Options {
        assets: PathBuf::new(),
        skeleton: PathBuf::new(),
        out: PathBuf::new(),
        converter: None,
        game: None,
        texture_size: 512,
        max_per_slot: 0,
        boards_only: false,
        weapon: false,
        import_decks: None,
        boards: None,
        prepare_boards: false,
        engine: None,
        python: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(usage);
        match arg.as_str() {
            "--assets" => o.assets = value()?.into(),
            "--skeleton" => o.skeleton = value()?.into(),
            "--out" => o.out = value()?.into(),
            "--converter" => o.converter = Some(value()?.into()),
            "--game" => o.game = Some(value()?.into()),
            "--texture-size" => o.texture_size = value()?.parse().map_err(|_| usage())?,
            "--max-per-slot" => o.max_per_slot = value()?.parse().map_err(|_| usage())?,
            "--boards-only" => o.boards_only = true,
            "--weapon" => o.weapon = true,
            "--import-decks" => o.import_decks = Some(PathBuf::from(value()?)),
            "--boards" => o.boards = Some(value()?.into()),
            "--prepare-boards" => o.prepare_boards = true,
            "--engine" => o.engine = Some(value()?.into()),
            "--python" => o.python = Some(value()?.into()),
            _ => return Err(usage()),
        }
    }
    if o.prepare_boards {
        return if o.assets.as_os_str().is_empty() || o.game.is_none() || o.engine.is_none() || o.python.is_none() { Err(usage()) } else { Ok(o) };
    }
    if o.assets.as_os_str().is_empty() || (o.skeleton.as_os_str().is_empty() && !o.boards_only && !o.weapon && o.import_decks.is_none()) || o.out.as_os_str().is_empty() {
        return Err(usage());
    }
    o.texture_size = o.texture_size.clamp(64, 2048).next_power_of_two();
    Ok(o)
}

#[derive(Clone, Debug)]
struct Model {
    id: String,
    name: String,
    scene: String,
    finishes: Vec<(String, String, [f32; 3])>,
}

/// The character library the converter wrote: models by slot.
fn library(dir: &Path) -> Result<BTreeMap<String, Vec<Model>>, String> {
    let path = dir.join("library.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let raw: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let materials = raw["materials"].as_object().cloned().unwrap_or_default();
    let mut out: BTreeMap<String, Vec<Model>> = BTreeMap::new();
    for (id, m) in raw["models"].as_object().ok_or("the library has no models")? {
        let finishes = m["materials"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mid| {
                let material = materials.get(mid.as_str()?)?;
                let tint = material["tint"].as_array().map(|t| {
                    [0, 1, 2].map(|i| t.get(i).and_then(Value::as_f64).unwrap_or(1.) as f32)
                });
                Some((
                    material["name"].as_str().unwrap_or("Original").trim().to_owned(),
                    material["diffuse"].as_str()?.to_owned(),
                    tint.unwrap_or([1.; 3]),
                ))
            })
            .collect();
        out.entry(m["slot"].as_str().unwrap_or("").to_owned()).or_default().push(Model {
            id: id.clone(),
            name: m["name"].as_str().unwrap_or(id).to_owned(),
            scene: m["scene"].as_str().ok_or("a model has no scene")?.to_owned(),
            finishes,
        });
    }
    Ok(out)
}

fn texture(assets: &Path, diffuse: &str, tint: [f32; 3], size: u32) -> Result<dds::Image, String> {
    let path = assets.join(diffuse);
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let decoded = skate_boards::decode_png(&bytes)?;
    let mut image = dds::Image { width: decoded.width, height: decoded.height, rgba: decoded.rgba };
    if tint.iter().any(|t| (t - 1.).abs() > 1e-3) {
        for p in image.rgba.chunks_exact_mut(4) {
            for c in 0..3 {
                p[c] = (f32::from(p[c]) * tint[c].clamp(0., 1.)).round() as u8;
            }
        }
    }
    Ok(image.power_of_two(size))
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(path, data).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Writes a texture dictionary of one texture: `<dir>/<name>.ytd.xml` and
/// `<dir>/<name>/<name>.dds`.
/// Every board finish (and the default skater's board) as textures for
/// the plugin's drawn board, in one dictionary per part
/// (`skate_boards::gta_texture`). Returns how many.
fn board_textures(assets: &Path, rpf: &Path) -> Result<usize, String> {
    use skate_boards::{Board, Library, Part, gta_texture};
    let mut dicts: BTreeMap<String, BTreeMap<String, dds::Image>> = BTreeMap::new();
    // Decks are long and carry the graphics; trucks and wheels are small.
    let size = |part: Part| if part == Part::Deck { 512 } else { 128 };
    let to_image = |t: skate_boards::Texture, limit: u32| dds::Image { width: t.width, height: t.height, rgba: t.rgba }.power_of_two(limit);
    if Library::exists(assets) {
        let library = Library::load(assets)?;
        for part in Part::ALL {
            for item in library.items(part) {
                let (dict, name) = gta_texture(part, &format!("iw4l_skate/board/{}", item.material));
                if dicts.get(&dict).is_some_and(|d| d.contains_key(&name)) {
                    continue;
                }
                match library.solid_texture(part, &item.material, size(part)) {
                    Ok(texture) => {
                        dicts.entry(dict).or_default().insert(name, to_image(texture, size(part)));
                    }
                    Err(e) => println!("  board finish {}: {e}", item.title()),
                }
            }
        }
    }
    if let Ok(board) = Board::default_board(assets, 512) {
        for surface in &board.surfaces {
            let (dict, name) = gta_texture(surface.part, &surface.material);
            if let Some(texture) = board.textures.get(surface.texture) {
                let mut texture = texture.clone();
                texture.make_opaque();
                dicts.entry(dict).or_default().insert(name, to_image(texture, size(surface.part)));
            }
        }
    }
    let mut count = 0;
    for (dict, textures) in &dicts {
        let mut items = Vec::new();
        let mut encoded = Vec::new();
        for (name, image) in textures {
            let (bytes, levels, format) = dds::encode(image);
            write(&rpf.join(dict).join(format!("{name}.dds")), &bytes)?;
            encoded.push((name.clone(), image.width, image.height, levels, format));
        }
        for (name, width, height, levels, format) in &encoded {
            items.push(xml::TextureItem { name, usage: "DIFFUSE", width: *width, height: *height, levels: *levels, format });
        }
        write(&rpf.join(format!("{dict}.ytd.xml")), xml::ytd(&items).as_bytes())?;
        count += textures.len();
    }
    Ok(count)
}

fn write_ytd(dir: &Path, name: &str, image: &dds::Image) -> Result<(), String> {
    let (bytes, levels, format) = dds::encode(image);
    write(&dir.join(name).join(format!("{name}.dds")), &bytes)?;
    let item = xml::TextureItem { name, usage: "DIFFUSE", width: image.width, height: image.height, levels, format };
    write(&dir.join(format!("{name}.ytd.xml")), xml::ytd(&[item]).as_bytes())
}

/// One part's meshes on the ped skeleton, split into geometries the game
/// can index (at most 65535 vertices each).
fn geometries(
    surfaces: &[skate::Surface],
    retarget: &retarget::Retarget,
) -> Vec<xml::Geometry> {
    let mut out = Vec::new();
    for surface in surfaces {
        let moved: Vec<(Vec3, Vec3, Vec<(usize, f32)>)> = surface.vertices.iter().map(|v| retarget.vertex(v)).collect();
        let positions: Vec<Vec3> = moved.iter().map(|m| m.0).collect();
        let normals: Vec<Vec3> = moved.iter().map(|m| m.1).collect();
        let uvs: Vec<glam::Vec2> = surface.vertices.iter().map(|v| v.uv).collect();
        let tangents = xml::tangents(&positions, &normals, &uvs, &surface.indices);
        // Triangles go into geometries until one would overflow.
        let mut geometry = xml::Geometry::default();
        let mut remap: HashMap<u32, u16> = HashMap::new();
        let mut slots: HashMap<usize, u8> = HashMap::new();
        for triangle in surface.indices.chunks_exact(3) {
            let new_vertices = triangle.iter().filter(|i| !remap.contains_key(i)).count();
            let new_bones: usize = {
                let mut bones: Vec<usize> = triangle
                    .iter()
                    .flat_map(|&i| moved[i as usize].2.iter().map(|w| w.0))
                    .filter(|b| !slots.contains_key(b))
                    .collect();
                bones.sort_unstable();
                bones.dedup();
                bones.len()
            };
            if geometry.vertices.len() + new_vertices > 65535 || geometry.palette.len() + new_bones > 250 {
                out.push(std::mem::take(&mut geometry));
                remap.clear();
                slots.clear();
            }
            for &i in triangle {
                let at = match remap.get(&i) {
                    Some(&at) => at,
                    None => {
                        let (position, normal, weights) = &moved[i as usize];
                        let mut bones = [0u8; 4];
                        let mut w = [0f32; 4];
                        for (k, (bone, weight)) in weights.iter().take(4).enumerate() {
                            let slot = *slots.entry(*bone).or_insert_with(|| {
                                geometry.palette.push(*bone);
                                (geometry.palette.len() - 1) as u8
                            });
                            bones[k] = slot;
                            w[k] = *weight;
                        }
                        geometry.vertices.push(xml::Vertex {
                            position: *position,
                            normal: *normal,
                            uv: uvs[i as usize],
                            tangent: tangents[i as usize],
                            bones,
                            weights: xml::weight_bytes(&w),
                        });
                        let at = (geometry.vertices.len() - 1) as u16;
                        remap.insert(i, at);
                        at
                    }
                };
                geometry.indices.push(at);
            }
        }
        if !geometry.indices.is_empty() {
            out.push(geometry);
        }
    }
    out
}

fn run() -> Result<(), String> {
    build(&options()?)
}

fn build(o: &Options) -> Result<(), String> {
    if o.prepare_boards
        && let (Some(game), Some(engine), Some(python)) = (&o.game, &o.engine, &o.python)
    {
        std::fs::create_dir_all(&o.assets).map_err(|e| e.to_string())?;
        let count = skate_boards::convert::prepare_with_python(python, engine, game, &o.assets, |line| println!("{line}"))?;
        println!("Board library: {count} parts in {}", o.assets.display());
        return Ok(());
    }
    // The player's own decks first, so the weapon pack has their models.
    if let Some(folder) = &o.import_decks {
        let count = import::import_decks(&o.assets, folder)?;
        eprintln!("imported {count} decks from {}", folder.display());
    }
    if o.skeleton.as_os_str().is_empty() && !o.boards_only && !o.weapon {
        return Ok(());
    }
    if o.weapon {
        // The board ridden in GTA (the plugin's board file), else the default
        // skater's, at the library's full texture size; parts placed by the
        // default board's joints.
        let defaults = skate_boards::Board::default_board(&o.assets, 2048)?;
        let riding = o
            .boards
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| text.lines().find_map(|l| l.strip_prefix("riding ").map(skate_boards::BoardChoice::from_text)));
        let board = match (riding, skate_boards::Library::exists(&o.assets)) {
            (Some(choice), true) => {
                let library = skate_boards::Library::load(&o.assets)?;
                println!("The ridden board: {}", choice.to_text());
                skate_boards::assemble(Some(&library), &defaults, &choice, 2048)?
            }
            _ => defaults.clone(),
        };
        weapon::write(&o.out, &board, &defaults)?;
        println!("The board as a weapon model: {}", o.out.join(format!("{}.ydr.xml", weapon::MODEL)).display());
        // Every part in every finish as its own model (`parts\`, listed in
        // `parts.txt`): the plugin puts any board together from them.
        if skate_boards::Library::exists(&o.assets) {
            let library = skate_boards::Library::load(&o.assets)?;
            let dir = o.out.join("parts");
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let names = weapon::write_parts(&dir, &library, &defaults, |i, n| {
                if i % 50 == 0 || i == n {
                    println!("  board parts {i}/{n}");
                }
            })?;
            std::fs::write(dir.join("parts.txt"), names.join("\n") + "\n").map_err(|e| e.to_string())?;
            println!("Board parts as models: {} in {}", names.len(), dir.display());
            // One wheel alone for the store's part view, a set of its own
            // (`wheel\`, `skate3wheel`).
            let dir = o.out.join("wheel");
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let frame = weapon::Frame::of(&defaults, &defaults);
            let made = wheel_one::write(&dir, &library, &defaults, &frame)?;
            std::fs::write(dir.join(format!("{}.ytyp.xml", wheel_one::YTYP)), weapon::ytyp(wheel_one::YTYP, &made)).map_err(|e| e.to_string())?;
            let names: Vec<&str> = made.iter().map(|(n, ..)| n.as_str()).collect();
            std::fs::write(dir.join("wheel.txt"), names.join("\n") + "\n").map_err(|e| e.to_string())?;
            println!("Single wheels as models: {} in {}", made.len(), dir.display());
            // Stickers, a set of their own (`stickers\`, `skate3stickers`):
            // should GTA not take them, the parts above still load. They
            // need Skate's own files (--game).
            let dir = o.out.join("stickers");
            let _ = std::fs::remove_dir_all(&dir);
            if let Some(game) = o.game.as_deref() {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let frame = weapon::Frame::of(&defaults, &defaults);
                match stickers::write_stickers(&dir, &o.assets, game, &library, &defaults, &frame) {
                    Ok(made) => {
                        std::fs::write(dir.join(format!("{}.ytyp.xml", stickers::YTYP)), weapon::ytyp(stickers::YTYP, &made)).map_err(|e| e.to_string())?;
                        let names: Vec<&str> = made.iter().map(|(n, ..)| n.as_str()).collect();
                        std::fs::write(dir.join("stickers.txt"), names.join("\n") + "\n").map_err(|e| e.to_string())?;
                        println!("Stickers as models: {} in {}", made.len(), dir.display());
                    }
                    Err(e) => println!("Stickers left out: {e}"),
                }
            }
        }
        return Ok(());
    }
    if o.boards_only {
        let rpf = o.out.join("x64").join("models").join("cdimages").join(PACK);
        let count = board_textures(&o.assets, &rpf)?;
        println!("Board textures for the drawn board: {count}");
        return Ok(());
    }
    let character = o.assets.join(skate_boards::convert::CHARACTER_DIR);
    if !character.join("library.json").is_file() {
        match (&o.converter, &o.game) {
            (Some(converter), Some(game)) => {
                skate_boards::convert::prepare_character(converter, game, &o.assets, |line| println!("  {line}"))?;
            }
            _ => {
                return Err(format!(
                    "{} has no character library yet: run again with --converter <mashup>\\skate\\iw4l-skate-convert.exe --game <Skate 3 folder>",
                    character.display()
                ));
            }
        }
    }
    let skeleton_text = std::fs::read_to_string(&o.skeleton).map_err(|e| format!("cannot read {}: {e}", o.skeleton.display()))?;
    let skeleton = skeleton::Skeleton::from_xml(&skeleton_text)?;
    println!("Ped skeleton: {} bones", skeleton.bones.len());
    let skater_path = o.assets.join("private").join("skater.glb");
    let skater = std::fs::read(&skater_path).map_err(|e| format!("cannot read {}: {e}", skater_path.display()))?;
    let rig = skate::Rig::from_glb(&skater)?;
    retarget::Retarget::new(&rig, &skeleton)?;
    let library = library(&character)?;
    let defaults: HashMap<&str, &str> = skate_boards::convert::DEFAULT_SKATER.iter().copied().collect();

    // Slots onto components; several slots may share one (their models
    // follow each other).
    let mut by_component: BTreeMap<u8, (&'static str, Vec<(String, Model)>)> = BTreeMap::new();
    for (slot, models) in &library {
        let Some((component, label)) = component_of(slot) else {
            println!("  (no GTA slot for Skate 3's {slot}; left out)");
            continue;
        };
        let mut models = models.clone();
        let default = defaults.get(slot.as_str()).copied();
        models.sort_by(|a, b| (Some(a.id.as_str()) != default).cmp(&(Some(b.id.as_str()) != default)).then(a.name.cmp(&b.name)));
        let entry = by_component.entry(component).or_insert((label, Vec::new()));
        entry.1.extend(models.into_iter().map(|m| (slot.clone(), m)));
    }
    let rpf = o.out.join("x64").join("models").join("cdimages").join(PACK);
    let files = rpf.join(PED);
    let mut components = Vec::new();
    let mut catalog = Vec::new();
    let flat_normal = dds::Image::solid(8, [128, 128, 255, 255]);
    let dull_spec = dds::Image::solid(8, [24, 24, 24, 255]);
    for (component, (label, models)) in &by_component {
        let name = ["head", "berd", "hair", "uppr", "lowr", "hand", "feet", "teef", "accs", "task", "decl", "jbib"][usize::from(*component)];
        let mut textures = Vec::new();
        let mut drawables = Vec::new();
        let mut default_drawable = 0;
        // GTA keeps a component's texture count in one byte.
        let mut total_textures = 0usize;
        for (slot, model) in models {
            if o.max_per_slot > 0 && drawables.len() >= o.max_per_slot {
                break;
            }
            if drawables.len() >= 255 {
                println!("  {label}: only the first 255 kept");
                break;
            }
            let scene = o.assets.join(&model.scene);
            let bytes = match std::fs::read(&scene) {
                Ok(bytes) => bytes,
                Err(e) => {
                    println!("  skipped {} ({}): {e}", model.name, scene.display());
                    continue;
                }
            };
            let (surfaces, part_rig) = match skate::surfaces(&bytes) {
                Ok(read) => read,
                Err(e) => {
                    println!("  skipped {}: {e}", model.name);
                    continue;
                }
            };
            let mut merged = rig.clone();
            merged.merge(part_rig);
            let retarget = retarget::Retarget::new(&merged, &skeleton)?;
            let geometries = geometries(&surfaces, &retarget);
            if geometries.is_empty() {
                continue;
            }
            let room = 255usize.saturating_sub(total_textures);
            if room == 0 {
                println!("  {label}: full at 255 textures, the rest left out");
                break;
            }
            let index = drawables.len();
            let mut labels = Vec::new();
            let mut alpha = false;
            for (letter, (finish, diffuse, tint)) in model.finishes.iter().take(26.min(room)).enumerate() {
                let image = match texture(&o.assets, diffuse, *tint, o.texture_size) {
                    Ok(image) => image,
                    Err(e) => {
                        println!("  {} / {finish}: {e}", model.name);
                        continue;
                    }
                };
                alpha |= letter == 0 && image.has_alpha();
                let letter = (b'a' + labels.len() as u8) as char;
                write_ytd(&files, &format!("{name}_diff_{index:03}_{letter}_uni"), &image)?;
                labels.push(finish.clone());
            }
            if labels.is_empty() {
                println!("  skipped {}: none of its finishes could be read", model.name);
                continue;
            }
            let drawable = format!("{name}_{index:03}_u");
            let normal = format!("{name}_normal_{index:03}");
            let spec = format!("{name}_spec_{index:03}");
            let mut embedded = Vec::new();
            for (tex, image, usage) in [(&normal, &flat_normal, "NORMAL"), (&spec, &dull_spec, "SPECULAR")] {
                let (bytes, levels, format) = dds::encode(image);
                write(&files.join(&drawable).join(format!("{tex}.dds")), &bytes)?;
                embedded.push((tex.clone(), usage, levels, format));
            }
            let items: Vec<xml::TextureItem> = embedded
                .iter()
                .map(|(n, usage, levels, format)| xml::TextureItem { name: n, usage, width: 8, height: 8, levels: *levels, format })
                .collect();
            let shader = xml::Shader {
                name: if alpha { "ped_alpha" } else { "ped" },
                diffuse: &format!("{name}_diff_{index:03}_a_uni"),
                normal: &normal,
                specular: &spec,
            };
            write(&files.join(format!("{drawable}.ydd.xml")), xml::ydd(&drawable, &shader, &items, &geometries).as_bytes())?;
            if defaults.get(slot.as_str()) == Some(&model.id.as_str()) {
                default_drawable = index;
            }
            total_textures += labels.len();
            textures.push(labels.len() as u8);
            drawables.push(json!({"name": model.name, "skate_slot": slot, "textures": labels}));
            println!("  {label} {index}: {} ({} finishes)", model.name, textures[index]);
        }
        if drawables.is_empty() {
            continue;
        }
        components.push(xml::Component { slot: *component, textures });
        catalog.push(json!({"component": component, "label": label, "default": default_drawable, "drawables": drawables}));
    }
    if components.is_empty() {
        return Err("no Create-a-Skater part could be converted".into());
    }
    write(&rpf.join(format!("{PED}.ymt.xml")), xml::ymt(PED, &components).as_bytes())?;
    let boards = board_textures(&o.assets, &rpf)?;
    if boards > 0 {
        println!("Board textures for the drawn board: {boards}");
    }
    let dlc = o.out.join("dlc");
    write(&dlc.join("setup2.xml"), xml::setup(DEVICE, PACK).as_bytes())?;
    write(&dlc.join("content.xml"), xml::content(DEVICE, PACK).as_bytes())?;
    write(&dlc.join("common").join("data").join("peds.meta"), xml::peds_meta(PED).as_bytes())?;
    let catalog = json!({"ped": PED, "components": catalog});
    write(&o.out.join("SkateGTA-skater.json"), serde_json::to_string_pretty(&catalog).unwrap().as_bytes())?;
    write(&o.out.join("IMPORT.txt"), import_steps().as_bytes())?;
    println!("Done: {} in {}", PED, o.out.display());
    println!("Next: follow IMPORT.txt there (CodeWalker / OpenIV).");
    Ok(())
}

fn import_steps() -> String {
    format!(
        "The Skate 3 skater as a GTA V add-on ped\n\
         =========================================\n\n\
         1. In OpenIV (Edit mode), make mods\\update\\x64\\dlcpacks\\{PACK}\\dlc.rpf and put in it:\n\
         \x20  - setup2.xml and content.xml from the dlc folder here\n\
         \x20  - common\\data\\peds.meta from the dlc folder here\n\
         \x20  - an empty archive x64\\models\\cdimages\\{PACK}.rpf\n\
         2. Open {PACK}.rpf in CodeWalker's RPF Explorer (Edit mode on). Import XML (Edit > Import XML):\n\
         \x20  - x64\\models\\cdimages\\{PACK}\\{PED}.ymt.xml from here\n\
         \x20  - make a folder {PED} inside {PACK}.rpf, open it, and import every .ydd.xml and .ytd.xml\n\
         \x20    from x64\\models\\cdimages\\{PACK}\\{PED} here (only the .xml files; their texture\n\
         \x20    folders must stay beside them).\n\
         3. Copy mp_m_freemode_01.yft (x64v.rpf\\models\\cdimages\\streamedpeds_mp.rpf) into {PACK}.rpf\n\
         \x20  and rename it {PED}.yft: the skater uses GTA's own ped skeleton.\n\
         4. Add <Item>dlcpacks:/{PACK}/</Item> to mods\\update\\update.rpf\\common\\data\\dlclist.xml.\n\
         5. Copy SkateGTA-skater.json from here beside SkateGTA.asi.\n\
         In game: F8 becomes your skater (and back); F11 opens the skater editor.\n"
    )
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ERROR: {e}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_a_skater_slots_find_ped_components() {
        assert_eq!(component_of("Rostral"), Some((0, "Head")));
        assert_eq!(component_of("Hair"), Some((2, "Hair")));
        assert_eq!(component_of("FacialHair"), Some((10, "Facial hair")));
        assert_eq!(component_of("OuterTorso"), Some((11, "Top")));
        assert_eq!(component_of("InnerTorso"), Some((8, "Undershirt")));
        assert_eq!(component_of("Pants"), Some((4, "Pants")));
        assert_eq!(component_of("Feet"), Some((6, "Shoes")));
        assert_eq!(component_of("Arm"), Some((3, "Arms")));
        assert_eq!(component_of("Organ"), Some((7, "Eyes")));
        assert_eq!(component_of("Hat"), Some((1, "Hat")));
        assert_eq!(component_of("Glasses"), Some((9, "Glasses")));
        assert_eq!(component_of("Gloves"), Some((5, "Gloves")));
        assert_eq!(component_of("Mystery"), None);
    }

    #[test]
    fn a_part_splits_into_indexable_geometries() {
        let skeleton = skeleton::Skeleton::from_xml(skeleton::TEST_SKELETON).unwrap();
        let rig = skate::Rig::from_glb(&skate::testing::glb()).unwrap();
        let retarget = retarget::Retarget::new(&rig, &skeleton).unwrap();
        let (surfaces, _) = skate::surfaces(&skate::testing::glb()).unwrap();
        let g = geometries(&surfaces, &retarget);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].indices, [0, 1, 2]);
        assert_eq!(g[0].palette, [skeleton.find("SKEL_Pelvis").unwrap()]);
        assert_eq!(g[0].vertices[0].weights, [255, 0, 0, 0]);
    }

    /// The whole run on a made-up converted Skate 3: a skater, one top in
    /// two finishes, and a ped skeleton.
    #[test]
    fn builds_the_add_on_ped() {
        let root = std::env::temp_dir().join(format!("skate-ped-{}", std::process::id()));
        let assets = root.join("assets");
        let glb = skate::testing::glb();
        write(&assets.join("private/skater.glb"), &glb).unwrap();
        write(&assets.join("private/character/parts/top.glb"), &glb).unwrap();
        let png = |rgba: [u8; 4]| {
            let mut out = Vec::new();
            let mut encoder = png::Encoder::new(&mut out, 8, 8);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&rgba.repeat(64)).unwrap();
            out
        };
        write(&assets.join("private/character/tex/red.png"), &png([200, 0, 0, 255])).unwrap();
        write(&assets.join("private/character/tex/blue.png"), &png([0, 0, 200, 255])).unwrap();
        let library = json!({
            "models": {"t1": {"slot": "OuterTorso", "name": "Plain Tee", "materials": ["m1", "m2"], "scene": "private/character/parts/top.glb"}},
            "materials": {
                "m1": {"name": "Red", "diffuse": "private/character/tex/red.png"},
                "m2": {"name": "Blue", "diffuse": "private/character/tex/blue.png", "tint": [1, 1, 0.5]}
            }
        });
        write(&assets.join("private/character/library.json"), library.to_string().as_bytes()).unwrap();
        let skeleton = root.join("mp_m_freemode_01.yft.xml");
        write(&skeleton, skeleton::TEST_SKELETON.as_bytes()).unwrap();
        let out = root.join("out");
        build(&Options { assets, skeleton, out: out.clone(), converter: None, game: None, texture_size: 512, max_per_slot: 0, import_decks: None, boards_only: false, weapon: false, boards: None, prepare_boards: false, engine: None, python: None }).unwrap();
        let files = out.join("x64/models/cdimages/skate3/skate3_skater");
        assert!(files.join("jbib_000_u.ydd.xml").is_file());
        assert!(files.join("jbib_000_u/jbib_normal_000.dds").is_file());
        assert!(files.join("jbib_diff_000_a_uni.ytd.xml").is_file());
        assert!(files.join("jbib_diff_000_b_uni/jbib_diff_000_b_uni.dds").is_file());
        let ymt = std::fs::read_to_string(out.join("x64/models/cdimages/skate3/skate3_skater.ymt.xml")).unwrap();
        assert!(ymt.contains("<availComp>255 255 255 255 255 255 255 255 255 255 255 0</availComp>"));
        let catalog: Value = serde_json::from_slice(&std::fs::read(out.join("SkateGTA-skater.json")).unwrap()).unwrap();
        assert_eq!(catalog["components"][0]["component"], 11);
        assert_eq!(catalog["components"][0]["drawables"][0]["textures"], json!(["Red", "Blue"]));
        assert!(out.join("dlc/common/data/peds.meta").is_file());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
