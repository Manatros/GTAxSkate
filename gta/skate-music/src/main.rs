//! Skate 3's soundtrack for GTA V's Self Radio, from the player's own
//! extracted Skate 3 (Xbox 360) game folder.
//!
//!     skate-music --game <folder with default.xex> --out <User Music folder>
//!                 [--vgmstream vgmstream-cli.exe] [--ffmpeg ffmpeg.exe]
//!                 [--min-seconds 60] [--keep-temp]
//!
//! It gathers the game's streamed audio (loose files and files inside its
//! BIG archives), asks vgmstream how long each stream is, keeps the long
//! ones (songs, not sound effects), and has vgmstream decode and ffmpeg
//! encode each to an MP3 that Self Radio plays. Nothing from the game
//! leaves the player's machine.

mod big;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The songs this tool wrote, kept in its output folder.
const MADE_LIST: &str = "skate-music-files.txt";

/// Extensions of the EA audio streams vgmstream reads.
const AUDIO: [&str; 22] = [
    "snr", "sns", "snu", "sps", "mus", "mpf", "asf", "str", "xma", "xwma", "xwm", "wav", "mp3", "ogg", "ast", "abk",
    "xas", "exa", "sph", "gin", "sbr", "sbs",
];

/// Headers and the data files they play: a header (first) reads its data
/// (second) beside it under the same name.
const PAIRS: [(&str, &str); 3] = [("snr", "sns"), ("mpf", "mus"), ("sbr", "sbs")];

struct Options {
    game: PathBuf,
    out: PathBuf,
    vgmstream: PathBuf,
    ffmpeg: PathBuf,
    min_seconds: f32,
    keep_temp: bool,
}

fn usage() -> String {
    "usage: skate-music --game <Skate 3 folder> --out <folder> [--vgmstream <exe>] [--ffmpeg <exe>] [--min-seconds 60] [--keep-temp]"
        .into()
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut o = Options {
        game: PathBuf::new(),
        out: PathBuf::new(),
        vgmstream: "vgmstream-cli".into(),
        ffmpeg: "ffmpeg".into(),
        min_seconds: 60.,
        keep_temp: false,
    };
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(usage);
        match arg.as_str() {
            "--game" => o.game = value()?.into(),
            "--out" => o.out = value()?.into(),
            "--vgmstream" => o.vgmstream = value()?.into(),
            "--ffmpeg" => o.ffmpeg = value()?.into(),
            "--min-seconds" => o.min_seconds = value()?.parse().map_err(|_| usage())?,
            "--keep-temp" => o.keep_temp = true,
            _ => return Err(usage()),
        }
    }
    if o.game.as_os_str().is_empty() || o.out.as_os_str().is_empty() {
        return Err(usage());
    }
    if o.game.file_name().is_some_and(|n| n.eq_ignore_ascii_case("default.xex")) {
        o.game.pop();
    }
    Ok(o)
}

fn is_audio(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO.iter().any(|a| a.eq_ignore_ascii_case(e)))
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// A safe relative path for an archive member.
fn relative(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains(':') {
            return None;
        }
        out.push(part);
    }
    Some(out)
}

/// Copies the game's audio streams into `work`, keeping their folders so
/// paired files (an .snr header and its .sns data) stay side by side.
fn gather(game: &Path, work: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    files_under(game, &mut files);
    let mut gathered = Vec::new();
    for file in files {
        let name = file.to_string_lossy().replace('\\', "/");
        let rel = file.strip_prefix(game).unwrap_or(&file).to_path_buf();
        if is_audio(&name) {
            let target = work.join("loose").join(&rel);
            std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::copy(&file, &target).map_err(|e| format!("{}: {e}", file.display()))?;
            gathered.push(target);
        } else if name.to_ascii_lowercase().ends_with(".big") {
            let archive = match big::Archive::open(&file) {
                Ok(archive) => archive,
                Err(e) => {
                    println!("  skipped {}: {e}", rel.display());
                    continue;
                }
            };
            let stem = rel.with_extension("");
            for entry in archive.entries.iter().filter(|e| is_audio(&e.path)) {
                let Some(inner) = relative(&entry.path) else { continue };
                let target = work.join("big").join(&stem).join(inner);
                match archive.read(entry) {
                    Ok(data) => {
                        std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
                        std::fs::write(&target, data).map_err(|e| e.to_string())?;
                        gathered.push(target);
                    }
                    Err(e) => println!("  skipped {}: {e}", entry.path),
                }
            }
        }
    }
    Ok(gathered)
}

