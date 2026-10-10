//! Skate 3's own files as the disc has them: EB archives (`.big`), their
//! `chunkref` / RefPack compression, and RenderWare 4 Xbox 360 textures
//! (`.rx2`), decoded to RGBA. Used for the logos of Skate's graphics
//! editor (`crate::stickers`).

/// djb2, as EB archives hash their entries' full paths.
pub fn djb2(name: &str) -> u32 {
    name.bytes().fold(5381u32, |h, c| h.wrapping_mul(33).wrapping_add(u32::from(c)))
}

fn be32(d: &[u8], at: usize) -> Result<u32, String> {
    d.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "file too short".to_owned())
}

/// An EB archive read whole: entries by their path's hash.
pub struct Archive {
    data: Vec<u8>,
    /// hash -> (offset, stored size, size)
    entries: std::collections::HashMap<u32, (usize, usize, usize)>,
}

impl Archive {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if data.get(..2) != Some(b"EB") {
            return Err(format!("{} is not an EB archive", path.display()));
        }
        let count = be32(&data, 4)? as usize;
        let step = 1usize << data.get(0x0A).copied().unwrap_or(0);
        let mut entries = std::collections::HashMap::new();
        for i in 0..count {
            let at = 0x30 + i * 16;
            let offset = be32(&data, at)? as usize * step;
            let stored = be32(&data, at + 4)? as usize;
            let size = be32(&data, at + 8)? as usize;
            entries.insert(be32(&data, at + 12)?, (offset, stored, size));
        }
        Ok(Self { data, entries })
    }

    /// The file at `path` (as Skate names it: `data/content/...`), unpacked.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let &(offset, stored, size) = self.entries.get(&djb2(path)).ok_or_else(|| format!("{path} is not in the archive"))?;
        let raw = self.data.get(offset..(offset + stored.max(size)).min(self.data.len())).ok_or("entry out of the archive")?;
        if raw.starts_with(b"chunkref") { chunkref(raw) } else { Ok(raw[..size.min(raw.len())].to_vec()) }
    }
}

/// EA's `chunkref`: a header, a table of (stored size, kind) per chunk,
/// and the chunks (kind 2: RefPack), each aligned.
pub fn chunkref(d: &[u8]) -> Result<Vec<u8>, String> {
    let total = be32(d, 12)? as usize;
    let count = be32(d, 20)? as usize;
    let align = (be32(d, 24)? as usize).max(1);
    let up = |p: usize| p.div_ceil(align) * align;
    let mut pos = up(0x28 + count * 8);
    let mut out = Vec::with_capacity(total);
    for i in 0..count {
        let size = be32(d, 0x28 + i * 8)? as usize;
        let kind = be32(d, 0x2C + i * 8)?;
        let chunk = d.get(pos..pos + size).ok_or("chunk out of the entry")?;
        if kind == 2 {
            out.extend(refpack(chunk)?);
        } else {
            out.extend_from_slice(chunk);
        }
        pos = up(pos + size);
    }
    out.truncate(total);
    Ok(out)
}

