//! Your own Skate 3 skater as a fourth character: the add-on ped
//! `skate-ped` builds, swapped in for the story character (F8, GTA's own
//! "fourth character" key, and the character wheel's fourth slot when the
//! game routes it the same way) and back, dressed from Create-a-Skater's
//! catalog with a Skate-style editor (F11).

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{input, natives as n, panel};
use crate::config::Config;
use crate::skater::{self, Catalog, Outfit, Row};

/// GTA's controls: the character wheel, the three story characters and
/// the fourth (online) one.
const WHEEL: i32 = 19;
const STORY: [i32; 3] = [166, 167, 168];
const FOURTH: i32 = 169;
const UP: i32 = 172;
const DOWN: i32 = 173;
const LEFT: i32 = 174;
const RIGHT: i32 = 175;
const CANCEL: i32 = 202;
const EDITOR_ROWS: usize = 14;

enum State {
    Story,
    Loading { model: u32, since: Instant },
    /// Playing the skater; the story character waits, hidden, where the
    /// swap happened.
    Skater { story: i32, skater: i32 },
}

pub struct Character {
    catalog: Option<Catalog>,
    outfit: Outfit,
    state: State,
    outfit_path: PathBuf,
    key_was_down: bool,
    editor_key_was_down: bool,
    /// The editor's selected row, while it is open.
    editor: Option<usize>,
}

/// What the script must do for a swap.
pub enum Event {
    None,
    /// Get off the board first.
    StopSkating,
}

impl Character {
    pub fn new(dir: &Path) -> (Self, Option<String>) {
        let catalog_path = dir.join("SkateGTA-skater.json");
        let outfit_path = dir.join("SkateGTA-skater.txt");
        let (catalog, problem) = match std::fs::read_to_string(&catalog_path) {
            Ok(text) => match Catalog::from_json(&text) {
                Ok(catalog) => (Some(catalog), None),
                Err(e) => (None, Some(e)),
            },
            Err(_) => (None, None),
        };
        let outfit = match &catalog {
            Some(c) => Outfit::from_text(&std::fs::read_to_string(&outfit_path).unwrap_or_default(), c),
            None => Outfit::default(),
        };
        let me = Self {
            catalog,
            outfit,
            state: State::Story,
            outfit_path,
            key_was_down: false,
            editor_key_was_down: false,
            editor: None,
        };
        (me, problem)
    }

    pub fn editing(&self) -> bool {
        self.editor.is_some()
    }

    pub fn is_skater(&self) -> bool {
        matches!(self.state, State::Skater { .. })
    }

    fn model(&self) -> Option<u32> {
        self.catalog.as_ref().map(|c| crate::coords::joaat(&c.ped))
    }

    /// One frame: the swap key, the loading model, the editor.
    pub fn tick(&mut self, config: &Config, skating: bool, menu_open: bool) -> Event {
        if self.model().is_none() {
            return Event::None;
        }
        // The fourth slot is ours, and while you play the skater the
        // story characters wait for F8.
        n::disable_control(0, FOURTH);
        if self.is_skater() {
            n::disable_control(0, WHEEL);
            for control in STORY {
                n::disable_control(0, control);
            }
        }
        let down = input::vk_down(config.character_key) && input::game_focused();
        let pressed = (down && !self.key_was_down) || n::disabled_control_just_pressed(0, FOURTH);
        self.key_was_down = down;
        if pressed && !menu_open {
            if skating {
                return Event::StopSkating;
            }
            self.toggle();
        }
        if let State::Loading { model, since } = self.state {
            if n::model_loaded(model) {
                self.become_skater(model);
            } else if since.elapsed().as_secs() > 10 {
                n::release_model(model);
                self.state = State::Story;
                n::notify("Skate 3: the skater did not load. Is the skate3 DLC installed?");
            }
        }
        // The game may have put another ped in charge (a death, a mission):
        // let the story character go back to normal.
        if let State::Skater { story, skater } = self.state
            && n::player_ped() != skater
        {
            if n::entity_exists(story) {
                n::set_visible(story, true);
                n::freeze(story, false);
                n::set_collision(story, true);
                n::set_invincible(story, false);
            }
            self.state = State::Story;
            self.editor = None;
        }
        let editor_down = input::vk_down(config.skater_editor_key) && input::game_focused();
        if editor_down && !self.editor_key_was_down && self.is_skater() && !menu_open {
            self.editor = if self.editor.is_some() { None } else { Some(0) };
        }
        self.editor_key_was_down = editor_down;
        if self.editor.is_some() {
            self.editor_tick();
        }
        Event::None
    }

