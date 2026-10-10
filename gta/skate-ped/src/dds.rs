//! DDS textures as GTA V ped textures want them: power-of-two sizes, a full
//! mip chain, DXT1 (opaque) or DXT5 (with alpha).

/// An RGBA image.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn solid(size: u32, rgba: [u8; 4]) -> Self {
        Self { width: size, height: size, rgba: rgba.repeat((size * size) as usize) }
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y.min(self.height - 1) * self.width + x.min(self.width - 1)) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// Whether any pixel is see-through enough to need alpha.
    pub fn has_alpha(&self) -> bool {
        self.rgba.chunks_exact(4).any(|p| p[3] < 250)
    }

    /// Resampled (box filter) to `width` x `height`.
    pub fn resized(&self, width: u32, height: u32) -> Self {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        let (sx, sy) = (self.width as f32 / width as f32, self.height as f32 / height as f32);
        for y in 0..height {
            for x in 0..width {
                let (x0, y0) = ((x as f32 * sx) as u32, (y as f32 * sy) as u32);
                let (x1, y1) = ((((x + 1) as f32 * sx).ceil() as u32).max(x0 + 1), (((y + 1) as f32 * sy).ceil() as u32).max(y0 + 1));
                let mut sum = [0u32; 4];
                let mut n = 0;
                for yy in y0..y1.min(self.height) {
                    for xx in x0..x1.min(self.width) {
                        let p = self.pixel(xx, yy);
                        for c in 0..4 {
                            sum[c] += u32::from(p[c]);
                        }
                        n += 1;
                    }
                }
                let n = n.max(1);
                rgba.extend(sum.map(|s| ((s + n / 2) / n) as u8));
            }
        }
        Self { width, height, rgba }
    }

    /// The nearest power-of-two size, at most `limit` and at least 4.
    pub fn power_of_two(&self, limit: u32) -> Self {
        let fit = |v: u32| {
            let mut p = 4;
            while p * 2 <= v.min(limit) {
                p *= 2;
            }
            // Round up when the next power is closer.
            if p * 2 <= limit && v > p + p / 2 { p * 2 } else { p }
        };
        let (w, h) = (fit(self.width), fit(self.height));
        if (w, h) == (self.width, self.height) { self.clone() } else { self.resized(w, h) }
    }
}

fn rgb565(c: [u8; 4]) -> u16 {
    (u16::from(c[0]) >> 3) << 11 | (u16::from(c[1]) >> 2) << 5 | u16::from(c[2]) >> 3
}

fn expand(c: u16) -> [i32; 3] {
    let r = i32::from((c >> 11) & 31);
    let g = i32::from((c >> 5) & 63);
    let b = i32::from(c & 31);
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// A 4x4 block's DXT1 colour part (always the four-colour mode).
fn colour_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let (mut lo, mut hi) = ([255u8; 3], [0u8; 3]);
    for p in block {
        for c in 0..3 {
            lo[c] = lo[c].min(p[c]);
            hi[c] = hi[c].max(p[c]);
        }
    }
    // Inset the box a little, as encoders do, for less banding.
    for c in 0..3 {
        let inset = (hi[c] - lo[c]) / 16;
        lo[c] += inset;
        hi[c] -= inset;
    }
    let mut c0 = rgb565([hi[0], hi[1], hi[2], 0]);
    let mut c1 = rgb565([lo[0], lo[1], lo[2], 0]);
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    if c0 == c1 {
        return out; // every index 0
    }
    let (a, b) = (expand(c0), expand(c1));
    let palette = [a, b, [0, 1, 2].map(|i| (2 * a[i] + b[i]) / 3), [0, 1, 2].map(|i| (a[i] + 2 * b[i]) / 3)];
    let mut indices = 0u32;
    for (i, p) in block.iter().enumerate() {
        let best = (0..4)
            .min_by_key(|&k| (0..3).map(|c| (i32::from(p[c]) - palette[k][c]).pow(2)).sum::<i32>())
            .unwrap();
        indices |= (best as u32) << (2 * i);
    }
    out[4..8].copy_from_slice(&indices.to_le_bytes());
    out
}

/// A 4x4 block's DXT5 alpha part.
fn alpha_block(block: &[[u8; 4]; 16]) -> [u8; 8] {
    let a0 = block.iter().map(|p| p[3]).max().unwrap();
    let a1 = block.iter().map(|p| p[3]).min().unwrap();
    let mut out = [0u8; 8];
    out[0] = a0;
    out[1] = a1;
    if a0 == a1 {
        return out;
    }
    let palette: Vec<i32> = (0..8)
        .map(|k| match k {
            0 => i32::from(a0),
            1 => i32::from(a1),
            k => ((8 - k) * i32::from(a0) + (k - 1) * i32::from(a1)) / 7,
        })
        .collect();
    let mut bits = 0u64;
    for (i, p) in block.iter().enumerate() {
        let best = (0..8).min_by_key(|&k| (i32::from(p[3]) - palette[k]).abs()).unwrap() as u64;
        bits |= best << (3 * i);
    }
    out[2..8].copy_from_slice(&bits.to_le_bytes()[..6]);
    out
}

