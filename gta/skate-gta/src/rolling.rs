//! Skate 3's own rolling sound: its "grains", one recording per surface
//! and wheel hardness (`get-skate-sounds.ps1` writes them as WAV files),
//! each running from slow rolling to fast. The board's speed picks the
//! place in the recording; short overlapping pieces from there (100 ms,
//! faded in and out, one every 50 ms, a little apart each time) make a
//! steady roll that follows the speed. The wheels spinning on after a
//! take-off are a recording too.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub type Sound = Arc<[f32]>;

/// Samples a piece lasts, and between pieces (44.1 kHz).
const PIECE: usize = 4410;
const HOP: usize = PIECE / 2;
/// The speed (m/s) the end of a recording is rolled at.
const TOP_SPEED: f32 = 14.;

/// The rate everything plays at.
pub const RATE: u32 = 44_100;

/// PCM WAV (16-bit, any channels: mixed down, any rate: brought to
/// [`RATE`]) as samples from -1 to 1. Skate 3's recordings are 48 kHz:
/// played as they were, they sounded too low and too slow.
pub fn parse_wav(bytes: &[u8]) -> Option<Vec<f32>> {
    let (samples, rate) = parse_wav_rate(bytes)?;
    Some(resample(&samples, rate, RATE))
}

/// `samples` at `from` Hz as `to` Hz (linear).
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || samples.is_empty() {
        return samples.to_vec();
    }
    let n = (samples.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = f64::from(from) / f64::from(to);
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let j = x as usize;
            let f = (x - j as f64) as f32;
            let a = samples[j.min(samples.len() - 1)];
            let b = samples[(j + 1).min(samples.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

/// [`parse_wav`] at the file's own rate, and the rate.
fn parse_wav_rate(bytes: &[u8]) -> Option<(Vec<f32>, u32)> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let (mut channels, mut bits, mut rate) = (0u16, 0u16, RATE);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = bytes.get(at + 8..(at + 8 + size).min(bytes.len()))?;
        if id == b"fmt " && body.len() >= 16 {
            channels = u16::from_le_bytes([body[2], body[3]]);
            rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            if bits != 16 || channels == 0 {
                return None;
            }
            let frame = usize::from(channels) * 2;
            return Some((
                body.chunks_exact(frame)
                    .map(|f| {
                        let sum: f32 = f.chunks_exact(2).map(|s| f32::from(i16::from_le_bytes([s[0], s[1]]))).sum();
                        sum / f32::from(channels) / 32768.
                    })
                    .collect(),
                rate,
            ));
        }
        at += 8 + size + (size & 1);
    }
    None
}

/// The recordings found, by name (the file name without `.wav`).
#[derive(Default)]
pub struct Grains {
    sounds: HashMap<String, Sound>,
}

impl Grains {
    pub fn load(dir: &Path) -> Self {
        let mut sounds = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav"))
                    && let (Some(name), Ok(bytes)) = (path.file_stem().and_then(|s| s.to_str()), std::fs::read(&path))
                    && let Some(samples) = parse_wav(&bytes)
                {
                    sounds.insert(name.to_ascii_lowercase(), Sound::from(samples));
                }
            }
        }
        Self { sounds }
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// The rolling recording for `surface` (`wood_ramp`, `concrete_smooth`,
    /// ...) on hard or soft wheels, or the other hardness when that is all
    /// there is.
    pub fn rolling(&self, surface: &str, hard: bool) -> Option<Sound> {
        let (first, second) = if hard { ("hard", "soft") } else { ("soft", "hard") };
        [first, second].iter().find_map(|h| self.sounds.get(&format!("{surface}_{h}")).cloned())
    }

    /// The wheels spinning on after a take-off.
    pub fn spin(&self) -> Option<Sound> {
        self.sounds.get("whls_spins_jump_1").cloned()
    }
}

struct Piece {
    sound: Sound,
    start: usize,
    at: usize,
    len: usize,
    gain: f32,
    /// Faded in and out (rolling), or just played out (the spin).
    windowed: bool,
}

/// Skate 3's recordings of what the board does, by event (a folder each in
/// `sounds\hits`: `pop`, `land_concrete`, `land_wood`, `land_metal`,
/// `grind_metal`, `grind_concrete`, `slide_wood`, `slide_concrete`, `step`),
/// a few takes each, one picked at random each time.
#[derive(Default)]
pub struct Hits {
    sets: HashMap<String, Vec<Sound>>,
    seed: std::cell::Cell<u32>,
}

impl Hits {
    pub fn load(dir: &Path) -> Self {
        let mut sets = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(event) = path.file_name().and_then(|n| n.to_str()).map(str::to_ascii_lowercase) else { continue };
                if !path.is_dir() {
                    continue;
                }
                let takes: Vec<Sound> = Grains::load(&path).sounds.into_values().collect();
                if !takes.is_empty() {
                    sets.insert(event, takes);
                }
            }
        }
        Self { sets, seed: std::cell::Cell::new(0x2545_f491) }
    }

    pub fn has(&self, event: &str) -> bool {
        self.sets.contains_key(event)
    }

    /// A take of `event`, picked at random, if there is one.
    pub fn pick(&self, event: &str) -> Option<Sound> {
        let takes = self.sets.get(event)?;
        let mut s = self.seed.get();
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        self.seed.set(s);
        takes.get(s as usize % takes.len()).cloned()
    }
}

