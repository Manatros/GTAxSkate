//! Where the board's sound is heard from: panned to its side of the camera,
//! quieter with distance, and with a room's echo where the skater is
//! closed in (a store, an underpass, a tunnel, a bowl's deep end).
//!
//! [`Room`] is a small reverb (four combs and two all-passes, as Schroeder
//! and Freeverb have it) over the board's mono sound; [`Space`] says how much
//! of it to hear and where the sound sits left to right.

/// How the board is heard this moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    /// -1 left of the camera to 1 right.
    pub pan: f32,
    /// 0 to 1 by distance.
    pub gain: f32,
    /// How closed in: 0 open air to 1 a closed room.
    pub room: f32,
}

impl Default for Space {
    fn default() -> Self {
        Self { pan: 0., gain: 1., room: 0. }
    }
}

impl Space {
    /// From the camera (its position and right-hand direction) to the board,
    /// and the share of sight lines round the skater that met something.
    pub fn heard(camera: [f32; 3], right: [f32; 3], board: [f32; 3], closed: f32) -> Self {
        let to = [board[0] - camera[0], board[1] - camera[1], board[2] - camera[2]];
        let dist = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
        let pan = if dist > 0.3 { ((to[0] * right[0] + to[1] * right[1] + to[2] * right[2]) / dist).clamp(-1., 1.) * 0.8 } else { 0. };
        let gain = (4. / dist.max(4.)).clamp(0.15, 1.);
        Self { pan, gain, room: closed.clamp(0., 1.) }
    }

    /// Left and right gains (equal power).
    pub fn sides(self) -> (f32, f32) {
        let a = (self.pan + 1.) * std::f32::consts::FRAC_PI_4;
        (a.cos() * self.gain, a.sin() * self.gain)
    }
}

struct Comb {
    buf: Vec<f32>,
    at: usize,
    feedback: f32,
    damp: f32,
    store: f32,
}

impl Comb {
    fn new(len: usize, feedback: f32) -> Self {
        Self { buf: vec![0.; len], at: 0, feedback, damp: 0.3, store: 0. }
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.buf[self.at];
        self.store = y * (1. - self.damp) + self.store * self.damp;
        self.buf[self.at] = x + self.store * self.feedback;
        self.at = (self.at + 1) % self.buf.len();
        y
    }
}

struct AllPass {
    buf: Vec<f32>,
    at: usize,
}

impl AllPass {
    fn run(&mut self, x: f32) -> f32 {
        let b = self.buf[self.at];
        let y = b - x;
        self.buf[self.at] = x + b * 0.5;
        self.at = (self.at + 1) % self.buf.len();
        y
    }
}

/// A room's echo at 44.1 kHz.
pub struct Room {
    combs: Vec<Comb>,
    passes: Vec<AllPass>,
    /// The echo's share, moved toward the wanted one slowly (no jumps).
    wet: f32,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            combs: [1116, 1188, 1277, 1356].iter().map(|&n| Comb::new(n, 0.78)).collect(),
            passes: [556, 441].iter().map(|&n| AllPass { buf: vec![0.; n], at: 0 }).collect(),
            wet: 0.,
        }
    }
}

impl Room {
    /// Mixes `mono` into interleaved stereo `out` (twice as long) as heard
    /// in `space`.
    pub fn render(&mut self, mono: &[f32], space: Space, out: &mut [f32]) {
        let (l, r) = space.sides();
        let target = space.room * 0.45;
        for (i, &x) in mono.iter().enumerate() {
            self.wet += (target - self.wet) * 0.0005;
            let mut echo = 0.;
            if self.wet > 0.002 {
                for c in &mut self.combs {
                    echo += c.run(x * 0.25);
                }
                for p in &mut self.passes {
                    echo = p.run(echo);
                }
            }
            let dry = x * (1. - self.wet * 0.5);
            // The echo surrounds: both sides alike.
            out[2 * i] = dry * l + echo * self.wet * 0.7;
            out[2 * i + 1] = dry * r + echo * self.wet * 0.7;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_board_right_of_the_camera_is_heard_right_and_far_is_quieter() {
        let near = Space::heard([0., 0., 0.], [1., 0., 0.], [3., 0., 0.], 0.);
        let (l, r) = near.sides();
        assert!(r > l * 2., "right: {l} {r}");
        let far = Space::heard([0., 0., 0.], [1., 0., 0.], [0., 40., 0.], 0.);
        assert!(far.gain < near.gain && far.pan.abs() < 0.01);
    }

    #[test]
    fn a_closed_room_echoes_and_open_air_does_not() {
        let mut click = vec![0.; 44_100];
        click[0] = 1.;
        let tail = |room: f32| {
            let mut r = Room::default();
            let mut out = vec![0.; 2 * click.len()];
            // Let the echo's share settle first.
            r.render(&vec![0.; 44_100], Space { pan: 0., gain: 1., room }, &mut vec![0.; 88_200]);
            r.render(&click, Space { pan: 0., gain: 1., room }, &mut out);
            out[8_000..].iter().map(|v| v.abs()).sum::<f32>()
        };
        assert!(tail(1.) > 10. * tail(0.).max(1e-6), "closed {} open {}", tail(1.), tail(0.));
    }
}