/// EA's RefPack (QFS) decompression.
pub fn refpack(src: &[u8]) -> Result<Vec<u8>, String> {
    let at = |i: usize| src.get(i).copied().ok_or_else(|| "RefPack data ends early".to_owned());
    let flags = (u16::from(at(0)?) << 8) | u16::from(at(1)?);
    let wide = if flags & 0x8000 != 0 { 4 } else { 3 };
    let mut pos = 2 + if flags & 0x0100 != 0 { wide } else { 0 };
    let size = (0..wide).try_fold(0usize, |s, i| Ok::<_, String>((s << 8) | usize::from(at(pos + i)?)))?;
    pos += wide;
    let mut out: Vec<u8> = Vec::with_capacity(size);
    while pos < src.len() && out.len() < size {
        let b0 = usize::from(at(pos)?);
        let (literal, offset, count, used, last) = if b0 < 0x80 {
            let b1 = usize::from(at(pos + 1)?);
            (b0 & 3, ((b0 & 0x60) << 3) + b1 + 1, ((b0 & 0x1c) >> 2) + 3, 2, false)
        } else if b0 < 0xC0 {
            let (b1, b2) = (usize::from(at(pos + 1)?), usize::from(at(pos + 2)?));
            (b1 >> 6, ((b1 & 0x3f) << 8) + b2 + 1, (b0 & 0x3f) + 4, 3, false)
        } else if b0 < 0xE0 {
            let (b1, b2, b3) = (usize::from(at(pos + 1)?), usize::from(at(pos + 2)?), usize::from(at(pos + 3)?));
            (b0 & 3, ((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1, ((b0 & 0x0c) << 6) + b3 + 5, 4, false)
        } else if b0 < 0xFC {
            (((b0 & 0x1f) << 2) + 4, 0, 0, 1, false)
        } else {
            (b0 & 3, 0, 0, 1, true)
        };
        pos += used;
        out.extend_from_slice(src.get(pos..pos + literal).ok_or("RefPack literal out of the data")?);
        pos += literal;
        if last {
            break;
        }
        if offset > out.len() {
            return Err("RefPack copy before the start".into());
        }
        for _ in 0..count {
            out.push(out[out.len() - offset]);
        }
    }
    out.truncate(size);
    Ok(out)
}

/// An `.rx2` texture's top mip as RGBA (width, height, pixels). DXT1 and
/// DXT5, tiled the Xbox 360 way, 16-bit big-endian.
pub fn rx2_image(d: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if d.get(..7) != Some(b"\x89RW4xb2") {
        return Err("not a RenderWare 4 Xbox 360 file".into());
    }
    // The GPU's texture fetch constant and where the pixels start.
    let format = be32(d, 0x164)? & 0x3f;
    let size = be32(d, 0x168)?;
    let (width, height) = ((size & 0x1fff) + 1, ((size >> 13) & 0x1fff) + 1);
    let data = be32(d, 0x44)? as usize;
    let block = match format {
        0x12 => 8,
        0x13 | 0x14 => 16,
        f => return Err(format!("texture format {f:#x} not handled")),
    };
    let (bw, bh) = (width.div_ceil(4) as usize, height.div_ceil(4) as usize);
    let (aw, ah) = ((bw + 31) & !31, (bh + 31) & !31);
    let mut raw = d.get(data..).ok_or("no pixels")?.to_vec();
    raw.truncate(aw * ah * block);
    for pair in raw.chunks_exact_mut(2) {
        pair.swap(0, 1);
    }
    let w = width as usize;
    let mut out = vec![255u8; w * height as usize * 4];
    for i in 0..aw * ah {
        let (x, y) = (tiled_x(i, aw, block), tiled_y(i, aw, block));
        if x >= bw || y >= bh {
            continue;
        }
        let Some(b) = raw.get(i * block..(i + 1) * block) else { continue };
        if block == 8 {
            colour_block(b, &mut out, x * 4, y * 4, w, height as usize, true);
        } else {
            colour_block(&b[8..], &mut out, x * 4, y * 4, w, height as usize, false);
            if format == 0x14 {
                alpha_block(b, &mut out, x * 4, y * 4, w, height as usize);
            }
        }
    }
    Ok((width, height, out))
}

fn log_bpp(p: usize) -> usize {
    (p >> 2) + ((p >> 1) >> (p >> 2))
}

/// Xbox 360 tiling (XGAddress2DTiledX/Y), for blocks of `p` bytes in rows
/// `w` blocks wide (a multiple of 32).
fn tiled_x(offset: usize, w: usize, p: usize) -> usize {
    let lb = log_bpp(p);
    let ob = offset << lb;
    let ot = ((ob & !4095) >> 3) + ((ob & 1792) >> 2) + (ob & 63);
    let om = ot >> (7 + lb);
    let macro_x = (om % (w >> 5)) << 2;
    let tile = (((ot >> (5 + lb)) & 2) + (ob >> 6)) & 3;
    let macro_ = (macro_x + tile) << 3;
    let micro = ((((ot >> 1) & !15) + (ot & 15)) & ((p << 3) - 1)) >> lb;
    macro_ + micro
}

fn tiled_y(offset: usize, w: usize, p: usize) -> usize {
    let lb = log_bpp(p);
    let ob = offset << lb;
    let ot = ((ob & !4095) >> 3) + ((ob & 1792) >> 2) + (ob & 63);
    let om = ot >> (7 + lb);
    let macro_y = (om / (w >> 5)) << 2;
    let tile = ((ot >> (6 + lb)) & 1) + ((ob & 2048) >> 10);
    let macro_ = (macro_y + tile) << 3;
    let micro = (((ot & (((p << 6) - 1) & !31)) + ((ot & 15) << 1)) >> (3 + lb)) & !1;
    macro_ + micro + ((ot & 16) >> 4)
}

fn rgb565(c: u16) -> [u32; 3] {
    [u32::from((c >> 11) & 31) * 255 / 31, u32::from((c >> 5) & 63) * 255 / 63, u32::from(c & 31) * 255 / 31]
}

fn colour_block(b: &[u8], out: &mut [u8], x0: usize, y0: usize, w: usize, h: usize, one_bit: bool) {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mix = |a: u32, b: u32, n: u32, d: u32| (a * (d - n) + b * n) / d;
    let pal: [[u32; 3]; 4] = if c0 > c1 || !one_bit {
        [p0, p1, std::array::from_fn(|i| mix(p0[i], p1[i], 1, 3)), std::array::from_fn(|i| mix(p0[i], p1[i], 2, 3))]
    } else {
        [p0, p1, std::array::from_fn(|i| mix(p0[i], p1[i], 1, 2)), [0; 3]]
    };
    for i in 0..16 {
        let (x, y) = (x0 + i % 4, y0 + i / 4);
        if x < w && y < h {
            let c = pal[((bits >> (2 * i)) & 3) as usize];
            out[(y * w + x) * 4..(y * w + x) * 4 + 3].copy_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8]);
        }
    }
}

