//! The board's sounds in GTA V: GTA plays no sound a script hands it, so
//! the plugin plays its own through Windows' wave output, mixed by Windows
//! with the game's. Rolling is Skate 3's own recording for the ground
//! rolled on (`crate::rolling`) when `get-skate-sounds.ps1` extracted them;
//! pops, landings, grinds, slides, steps and pavement seams are Skate's own
//! recordings too. Nothing is synthesised: an event without a recording
//! stays silent (the maintainer: only real sounds).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::rolling::{Grains, Hits, Roller, Sound};
use skate_sound::{Grind, Hit, Rolling, Tracker};

#[repr(C)]
struct WaveFormat {
    format: u16,
    channels: u16,
    samples_per_second: u32,
    bytes_per_second: u32,
    block_align: u16,
    bits_per_sample: u16,
    extra: u16,
}

#[repr(C)]
struct WaveHeader {
    data: *mut i16,
    length: u32,
    recorded: u32,
    user: usize,
    flags: u32,
    loops: u32,
    next: *mut WaveHeader,
    reserved: usize,
}

#[cfg_attr(windows, link(name = "winmm"))]
unsafe extern "system" {
    fn waveOutOpen(out: *mut usize, device: u32, format: *const WaveFormat, callback: usize, instance: usize, flags: u32) -> u32;
    fn waveOutPrepareHeader(out: usize, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutUnprepareHeader(out: usize, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutWrite(out: usize, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutReset(out: usize) -> u32;
    fn waveOutClose(out: usize) -> u32;
}

const WAVE_MAPPER: u32 = u32::MAX;
const WHDR_DONE: u32 = 1;
/// Samples a buffer holds (about 23 ms), and buffers queued.
const BLOCK: usize = 1024;
const BUFFERS: usize = 4;

struct Shared {
    tracker: Tracker,
    grains: Grains,
    /// Skate 3's recorded hits (pops, landings, grinds, slides, steps).
    hits: Hits,
    roller: Roller,
    /// The rolling recording now (none: not rolling, or no recording).
    rolling: Option<Sound>,
    speed: f32,
    volume: f32,
    wheels_before: u8,
    /// The grind's recording while it lasts.
    grind_take: Option<Sound>,
    /// Where the board is heard from, and how closed in.
    space: crate::space::Space,
    /// When the board last told its state: a rolling sound not renewed
    /// for a moment stops (it went on after skating ended).
    updated: std::time::Instant,
}

/// The board's sound while skating.
pub struct BoardSound {
    shared: Arc<Mutex<Shared>>,
    running: Arc<AtomicBool>,
}

impl BoardSound {
    /// Opens the sound output; none when Windows has no output for it.
    /// The rolling recordings are read from `grains` (a folder of WAV files).
    pub fn start(grains: Option<std::path::PathBuf>) -> Option<Self> {
        let shared = Arc::new(Mutex::new(Shared {
            tracker: Tracker::default(),
            grains: Grains::default(),
            hits: Hits::default(),
            roller: Roller::default(),
            rolling: None,
            grind_take: None,
            space: crate::space::Space::default(),
            speed: 0.,
            volume: 0.,
            wheels_before: 0,
            updated: std::time::Instant::now(),
        }));
        let running = Arc::new(AtomicBool::new(true));
        let (thread_shared, thread_running) = (Arc::clone(&shared), Arc::clone(&running));
        let (ready_send, ready) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("skate-gta-sound".into())
            .spawn(move || {
                // Skate's recordings (hundreds of files) load beside the
                // output: waiting for them first made the output's 2 s start
                // check fail ("no sound output"), and the board was silent.
                if let Some(dir) = grains {
                    let loader_shared = Arc::clone(&thread_shared);
                    let _ = std::thread::Builder::new().name("skate-gta-sound-load".into()).spawn(move || {
                        let loaded = Grains::load(&dir);
                        let hits = Hits::load(&dir.join("hits"));
                        if let Ok(mut shared) = loader_shared.lock() {
                            shared.grains = loaded;
                            shared.hits = hits;
                        }
                    });
                }
                play(&thread_shared, &thread_running, &ready_send)
            })
            .ok()?;
        ready.recv_timeout(Duration::from_secs(2)).ok().filter(|ok| *ok)?;
        Some(Self { shared, running })
    }

    /// What the board does this frame: Skate's state, speed and vertical
    /// speed (m/s), wheels down, surface, the frame's seconds, and the volume
    /// (0 to 1).
    /// `ground` is the rolling recording's surface (`crate::ground::rolling_sound_of`),
    /// `hard` the wheels' hardness.
    #[allow(clippy::too_many_arguments)]
    pub fn update(&self, state: &str, speed: f32, vertical: f32, wheels: u8, surface: u8, dt: f32, volume: f32, ground: Option<&str>, hard: bool) {
        let Ok(mut shared) = self.shared.lock() else { return };
        let (heard, hits): (Rolling, Vec<Hit>) = shared.tracker.update(state, speed, vertical, wheels, surface, dt);
        let volume = volume.clamp(0., 1.);
        // Off the ground fast: the wheels spin on.
        if shared.wheels_before > 0
            && heard.wheels == 0
            && speed > 2.
            && let Some(spin) = shared.grains.spin()
        {
            shared.roller.once(spin, (speed / 10.).min(1.) * volume * 0.5);
        }
        let wheels_before = shared.wheels_before;
        shared.wheels_before = heard.wheels;
        // With Skate 3's recordings there, unknown ground rolls as concrete:
        // the synthesised roll under them sounded doubled.
        let recording = shared.grains.rolling(ground.unwrap_or("concrete_smooth"), hard);
        shared.updated = std::time::Instant::now();
        let rolling = heard.wheels > 0 && matches!(heard.grind, Grind::None) && !heard.slide;
        let material = crate::rolling::material_of(ground);
        // Grinding or sliding: Skate's own recording of it, on the edge's
        // material (wood and metal ledges ring as metal under the trucks).
        let grind = match heard.grind {
            Grind::Trucks => shared.hits.pick(if material == "concrete" { "grind_concrete" } else { "grind_metal" }),
            Grind::Deck => shared.hits.pick(if material == "concrete" { "slide_concrete" } else { "slide_wood" }),
            Grind::None => None,
        };
        let grinding = !matches!(heard.grind, Grind::None);
        if grinding && grind.is_some() {
            // One take for the whole grind (picked once, kept while it lasts).
            if shared.grind_take.is_none() {
                shared.grind_take = grind;
            }
            shared.rolling = shared.grind_take.clone();
        } else {
            shared.grind_take = None;
            shared.rolling = recording.filter(|_| rolling);
        }
        shared.speed = speed;
        shared.volume = volume;
        // Out of a manual: the raised truck comes down (Skate's category 2).
        if wheels_before == 2 && heard.wheels >= 3 && !hits.iter().any(|h| matches!(h, Hit::Land(_))) && speed > 1. {
            for name in crate::rolling::skate_hits::second_truck(material, speed) {
                if let Some(sound) = crate::rolling::skate_hits::pick(&shared.hits, &name) {
                    shared.roller.once(sound, volume);
                }
            }
        }
        for hit in hits {
            // Skate 3's own recordings where there are some.
            // Pops and landings as Skate picks them (see `crate::rolling::skate_hits`);
            // their volumes are in the recordings.
            let takes: Vec<(String, f32)> = match hit {
                Hit::Pop(strength) => crate::rolling::skate_hits::pop(material, strength).into_iter().map(|n| (n, 1.)).collect(),
                Hit::Land(strength) => crate::rolling::skate_hits::land(material, strength, crate::rolling::skate_hits::category(heard.wheels)).into_iter().map(|n| (n, 1.)).collect(),
                Hit::Bail => crate::rolling::skate_hits::land(material, 1., 0).into_iter().map(|n| (n, 1.)).collect(),
                Hit::Step(strength) => vec![("step".into(), 0.4 + 0.4 * strength.clamp(0., 1.))],
                // A crack under a truck: Skate's own seam clicks (`Seams.abk`).
                // Only on paving slabs (concrete): on asphalt there are no
                // seams to click over (the maintainer, video 10-10 10-22 02:29).
                Hit::Joint(strength) if ground.is_some_and(|g| g.starts_with("concrete")) => {
                    vec![("seam".into(), 0.3 + 0.5 * strength.clamp(0., 1.))]
                }
                _ => Vec::new(),
            };
            for (name, gain) in takes {
                if let Some(sound) = crate::rolling::skate_hits::pick(&shared.hits, &name) {
                    shared.roller.once(sound, gain * volume);
                }
            }
        }
    }

    /// Where the board is heard from (see `crate::space`).
    pub fn set_space(&self, space: crate::space::Space) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.space = space;
        }
    }

    /// Skating stopped or paused: the board falls silent and starts anew.
    pub fn quiet(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.tracker.reset();
            shared.rolling = None;
        }
    }
}