/// Data files whose header is in another archive or folder: each with
/// the headers of the same name elsewhere, closest folder first. Which one
/// belongs to it is tried with vgmstream ([`resolve_pairs`]): a header of
/// another language or version plays the data too fast or too slow.
fn pair_candidates(files: &[PathBuf]) -> Vec<(PathBuf, &'static str, Vec<PathBuf>)> {
    let key = |f: &Path| -> Option<(String, String)> {
        Some((f.file_stem()?.to_string_lossy().to_lowercase(), f.extension()?.to_string_lossy().to_lowercase()))
    };
    let mut by_name: BTreeMap<(String, String), Vec<PathBuf>> = BTreeMap::new();
    for f in files {
        if let Some(k) = key(f) {
            by_name.entry(k).or_default().push(f.clone());
        }
    }
    let mut out = Vec::new();
    for (header, data) in PAIRS {
        for ((stem, ext), places) in &by_name {
            if ext != data {
                continue;
            }
            let Some(headers) = by_name.get(&(stem.clone(), header.to_owned())) else { continue };
            for place in places {
                if place.with_extension(header).is_file() {
                    continue;
                }
                let mut ranked = headers.clone();
                ranked.sort_by_key(|h| std::cmp::Reverse(shared_folders(place, h)));
                out.push((place.clone(), header, ranked));
            }
        }
    }
    out
}

/// How many leading folders two paths share.
fn shared_folders(a: &Path, b: &Path) -> usize {
    let (a, b) = (a.parent().unwrap_or(a), b.parent().unwrap_or(b));
    a.components().zip(b.components()).take_while(|(x, y)| x == y).count()
}

/// Bytes a codec takes per sample and channel, for the codecs whose size
/// is fixed: a header that does not fit its data is the wrong one.
fn bytes_per_sample(encoding: &str) -> Option<f32> {
    let e = encoding.to_ascii_lowercase();
    if e.contains("xas") {
        Some(76. / 128.)
    } else if e.contains("ea-xa") || e.contains("ea xa") {
        Some(15. / 28.)
    } else if e.contains("16-bit") || e.contains("pcm16") {
        Some(2.)
    } else if e.contains("8-bit") || e.contains("pcm8") {
        Some(1.)
    } else {
        None
    }
}

/// Whether `info` (read through a header) fits `data_bytes` of data.
fn fits(info: &Info, data_bytes: u64) -> bool {
    if info.samples <= 0. || info.channels == 0 || data_bytes == 0 {
        return false;
    }
    let per = data_bytes as f32 / (info.samples * f32::from(info.channels));
    match info.encoding.as_deref().and_then(bytes_per_sample) {
        Some(want) => (per / want - 1.).abs() < 0.2,
        // Codecs of varying size (XMA, MP3...): only sizes no codec has.
        None => (0.03..4.).contains(&per),
    }
}

/// Puts beside each data file the header of the same name that fits it
/// (see [`pair_candidates`]); returns the headers placed.
fn resolve_pairs(vgmstream: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut placed = Vec::new();
    let mut unfit = 0;
    for (data, header, candidates) in pair_candidates(files) {
        let beside = data.with_extension(header);
        let size = std::fs::metadata(&data).map_or(0, |m| m.len());
        let mut found = false;
        for candidate in candidates.iter().take(8) {
            if std::fs::copy(candidate, &beside).is_err() {
                continue;
            }
            if describe(vgmstream, &beside, 1).is_some_and(|info| fits(&info, size)) {
                found = true;
                break;
            }
            let _ = std::fs::remove_file(&beside);
        }
        if found {
            placed.push(beside);
        } else {
            unfit += 1;
        }
    }
    if !placed.is_empty() || unfit > 0 {
        println!("Paired {} stream(s) with their headers from other archives ({unfit} had none that fits)", placed.len());
    }
    placed
}

/// A stream as vgmstream describes it (`-m`).
#[derive(Debug, Default, PartialEq)]
struct Info {
    seconds: f32,
    streams: u32,
    name: Option<String>,
    /// Samples per channel, channels and the codec, as the header says.
    samples: f32,
    channels: u16,
    encoding: Option<String>,
}