fn compress(image: &Image, alpha: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for by in (0..image.height).step_by(4) {
        for bx in (0..image.width).step_by(4) {
            let mut block = [[0u8; 4]; 16];
            for (i, p) in block.iter_mut().enumerate() {
                *p = image.pixel(bx + (i as u32 % 4), by + (i as u32 / 4));
            }
            if alpha {
                out.extend(alpha_block(&block));
            }
            out.extend(colour_block(&block));
        }
    }
    out
}

/// The texture as a DDS file (DXT5 with alpha, else DXT1) with all mips,
/// and the format name for the XML.
pub fn encode(image: &Image) -> (Vec<u8>, u32, &'static str) {
    let alpha = image.has_alpha();
    let mut levels = vec![image.clone()];
    while levels.last().unwrap().width > 4 && levels.last().unwrap().height > 4 {
        let last = levels.last().unwrap();
        levels.push(last.resized(last.width / 2, last.height / 2));
    }
    let mut data = Vec::new();
    for level in &levels {
        data.extend(compress(level, alpha));
    }
    let block = if alpha { 16 } else { 8 };
    let mut h = vec![0u8; 128];
    let put = |h: &mut Vec<u8>, at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
    h[0..4].copy_from_slice(b"DDS ");
    put(&mut h, 4, 124);
    put(&mut h, 8, 0x1 | 0x2 | 0x4 | 0x1000 | 0x20000 | 0x80000); // caps, height, width, pixel format, mips, linear size
    put(&mut h, 12, image.height);
    put(&mut h, 16, image.width);
    put(&mut h, 20, (image.width / 4).max(1) * (image.height / 4).max(1) * block);
    put(&mut h, 28, levels.len() as u32);
    put(&mut h, 76, 32);
    put(&mut h, 80, 0x4); // FourCC
    h[84..88].copy_from_slice(if alpha { b"DXT5" } else { b"DXT1" });
    put(&mut h, 108, 0x1000 | 0x8 | 0x400000); // texture, complex, mipmap
    h.extend(data);
    (h, levels.len() as u32, if alpha { "D3DFMT_DXT5" } else { "D3DFMT_DXT1" })
}

/// The texture as an uncompressed DDS file (BGRA, 32 bits) with all mips:
/// no block artefacts on a board's graphic, for the weapon model.
pub fn encode_raw(image: &Image) -> (Vec<u8>, u32, &'static str) {
    let mut levels = vec![image.clone()];
    while levels.last().unwrap().width > 1 && levels.last().unwrap().height > 1 {
        let last = levels.last().unwrap();
        levels.push(last.resized((last.width / 2).max(1), (last.height / 2).max(1)));
    }
    let mut h = vec![0u8; 128];
    let put = |h: &mut Vec<u8>, at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
    h[0..4].copy_from_slice(b"DDS ");
    put(&mut h, 4, 124);
    put(&mut h, 8, 0x1 | 0x2 | 0x4 | 0x8 | 0x1000 | 0x20000); // caps, height, width, pitch, pixel format, mips
    put(&mut h, 12, image.height);
    put(&mut h, 16, image.width);
    put(&mut h, 20, image.width * 4);
    put(&mut h, 28, levels.len() as u32);
    put(&mut h, 76, 32);
    put(&mut h, 80, 0x40 | 0x1); // RGB, alpha pixels
    put(&mut h, 88, 32);
    put(&mut h, 92, 0x00FF_0000);
    put(&mut h, 96, 0x0000_FF00);
    put(&mut h, 100, 0x0000_00FF);
    put(&mut h, 104, 0xFF00_0000);
    put(&mut h, 108, 0x1000 | 0x8 | 0x400000); // texture, complex, mipmap
    for level in &levels {
        for p in level.rgba.chunks_exact(4) {
            h.extend([p[2], p[1], p[0], p[3]]);
        }
    }
    (h, levels.len() as u32, "D3DFMT_A8R8G8B8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_dxt_with_mips() {
        let image = Image::solid(16, [200, 100, 50, 255]);
        let (dds, levels, format) = encode(&image);
        assert_eq!(format, "D3DFMT_DXT1");
        assert_eq!(levels, 3); // 16, 8, 4
        assert_eq!(dds.len(), 128 + 16 * 8 + 4 * 8 + 8);
        assert_eq!(&dds[84..88], b"DXT1");
        // A solid block decodes back to (about) its colour.
        let c0 = u16::from_le_bytes([dds[128], dds[129]]);
        let rgb = expand(c0);
        assert!((rgb[0] - 200).abs() < 9 && (rgb[1] - 100).abs() < 5 && (rgb[2] - 50).abs() < 9);
        let clear = Image::solid(8, [0, 0, 0, 10]);
        assert_eq!(encode(&clear).2, "D3DFMT_DXT5");
        let odd = Image { width: 300, height: 90, rgba: vec![255; 300 * 90 * 4] };
        let fitted = odd.power_of_two(256);
        assert_eq!((fitted.width, fitted.height), (256, 64));
    }

    #[test]
    fn raw_keeps_every_pixel_with_its_mips() {
        let image = Image { width: 4, height: 4, rgba: [10u8, 20, 30, 255].repeat(16) };
        let (dds, levels, format) = encode_raw(&image);
        assert_eq!(format, "D3DFMT_A8R8G8B8");
        assert_eq!(levels, 3);
        assert_eq!(dds.len(), 128 + (16 + 4 + 1) * 4);
        assert_eq!(&dds[128..132], &[30, 20, 10, 255], "BGRA");
    }
}
