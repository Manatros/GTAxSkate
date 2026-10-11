//! Skate 3's BIG archives: EB BIG v3 (the index, and each entry's data
//! stored, RefPack, or chunked "chunkref" with RefPack or zlib chunks), and
//! EA's older BIGF / BIG4 archives some of its audio still comes in.
//!
//! Adapted from the MIT-licensed readers of the Skate 3 Custom Engine Layer
//! tooling (`tools/owned_game` in SK8-ENGINE/skate-3-rust-engine).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: String,
    pub offset: u64,
    pub stored_size: u32,
    pub unpacked_size: u32,
    pub compression: u8,
}

pub struct Archive {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

fn u8_at(data: &[u8], at: usize) -> Result<u8, String> {
    data.get(at).copied().ok_or_else(|| "truncated BIG index".to_owned())
}

fn u16_at(data: &[u8], at: usize) -> Result<u16, String> {
    let b = data.get(at..at + 2).ok_or("truncated BIG index")?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, String> {
    let b = data.get(at..at + 4).ok_or("truncated BIG index")?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn align(value: usize, to: usize) -> usize {
    (value + to - 1) & !(to - 1)
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        let mut magic = [0u8; 16];
        file.read_exact(&mut magic).map_err(|_| format!("{}: not a BIG archive", path.display()))?;
        if matches!(&magic[..4], b"BIGF" | b"BIG4" | b"BIGH" | b"BIG ") {
            let header_size = u64::from(u32_at(&magic, 12)?);
            if header_size > size || header_size < 16 {
                return Err(format!("{}: BIG header exceeds the file", path.display()));
            }
            let mut metadata = vec![0u8; header_size as usize];
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            file.read_exact(&mut metadata).map_err(|e| e.to_string())?;
            let entries = parse_classic(&metadata, size).map_err(|e| format!("{}: {e}", path.display()))?;
            return Ok(Self { path: path.to_owned(), entries });
        }
        let mut header = [0u8; 48];
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        file.read_exact(&mut header).map_err(|_| format!("{}: not a BIG archive", path.display()))?;
        let index_size = u32_at(&header, 12)? as u64;
        let names_size = u32_at(&header, 16)? as u64;
        if index_size + names_size > size {
            return Err(format!("{}: BIG index exceeds the file", path.display()));
        }
        let mut metadata = vec![0u8; (index_size + names_size) as usize];
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        file.read_exact(&mut metadata).map_err(|e| e.to_string())?;
        let entries = parse(&metadata, size).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { path: path.to_owned(), entries })
    }

    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>, String> {
        let mut file = File::open(&self.path).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(entry.offset)).map_err(|e| e.to_string())?;
        let mut packed = vec![0u8; entry.stored_size as usize];
        file.read_exact(&mut packed).map_err(|_| format!("{}: truncated", entry.path))?;
        let mut expected = entry.unpacked_size as usize;
        let data = match entry.compression {
            // An old archive's entry: RefPack when it starts like it.
            CLASSIC => match packed.get(..5) {
                Some([0x10 | 0x90, 0xFB, a, b, c]) => {
                    expected = (usize::from(*a) << 16) | (usize::from(*b) << 8) | usize::from(*c);
                    refpack(&packed, expected)?
                }
                _ => packed,
            },
            0 => packed,
            1 => refpack(&packed, expected)?,
            2..=4 => chunkref(&packed, expected)?,
            other => return Err(format!("{}: unknown BIG compression {other}", entry.path)),
        };
        if data.len() != expected {
            return Err(format!("{}: unpacked {} bytes, expected {expected}", entry.path, data.len()));
        }
        Ok(data)
    }
}

/// Entries of an old archive, read as RefPack or stored as they are.
const CLASSIC: u8 = 0xC1;