fn parse_info(text: &str) -> Option<Info> {
    let mut rate = None;
    let mut samples = None;
    let mut info = Info { streams: 1, ..Default::default() };
    for line in text.lines() {
        let line = line.trim();
        let value = |prefix: &str| line.strip_prefix(prefix).map(str::trim);
        if let Some(v) = value("sample rate:") {
            rate = v.split_whitespace().next()?.parse::<f32>().ok();
        } else if let Some(v) = value("stream total samples:") {
            info.samples = v.split_whitespace().next().and_then(|n| n.parse::<f32>().ok()).unwrap_or(0.);
            samples = samples.or(Some(info.samples));
        } else if let Some(v) = value("play duration:") {
            samples = v.split_whitespace().next().and_then(|n| n.parse::<f32>().ok()).or(samples);
        } else if let Some(v) = value("channels:") {
            info.channels = v.split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0);
        } else if let Some(v) = value("encoding:") {
            info.encoding = Some(v.to_owned()).filter(|e| !e.is_empty());
        } else if let Some(v) = value("stream count:") {
            info.streams = v.parse().unwrap_or(1);
        } else if let Some(v) = value("stream name:") {
            info.name = Some(v.to_owned()).filter(|n| !n.is_empty());
        }
    }
    info.seconds = samples? / rate.filter(|r| *r > 0.)?;
    Some(info)
}

fn describe(vgmstream: &Path, file: &Path, stream: u32) -> Option<Info> {
    let output = Command::new(vgmstream).args(["-m", "-s", &stream.to_string()]).arg(file).output().ok()?;
    output.status.success().then(|| parse_info(&String::from_utf8_lossy(&output.stdout)))?
}

/// "music_licensed_some_track" to "Some Track".
fn title(name: &str) -> String {
    let words: Vec<String> = name
        .split(['_', '-', ' ', '.'])
        .filter(|w| !w.is_empty())
        .filter(|w| !["music", "mus", "licensed", "track", "stream", "snr", "sns"].contains(&w.to_ascii_lowercase().as_str()))
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        })
        .collect();
    if words.is_empty() { name.to_owned() } else { words.join(" ") }
}

/// Words of streams that are the world's sound, not music: ambience beds,
/// machines, crowds, water, the fake TV and radio programmes.
const NOT_MUSIC: [&str; 40] = [
    "amb", "ambience", "ambient", "atmos", "interior", "exterior", "loop", "sfx", "fx", "program", "sermon", "fountain",
    "fountains", "flow", "tank", "rustle", "flap", "flapping", "lawnmower", "leafblower", "weedwhacker", "trimmer",
    "elevator", "laundry", "sports", "crowd", "party", "convention", "pub", "restaurant", "movie", "symphony",
    "plant", "trtmnt", "waterlaps", "wind", "traffic", "tunnel", "liq", "machine",
];

/// Districts and places of San Vanelona: their ambience beds share the
/// songs' length.
const PLACES: [&str; 14] = [
    "indu", "quarry", "reclaimed", "spillway", "space", "school", "factory", "res", "downtown", "uni", "harbour",
    "park", "arena", "mall",
];

/// Whether a stream (its name and where it was found) is the world's sound
/// rather than a song.
fn not_music(name: &str, path: &str) -> bool {
    let words: Vec<String> = format!("{name} {path}")
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    // Path words that mean music keep a stream, whatever its name.
    if words.iter().any(|w| ["music", "song", "songs", "soundtrack", "licensed"].contains(&w.as_str())) {
        return false;
    }
    has(&NOT_MUSIC) || has(&PLACES)
}

fn file_name(text: &str) -> String {
    text.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect()
}

