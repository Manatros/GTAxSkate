//! Skate 3's HUD while skating, as lines of text: the trick being done, the
//! sequence's points and multiplier, a landing's points or the bail, and
//! the line and session totals. (The mashup draws the same in MW2 fonts.)

use skate_host::{MarkerView, ScoreView};

const TRICK_SECONDS: f64 = 2.5;
const LANDING_SECONDS: f64 = 2.0;
const MARKER_SECONDS: f64 = 1.5;

/// A line at screen share `x` (centre, or right edge when `right`) and `y`.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rgba: [u8; 4],
    pub right: bool,
}

impl Line {
    /// Text by the left edge starts at `x` instead of centring on it: the
    /// controls list was centred on 1% of the screen, half of it off the edge.
    pub fn from_left(&self) -> bool {
        !self.right && self.x < 0.2
    }
}

#[derive(Default)]
pub struct Hud {
    tricks: Option<(u32, f64)>,
    landings: Option<(u32, f64)>,
    markers: Option<(u64, f64)>,
    retries: Option<(u32, f64)>,
}

/// The object dropper as the HUD shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DropperView {
    pub open: bool,
    pub object: String,
    /// The objects either side, as LB and RB pick them.
    pub previous: String,
    pub next: String,
    pub placed: usize,
    pub max: usize,
    /// Hints for a controller rather than the keyboard.
    pub pad: bool,
}

