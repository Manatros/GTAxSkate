//! The cinematic camera for skating, as GTA's cinematic camera in a car
//! (R): it cuts between shots every few seconds. A low chase behind the
//! board, a tracking shot from the side at wheel height, a roadside camera
//! that waits ahead and turns as the skater passes, a low shot from in front
//! looking back. Each shot is worked out from where the skater is and goes;
//! the script keeps it out of walls.

use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Chase,
    Side,
    Roadside,
    Front,
}

const ORDER: [Shot; 6] = [Shot::Chase, Shot::Roadside, Shot::Side, Shot::Chase, Shot::Front, Shot::Roadside];

/// Seconds a moving shot lasts.
const SHOT_SECONDS: f32 = 4.5;

pub struct ActionCam {
    shot: usize,
    /// Seconds into the shot.
    time: f32,
    /// Which side a side shot is on (+1 right of travel, -1 left).
    side: f32,
    /// Where a roadside camera stands, fixed for its shot.
    post: Option<Vec3>,
    /// The way the skater went, kept while they stand.
    heading: Vec3,
}

impl Default for ActionCam {
    fn default() -> Self {
        Self { shot: 0, time: 0., side: 1., post: None, heading: Vec3::Y }
    }
}

/// A camera placement: where it is and the point it looks at, and its
/// field of view (degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub eye: Vec3,
    pub look: Vec3,
    pub fov: f32,
}

impl ActionCam {
    pub fn shot(&self) -> Shot {
        ORDER[self.shot % ORDER.len()]
    }

    /// The camera for this frame: the skater's pelvis `at`, their velocity
    /// (m/s), the frame's seconds.
    pub fn update(&mut self, at: Vec3, velocity: Vec3, dt: f32) -> Placed {
        let flat = Vec3::new(velocity.x, velocity.y, 0.);
        if flat.length() > 0.8 {
            self.heading = flat.normalize();
        }
        let ahead = self.heading;
        let right = Vec3::new(ahead.y, -ahead.x, 0.);
        self.time += dt;
        let speed = flat.length();
        // A roadside camera cuts once the skater is well past it (or never
        // reached it); the others after a few seconds.
        let done = match self.shot() {
            Shot::Roadside => self.post.is_some_and(|p| (at - p).dot(ahead) > 6. || self.time > 9.),
            _ => self.time > SHOT_SECONDS,
        };
        if done {
            self.cut();
        }
        let look = at + Vec3::Z * 0.2;
        match self.shot() {
            Shot::Chase => Placed { eye: at - ahead * 3.4 + Vec3::Z * -0.45, look: look + ahead * 2., fov: 55. },
            Shot::Side => Placed { eye: at + right * self.side * (3. + speed * 0.1) - Vec3::Z * 0.55, look, fov: 50. },
            Shot::Front => Placed { eye: at + ahead * (3.2 + speed * 0.15) - Vec3::Z * 0.6, look, fov: 55. },
            Shot::Roadside => {
                let post = *self.post.get_or_insert(at + ahead * (6. + speed * 1.6) + right * self.side * 3.5 - Vec3::Z * 0.4);
                Placed { eye: post, look, fov: 45. }
            }
        }
    }

    fn cut(&mut self) {
        self.shot = (self.shot + 1) % ORDER.len();
        self.time = 0.;
        self.post = None;
        self.side = -self.side;
    }

    /// Starts over at the first shot.
    pub fn reset(&mut self) {
        *self = Self { heading: self.heading, ..Self::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shots_cut_and_the_roadside_camera_waits_ahead() {
        let mut cam = ActionCam::default();
        let v = Vec3::new(0., 8., 0.);
        let first = cam.update(Vec3::ZERO, v, 0.016);
        assert_eq!(cam.shot(), Shot::Chase);
        assert!(first.eye.y < -3. && first.eye.z < 0., "behind and low: {:?}", first.eye);
        // Ride on until the chase ends: a roadside camera ahead.
        let mut at = Vec3::ZERO;
        let mut placed = first;
        for _ in 0..300 {
            at += v * 0.016;
            placed = cam.update(at, v, 0.016);
        }
        assert_eq!(cam.shot(), Shot::Roadside);
        let post = placed.eye;
        assert!(post.y > at.y, "ahead of the skater");
        // It stays put while they ride past, then cuts.
        for _ in 0..60 {
            at += v * 0.016;
            assert!(cam.update(at, v, 0.016).eye == post || cam.shot() != Shot::Roadside);
        }
        for _ in 0..400 {
            at += v * 0.016;
            cam.update(at, v, 0.016);
        }
        assert_ne!(cam.shot(), Shot::Roadside, "cut once passed");
    }
}
