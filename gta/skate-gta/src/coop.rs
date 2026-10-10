//! Co-op between two copies of the plugin (over FiveM, or two games on a
//! LAN): each sends the other its skater several times a second, as one UDP
//! packet: whether it skates, where (GTA origin and Skate's root), the
//! bones the other needs to pose and draw it, its board, what the board
//! does (for its sounds) and the objects it dropped.

use bevy_math::{Mat4, Quat, Vec3};

const MAGIC: &[u8; 4] = b"SK8C";
const VERSION: u8 = 1;
/// Most bones and objects a packet carries.
pub const MAX_BONES: usize = 64;
pub const MAX_OBJECTS: usize = skate_park::MAX_OBJECTS;

/// A skater as the other player hears of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Packet {
    pub seq: u32,
    pub skating: bool,
    /// The GTA point Skate's origin is, and Skate's root (Skate space).
    pub origin: Vec3,
    pub root: Mat4,
    /// Bones by their index in Skate's pose names, Skate space.
    pub bones: Vec<(u16, Mat4)>,
    /// The board: `skate_boards::BoardChoice` text, as it looks (worn).
    pub board: String,
    /// What the board does: Skate's state (`skate_sound::state_code`),
    /// speed and vertical speed (m/s), wheels down, surface.
    pub state: u8,
    pub speed: f32,
    pub vertical: f32,
    pub wheels: u8,
    pub surface: u8,
    /// Objects dropped with the object dropper (GTA metres).
    pub objects: Vec<skate_park::ParkObject>,
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend(v.to_le_bytes());
}

fn put_matrix(out: &mut Vec<u8>, m: Mat4) {
    let (_, rotation, position) = m.to_scale_rotation_translation();
    for v in rotation.normalize().to_array() {
        out.extend(((v.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes());
    }
    for v in position.to_array() {
        put_f32(out, v);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let slice = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(slice)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Option<f32> {
        self.take(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).filter(|v| v.is_finite())
    }
    fn vec3(&mut self) -> Option<Vec3> {
        Some(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }
    fn matrix(&mut self) -> Option<Mat4> {
        let mut q = [0f32; 4];
        for v in &mut q {
            *v = f32::from(self.u16()? as i16) / 32767.;
        }
        let rotation = Quat::from_array(q).normalize();
        Some(Mat4::from_rotation_translation(rotation, self.vec3()?))
    }
}

impl Packet {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(512 + self.bones.len() * 22);
        out.extend(MAGIC);
        out.push(VERSION);
        out.extend(self.seq.to_le_bytes());
        out.push(u8::from(self.skating));
        for v in self.origin.to_array() {
            put_f32(&mut out, v);
        }
        put_matrix(&mut out, self.root);
        let board = if self.board.len() <= 255 && self.board.is_ascii() { self.board.as_str() } else { "" };
        out.push(board.len() as u8);
        out.extend(board.as_bytes());
        out.push(self.state);
        put_f32(&mut out, self.speed);
        put_f32(&mut out, self.vertical);
        out.push(self.wheels);
        out.push(self.surface);
        let bones = &self.bones[..self.bones.len().min(MAX_BONES)];
        out.push(bones.len() as u8);
        for (index, m) in bones {
            out.extend(index.to_le_bytes());
            put_matrix(&mut out, *m);
        }
        let objects = &self.objects[..self.objects.len().min(MAX_OBJECTS)];
        out.push(objects.len() as u8);
        for o in objects {
            out.push(o.kind);
            for v in o.at {
                put_f32(&mut out, v);
            }
            put_f32(&mut out, o.yaw);
        }
        out
    }

    /// Reads [`Self::encode`]; anything malformed is no packet.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(4)? != MAGIC || r.u8()? != VERSION {
            return None;
        }
        let seq = r.u32()?;
        let skating = r.u8()? != 0;
        let origin = r.vec3()?;
        let root = r.matrix()?;
        let len = usize::from(r.u8()?);
        let board = String::from_utf8(r.take(len)?.to_vec()).ok().filter(|b| b.is_ascii())?;
        let state = r.u8()?;
        let speed = r.f32()?;
        let vertical = r.f32()?;
        let wheels = r.u8()?;
        let surface = r.u8()?;
        let count = usize::from(r.u8()?);
        if count > MAX_BONES {
            return None;
        }
        let mut bones = Vec::with_capacity(count);
        for _ in 0..count {
            bones.push((r.u16()?, r.matrix()?));
        }
        let count = usize::from(r.u8()?);
        if count > MAX_OBJECTS {
            return None;
        }
        let mut objects = Vec::with_capacity(count);
        for _ in 0..count {
            let object = skate_park::ParkObject { kind: r.u8()?, at: r.vec3()?.to_array(), yaw: r.f32()? };
            objects.push(object);
        }
        Some(Self { seq, skating, origin, root, bones, board, state, speed, vertical, wheels, surface, objects })
    }

    /// The pose as Skate's full bone list (`count` bones), the bones not
    /// sent left at rest.
    pub fn pose(&self, count: usize) -> Vec<Mat4> {
        let mut pose = vec![Mat4::IDENTITY; count];
        for (index, m) in &self.bones {
            if let Some(slot) = pose.get_mut(usize::from(*index)) {
                *slot = *m;
            }
        }
        pose
    }
}

/// Whether `newer` follows `older` (sequence numbers wrap).
pub fn newer(newer: u32, older: u32) -> bool {
    newer != older && newer.wrapping_sub(older) < u32::MAX / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skater_crosses_the_wire() {
        let turn = Mat4::from_rotation_translation(Quat::from_rotation_y(0.7), Vec3::new(1., 2., 3.));
        let packet = Packet {
            seq: 41,
            skating: true,
            origin: Vec3::new(-1200.5, 300.25, 31.),
            root: turn,
            bones: vec![(3, turn), (17, Mat4::IDENTITY)],
            board: "a841a65d5c4dd593:0011,,".into(),
            state: 7,
            speed: 5.5,
            vertical: -1.25,
            wheels: 4,
            surface: 2,
            objects: vec![skate_park::ParkObject { kind: 3, at: [1., 2., 3.], yaw: 0.5 }],
        };
        let bytes = packet.encode();
        assert!(bytes.len() < 1200, "one datagram: {}", bytes.len());
        let back = Packet::decode(&bytes).unwrap();
        assert_eq!((back.seq, back.skating, back.origin, &back.board), (41, true, packet.origin, &packet.board));
        assert!(back.root.abs_diff_eq(turn, 1e-3));
        assert_eq!(back.objects, packet.objects);
        let pose = back.pose(20);
        assert!(pose[3].abs_diff_eq(turn, 1e-3) && pose[0] == Mat4::IDENTITY);
        assert!(Packet::decode(&bytes[..bytes.len() - 1]).is_none(), "cut short");
        assert!(Packet::decode(b"nonsense").is_none());
        assert!(newer(5, 4) && !newer(4, 5) && newer(0, u32::MAX));
    }
}
