//! Skate 3's soundtrack for GTA V's Self Radio, from the player's own
//! extracted Skate 3 (Xbox 360) game folder.
//!
//!     skate-music --game <folder with default.xex> --out <User Music folder>
//!                 [--vgmstream vgmstream-cli.exe] [--keep-temp]
//!
//! The soundtrack is the in-game iPod's: `data/audio/music/ipod.mpf` (EA's
//! interactive music header, MPF v5, big-endian) with its data in
//! `Ipod_Stream.mus`. The MPF cuts each song into 30 equal sections; its
//! sample table lists every section (offset, the song's length) in play
//! order, so the sections of one song are a run of equal lengths.
//! vgmstream decodes each song's sections in one go (a TXTP file lists
//! them) and LAME, built in, writes the MP3 Self Radio plays. Nothing from
//! the game leaves the player's machine.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The songs this tool wrote, kept in its output folder.
const MADE_LIST: &str = "skate-music-files.txt";

struct Options {
    game: PathBuf,
    out: PathBuf,
    vgmstream: PathBuf,
    keep_temp: bool,
}

fn usage() -> String {
    "usage: skate-music --game <Skate 3 folder> --out <folder> [--vgmstream <exe>] [--keep-temp]".into()
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut o = Options { game: PathBuf::new(), out: PathBuf::new(), vgmstream: "vgmstream-cli".into(), keep_temp: false };
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(usage);
        match arg.as_str() {
            "--game" => o.game = value()?.into(),
            "--out" => o.out = value()?.into(),
            "--vgmstream" => o.vgmstream = value()?.into(),
            "--keep-temp" => o.keep_temp = true,
            // Older installers passed these; LAME is built in now.
            "--ffmpeg" | "--min-seconds" => {
                value()?;
            }
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

/// `name` in `dir`, whatever its case (a disc copied by other tools may
/// change it).
fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name)))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, String> {
    data.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "ipod.mpf is cut short".into())
}

/// The songs of an MPF v5 file: each a range of section numbers (0-based,
/// as listed in its sample table) and its length in seconds.
fn songs(mpf: &[u8]) -> Result<Vec<(std::ops::Range<usize>, f32)>, String> {
    if mpf.get(..4) != Some(b"PFDx".as_slice()) || mpf.get(4) != Some(&5) {
        return Err("ipod.mpf is not an EA MPF v5 file".into());
    }
    let (start, end) = (u32_at(mpf, 0x34)? as usize, u32_at(mpf, 0x38)? as usize);
    if end <= start || end > mpf.len() || (end - start) % 8 != 0 {
        return Err("ipod.mpf has no sample table".into());
    }
    let lengths: Vec<u32> = (0..(end - start) / 8).map(|i| u32_at(mpf, start + i * 8 + 4)).collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    let mut first = 0;
    for i in 1..=lengths.len() {
        if i == lengths.len() || lengths[i] != lengths[first] {
            // The length is counted in 30 ms steps.
            out.push((first..i, lengths[first] as f32 * 0.03));
            first = i;
        }
    }
    Ok(out)
}

