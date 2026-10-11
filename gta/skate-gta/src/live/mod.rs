//! GTA's own collision as Skate's: the static map read live from GTA's
//! physics level (the table of every collision object the game has loaded),
//! as SK8V does it, instead of ray probes.
//!
//! The decoding (`bounds`, `clean`, `level`), the rail finder (`rails`,
//! `grid`) are SK8V's (https://github.com/becaring/SK8V, Apache-2.0,
//! Copyright 2026 becaring; the live reader adapted from Sol4ra's
//! LS-Skate-LiveCollision). `locate` is a Rust port of SK8V's
//! `host/src/physics_level.cpp`. This module runs them on a thread of its
//! own and hands the script the area round the skater.

pub mod bounds;
pub mod clean;
pub mod grid;
pub mod level;
#[cfg(windows)]
pub mod locate;
pub mod rails;

use bevy_math::Vec3;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// A collision triangle in GTA space and its GTA material (`materials.dat`
/// index).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tri {
    pub v: [[f32; 3]; 3],
    pub material: u8,
}

/// Half the side of the square of collision built round the skater.
pub const HALF: f32 = 64.0;
/// Moved this far from where the area was built, a new one is asked for.
pub const RECENTRE: f32 = 24.0;

/// The static map round a point, ready for Skate.
pub struct Area {
    pub centre: Vec3,
    /// GTA space.
    pub triangles: Vec<[Vec3; 3]>,
    /// Skate's ground per triangle (`crate::ground`).
    pub grounds: Vec<u8>,
    /// GTA space polylines.
    pub rails: Vec<Vec<Vec3>>,
    /// What was read, for the log.
    pub note: String,
}

/// Skate's ground for a GTA material index (`materials.dat` order: 1 is
/// CONCRETE, as `crate::ground::MATERIALS` lists them from there).
pub fn ground_of_index(material: u8) -> u8 {
    match material {
        0 => 0,
        i => crate::ground::MATERIALS.get(i as usize - 1).map_or(0, |(_, hash)| crate::ground::ground_of(*hash)),
    }
}

/// The area round `centre` from `level`: its triangles, their grounds and
/// the rails on them.
pub fn build(level: &level::Level, centre: Vec3) -> Result<Area, String> {
    let (tris, census) = level.walk([centre.x, centre.y], HALF)?;
    let (triangles, grounds): (Vec<[Vec3; 3]>, Vec<u8>) = tris
        .iter()
        .map(|t| (t.v.map(Vec3::from_array), ground_of_index(t.material)))
        .filter(|(p, _)| p.iter().all(|v| v.is_finite()) && (p[1] - p[0]).cross(p[2] - p[0]).length_squared() > 1e-10)
        .unzip();
    let (rails, rail_census) = rails::find_metres(&triangles);
    let note = format!("{census}; rails {rail_census:?}");
    Ok(Area { centre, triangles, grounds, rails, note })
}

enum Ask {
    /// Where the table is.
    Level(level::Level),
    /// An area round this point.
    Build(Vec3),
}

/// The thread that reads GTA's collision; the script asks it for areas and
/// takes them when ready.
pub struct Service {
    ask: mpsc::Sender<Ask>,
    ready: Arc<Mutex<Option<Result<Area, String>>>>,
    pub level: Option<level::Level>,
    /// Where the last area was asked for.
    pub asked: Option<Vec3>,
}

impl Default for Service {
    fn default() -> Self {
        Self::start()
    }
}

impl Service {
    pub fn start() -> Self {
        let (ask, asks) = mpsc::channel::<Ask>();
        let ready = Arc::new(Mutex::new(None));
        let out = ready.clone();
        std::thread::Builder::new()
            .name("gta collision".into())
            .spawn(move || {
                let mut level = None;
                while let Ok(first) = asks.recv() {
                    // Only the newest request counts.
                    let mut newest = first;
                    while let Ok(next) = asks.try_recv() {
                        if let Ask::Level(l) = next {
                            level = Some(l);
                        } else {
                            newest = next;
                        }
                    }
                    match newest {
                        Ask::Level(l) => level = Some(l),
                        Ask::Build(centre) => {
                            let Some(l) = level.as_ref() else { continue };
                            let built = build(l, centre);
                            if let Ok(mut slot) = out.lock() {
                                *slot = Some(built);
                            }
                        }
                    }
                }
            })
            .expect("collision thread");
        Self { ask, ready, level: None, asked: None }
    }

    pub fn set_level(&mut self, level: level::Level) {
        self.level = Some(level);
        let _ = self.ask.send(Ask::Level(level));
    }

    /// Asks for the area round `centre` (built on the thread).
    pub fn request(&mut self, centre: Vec3) {
        if self.level.is_some() {
            self.asked = Some(centre);
            let _ = self.ask.send(Ask::Build(centre));
        }
    }

    /// An area finished since last asked.
    pub fn take(&self) -> Option<Result<Area, String>> {
        self.ready.lock().ok()?.take()
    }
}