/// The index of a BIGF / BIG4 archive: count and header size, then each
/// entry's offset, size and name.
pub fn parse_classic(meta: &[u8], file_size: u64) -> Result<Vec<Entry>, String> {
    let count = u32_at(meta, 8)? as usize;
    if count > 1_000_000 {
        return Err("unreasonable BIG entry count".into());
    }
    let mut at = 16;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = u64::from(u32_at(meta, at)?);
        let size = u32_at(meta, at + 4)?;
        let rest = meta.get(at + 8..).ok_or("truncated BIG index")?;
        let end = rest.iter().position(|b| *b == 0).ok_or("truncated BIG name")?;
        let path = String::from_utf8_lossy(&rest[..end]).replace('\\', "/");
        at += 8 + end + 1;
        if offset + u64::from(size) > file_size {
            return Err(format!("{path}: data past the end of the archive"));
        }
        entries.push(Entry { path, offset, stored_size: size, unpacked_size: size, compression: CLASSIC });
    }
    Ok(entries)
}

pub fn parse(meta: &[u8], file_size: u64) -> Result<Vec<Entry>, String> {
    if u16_at(meta, 0)? != 0x4542 {
        return Err("not an EB BIG archive".into());
    }
    let version = u16_at(meta, 2)?;
    if version != 3 {
        return Err(format!("unsupported BIG version {version}"));
    }
    let count = u32_at(meta, 4)? as usize;
    if count > 1_000_000 {
        return Err("unreasonable BIG entry count".into());
    }
    let flags = u16_at(meta, 8)?;
    let shift = u8_at(meta, 10)?;
    if shift > 31 {
        return Err("bad BIG alignment".into());
    }
    let index_size = u32_at(meta, 12)? as usize;
    let names_size = u32_at(meta, 16)? as usize;
    let name_record = usize::from(u8_at(meta, 20)?);
    let dir_record = usize::from(u8_at(meta, 21)?);
    if name_record < 3 || dir_record < 2 {
        return Err("bad BIG name records".into());
    }
    let entry_size = if flags & 1 != 0 { 20 } else { 16 };
    let entries_start = 48;
    let compression_start = entries_start + align(entry_size * count, 16);
    let names_start = index_size;
    let dirs_start = names_start + align(name_record * count, 16);
    let dirs_end = index_size + names_size;
    let mut directories = Vec::new();
    let mut at = dirs_start;
    while at + dir_record <= dirs_end && at + dir_record <= meta.len() {
        directories.push(c_string(&meta[at..at + dir_record]));
        at += dir_record;
    }
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let e = entries_start + index * entry_size;
        let offset = u64::from(u32_at(meta, e)?) << shift;
        let declared = u32_at(meta, e + 4)?;
        let unpacked = match u32_at(meta, e + 8)? {
            0 => declared,
            n => n,
        };
        let stored = if declared == 0 { unpacked } else { declared };
        let compression = u8_at(meta, compression_start + index)?;
        let n = names_start + index * name_record;
        let dir = usize::from(u16_at(meta, n)?);
        let name = c_string(meta.get(n + 2..n + name_record).ok_or("truncated BIG names")?);
        let path = match directories.get(dir).map(String::as_str) {
            Some("") | Some(".") | None => name,
            Some(d) => format!("{d}/{name}"),
        }
        .replace('\\', "/");
        if offset + u64::from(stored) > file_size {
            return Err(format!("{path}: data past the end of the archive"));
        }
        entries.push(Entry { path, offset, stored_size: stored, unpacked_size: unpacked, compression });
    }
    Ok(entries)
}

