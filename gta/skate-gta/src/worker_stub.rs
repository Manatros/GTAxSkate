//! The plugin's older standalone core is not built in (no `own-core`
//! feature): SK8V skates (A28), and the plugin builds without the Skate
//! engine, so without the 2010 Rust Rewrite Mashup (A29). The same shape as
//! `worker.rs`; a worker never starts.

use bevy_math::{Mat3, Mat4, Vec3};
use std::path::PathBuf;

use crate::core_types::{MarkerView, ScoreView};

/// Skate 3's options (see `config`).
#[derive(Clone, Debug)]
pub struct Options {
    pub difficulty: String,
    pub trucks: f32,
    pub wheels: f32,
    pub tuning: [f32; 11],
    pub hold_fakie: bool,
    pub hippie_jump: f32,
}

/// One published Skate pose, in Skate space.
#[derive(Clone, Debug)]
pub struct Frame {
    pub root: Mat4,
    pub bones: Vec<Mat4>,
    pub camera: Option<(Vec3, Mat3, f32)>,
    pub board: Mat4,
    pub velocity: Vec3,
    pub state: String,
    pub tick: u64,
    pub score: ScoreView,
    pub marker: MarkerView,
    pub surface: u8,
    pub wheels_down: u8,
}

pub enum Event {
    Ready { names: Vec<String> },
    Activated(Frame),
    Frame(Frame),
    Failed(String),
    Note(String),
}

pub struct Worker;

impl Worker {
    pub fn start(_assets: PathBuf) -> Result<Self, String> {
        Err("this build has no standalone Skate core (SK8V skates; build with the own-core feature for the old one)".into())
    }
    pub fn options(&self, _options: Options) {}
    pub fn collision(&self, _triangles: Vec<[[f32; 3]; 3]>, _surfaces: Option<Vec<u8>>, _rails: Vec<Vec<[f32; 3]>>, _now: bool) {}
    pub fn activate(&self, _spawn: [f32; 3], _heading: f32, _on_foot: bool, _speed: f32) {}
    pub fn step(&self, _dt: f32, _aspect_ratio: f32, _pad: Option<skate_input::Pad>) {}
    pub fn dynamic(&self, _triangles: Vec<[[f32; 3]; 3]>) {}
    pub fn suspend(&self) {}
    pub fn retry(&self) {}
    pub fn events(&self) -> Vec<Event> {
        Vec::new()
    }
}