fn alpha_block(b: &[u8], out: &mut [u8], x0: usize, y0: usize, w: usize, h: usize) {
    let (a0, a1) = (u32::from(b[0]), u32::from(b[1]));
    let bits = b[2..8].iter().rev().fold(0u64, |s, &v| (s << 8) | u64::from(v));
    let mut pal = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 0..6 {
            pal[2 + i] = ((6 - i as u32) * a0 + (i as u32 + 1) * a1) / 7;
        }
    } else {
        for i in 0..4 {
            pal[2 + i] = ((4 - i as u32) * a0 + (i as u32 + 1) * a1) / 5;
        }
        pal[7] = 255;
    }
    for i in 0..16 {
        let (x, y) = (x0 + i % 4, y0 + i / 4);
        if x < w && y < h {
            out[(y * w + x) * 4 + 3] = pal[((bits >> (3 * i)) & 7) as usize] as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refpack_copies_back_references() {
        // Header: flags 0x10FB, size 6; "ab" as a literal run of 4 needs
        // 4 bytes, so: literal "abab" (0xE0: 4 literals), then a copy of 2
        // from 2 back (0x00 0x01: count 3? see below), then the end.
        // Literal-only stream with the end code carrying the last bytes.
        let src = [0x10, 0xFB, 0, 0, 6, 0xE0, b'a', b'b', b'c', b'd', 0xFE, b'e', b'f'];
        assert_eq!(refpack(&src).unwrap(), b"abcdef");
        // A short copy: 1 literal 'x', then 3 more from 1 back.
        let src = [0x10, 0xFB, 0, 0, 4, 0x01, 0x00, b'x', 0xFC];
        assert_eq!(refpack(&src).unwrap(), b"xxxx");
    }

    #[test]
    fn djb2_is_skates_path_hash() {
        assert_eq!(djb2(""), 5381);
        assert_eq!(djb2("a"), 5381 * 33 + 97);
    }

    #[test]
    fn tiling_maps_every_block_once() {
        let (w, h, p) = (32, 32, 16);
        let mut seen = vec![false; w * h];
        for i in 0..w * h {
            let (x, y) = (tiled_x(i, w, p), tiled_y(i, w, p));
            assert!(x < w && y < h);
            assert!(!seen[y * w + x]);
            seen[y * w + x] = true;
        }
    }
}