    /// Becomes the skater, or goes back to the story character.
    pub fn toggle(&mut self) {
        let Some(model) = self.model() else { return };
        match self.state {
            State::Story => self.begin(model),
            State::Loading { .. } => {}
            State::Skater { .. } => self.back(),
        }
    }

    fn begin(&mut self, model: u32) {
        let ped = n::player_ped();
        if n::is_dead(ped) || n::in_vehicle(ped) {
            n::notify("Skate 3: get out of the vehicle first.");
            return;
        }
        if !n::model_in_cdimage(model) {
            n::notify("Skate 3: the skater is not installed (see the guide: the skate3 DLC pack).");
            return;
        }
        n::request_model(model);
        self.state = State::Loading { model, since: Instant::now() };
    }

    fn become_skater(&mut self, model: u32) {
        let story = n::player_ped();
        let at = n::entity_coords(story);
        let heading = n::entity_heading(story);
        let skater = n::create_ped(model, at, heading);
        n::release_model(model);
        if skater == 0 {
            self.state = State::Story;
            n::notify("Skate 3: the skater could not be created.");
            return;
        }
        n::keep_entity(story);
        n::keep_entity(skater);
        n::change_player_ped(skater);
        // CHANGE_PLAYER_PED hands back a new handle for the player.
        let skater = n::player_ped();
        self.dress(skater);
        n::set_visible(story, false);
        n::freeze(story, true);
        n::set_collision(story, false);
        n::set_invincible(story, true);
        self.state = State::Skater { story, skater };
        n::notify("Skate 3: your skater. F11 edits, F8 goes back.");
    }

    fn back(&mut self) {
        let State::Skater { story, mut skater } = self.state else { return };
        self.editor = None;
        if !n::entity_exists(story) {
            n::notify("Skate 3: the story character is gone; use the character wheel.");
            self.state = State::Story;
            return;
        }
        let at = n::entity_coords(skater);
        let heading = n::entity_heading(skater);
        n::set_coords(story, at);
        n::set_heading(story, heading);
        n::set_visible(story, true);
        n::freeze(story, false);
        n::set_collision(story, true);
        n::set_invincible(story, false);
        n::change_player_ped(story);
        n::delete_ped(&mut skater);
        self.state = State::Story;
    }

    /// Puts the outfit on the skater.
    fn dress(&self, ped: i32) {
        n::set_ped_default_components(ped);
        for (component, (drawable, texture)) in &self.outfit.worn {
            n::set_ped_component(ped, *component, *drawable, *texture);
        }
    }

    fn editor_tick(&mut self) {
        let (Some(catalog), Some(mut row), State::Skater { skater, .. }) = (self.catalog.as_ref(), self.editor, &self.state) else {
            return;
        };
        let skater = *skater;
        n::disable_player_controls();
        let rows = skater::rows(catalog);
        row = row.min(rows.len() - 1);
        let mut changed = Vec::new();
        if n::control_just_pressed(2, UP) {
            row = (row + rows.len() - 1) % rows.len();
        } else if n::control_just_pressed(2, DOWN) {
            row = (row + 1) % rows.len();
        } else if n::control_just_pressed(2, LEFT) || n::control_just_pressed(2, RIGHT) {
            let dir = if n::control_just_pressed(2, LEFT) { -1 } else { 1 };
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(7);
            changed = skater::change(rows[row], dir, catalog, &mut self.outfit, seed);
        } else if n::control_just_pressed(2, CANCEL) {
            self.editor = None;
            return;
        }
        if !changed.is_empty() {
            for component in changed {
                if let Some((d, t)) = self.outfit.worn.get(&component) {
                    n::set_ped_component(skater, component, *d, *t);
                }
            }
            let _ = std::fs::write(&self.outfit_path, self.outfit.to_text());
        }
        self.editor = Some(row);
        let list: Vec<(String, String)> = rows.iter().map(|r| skater::row_text(*r, catalog, &self.outfit)).collect();
        let hint = match rows[row] {
            Row::Random | Row::Reset => "Left/right: apply   Esc: close",
            _ => "Up/down: pick   Left/right: change   Esc: close",
        };
        panel::draw_list(
            "Skater",
            "Create-a-Skater",
            &list,
            row,
            panel::first_visible(row, list.len(), EDITOR_ROWS),
            EDITOR_ROWS,
            hint,
        );
    }
}