fn chunkref(data: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    if data.get(..8) != Some(b"chunkref".as_slice()) || u32_at(data, 8)? != 2 {
        return Err("bad chunkref header".into());
    }
    let total = u32_at(data, 12)? as usize;
    let chunk_size = u32_at(data, 16)? as usize;
    let chunks = u32_at(data, 20)? as usize;
    let alignment = u32_at(data, 24)? as usize;
    if total != expected || chunk_size == 0 || chunks == 0 || alignment == 0 || alignment > 4096 || !alignment.is_power_of_two()
    {
        return Err("bad chunkref sizes".into());
    }
    let mut out = Vec::with_capacity(total);
    let mut cursor = 28;
    for _ in 0..chunks {
        let payload = align(cursor + 8, alignment);
        let packed_size = u32_at(data, payload - 8)? as usize;
        let method = u32_at(data, payload - 4)?;
        let packed = data.get(payload..payload + packed_size).ok_or("truncated chunkref chunk")?;
        let want = chunk_size.min(total - out.len());
        let decoded = match method {
            0 | 4 => packed.to_vec(),
            2 => refpack(packed, want)?,
            3 => miniz_oxide::inflate::decompress_to_vec_zlib(packed).map_err(|_| "bad zlib chunk".to_owned())?,
            other => return Err(format!("unknown chunkref method {other}")),
        };
        if decoded.len() != want {
            return Err("chunkref chunk has the wrong size".into());
        }
        out.extend_from_slice(&decoded);
        cursor = payload + packed_size;
    }
    Ok(out)
}