/// 16-bit PCM from a WAV file: sample rate, channels, interleaved samples.
fn read_wav(path: &Path) -> Result<(u32, u16, Vec<i16>), String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if data.get(..4) != Some(b"RIFF".as_slice()) || data.get(8..12) != Some(b"WAVE".as_slice()) {
        return Err("vgmstream wrote no WAV".into());
    }
    let le32 = |at: usize| data.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let le16 = |at: usize| data.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let riff_end = 8usize.checked_add(le32(4).ok_or("WAV without RIFF size")? as usize)
        .filter(|&end| end <= data.len() && end >= 12).ok_or("WAV RIFF bounds are invalid")?;
    let (mut at, mut format) = (12usize, None);
    while at < riff_end {
        if riff_end - at < 8 { return Err("WAV has a truncated chunk header".into()); }
        let id = &data[at..at + 4];
        let size = le32(at + 4).ok_or("WAV has a truncated chunk header")? as usize;
        let body = at + 8;
        let end = body.checked_add(size).filter(|&end| end <= riff_end).ok_or("WAV chunk exceeds RIFF bounds")?;
        match id {
            b"fmt " => {
                if size < 16 { return Err("WAV format chunk is too short".into()); }
                let encoding = le16(body).ok_or("WAV format chunk is truncated")?;
                let channels = le16(body + 2).ok_or("WAV format chunk is truncated")?;
                let rate = le32(body + 4).ok_or("WAV format chunk is truncated")?;
                let align = le16(body + 12).ok_or("WAV format chunk is truncated")?;
                let bits = le16(body + 14).ok_or("WAV format chunk is truncated")?;
                if encoding != 1 { return Err(format!("WAV format is {encoding}, expected PCM (1)")); }
                if !(1..=2).contains(&channels) { return Err(format!("WAV has {channels} channels, expected 1 or 2")); }
                if bits != 16 { return Err(format!("WAV is {bits}-bit, expected 16-bit PCM")); }
                if rate == 0 || align != channels * 2 { return Err("WAV has invalid sample rate or block alignment".into()); }
                format = Some((channels, rate, bits, align));
            }
            b"data" => {
                let (channels, rate, _, align) = format.ok_or("WAV without format")?;
                if size % usize::from(align) != 0 { return Err("WAV data ends with an incomplete sample frame".into()); }
                let samples = data[body..end].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
                return Ok((rate, channels, samples));
            }
            _ => {}
        }
        at = end.checked_add(size & 1).filter(|&next| next <= riff_end).ok_or("WAV chunk padding exceeds RIFF bounds")?;
    }
    Err("WAV without data".into())
}

fn safe_manifest_entries(text: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for raw in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let path = Path::new(raw);
        if path.components().count() != 1 || path.file_name().is_none_or(|n| n != raw)
            || raw == "." || raw == ".." || raw.contains('/') || raw.contains('\\')
        {
            return Err(format!("unsafe entry in {MADE_LIST}: {raw:?}"));
        }
        names.push(raw.to_owned());
    }
    Ok(names)
}

fn legacy_mp3_files(out: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(out).map_err(|e| format!("read {}: {e}", out.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", out.display()))?;
        let kind = entry.file_type().map_err(|e| format!("inspect {}: {e}", entry.path().display()))?;
        // An MP3-looking directory or link belongs to the user, not this tool.
        if !kind.is_file() { continue; }
        if let Ok(name) = entry.file_name().into_string() {
            if Path::new(&name).extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")) {
                names.push(name);
            }
        }
    }
    Ok(names)
}

fn promote(out: &Path, stage: &Path, earlier: &[String], names: &[String]) -> Result<(), String> {
    let backup = stage.join("previous");
    std::fs::create_dir(&backup).map_err(|e| e.to_string())?;
    let manifest = out.join(MADE_LIST);
    let old_manifest = match std::fs::read(&manifest) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("read {} before promotion: {error}", manifest.display())),
    };
    let mut moved_old: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut moved_new: Vec<PathBuf> = Vec::new();
    let old_paths: Vec<PathBuf> = earlier.iter().map(|n| out.join(n)).collect();
    let operation = (|| -> Result<(), String> {
        for (i, src) in old_paths.iter().enumerate() {
            let kind = match std::fs::symlink_metadata(src) {
                Ok(metadata) => metadata.file_type(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("inspect {} before backup: {error}", src.display())),
            };
            if !kind.is_file() {
                return Err(format!("managed soundtrack entry is not a regular file: {}", src.display()));
            }
            let dst = backup.join(i.to_string());
            std::fs::rename(src, &dst).map_err(|e| format!("backup {}: {e}", src.display()))?;
            moved_old.push((src.clone(), dst));
        }
        for name in names {
            let src = stage.join(name);
            let dst = out.join(name);
            std::fs::rename(&src, &dst).map_err(|e| format!("install {}: {e}", dst.display()))?;
            moved_new.push(dst);
        }
        std::fs::write(&manifest, names.join("\n")).map_err(|e| format!("write {}: {e}", manifest.display()))?;
        Ok(())
    })();
    if let Err(error) = operation {
        let mut restore_errors: Vec<String> = moved_new.iter().rev().filter_map(|path|
            std::fs::remove_file(path).err().map(|e| format!("remove {}: {e}", path.display()))
        ).collect();
        match old_manifest.as_deref() {
            Some(contents) => if let Err(e) = std::fs::write(&manifest, contents) {
                restore_errors.push(format!("restore {}: {e}", manifest.display()));
            },
            None => if let Err(e) = std::fs::remove_file(&manifest) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    restore_errors.push(format!("remove partial {}: {e}", manifest.display()));
                }
            },
        }
        restore_errors.extend(moved_old.iter().rev().filter_map(|(src, saved)|
            std::fs::rename(saved, src).err().map(|e| format!("restore {}: {e}", src.display()))
        ));
        return Err(if restore_errors.is_empty() {
            format!("soundtrack promotion failed; previous files restored: {error}")
        } else {
            format!("soundtrack promotion failed: {error}; restore errors: {}", restore_errors.join("; "))
        });
    }
    Ok(())
}