pub fn points(value: f32) -> String {
    let digits = (value.max(0.).round() as u64).to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn alpha(since: f64, lasting: f64) -> u8 {
    ((1.0 - ((since - lasting * 0.75) / (lasting * 0.25)).clamp(0.0, 1.0)) * 255.0) as u8
}

impl Hud {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Skate 3's session marker: its prompt while LB is held, "Marker set",
    /// and the hold to go back to it.
    pub fn marker_lines(&mut self, marker: &MarkerView, keys: &skate_input::Controls, pad: bool, now: f64) -> Vec<Line> {
        match self.markers {
            Some((seen, _)) if seen == marker.placements => {}
            Some(_) => self.markers = Some((marker.placements, now)),
            None => self.markers = Some((marker.placements, f64::NEG_INFINITY)),
        }
        let line = |text: String, y: f32, rgba: [u8; 4]| Line { text, x: 0.5, y, scale: 0.4, rgba, right: false };
        let mut lines = Vec::new();
        if marker.progress > 0. {
            lines.push(line(format!("Back to the marker {:.0}%", marker.progress.clamp(0., 1.) * 100.), 0.42, [255, 255, 255, 230]));
        } else if marker.prompt {
            let set = skate_input::hints::one(keys, pad, "LB + Down", "marker_place");
            let return_key = skate_input::hints::one(keys, pad, "Hold LB + Up", "marker_return");
            let place = if marker.can_place { format!("[{set}] Set marker") } else { "No marker here".into() };
            let back = if marker.placed { format!("   [{return_key}] Back to it") } else { String::new() };
            lines.push(line(format!("{place}{back}"), 0.42, [255, 255, 255, 230]));
        }
        let age = self.markers.map_or(f64::INFINITY, |(_, at)| now - at);
        if age < MARKER_SECONDS {
            lines.push(line("Marker set".into(), 0.45, [128, 217, 255, alpha(age, MARKER_SECONDS)]));
        }
        lines
    }

    /// The object dropper's panel, and a flash on going back to the
    /// checkpoint (`retries` counts those).
    pub fn dropper_lines(&mut self, dropper: &DropperView, retries: u32, keys: &skate_input::Controls, now: f64) -> Vec<Line> {
        match self.retries {
            Some((seen, _)) if seen == retries => {}
            Some(_) => self.retries = Some((retries, now)),
            None => self.retries = Some((retries, f64::NEG_INFINITY)),
        }
        let line = |text: String, y: f32, scale: f32, rgba: [u8; 4]| Line { text, x: 0.5, y, scale, rgba, right: false };
        let mut lines = Vec::new();
        let age = self.retries.map_or(f64::INFINITY, |(_, at)| now - at);
        if age < MARKER_SECONDS {
            lines.push(line("Back to the checkpoint".into(), 0.45, 0.4, [128, 217, 255, alpha(age, MARKER_SECONDS)]));
        }
        if dropper.open {
            lines.push(line("OBJECT DROPPER".into(), 0.62, 0.4, [255, 217, 77, 255]));
            lines.push(line(
                format!("<  {}      {}      {}  >", dropper.previous, dropper.object.to_uppercase(), dropper.next),
                0.655,
                0.6,
                [255, 255, 255, 255],
            ));
            lines.push(line(format!("{} of {} placed", dropper.placed, dropper.max), 0.705, 0.35, [200, 205, 220, 230]));
            let hints = skate_input::hints::dropper(keys, dropper.pad);
            for (row, pair) in hints.chunks(2).enumerate() {
                let text = pair.iter().map(|(key, what)| format!("[{key}] {what}")).collect::<Vec<_>>().join("        ");
                lines.push(line(text, 0.74 + row as f32 * 0.03, 0.35, [255, 255, 255, 240]));
            }
        }
        lines
    }

    /// The controls while skating, down the left side, on the player's own
    /// keys (or the controller's buttons).
    pub fn controls_lines(&self, keys: &skate_input::Controls, pad: bool, on_foot: bool) -> Vec<Line> {
        skate_input::hints::skating(keys, pad, on_foot)
            .into_iter()
            .enumerate()
            .map(|(i, (key, what))| Line {
                text: format!("[{key}] {what}"),
                x: 0.012,
                y: 0.5 + i as f32 * 0.021,
                scale: 0.24,
                rgba: [255, 255, 255, 190],
                right: false,
            })
            .collect()
    }

    pub fn lines(&mut self, score: &ScoreView, now: f64) -> Vec<Line> {
        let seen = |slot: &mut Option<(u32, f64)>, count: u32| match *slot {
            Some((c, _)) if c == count => {}
            Some(_) => *slot = Some((count, now)),
            None => *slot = Some((count, f64::NEG_INFINITY)),
        };
        seen(&mut self.tricks, score.tricks);
        seen(&mut self.landings, score.landings);
        let mut lines = Vec::new();
        let age = self.tricks.map_or(f64::INFINITY, |(_, at)| now - at);
        if age < TRICK_SECONDS && !score.trick.is_empty() {
            // Skate 3's names for its tricks, not the data's keys.
            let trick = skate_rules::trick_title(score.trick_id, &score.trick);
            let mut name = String::new();
            for (on, word) in [(score.switch, "Switch "), (score.fakie, "Fakie "), (score.nollie, "Nollie ")] {
                if on && !trick.starts_with(word) {
                    name.push_str(word);
                }
            }
            name.push_str(&trick);
            let a = alpha(age, TRICK_SECONDS);
            lines.push(Line { text: name, x: 0.5, y: 0.2, scale: 0.75, rgba: [255, 255, 255, a], right: false });
            if score.sequence > 0. {
                let text = if score.multiplier > 1.01 {
                    format!("{}  x{:.1}", points(score.sequence), score.multiplier)
                } else {
                    points(score.sequence)
                };
                lines.push(Line { text, x: 0.5, y: 0.25, scale: 0.5, rgba: [255, 216, 77, a], right: false });
            }
        }
        let age = self.landings.map_or(f64::INFINITY, |(_, at)| now - at);
        if age < LANDING_SECONDS {
            let a = alpha(age, LANDING_SECONDS);
            let line = if score.bailed {
                ("BAIL".to_owned(), [255, 64, 51, a])
            } else {
                let quality = if score.clean {
                    "  Clean!"
                } else if score.sketchy {
                    "  Sketchy"
                } else {
                    ""
                };
                (format!("+{}{quality}", points(score.last_reward)), [115, 255, 115, a])
            };
            lines.push(Line { text: line.0, x: 0.5, y: 0.29, scale: 0.5, rgba: line.1, right: false });
        }
        if score.line > 0. {
            lines.push(Line {
                text: format!("LINE {}", points(score.line)),
                x: 0.97,
                y: 0.06,
                scale: 0.45,
                rgba: [255, 255, 255, 230],
                right: true,
            });
        }
        let session = score.completed + score.line;
        if session > 0. {
            lines.push(Line {
                text: format!("SESSION {}", points(session)),
                x: 0.97,
                y: 0.095,
                scale: 0.45,
                rgba: [204, 230, 255, 230],
                right: true,
            });
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tricks_show_and_old_ones_do_not() {
        let mut hud = Hud::default();
        let mut score = ScoreView { trick: "STR_HEELFLIP".into(), trick_id: "heelflip", tricks: 3, landings: 1, line: 500., ..Default::default() };
        assert!(hud.lines(&score, 0.).iter().all(|l| l.text != "Heelflip"));
        score.tricks = 4;
        score.fakie = true;
        assert_eq!(hud.lines(&score, 1.)[0].text, "Fakie Heelflip");
        score.landings = 2;
        score.bailed = true;
        assert!(hud.lines(&score, 1.5).iter().any(|l| l.text == "BAIL"));
        assert!(hud.lines(&score, 9.).iter().all(|l| l.right));
    }

    #[test]
    fn the_dropper_shows_its_object_and_the_checkpoint_flashes() {
        let mut hud = Hud::default();
        let keys = skate_input::Controls::default();
        let mut view = DropperView {
            open: true,
            object: "Kicker".into(),
            previous: "Crate".into(),
            next: "Bench".into(),
            placed: 1,
            max: 24,
            pad: false,
        };
        let lines = hud.dropper_lines(&view, 0, &keys, 0.);
        assert_eq!(lines[1].text, "<  Crate      KICKER      Bench  >");
        assert!(lines[3].text.starts_with("[W A S D] Move"), "{}", lines[3].text);
        view.open = false;
        assert_eq!(hud.dropper_lines(&view, 1, &keys, 0.5)[0].text, "Back to the checkpoint");
        assert!(hud.dropper_lines(&view, 1, &keys, 5.).is_empty());
        assert!(hud.controls_lines(&keys, true, false).iter().any(|l| l.text == "[Space / A] Push"));
    }

    #[test]
    fn the_session_marker_prompts_and_flashes() {
        let mut hud = Hud::default();
        let keys = skate_input::Controls::default();
        let mut marker = MarkerView { prompt: true, can_place: true, ..Default::default() };
        assert_eq!(hud.marker_lines(&marker, &keys, false, 0.)[0].text, "[N] Set marker");
        marker.placements = 1;
        marker.placed = true;
        assert!(hud.marker_lines(&marker, &keys, true, 0.5).iter().any(|l| l.text == "Marker set"));
        marker.prompt = false;
        assert!(hud.marker_lines(&marker, &keys, true, 5.).is_empty());
    }
}
