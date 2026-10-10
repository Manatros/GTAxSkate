//! A GTA-style list menu, drawn with the game's rectangles and text.

use super::natives as n;

pub const LEFT: f32 = 0.03;
pub const WIDTH: f32 = 0.34;
const ROW_H: f32 = 0.034;

/// Draws a titled list of `(label, value)` rows from `first`, `visible` of
/// them, with `selected` highlighted, and a footer line.
pub fn draw_list(title: &str, subtitle: &str, rows: &[(String, String)], selected: usize, first: usize, visible: usize, footer: &str) {
    let centre = LEFT + WIDTH / 2.;
    n::draw_rect(centre, 0.115, WIDTH, 0.07, [12, 12, 12, 235]);
    n::draw_text(title, LEFT + 0.01, 0.087, 0.7, [255, 255, 255, 255], false);
    n::draw_rect(centre, 0.167, WIDTH, ROW_H, [0, 0, 0, 220]);
    n::draw_text(subtitle, LEFT + 0.01, 0.153, 0.32, [93, 182, 229, 255], false);
    let shown = rows.iter().enumerate().skip(first).take(visible);
    let mut count = 0;
    for (i, (label, value)) in shown {
        let y = 0.167 + ROW_H * (i - first + 1) as f32;
        let (back, fore) = if i == selected {
            ([240, 240, 240, 235], [0, 0, 0, 255])
        } else {
            ([0, 0, 0, 170], [245, 245, 245, 255])
        };
        n::draw_rect(centre, y, WIDTH, ROW_H, back);
        n::draw_text(label, LEFT + 0.01, y - 0.013, 0.32, fore, false);
        if !value.is_empty() {
            n::draw_text(value, LEFT + WIDTH - 0.008, y - 0.013, 0.32, fore, true);
        }
        count += 1;
    }
    let footer_y = 0.167 + ROW_H * (count + 1) as f32;
    n::draw_rect(centre, footer_y, WIDTH, ROW_H, [0, 0, 0, 220]);
    n::draw_text(footer, LEFT + 0.01, footer_y - 0.013, 0.27, [200, 200, 200, 255], false);
}

/// The first row to show so `selected` stays in view.
pub fn first_visible(selected: usize, total: usize, visible: usize) -> usize {
    selected.saturating_sub(visible / 2).min(total.saturating_sub(visible))
}