fn run() -> Result<(), String> {
    let o = options()?;
    if !o.game.join("default.xex").is_file() && !o.game.join("data").is_dir() {
        return Err(format!("{} is not an extracted Skate 3 folder (no default.xex)", o.game.display()));
    }
    for (tool, what) in [(&o.vgmstream, "vgmstream-cli"), (&o.ffmpeg, "ffmpeg")] {
        if Command::new(tool).arg("-h").output().is_err() && Command::new(tool).arg("-version").output().is_err() {
            return Err(format!("{what} not found ({}); see get-skate-music.ps1", tool.display()));
        }
    }
    let work = std::env::temp_dir().join("skate-music-work");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&o.out).map_err(|e| e.to_string())?;
    println!("Gathering Skate 3's audio streams...");
    let files = gather(&o.game, &work)?;
    let mut files = files;
    let placed = resolve_pairs(&o.vgmstream, &files);
    files.extend(placed);
    println!("{} audio files; finding the songs (at least {:.0} seconds)...", files.len(), o.min_seconds);
    // An .sns holds the data of the .snr beside it: vgmstream reads the pair
    // through the .snr.
    let headers: Vec<&PathBuf> = files
        .iter()
        .filter(|f| {
            !PAIRS.iter().any(|(header, data)| {
                f.extension().is_some_and(|e| e.eq_ignore_ascii_case(data)) && f.with_extension(header).is_file()
            })
        })
        .collect();
    let mut songs: BTreeMap<String, (PathBuf, u32, f32)> = BTreeMap::new();
    let (mut readable, mut unreadable) = (0usize, 0usize);
    let mut unread_kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut longest: Vec<(f32, String)> = Vec::new();
    // Long streams that are not songs, listed for the record.
    let mut skipped: Vec<String> = Vec::new();
    for file in headers {
        let Some(first) = describe(&o.vgmstream, file, 1) else {
            unreadable += 1;
            let kind = file.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            *unread_kinds.entry(kind).or_default() += 1;
            continue;
        };
        readable += 1;
        longest.push((first.seconds, file.strip_prefix(&work).unwrap_or(file).display().to_string()));
        for stream in 1..=first.streams.min(500) {
            let info = if stream == 1 { Some(first_copy(&first)) } else { describe(&o.vgmstream, file, stream) };
            let Some(info) = info else { continue };
            if info.seconds < o.min_seconds {
                continue;
            }
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let mut name = title(info.name.as_deref().unwrap_or(&stem));
            if first.streams > 1 && info.name.is_none() {
                name = format!("{name} {stream}");
            }
            let source = file.strip_prefix(&work).unwrap_or(file).display().to_string();
            if not_music(&name, &source) {
                skipped.push(format!("{name}  ({source})"));
                continue;
            }
            let mut unique = name.clone();
            let mut n = 2;
            while songs.contains_key(&unique) {
                unique = format!("{name} ({n})");
                n += 1;
            }
            songs.insert(unique, (file.clone(), stream, info.seconds));
        }
    }
    // What was found, so a run that finds no songs says why.
    longest.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("vgmstream read {readable} of {} files ({unreadable} not readable). Longest:", readable + unreadable);
    for (seconds, name) in longest.iter().take(8) {
        println!("  {seconds:7.1} s  {name}");
    }
    if !unread_kinds.is_empty() {
        let kinds: Vec<String> = unread_kinds.iter().map(|(k, n)| format!(".{k} {n}")).collect();
        println!("Not readable, by kind: {}", kinds.join(", "));
    }
    if !skipped.is_empty() {
        println!("{} long streams are the world's sound, not songs: left out (listed in skate-music-left-out.txt).", skipped.len());
        let _ = std::fs::write(o.out.join("skate-music-left-out.txt"), skipped.join("\n"));
    }
    if songs.is_empty() {
        return Err(if readable == 0 {
            "no songs found: vgmstream could not read any of the files. Update vgmstream (delete the tools folder beside the script and run it again)".into()
        } else {
            format!("no songs found at {:.0} seconds or longer; the longest are listed above. Try --min-seconds 30", o.min_seconds)
        });
    }
    println!("{} songs. Converting to MP3 for Self Radio...", songs.len());
    // What an earlier run wrote goes first: a song once decoded wrong (too
    // fast or slow) is not kept. Only this tool's files: those it listed,
    // or, before it kept a list, the MP3s of its own "Skate 3" folder.
    let list = o.out.join(MADE_LIST);
    let earlier: Vec<PathBuf> = match std::fs::read_to_string(&list) {
        Ok(text) => text.lines().filter(|l| !l.trim().is_empty()).map(|l| o.out.join(l.trim())).collect(),
        Err(_) if o.out.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Skate 3")) => std::fs::read_dir(&o.out)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")))
            .collect(),
        Err(_) => Vec::new(),
    };
    for old in &earlier {
        let _ = std::fs::remove_file(old);
    }
    let mut written = Vec::new();
    let mut made = 0;
    for (index, (name, (file, stream, seconds))) in songs.iter().enumerate() {
        let wav = work.join(format!("song{index}.wav"));
        let mp3 = o.out.join(format!("{}.mp3", file_name(name)));
        let decoded = Command::new(&o.vgmstream)
            .args(["-s", &stream.to_string(), "-o"])
            .arg(&wav)
            .arg(file)
            .output()
            .is_ok_and(|out| out.status.success());
        if !decoded {
            println!("  could not decode {name}");
            continue;
        }
        let encoded = Command::new(&o.ffmpeg)
            .args(["-y", "-loglevel", "error", "-i"])
            .arg(&wav)
            // Self Radio plays at 44.1 kHz stereo: anything else is
            // resampled here rather than played at the wrong speed.
            .args(["-ar", "44100", "-ac", "2", "-codec:a", "libmp3lame", "-q:a", "2", "-metadata"])
            .arg(format!("title={name}"))
            .args(["-metadata", "album=Skate 3", "-metadata"])
            .arg(format!("track={}", index + 1))
            .arg(&mp3)
            .output()
            .is_ok_and(|out| out.status.success());
        let _ = std::fs::remove_file(&wav);
        if encoded {
            made += 1;
            written.push(mp3.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            println!("  {name} ({:.0}:{:02.0})", (seconds / 60.).floor(), seconds % 60.);
        } else {
            println!("  could not encode {name}");
        }
    }
    if !o.keep_temp {
        let _ = std::fs::remove_dir_all(&work);
    }
    let _ = std::fs::write(&list, written.join("\n"));
    println!("Done: {made} songs in {}", o.out.display());
    println!("In GTA V: Settings > Audio > Self Radio, run a Full Scan once.");
    Ok(())
}

