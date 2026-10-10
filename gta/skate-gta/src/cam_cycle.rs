//! V while skating cycles the cameras: GTA's own views (near, middle, far
//! and first person, as on foot) and then Skate 3's camera (third, then
//! first person), and round again. Each start of skate mode begins at the
//! default camera (`Camera` in the ini, or the options menu).

use crate::config::CameraMode;

/// GTA's follow camera views (GET_FOLLOW_PED_CAM_VIEW_MODE): 0 near, 1
/// middle, 2 far, 4 first person.
const GTA_VIEWS: [i32; 4] = [0, 1, 2, 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// GTA's camera in one of its views.
    Gta(i32),
    /// Skate 3's camera, first person or not.
    Skate { first: bool },
}

impl View {
    /// The view skate mode starts in: the default camera, GTA's in the view
    /// it is in now.
    pub fn start(default: CameraMode, gta_view: i32) -> Self {
        match default {
            CameraMode::Gta => View::Gta(gta_view),
            CameraMode::Skate => View::Skate { first: false },
        }
    }

    /// The next view (V).
    pub fn next(self) -> Self {
        match self {
            View::Gta(mode) => match GTA_VIEWS.iter().position(|&m| m == mode) {
                Some(i) if i + 1 < GTA_VIEWS.len() => View::Gta(GTA_VIEWS[i + 1]),
                Some(_) => View::Skate { first: false },
                // A view not in the list (a cinematic one): GTA's nearest.
                None => View::Gta(GTA_VIEWS[0]),
            },
            View::Skate { first: false } => View::Skate { first: true },
            View::Skate { first: true } => View::Gta(GTA_VIEWS[0]),
        }
    }

    pub fn camera(self) -> CameraMode {
        match self {
            View::Gta(_) => CameraMode::Gta,
            View::Skate { .. } => CameraMode::Skate,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            View::Gta(0) => "GTA camera (near)",
            View::Gta(1) => "GTA camera (middle)",
            View::Gta(2) => "GTA camera (far)",
            View::Gta(4) => "GTA camera (first person)",
            View::Gta(_) => "GTA camera",
            View::Skate { first: false } => "Skate 3 camera",
            View::Skate { first: true } => "Skate 3 camera (first person)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v_goes_through_gtas_views_then_skates_and_round() {
        let mut v = View::start(CameraMode::Gta, 1);
        let mut seen = vec![v];
        for _ in 0..6 {
            v = v.next();
            seen.push(v);
        }
        assert_eq!(
            seen,
            [
                View::Gta(1),
                View::Gta(2),
                View::Gta(4),
                View::Skate { first: false },
                View::Skate { first: true },
                View::Gta(0),
                View::Gta(1),
            ]
        );
        assert_eq!(View::start(CameraMode::Skate, 2), View::Skate { first: false });
        assert_eq!(View::Gta(3).next(), View::Gta(0));
    }
}