/// Which of the board-hit materials a rolling surface is (`wood`, `metal`
/// or `concrete`; see `ground::rolling_sound_of`).
pub fn material_of(ground: Option<&str>) -> &'static str {
    match ground {
        Some(g) if g.starts_with("wood") => "wood",
        Some(g) if g.starts_with("metal") => "metal",
        _ => "concrete",
    }
}

/// Which of Skate 3's hit recordings a pop or a landing plays, as Skate 3
/// picks them (its `aud_contacts` table; `get-skate-sounds.ps1` renders
/// them): by surface kind (wood, or not), hard or soft, how many wheels came
/// down (here all four) and the strength (0 to 2). A hard landing is two
/// parts spliced, its strength's and the one below; metal rings on top.
pub mod skate_hits {
    use super::{Hits, Sound};

    /// 0 to 2 from a strength of 0 to 1.
    pub fn level(strength: f32) -> u32 {
        match strength {
            s if s < 0.34 => 0,
            s if s < 0.67 => 1,
            _ => 2,
        }
    }

    fn kind(material: &str) -> &'static str {
        if material == "wood" { "wood" } else { "concrete" }
    }

    pub fn pop(material: &str, strength: f32) -> Vec<String> {
        let set = if kind(material) == "concrete" { "hard" } else { "other" };
        vec![format!("pop_{set}_{}", level(strength))]
    }

    /// Skate's landing category from the wheels down when the board lands:
    /// 0 all four (or three), 1 two first (into a manual), 3 one wheel.
    /// Category 2 is the second truck coming down after a manual
    /// ([`second_truck`]).
    pub fn category(wheels: u8) -> u32 {
        match wheels {
            1 => 3,
            2 => 1,
            _ => 0,
        }
    }

    pub fn land(material: &str, strength: f32, category: u32) -> Vec<String> {
        let kind = kind(material);
        let level = level(strength);
        let mut out = vec![format!("land_{kind}_hard_{category}_{level}")];
        if level == 2 {
            out.push(format!("land_{kind}_hard_{category}_1"));
        }
        if material == "metal" {
            out.push("land_metal".into());
        }
        out
    }

    /// The second truck slapping down at the end of a manual (Skate's
    /// category 2), as strong as the board is fast.
    pub fn second_truck(material: &str, speed: f32) -> Vec<String> {
        land(material, (speed / 9.).clamp(0., 0.6), 2)
            .into_iter()
            .filter(|n| n != "land_metal")
            .collect()
    }

    /// A take of `name`, or of the same set a strength lower where Skate has
    /// none (its soft and wood sets leave the hardest empty).
    pub fn pick(hits: &Hits, name: &str) -> Option<Sound> {
        if let Some(s) = hits.pick(name) {
            return Some(s);
        }
        let (stem, last) = name.rsplit_once('_')?;
        let level: u32 = last.parse().ok()?;
        (0..level).rev().find_map(|l| hits.pick(&format!("{stem}_{l}")))
    }
}

/// Plays the rolling pieces and one-off recordings.
pub struct Roller {
    pieces: Vec<Piece>,
    until_next: usize,
    seed: u32,
    level: f32,
}

impl Default for Roller {
    fn default() -> Self {
        Self { pieces: Vec::new(), until_next: 0, seed: 0x9e37_79b9, level: 0. }
    }
}

impl Roller {
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    /// Plays `sound` once from its start (the wheels spinning on).
    pub fn once(&mut self, sound: Sound, gain: f32) {
        let len = sound.len();
        self.pieces.push(Piece { sound, start: 0, at: 0, len, gain, windowed: false });
    }