/// EA's RefPack (including its 0x10FB / 0x90FB headers).
pub fn refpack(data: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    let mut at = 0;
    if data.len() >= 2 && data[1] == 0xFB && (data[0] == 0x10 || data[0] == 0x90) {
        at = if data[0] & 0x80 != 0 { 6 } else { 5 };
    }
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let byte = |i: usize| data.get(i).copied().ok_or_else(|| "truncated RefPack".to_owned());
    fn literal(out: &mut Vec<u8>, data: &[u8], at: &mut usize, count: usize) -> Result<(), String> {
        let bytes = data.get(*at..*at + count).ok_or("truncated RefPack literal")?;
        out.extend_from_slice(bytes);
        *at += count;
        Ok(())
    }
    fn copy(out: &mut Vec<u8>, distance: usize, count: usize) -> Result<(), String> {
        if distance == 0 || distance > out.len() {
            return Err("RefPack reference before the start".into());
        }
        for _ in 0..count {
            out.push(out[out.len() - distance]);
        }
        Ok(())
    }
    while at < data.len() {
        let control = usize::from(data[at]);
        at += 1;
        if control < 0x80 {
            let b1 = usize::from(byte(at)?);
            at += 1;
            literal(&mut out, data, &mut at, control & 3)?;
            copy(&mut out, ((control & 0x60) << 3) + b1 + 1, ((control >> 2) & 7) + 3)?;
        } else if control < 0xC0 {
            let (b1, b2) = (usize::from(byte(at)?), usize::from(byte(at + 1)?));
            at += 2;
            literal(&mut out, data, &mut at, b1 >> 6)?;
            copy(&mut out, ((b1 & 0x3F) << 8) + b2 + 1, (control & 0x3F) + 4)?;
        } else if control < 0xE0 {
            let (b1, b2, b3) = (usize::from(byte(at)?), usize::from(byte(at + 1)?), usize::from(byte(at + 2)?));
            at += 3;
            literal(&mut out, data, &mut at, control & 3)?;
            copy(&mut out, ((control & 0x10) << 12) + (b1 << 8) + b2 + 1, ((control & 0x0C) << 6) + b3 + 5)?;
        } else if control < 0xFC {
            literal(&mut out, data, &mut at, ((control & 0x1F) << 2) + 4)?;
        } else {
            literal(&mut out, data, &mut at, control & 3)?;
            break;
        }
        if out.len() > expected {
            return Err("RefPack output too long".into());
        }
    }
    if out.len() != expected {
        return Err(format!("RefPack made {} bytes, expected {expected}", out.len()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BIG v3 archive of `files` (path, data, compression), as Skate 3
    /// lays it out.
    fn build(files: &[(&str, &str, Vec<u8>, u8)]) -> Vec<u8> {
        let count = files.len();
        let name_record = 32usize;
        let dir_record = 16usize;
        let mut dirs: Vec<&str> = Vec::new();
        for (dir, ..) in files {
            if !dirs.contains(dir) {
                dirs.push(dir);
            }
        }
        let entries_bytes = 16 * count;
        let compression_start = 48 + align(entries_bytes, 16);
        let index_size = align(compression_start + count, 16);
        let names_bytes = align(name_record * count, 16) + dir_record * dirs.len();
        let data_start = align(index_size + names_bytes, 16);
        let mut out = vec![0u8; data_start];
        out[0..2].copy_from_slice(&0x4542u16.to_be_bytes());
        out[2..4].copy_from_slice(&3u16.to_be_bytes());
        out[4..8].copy_from_slice(&(count as u32).to_be_bytes());
        out[12..16].copy_from_slice(&(index_size as u32).to_be_bytes());
        out[16..20].copy_from_slice(&(names_bytes as u32).to_be_bytes());
        out[20] = name_record as u8;
        out[21] = dir_record as u8;
        for (i, (dir, name, data, compression)) in files.iter().enumerate() {
            let offset = out.len();
            out.extend_from_slice(data);
            let e = 48 + 16 * i;
            out[e..e + 4].copy_from_slice(&(offset as u32).to_be_bytes());
            out[e + 4..e + 8].copy_from_slice(&(data.len() as u32).to_be_bytes());
            let unpacked = if *compression == 1 { 4u32 } else { data.len() as u32 };
            out[e + 8..e + 12].copy_from_slice(&unpacked.to_be_bytes());
            out[compression_start + i] = *compression;
            let n = index_size + name_record * i;
            let d = dirs.iter().position(|x| x == dir).unwrap() as u16;
            out[n..n + 2].copy_from_slice(&d.to_be_bytes());
            out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        }
        for (i, dir) in dirs.iter().enumerate() {
            let at = index_size + align(name_record * count, 16) + dir_record * i;
            out[at..at + dir.len()].copy_from_slice(dir.as_bytes());
        }
        out
    }

    #[test]
    fn reads_a_big_archive() {
        // RefPack: four literal bytes, then the end.
        let packed = vec![0xE0, b'a', b'b', b'c', b'd', 0xFC];
        let bytes = build(&[
            ("data/audio/music", "track01.snr", b"SNR!".to_vec(), 0),
            (".", "readme.txt", packed, 1),
        ]);
        let dir = std::env::temp_dir().join(format!("skate-music-big-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.big");
        std::fs::write(&path, &bytes).unwrap();
        let archive = Archive::open(&path).unwrap();
        assert_eq!(archive.entries[0].path, "data/audio/music/track01.snr");
        assert_eq!(archive.entries[1].path, "readme.txt");
        assert_eq!(archive.read(&archive.entries[0]).unwrap(), b"SNR!");
        assert_eq!(archive.read(&archive.entries[1]).unwrap(), b"abcd");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_an_old_big_archive() {
        let files: [(&str, &[u8]); 2] = [("music\\song.snu", b"SNU!data"), ("a.sns", b"xy")];
        let header_size = 16 + files.iter().map(|(n, _)| 8 + n.len() + 1).sum::<usize>();
        let mut out = Vec::new();
        out.extend_from_slice(b"BIGF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(files.len() as u32).to_be_bytes());
        out.extend_from_slice(&(header_size as u32).to_be_bytes());
        let mut offset = header_size;
        for (name, data) in &files {
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(name.as_bytes());
            out.push(0);
            offset += data.len();
        }
        for (_, data) in &files {
            out.extend_from_slice(data);
        }
        let path = std::env::temp_dir().join(format!("old-big-{}.big", std::process::id()));
        std::fs::write(&path, &out).unwrap();
        let archive = Archive::open(&path).unwrap();
        assert_eq!(archive.entries[0].path, "music/song.snu");
        assert_eq!(archive.read(&archive.entries[0]).unwrap(), b"SNU!data");
        assert_eq!(archive.read(&archive.entries[1]).unwrap(), b"xy");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn refpack_copies_back() {
        // Four literals, then a two-byte command: no literals, distance 2,
        // count 3; then the end.
        let stream = [0xE0, b'a', b'b', b'x', b'y', 0x00, 0x01, 0xFC];
        assert_eq!(refpack(&stream, 7).unwrap(), b"abxyxyx");
        assert!(refpack(&[0x00, 0x09, 0xFC], 3).is_err(), "reference before the start");
    }
}
