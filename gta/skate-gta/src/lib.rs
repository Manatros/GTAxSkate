//! Skate 3 for GTA V.
//!
//! The Skate 3 Rust engine (physics, flick-it tricks, grinds, wallrides,
//! bails and the skater's animations) drives the GTA V player while skate
//! mode is on. GTA V gives scripts no collision meshes, so the world around
//! the skater is scanned with ray probes and rebuilt as Skate collision,
//! with grind rails found on the ledges, curbs and handrails it meets. The
//! skater's pose is retargeted onto the player's skeleton.
//!
//! Everything but the Script Hook V layer (`win`) is platform-free and
//! tested on any system; the plugin itself is Windows-only.

pub mod action_cam;
pub mod aim;
pub mod board_look;
pub mod cam_cycle;
pub mod config;
pub mod coop;
pub mod coords;
pub mod ground;
pub mod hom;
pub mod stamina;
pub mod hud;
pub mod live;
pub mod options;
pub mod parks;
pub mod ped_rig;
pub mod rig;
pub mod rolling;
pub mod shop;
pub mod space;
pub mod skater;
pub mod terrain;
pub mod vehicles;
pub mod core_types;
#[cfg(feature = "own-core")]
pub mod worker;
#[cfg(not(feature = "own-core"))]
#[path = "worker_stub.rs"]
pub mod worker;

#[cfg(any(windows, feature = "check-windows"))]
mod win;

/// For the core's runtime, which links the plugin in (no `asi` feature).
#[cfg(windows)]
pub use win::{embedded_keyboard_pad, embedded_tick};