    /// Adds this stretch of rolling (`sound` at `speed` m/s; none or slow:
    /// it fades out) into `out`, scaled by `volume`.
    pub fn render(&mut self, out: &mut [f32], sound: Option<&Sound>, speed: f32, volume: f32) {
        let target = sound.map_or(0., |_| (speed / 4.).clamp(0., 1.) * volume);
        let mut i = 0;
        while i < out.len() {
            if self.until_next == 0 {
                self.until_next = HOP;
                if let Some(sound) = sound.filter(|s| s.len() > PIECE * 2 && speed > 0.3) {
                    let span = (sound.len() - PIECE) as f32;
                    let fraction = (speed / TOP_SPEED).clamp(0.02, 1.);
                    let jitter = (self.random() - 0.5) * 0.06;
                    let start = ((fraction + jitter).clamp(0., 1.) * span) as usize;
                    self.pieces.push(Piece { sound: Sound::clone(sound), start, at: 0, len: PIECE, gain: 1., windowed: true });
                }
            }
            let run = self.until_next.min(out.len() - i);
            for sample in &mut out[i..i + run] {
                self.level += (target - self.level) * 0.0005;
                let mut sum = 0.;
                for piece in &mut self.pieces {
                    if piece.at < piece.len {
                        let s = piece.sound.get(piece.start + piece.at).copied().unwrap_or(0.);
                        let w = if piece.windowed {
                            let x = piece.at as f32 / piece.len as f32;
                            0.5 - 0.5 * (std::f32::consts::TAU * x).cos()
                        } else {
                            piece.gain
                        };
                        sum += s * w * if piece.windowed { self.level } else { 1. };
                        piece.at += 1;
                    }
                }
                *sample += sum;
            }
            self.pieces.retain(|p| p.at < p.len);
            self.until_next -= run;
            i += run;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16], channels: u16) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend(b"RIFF");
        out.extend(((36 + data.len()) as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(channels.to_le_bytes());
        out.extend(44_100u32.to_le_bytes());
        out.extend((44_100u32 * 2 * u32::from(channels)).to_le_bytes());
        out.extend((2 * channels).to_le_bytes());
        out.extend(16u16.to_le_bytes());
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    #[test]
    fn wav_files_read_and_mix_down() {
        let mono = parse_wav(&wav(&[16384, -16384], 1)).unwrap();
        assert_eq!(mono, vec![0.5, -0.5]);
        let stereo = parse_wav(&wav(&[16384, 0, -32768, -32768], 2)).unwrap();
        assert_eq!(stereo, vec![0.25, -1.]);
        assert!(parse_wav(b"not a wav").is_none());
    }

    #[test]
    fn rolling_follows_the_speed_and_stops_with_the_board() {
        let sound: Sound = Sound::from(vec![1.; 44_100 * 4]);
        let mut roller = Roller::default();
        let mut out = vec![0.; 44_100 * 2];
        roller.render(&mut out, Some(&sound), 8., 1.);
        // The overlapping pieces add up to a steady level once faded in.
        let tail = &out[out.len() - 4410..];
        let (lo, hi) = tail.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        assert!(lo > 0.5 && hi - lo < 0.1, "steady: {lo}..{hi}");
        let mut silent = vec![0.; 44_100 * 3];
        roller.render(&mut silent, None, 0., 1.);
        assert!(silent[silent.len() - 100..].iter().all(|v| v.abs() < 0.05), "stopped: fades out");
    }

    #[test]
    fn recordings_come_to_the_play_rate_and_hits_pick_their_takes() {
        let up = resample(&[0., 1., 0., -1.], 48_000, 44_100);
        assert_eq!(up.len(), 3);
        let mut bytes = wav(&[100; 480], 1);
        bytes[24..28].copy_from_slice(&48_000u32.to_le_bytes());
        assert_eq!(parse_wav(&bytes).unwrap().len(), 441, "48 kHz brought to 44.1 kHz");
        let dir = std::env::temp_dir().join("skate-gta-hits-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pop")).unwrap();
        std::fs::write(dir.join("pop").join("a.wav"), wav(&[1000; 100], 1)).unwrap();
        let hits = Hits::load(&dir);
        assert!(hits.has("pop") && !hits.has("land_wood"));
        assert_eq!(hits.pick("pop").unwrap().len(), 100);
        assert_eq!(material_of(Some("wood_ramp")), "wood");
        assert_eq!(material_of(Some("metal_smooth")), "metal");
        assert_eq!(material_of(None), "concrete");
        // Skate's sets: a missing hardest take falls back a strength lower.
        std::fs::create_dir_all(dir.join("land_wood_hard_0_1")).unwrap();
        std::fs::write(dir.join("land_wood_hard_0_1").join("a.wav"), wav(&[1000; 50], 1)).unwrap();
        let hits = Hits::load(&dir);
        assert_eq!(skate_hits::land("wood", 1., 0), ["land_wood_hard_0_2", "land_wood_hard_0_1"]);
        assert_eq!(skate_hits::land("concrete", 0., skate_hits::category(2)), ["land_concrete_hard_1_0"]);
        assert_eq!(skate_hits::land("concrete", 0.5, skate_hits::category(1)), ["land_concrete_hard_3_1"]);
        assert_eq!(skate_hits::category(4), 0);
        assert_eq!(skate_hits::second_truck("concrete", 4.5), ["land_concrete_hard_2_1"]);
        assert_eq!(skate_hits::pick(&hits, "land_wood_hard_0_2").unwrap().len(), 50);
        assert_eq!(skate_hits::land("metal", 0., 0), ["land_concrete_hard_0_0", "land_metal"]);
        assert_eq!(skate_hits::pop("concrete", 0.5), ["pop_hard_1"]);
    }

    #[test]
    fn a_spin_plays_once() {
        let mut roller = Roller::default();
        roller.once(Sound::from(vec![0.5; 100]), 1.);
        let mut out = vec![0.; 300];
        roller.render(&mut out, None, 0., 1.);
        assert_eq!(out[50], 0.5);
        assert_eq!(out[200], 0.);
    }
}