/// An MP3 of the samples (192 kbps, LAME's -q 2, ID3 tags).
fn mp3(rate: u32, channels: u16, samples: &[i16], title: &str, track: usize) -> Result<Vec<u8>, String> {
    use mp3lame_encoder::{Bitrate, Builder, FlushNoGap, Id3Tag, InterleavedPcm, MonoPcm, Quality};
    let mut b = Builder::new().ok_or("LAME did not start")?;
    b.set_num_channels(channels as u8).map_err(|e| format!("LAME channels: {e:?}"))?;
    b.set_sample_rate(rate).map_err(|e| format!("LAME rate: {e:?}"))?;
    b.set_brate(Bitrate::Kbps192).map_err(|e| format!("LAME bitrate: {e:?}"))?;
    b.set_quality(Quality::NearBest).map_err(|e| format!("LAME quality: {e:?}"))?;
    let comment = format!("Track {track}");
    b.set_id3_tag(Id3Tag { title: title.as_bytes(), artist: b"Skate 3", album: b"Skate 3", album_art: &[], year: b"2010", comment: comment.as_bytes() })
        .map_err(|e| format!("ID3: {e:?}"))?;
    let mut encoder = b.build().map_err(|e| format!("LAME: {e:?}"))?;
    let mut out = Vec::new();
    for chunk in samples.chunks(1 << 20) {
        let frames = chunk.len() / usize::from(channels);
        out.reserve(mp3lame_encoder::max_required_buffer_size(frames));
        let written = if channels == 1 {
            encoder.encode_to_vec(MonoPcm(chunk), &mut out)
        } else {
            encoder.encode_to_vec(InterleavedPcm(chunk), &mut out)
        };
        written.map_err(|e| format!("LAME: {e:?}"))?;
    }
    out.reserve(7200);
    encoder.flush_to_vec::<FlushNoGap>(&mut out).map_err(|e| format!("LAME: {e:?}"))?;
    Ok(out)
}

