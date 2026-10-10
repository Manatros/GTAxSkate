//! What the HUD shows of a Skate session (`hud.rs`): the engine's own
//! types with the `own-core` feature (the plugin's older standalone core),
//! else the same shape without the engine. With SK8V as the core (A28) the
//! plugin builds without the engine, so without the 2010 Rust Rewrite
//! Mashup (A29).

#[cfg(feature = "own-core")]
pub use skate_host::{MarkerView, ScoreView};

/// The trick being done, the running sequence and the line, and what was
/// landed (as `skate_host::ScoreView`).
#[cfg(not(feature = "own-core"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScoreView {
    pub trick: String,
    pub trick_id: &'static str,
    pub tricks: u32,
    pub switch: bool,
    pub fakie: bool,
    pub nollie: bool,
    pub sequence: f32,
    pub multiplier: f32,
    pub line: f32,
    pub completed: f32,
    pub last_reward: f32,
    pub landings: u32,
    pub bailed: bool,
    pub clean: bool,
    pub sketchy: bool,
}

/// The session marker's prompt (as `skate_host::MarkerView`).
#[cfg(not(feature = "own-core"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkerView {
    pub prompt: bool,
    pub can_place: bool,
    pub placed: bool,
    pub placements: u64,
    pub progress: f32,
}
