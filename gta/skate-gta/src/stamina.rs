//! GTA's stamina skill and skating (the maintainer, V 19-54 24:04): skating trains it
//! as sprinting does, and with little of it the push and the pop are
//! weaker.

/// The tuning multipliers stamina scales (`crate::config::TUNING`): pop,
/// push speed, push power and the jump on foot.
pub const SCALED: [usize; 4] = [0, 2, 3, 7];

/// Seconds of skating at speed for one point of stamina (0 to 100).
pub const SECONDS_PER_POINT: f32 = 60.;
/// Slower than this, skating trains nothing.
pub const TRAINING_SPEED: f32 = 3.;

/// How strong the legs are: 85 % with no stamina, all with full.
pub fn strength(stamina: i32) -> f32 {
    0.85 + 0.15 * stamina.clamp(0, 100) as f32 / 100.
}

/// The tuning with stamina's strength on the multipliers it scales.
pub fn apply(mut tuning: [f32; 11], stamina: Option<i32>) -> [f32; 11] {
    if let Some(stamina) = stamina {
        for i in SCALED {
            tuning[i] *= strength(stamina);
        }
    }
    tuning
}

/// The character's stamina stat (`SP<n>_STAMINA`) by their model's name.
pub fn stat_name(model: &str) -> Option<&'static str> {
    match model {
        "player_zero" => Some("SP0_STAMINA"),
        "player_one" => Some("SP1_STAMINA"),
        "player_two" => Some("SP2_STAMINA"),
        _ => None,
    }
}

/// Time skated at speed, counted towards the next point.
#[derive(Default)]
pub struct Training {
    seconds: f32,
}

impl Training {
    /// A frame of `dt` seconds at `speed` m/s on the board; true when it
    /// earned a point.
    pub fn ride(&mut self, dt: f32, speed: f32) -> bool {
        if speed < TRAINING_SPEED || !dt.is_finite() {
            return false;
        }
        self.seconds += dt.min(0.25);
        if self.seconds >= SECONDS_PER_POINT {
            self.seconds -= SECONDS_PER_POINT;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_stamina_weakens_push_and_pop_only() {
        let tuning = apply([1.; 11], Some(0));
        assert!((tuning[0] - 0.85).abs() < 1e-6 && (tuning[3] - 0.85).abs() < 1e-6);
        assert_eq!(tuning[1], 1.);
        assert_eq!(apply([1.; 11], Some(100)), [1.; 11]);
        assert_eq!(apply([1.2; 11], None), [1.2; 11]);
    }

    #[test]
    fn a_minute_of_skating_at_speed_is_a_point() {
        let mut training = Training::default();
        let points = (0..3700).filter(|_| training.ride(1. / 60., 5.)).count();
        assert_eq!(points, 1);
        assert!(!(0..10_000).any(|_| training.ride(0.1, 1.)));
    }
}
