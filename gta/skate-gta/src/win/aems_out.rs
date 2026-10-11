//! Skate 3's own board and body sounds (SK8V's AEMS port, `crate::aems`)
//! played on the PC's sound output: each emitter's dry 48 kHz voice panned
//! and attenuated from GTA's camera, as `crate::space` does for the older
//! board sounds. SK8V plays them through GTA's own mixer instead (its C++
//! host); a stereo output of our own was the smaller step (B88).

use crate::aems;
use std::sync::Mutex;
use std::time::Duration;

const RATE: u32 = aems::SAMPLE_RATE;
const FRAMES: usize = 1024;
const BUFFERS: usize = 4;
const WAVE_MAPPER: u32 = 0xFFFF_FFFF;
const WHDR_DONE: u32 = 1;

#[repr(C)]
struct WaveFormat {
    format_tag: u16,
    channels: u16,
    samples_per_second: u32,
    bytes_per_second: u32,
    block_align: u16,
    bits_per_sample: u16,
    size: u16,
}

#[repr(C)]
struct WaveHeader {
    data: *mut i8,
    buffer_length: u32,
    bytes_recorded: u32,
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
}

/// Where the camera is and which way its right points, relative to Skate's
/// origin (the emitters' space), and how closed in it is.
#[derive(Clone, Copy, Default)]
struct Listener {
    at: [f32; 3],
    right: [f32; 3],
    closed: f32,
    gain: f32,
}

static LISTENER: Mutex<Listener> = Mutex::new(Listener { at: [0.; 3], right: [1., 0., 0.], closed: 0., gain: 1. });

/// The camera this frame, in GTA space relative to Skate's origin; `gain`
/// is the board sound volume (the options menu).
pub fn set_listener(at: [f32; 3], right: [f32; 3], closed: f32, gain: f32) {
    if let Ok(mut l) = LISTENER.lock() {
        *l = Listener { at, right, closed, gain };
    }
}

/// Whether Skate 3's audio engine is loaded and playing (the older board
/// sounds stay quiet then).
pub fn ready() -> bool {
    let mut status = aems::SvAudioStatus { size: 0, state: 0, voices: 0, underruns: 0, frames: 0, message_utf8: [0; 96] };
    aems::status(&mut status);
    status.state == aems::STATE_READY
}

/// What the audio engine reported (taken by the script into its log).
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The audio engine's notes since the last call.
pub fn take_notes() -> Vec<String> {
    NOTES.lock().map(|mut n| std::mem::take(&mut *n)).unwrap_or_default()
}

/// Loads Skate 3's audio banks from `audio` (the converter's `audio`
/// folder) and starts the output. False when the folder is missing or no
/// output opens.
pub fn start(audio: std::path::PathBuf) -> bool {
    // Once per game: a restarted Skate session keeps the loaded banks.
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if STARTED.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return true;
    }
    let log: aems::Log = std::sync::Arc::new(|line: &str| {
        if let Ok(mut n) = NOTES.lock() {
            n.push(line.to_owned());
        }
    });
    if !audio.is_dir() || !aems::configure(audio, 1.0, log) {
        return false;
    }
    std::thread::Builder::new().name("skate-gta-aems-out".into()).spawn(play).is_ok()
}

fn play() {
    let format = WaveFormat {
        format_tag: 1,
        channels: 2,
        samples_per_second: RATE,
        bytes_per_second: RATE * 4,
        block_align: 4,
        bits_per_sample: 16,
        size: 0,
    };
    let mut out = 0usize;
    if unsafe { waveOutOpen(&mut out, WAVE_MAPPER, &format, 0, 0, 0) } != 0 {
        return;
    }
    let size = size_of::<WaveHeader>() as u32;
    let mut data = vec![vec![0i16; FRAMES * 2]; BUFFERS];
    let mut headers: Vec<WaveHeader> = data
        .iter_mut()
        .map(|d| WaveHeader {
            data: d.as_mut_ptr().cast(),
            buffer_length: (FRAMES * 4) as u32,
            bytes_recorded: 0,
            user: 0,
            flags: WHDR_DONE,
            loops: 0,
            next: std::ptr::null_mut(),
            reserved: 0,
        })
        .collect();
    let mut prepared = [false; BUFFERS];
    let mut mono = vec![0f32; FRAMES];
    let mut stereo = vec![0f32; FRAMES * 2];
    loop {
        let mut wrote = false;
        for i in 0..BUFFERS {
            if unsafe { std::ptr::read_volatile(&headers[i].flags) } & WHDR_DONE == 0 {
                continue;
            }
            if prepared[i] {
                unsafe { waveOutUnprepareHeader(out, &mut headers[i], size) };
            }
            let listener = LISTENER.lock().map(|l| *l).unwrap_or_default();
            stereo.fill(0.);
            for e in 0..skate_aems::live::EMITTERS {
                let mut info = aems::SvAudioEmitter::default();
                let n = aems::pull(e, &mut mono, Some(&mut info)) as usize;
                if n == 0 {
                    continue;
                }
                let at = [info.position.x, info.position.y, info.position.z];
                let space = crate::space::Space::heard(listener.at, listener.right, at, listener.closed);
                let (left, right) = space.sides();
                for (k, v) in mono[..n].iter().enumerate() {
                    stereo[2 * k] += v * left * listener.gain;
                    stereo[2 * k + 1] += v * right * listener.gain;
                }
            }
            for (to, from) in data[i].iter_mut().zip(&stereo) {
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
}