fn run() -> Result<(), String> {
    let o = options()?;
    let music = find(&o.game, "data").and_then(|d| find(&d, "audio")).and_then(|d| find(&d, "music"));
    let mpf = music.as_deref().and_then(|d| find(d, "ipod.mpf"));
    let mus = music.as_deref().and_then(|d| find(d, "Ipod_Stream.mus"));
    let (Some(mpf), Some(_)) = (mpf, mus) else {
        return Err(format!("{} has no data\\audio\\music\\ipod.mpf and Ipod_Stream.mus (the soundtrack)", o.game.display()));
    };
    if Command::new(&o.vgmstream).arg("-h").output().is_err() {
        return Err(format!("vgmstream-cli not found ({})", o.vgmstream.display()));
    }
    let list = songs(&std::fs::read(&mpf).map_err(|e| format!("{}: {e}", mpf.display()))?)?;
    if list.is_empty() {
        return Err("ipod.mpf lists no songs".into());
    }
    std::fs::create_dir_all(&o.out).map_err(|e| format!("{}: {e}", o.out.display()))?;
    // What an earlier run wrote goes first (older versions also wrote the
    // world's ambience). Only this tool's files: those it listed or, before
    // it kept a list, the MP3s of its own "Skate 3" folder.
    let made = o.out.join(MADE_LIST);
    let earlier: Vec<String> = match std::fs::read_to_string(&made) {
        Ok(text) => safe_manifest_entries(&text)?,
        Err(_) if o.out.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Skate 3")) => legacy_mp3_files(&o.out)?,
        Err(_) => Vec::new(),
    };
    // Keep staging on the output volume: Windows rename promotion cannot cross volumes.
    let work = o.out.join(format!(".skate-music-staging-{}", std::process::id()));
    if work.exists() { return Err(format!("staging directory already exists: {}", work.display())); }
    std::fs::create_dir(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    println!("{} songs in Skate 3's soundtrack. Converting to MP3 for Self Radio...", list.len());
    let mpf_text = mpf.to_string_lossy().into_owned();
    let convert = |index: usize| -> Result<String, String> {
        let (sections, _) = &list[index];
        let track = index + 1;        let title = format!("Skate 3 Track {track:02}");
        let txtp = work.join(format!("song{track}.txtp"));
        let wav = work.join(format!("song{track}.wav"));
        // vgmstream numbers the sections from 1.
        let lines: Vec<String> = sections.clone().map(|s| format!("{mpf_text}#{}", s + 1)).collect();
        let result = std::fs::write(&txtp, lines.join("\r\n"))
            .map_err(|e| e.to_string())
            .and_then(|()| {
                let out = Command::new(&o.vgmstream).arg("-o").arg(&wav).arg(&txtp).output().map_err(|e| e.to_string())?;
                if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_owned()) }
            })
            .and_then(|()| read_wav(&wav))
            .and_then(|(rate, channels, samples)| mp3(rate, channels, &samples, &title, track))
            .and_then(|bytes| {
                let name = format!("{title}.mp3");
                std::fs::write(work.join(&name), bytes).map_err(|e| e.to_string())?;
                Ok(name)
            });
        let _ = std::fs::remove_file(&wav);
        result
    };
    // Several songs at once (LAME is the slow part); about 1 GB of WAV at most.
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(1, 6);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<String, String>>>> = list.iter().map(|_| Default::default()).collect();
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if index >= list.len() {
                    break;
                }
                let result = convert(index);
                match &result {
                    Ok(_) => {
                        let seconds = list[index].1;
                        println!("  Skate 3 Track {:02} ({:.0}:{:02.0})", index + 1, (seconds / 60.).floor(), seconds % 60.);
                    }
                    Err(e) => println!("  Skate 3 Track {:02}: {e}", index + 1),
                }
                *results[index].lock().unwrap() = Some(result);
            });
        }
    });
    let mut written = Vec::new();
    let mut failed = 0;
    for result in &results {
        match result.lock().unwrap().take() {
            Some(Ok(name)) => written.push(name),
            _ => failed += 1,
        }
    }
    if failed > 0 {
        // Errors are printed as each worker finishes; return a useful summary too.
        let _ = std::fs::remove_dir_all(&work);
        return Err(format!("{failed} of {} tracks failed; see per-track errors above. Previous soundtrack preserved.", list.len()));
    }
    promote(&o.out, &work, &earlier, &written)?;
    let _ = std::fs::remove_file(o.out.join("skate-music-left-out.txt"));
    if !o.keep_temp { let _ = std::fs::remove_dir_all(&work); }
    println!("Done: {} songs in {}", written.len(), o.out.display());
    Ok(())
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

    /// An MPF v5 with a sample table of the given song lengths (each song
    /// `sections` long).
    fn mpf(lengths: &[u32], sections: usize) -> Vec<u8> {
        let mut data = vec![0u8; 0x40];
        data[..4].copy_from_slice(b"PFDx");
        data[4] = 5;
        let start = data.len() as u32;
        for (i, length) in lengths.iter().flat_map(|l| std::iter::repeat_n(*l, sections)).enumerate() {
            data.extend_from_slice(&(i as u32 * 0x100).to_be_bytes());
            data.extend_from_slice(&length.to_be_bytes());
        }
        let end = data.len() as u32;
        data[0x34..0x38].copy_from_slice(&start.to_be_bytes());
        data[0x38..0x3C].copy_from_slice(&end.to_be_bytes());
        data
    }

    #[test]
    fn songs_are_runs_of_one_length() {
        // Skate 3's first two songs: 4:45 and 1:49 (lengths in 30 ms steps).
        let list = songs(&mpf(&[0x2520, 0x0E3C, 0x2520], 30)).unwrap();
        assert_eq!(list.len(), 3, "the same length again is another song");
        assert_eq!(list[0].0, 0..30);
        assert_eq!(list[1].0, 30..60);
        assert_eq!(list[2].0, 60..90);
        assert!((list[0].1 - 285.1).abs() < 0.1);
        assert!(songs(b"PFDx\x04").is_err(), "only version 5");
        assert!(songs(&mpf(&[], 30)).is_err(), "no sample table");
    }

    #[test]
    fn manifest_accepts_only_plain_filenames() {
        assert_eq!(safe_manifest_entries("one.mp3\ntwo.mp3\n").unwrap(), ["one.mp3", "two.mp3"]);
        for unsafe_entry in ["../outside.mp3", "sub/file.mp3", "sub\\file.mp3", "C:\\outside.mp3", ".", ".."] {
            assert!(safe_manifest_entries(unsafe_entry).is_err(), "accepted {unsafe_entry:?}");
        }
    }

    #[test]
    fn legacy_migration_preserves_mp3_named_directories() {
        let root = std::env::current_dir().unwrap().join(format!(".skate-music-legacy-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("notes.mp3")).unwrap();
        std::fs::write(root.join("notes.mp3").join("keep.txt"), b"user notes").unwrap();
        std::fs::write(root.join("track.MP3"), b"old track").unwrap();
        std::fs::write(root.join("other.txt"), b"unrelated file").unwrap();
        let stage = root.join("stage");
        std::fs::create_dir(&stage).unwrap();
        std::fs::write(stage.join("new.mp3"), b"new track").unwrap();
        let earlier = legacy_mp3_files(&root).unwrap();
        promote(&root, &stage, &earlier, &["new.mp3".into()]).unwrap();
        assert_eq!(std::fs::read(root.join("notes.mp3").join("keep.txt")).unwrap(), b"user notes");
        assert_eq!(std::fs::read(root.join("other.txt")).unwrap(), b"unrelated file");
        assert!(!root.join("track.MP3").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promotion_refuses_a_manifest_entry_replaced_with_a_directory() {
        let root = std::env::current_dir().unwrap().join(format!(".skate-music-manifest-dir-test-{}", std::process::id()));
        let stage = root.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::create_dir(root.join("old.mp3")).unwrap();
        std::fs::write(root.join("old.mp3").join("keep.txt"), b"user notes").unwrap();
        std::fs::write(root.join(MADE_LIST), "old.mp3").unwrap();
        std::fs::write(stage.join("new.mp3"), b"new soundtrack").unwrap();
        assert!(promote(&root, &stage, &["old.mp3".into()], &["new.mp3".into()]).is_err());
        assert_eq!(std::fs::read(root.join("old.mp3").join("keep.txt")).unwrap(), b"user notes");
        assert_eq!(std::fs::read_to_string(root.join(MADE_LIST)).unwrap(), "old.mp3");
        assert!(!root.join("new.mp3").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_promotion_restores_previous_soundtrack_and_manifest() {
        let root = std::env::current_dir().unwrap().join(format!(".skate-music-promote-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let out = root.join("out");
        let stage = out.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(out.join("old.mp3"), b"old soundtrack").unwrap();
        std::fs::write(out.join(MADE_LIST), "old.mp3").unwrap();
        std::fs::write(stage.join("new.mp3"), b"new soundtrack").unwrap();

        let error = promote(&out, &stage, &["old.mp3".into()], &["new.mp3".into(), "missing.mp3".into()]).unwrap_err();
        assert_eq!(std::fs::read(out.join("old.mp3")).unwrap(), b"old soundtrack");
        assert_eq!(std::fs::read_to_string(out.join(MADE_LIST)).unwrap(), "old.mp3", "{error}");
        assert!(!out.join("new.mp3").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn successful_promotion_replaces_managed_files_and_manifest() {
        let root = std::env::current_dir().unwrap().join(format!(".skate-music-promote-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let out = root.join("out");
        let stage = out.join(".skate-music-staging-test");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(out.join("old.mp3"), b"old soundtrack").unwrap();
        std::fs::write(out.join(MADE_LIST), "old.mp3").unwrap();
        std::fs::write(stage.join("new.mp3"), b"new soundtrack").unwrap();

        promote(&out, &stage, &["old.mp3".into()], &["new.mp3".into()]).unwrap();
        assert!(!out.join("old.mp3").exists());
        assert_eq!(std::fs::read(out.join("new.mp3")).unwrap(), b"new soundtrack");
        assert_eq!(std::fs::read_to_string(out.join(MADE_LIST)).unwrap(), "new.mp3");
        let _ = std::fs::remove_dir_all(root);
    }

    fn tiny_wav(encoding: u16, channels: u16, payload: &[u8], riff_adjust: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        let size = (36usize + payload.len()) as i32 + riff_adjust;
        data.extend_from_slice(&(size as u32).to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&encoding.to_le_bytes());
        data.extend_from_slice(&channels.to_le_bytes());
        data.extend_from_slice(&44_100u32.to_le_bytes());
        data.extend_from_slice(&(44_100u32 * u32::from(channels) * 2).to_le_bytes());
        data.extend_from_slice(&(channels * 2).to_le_bytes());
        data.extend_from_slice(&16u16.to_le_bytes());
        data.extend_from_slice(b"data");
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        data
    }

    #[test]
    fn wav_rejects_non_pcm_channels_bad_bounds_and_partial_frames() {
        let dir = std::env::temp_dir().join(format!("skate-music-wav-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("check.wav");
        for (label, bytes) in [
            ("non-PCM", tiny_wav(3, 2, &[0; 4], 0)),
            ("too many channels", tiny_wav(1, 3, &[0; 6], 0)),
            ("partial frame", tiny_wav(1, 2, &[0; 2], 0)),
            ("riff beyond file", tiny_wav(1, 2, &[0; 4], 10)),
            ("chunk beyond RIFF", { let mut b = tiny_wav(1, 2, &[0; 4], 0); let n = b.len(); b[n - 8..n - 4].copy_from_slice(&100u32.to_le_bytes()); b }),
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(read_wav(&path).is_err(), "accepted {label}");
        }
        std::fs::write(&path, tiny_wav(1, 2, &[1, 0, 2, 0], 0)).unwrap();
        let (rate, channels, samples) = read_wav(&path).unwrap();
        assert_eq!((rate, channels, samples), (44_100, 2, vec![1, 2]));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_wav_and_writes_mp3() {
        let dir = std::env::temp_dir().join(format!("skate-music-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("a.wav");
        let samples: Vec<i16> = (0..44_100 * 2).map(|i| ((i as f32 * 0.05).sin() * 8000.) as i16).collect();
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&(36 + samples.len() as u32 * 2).to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 2] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.extend_from_slice(&44_100u32.to_le_bytes());
        data.extend_from_slice(&(44_100u32 * 4).to_le_bytes());
        for v in [4u16, 16] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.extend_from_slice(b"LIST");
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(b"abc\0");
        data.extend_from_slice(b"data");
        data.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for s in &samples {
            data.extend_from_slice(&s.to_le_bytes());
        }
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        std::fs::write(&wav, data).unwrap();
        let (rate, channels, read) = read_wav(&wav).unwrap();
        assert_eq!((rate, channels), (44_100, 2));
        assert_eq!(read, samples, "an odd-sized chunk before the data is skipped");
        let bytes = mp3(rate, channels, &read, "Skate 3 Track 01", 1).unwrap();
        assert_eq!(&bytes[..3], b"ID3");
        // One second at 192 kbps is about 24 KB.
        assert!((15_000..40_000).contains(&bytes.len()), "{} bytes", bytes.len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