impl Drop for BoardSound {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn play(shared: &Mutex<Shared>, running: &AtomicBool, ready: &std::sync::mpsc::Sender<bool>) {
    let format = WaveFormat {
        format: 1,
        channels: 2,
        samples_per_second: skate_sound::RATE,
        bytes_per_second: skate_sound::RATE * 4,
        block_align: 4,
        bits_per_sample: 16,
        extra: 0,
    };
    let mut out = 0usize;
    if unsafe { waveOutOpen(&mut out, WAVE_MAPPER, &format, 0, 0, 0) } != 0 {
        let _ = ready.send(false);
        return;
    }
    let _ = ready.send(true);
    let mut data = vec![[0i16; 2 * BLOCK]; BUFFERS];
    let mut room = crate::space::Room::default();
    let mut stereo = [0f32; 2 * BLOCK];
    let mut headers: Vec<WaveHeader> = data
        .iter_mut()
        .map(|block| WaveHeader {
            data: block.as_mut_ptr(),
            length: (BLOCK * 4) as u32,
            recorded: 0,
            user: 0,
            flags: WHDR_DONE,
            loops: 0,
            next: std::ptr::null_mut(),
            reserved: 0,
        })
        .collect();
    let size = size_of::<WaveHeader>() as u32;
    let mut prepared = [false; BUFFERS];
    let mut samples = [0f32; BLOCK];
    while running.load(Ordering::Relaxed) {
        let mut wrote = false;
        for i in 0..BUFFERS {
            // Volatile: Windows sets the done flag from its own thread.
            let flags = unsafe { std::ptr::read_volatile(&headers[i].flags) };
            if flags & WHDR_DONE == 0 {
                continue;
            }
            if prepared[i] {
                unsafe { waveOutUnprepareHeader(out, &mut headers[i], size) };
            }
            let mut space = crate::space::Space::default();
            if let Ok(mut shared) = shared.lock() {
                space = shared.space;
                samples.fill(0.);
                if shared.updated.elapsed() > Duration::from_millis(250) {
                    shared.rolling = None;
                }
                let Shared { roller, rolling, speed, volume, .. } = &mut *shared;
                roller.render(&mut samples, rolling.as_ref(), *speed, *volume);
            }
            room.render(&samples, space, &mut stereo);
            for (to, from) in data[i].iter_mut().zip(stereo) {
                *to = (from.clamp(-1., 1.) * 32000.) as i16;
            }
            headers[i].flags = 0;
            unsafe {
                waveOutPrepareHeader(out, &mut headers[i], size);
                waveOutWrite(out, &mut headers[i], size);
            }
            prepared[i] = true;
            wrote = true;
        }
        if !wrote {
            std::thread::sleep(Duration::from_millis(4));
        }
    }
    unsafe {
        waveOutReset(out);
        for (i, header) in headers.iter_mut().enumerate() {
            if prepared[i] {
                waveOutUnprepareHeader(out, header, size);
            }
        }
        waveOutClose(out);
    }
}