fn first_copy(info: &Info) -> Info {
    Info {
        seconds: info.seconds,
        streams: info.streams,
        name: info.name.clone(),
        samples: info.samples,
        channels: info.channels,
        encoding: info.encoding.clone(),
    }
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
    fn ambience_is_left_out_and_music_kept() {
        for name in ["Interior Arena Amb", "Lawnmower 2", "TV Program 1", "13 Indu Quarry", "Liq Flow Lrg Thin", "Fountains Waterlaps Left"] {
            assert!(not_music(name, "audio/streams/x.snr"), "{name}");
        }
        assert!(!not_music("Bad Brains Right Brigade", "audio/x.snr"));
        assert!(!not_music("Track 7", "data/music/licensed/7.snr"), "a music folder keeps it");
    }

    #[test]
    fn reads_vgmstream_metadata() {
        let text = "metadata for song.snr\nsample rate: 48000 Hz\nchannels: 2\nstream total samples: 8640000 (3:00.000 seconds)\nplay duration: 8640000 samples (3:00.000 seconds)\nencoding: EA EA-XAS v1\nstream count: 1\n";
        let info = parse_info(text).unwrap();
        assert_eq!(info.seconds, 180.);
        assert_eq!(info.streams, 1);
        let bank = "sample rate: 32000 Hz\nplay duration: 16000 samples (0:00.500 seconds)\nstream count: 40\nstream name: door_slam\n";
        let info = parse_info(bank).unwrap();
        assert!(info.seconds < 1.);
        assert_eq!(info.streams, 40);
        assert_eq!(info.name.as_deref(), Some("door_slam"));
        assert!(parse_info("not a stream").is_none());
    }

    #[test]
    fn names_tracks_and_files() {
        assert_eq!(title("music_licensed_ace_of_spades"), "Ace Of Spades");
        assert_eq!(title("trk01"), "Trk01");
        assert_eq!(file_name("AC/DC: Live?"), "AC_DC_ Live_");
        assert!(is_audio("data/audio/music/x.SNR"));
        assert!(!is_audio("data/content/x.big"));
        assert_eq!(relative("a/../b"), None);
        assert_eq!(relative("a/b.snr"), Some(PathBuf::from("a").join("b.snr")));
    }

    #[test]
    fn headers_elsewhere_are_ranked_and_must_fit_their_data() {
        let dir = std::env::temp_dir().join(format!("skate-music-pair-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let data = dir.join("big").join("music").join("streams").join("Track01.sns");
        let near = dir.join("big").join("music").join("headers").join("track01.snr");
        let far = dir.join("big").join("fr").join("track01.snr");
        for f in [&data, &near, &far] {
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b"x").unwrap();
        }
        let pairs = pair_candidates(&[data.clone(), far.clone(), near.clone()]);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].2, vec![near.clone(), far.clone()], "the closer folder first");
        let _ = std::fs::remove_dir_all(&dir);
        // 3 minutes of 48 kHz stereo EA-XAS is about 10 MB; read through a
        // header of another version (half the samples) it does not fit.
        let text = "sample rate: 48000 Hz\nchannels: 2\nstream total samples: 8640000 (3:00.000 seconds)\nencoding: EA EA-XAS v1\n";
        let info = parse_info(text).unwrap();
        assert_eq!((info.channels, info.samples), (2, 8_640_000.));
        let bytes = (8_640_000. * 2. * 76. / 128.) as u64;
        assert!(fits(&info, bytes));
        assert!(!fits(&info, bytes / 2));
        assert!(!fits(&info, bytes * 2));
    }
}
