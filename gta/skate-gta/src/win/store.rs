//! The skateboard counter in GTA's clothing stores, with the board shown in
//! 3D beside the menu, turning over as in Skate 3's customiser, and the wear
//! of the parts the player rides.

use super::natives as n;
use crate::board_look::{BoardLook, Rest, showcase};
use crate::config::Config;
use crate::coords::{joaat, to_skate};
use crate::shop::{Action, Garage, Goods, Input, Menu, Offer, Tier, VISIBLE, stock};
use skate_boards::wear::{Usage, Wear};
use skate_boards::{Board, BoardChoice, Library, Part};
use std::path::{Path, PathBuf};

/// GTA's controls (group 2: menus). The board counter opens with
/// INPUT_DETONATE (G, or the D-pad's left): E (context) is the store's own
/// clothing counter, which stays GTA's.
const OPEN: i32 = 47;
const UP: i32 = 172;
const DOWN: i32 = 173;
const LEFT: i32 = 174;
const RIGHT: i32 = 175;
const ACCEPT: i32 = 201;
const CANCEL: i32 = 202;
const RESET: i32 = 203;

pub struct Store {
    path: PathBuf,
    garage: Garage,
    menu: Option<Menu>,
    /// How worn the parts the player rode are.
    wear: Wear,
    wear_path: PathBuf,
    /// The board shown in 3D: what it shows, and the board at rest.
    preview: Option<(BoardChoice, Option<(BoardLook, Rest)>)>,
    /// When the preview opened, for its turn.
    opened: std::time::Instant,
    /// Your character holds the board (the script poses them) rather than
    /// the board floating beside the menu; Tab (Y) switches.
    pub skater_view: bool,
    /// The preview board's turn, from the mouse (radians, the drawn one).
    turn: f32,
    /// The HD preview's turn in the view's axes, from the mouse while the
    /// left button is held.
    orient: bevy::math::Quat,
    /// The script shows the character: no floating board.
    pub posed: bool,
    /// The preview as the pack's HD part models, when it has them.
    parts: super::board_parts::Parts,
    /// The preview was shown this frame (else its models go).
    previewed: bool,
    /// Boards on show against a wall of the store the player is in: each
    /// one's place and look, and its models.
    display: Vec<(bevy::math::Mat4, BoardChoice, super::board_parts::Parts)>,
    /// When the store's walls are looked at next (until boards are on
    /// show: the player walks in from the door), and whether a miss was
    /// logged this visit.
    display_next: Option<std::time::Instant>,
    display_missed: bool,
    /// Lines for the plugin's log.
    pub notes: Vec<String>,
    /// The ridden deck's scuff stage.
    scuff: Option<u8>,
    /// The middle of this store's boards on show, while they stand.
    show_at: Option<bevy::math::Vec3>,
    /// Every store's boards on show, kept (`SkateGTA-display.txt`).
    shows: Vec<crate::shop::ShowCase>,
    shows_path: PathBuf,
}

/// Boards on show in a store.
const ON_SHOW: usize = 5;
/// How near the boards on show the prompt to look at them comes (m): they
/// may hang behind the counter.
const SHOW_REACH: f32 = 3.5;
/// INPUT_CONTEXT (E): at the boards on show, as at the store's other goods.
const BROWSE: i32 = 51;

/// GTA's control switching the view (INPUT_FRONTEND_Y: Tab, or Y).
const VIEW: i32 = 204;


/// The cash stat of the character `model` is, for Michael, Franklin and
/// Trevor.
fn cash_stat(model: u32) -> Option<u32> {
    [("player_zero", "SP0_TOTAL_CASH"), ("player_one", "SP1_TOTAL_CASH"), ("player_two", "SP2_TOTAL_CASH")]
        .iter()
        .find(|(name, _)| joaat(name) == model)
        .map(|(_, stat)| joaat(stat))
}

fn money(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("${out}")
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        let path = dir.join("SkateGTA-boards.txt");
        let garage = std::fs::read_to_string(&path).map(|t| Garage::from_text(&t)).unwrap_or_default();
        let wear_path = dir.join("SkateGTA-wear.txt");
        let wear = std::fs::read_to_string(&wear_path).map(|t| Wear::from_text(&t)).unwrap_or_default();
        let shows_path = dir.join("SkateGTA-display.txt");
        let shows = std::fs::read_to_string(&shows_path).map(|t| crate::shop::shows_from_text(&t)).unwrap_or_default();
        Self {
            path,
            garage,
            menu: None,
            wear,
            wear_path,
            preview: None,
            opened: std::time::Instant::now(),
            skater_view: false,
            turn: 0.,
            orient: crate::shop::preview_start(),
            posed: false,
            parts: Default::default(),
            previewed: false,
            display: Vec::new(),
            display_next: None,
            display_missed: false,
            notes: Vec::new(),
            scuff: None,
            show_at: None,
            shows,
            shows_path,
        }
    }

    /// The board the menu shows now, when it is open.
    pub fn shown_board(&self, library: Option<&Library>) -> Option<BoardChoice> {
        let offer = self.menu.as_ref()?.current()?;
        match library {
            Some(library) => Some(self.shown(offer, library)),
            None => Some(self.garage.riding.clone()),
        }
    }

    /// The board ridden, as bought.
    pub fn riding(&self) -> &BoardChoice {
        &self.garage.riding
    }

    /// The board ridden as it looks now: worn-through parts in their worn
    /// finish.
    pub fn ridden_look(&self, library: Option<&Library>) -> BoardChoice {
        match library {
            Some(library) => self.wear.look(&library.fresh(&self.garage.riding), library),
            None => self.garage.riding.clone(),
        }
    }

    /// Riding wears the board; true when a part wore through.
    pub fn wear_board(&mut self, library: Option<&Library>, usage: &Usage) -> bool {
        let board = library.map_or_else(|| self.garage.riding.clone(), |l| l.fresh(&self.garage.riding));
        self.wear.ride(&board, usage)
    }

    pub fn save_wear(&self) {
        let _ = std::fs::write(&self.wear_path, self.wear.to_text());
    }

    pub fn open(&self) -> bool {
        self.menu.is_some()
    }

    fn save(&self) {
        let _ = std::fs::write(&self.path, self.garage.to_text());
    }

    fn tier_here(config: &Config, ped: i32) -> Option<Tier> {
        let hash = n::interior_name(ped)?;
        // Boards are street gear: Binco, the Discount Store and Suburban
        // sell them, Ponsonbys does not.
        config
            .shop_interiors
            .iter()
            .find(|(name, _)| joaat(name) == hash)
            .map(|(_, tier)| *tier)
            .filter(|tier| tier.sells_boards())
    }

    /// The store's clerk, if one stands near.
    fn clerk(ped: i32) -> Option<i32> {
        let models = [joaat("s_f_y_shop_low"), joaat("s_f_y_shop_mid"), joaat("s_m_y_shop_mask")];
        let at = n::entity_coords(ped);
        super::shv::all_peds()
            .into_iter()
            .filter(|p| *p != ped && n::entity_exists(*p) && !n::is_dead(*p) && models.contains(&n::entity_model(*p)))
            .filter(|p| n::entity_coords(*p).distance(at) < 20.)
            .min_by(|a, b| n::entity_coords(*a).distance(at).total_cmp(&n::entity_coords(*b).distance(at)))
    }

    fn clear_display(&mut self) {
        for (_, _, parts) in &mut self.display {
            parts.clear();
        }
        self.display.clear();
        self.show_at = None;
        self.display_next = None;
        self.display_missed = false;
    }

    /// Boards from the stock on show against a flat stretch of the store's
    /// wall, as GTA's own models: found once (lines cast round the player,
    /// again every 1.5 s while none is found) and kept for good, the same
    /// wall and the same decks every visit.
    fn show_display(&mut self, library: Option<&Library>, ped: i32, tier: Tier) {
        let now = std::time::Instant::now();
        if self.display.is_empty() && self.display_next.is_none_or(|at| now >= at) {
            // The library loads a moment after the game: until then, try again.
            let Some(library) = library else { return };
            self.display_next = Some(now + std::time::Duration::from_millis(1500));
            if !super::board_parts::available() {
                self.display_next = Some(now + std::time::Duration::from_secs(3600));
                self.notes.push("store: no boards on show (the skate3weapon pack has no part models)".into());
                return;
            }
            let Some(store) = n::interior_place(ped).map(crate::shop::store_key) else { return };
            let show = match self.shows.iter().find(|s| s.store == store) {
                Some(kept) => kept.clone(),
                None => {
                    let eye = n::entity_coords(ped) + bevy::math::Vec3::Z * 0.3;
                    let probe = |a: bevy::math::Vec3, b: bevy::math::Vec3| n::probe_line(a, b, 1 | 16, ped).map(|(p, _)| p);
                    // Behind the till first: the clerk stands before that wall.
                    let till = Self::clerk(ped).and_then(|clerk| {
                        let m = n::entity_matrix(clerk);
                        let from = m.w_axis.truncate() + bevy::math::Vec3::Z * 0.3;
                        crate::shop::wall_behind(from, m.y_axis.truncate(), |a, b| n::probe_line(a, b, 1 | 16, clerk).map(|(p, _)| p))
                    });
                    let found = till.map(|w| (w, true)).or_else(|| crate::shop::display_wall(eye, probe).map(|w| (w, false)));
                    let Some(((at, normal), till)) = found else {
                        if !std::mem::replace(&mut self.display_missed, true) {
                            self.notes.push(format!("store: no free wall for boards round {eye} yet"));
                        }
                        return;
                    };
                    let above = at + normal * 0.3;
                    let floor = n::probe_line(above, above - bevy::math::Vec3::Z * 4., 1 | 16, ped).map_or(eye.z - 1.3, |(p, _)| p.z);
                    let show = crate::shop::ShowCase { store, at, normal, floor, till };
                    self.shows.push(show.clone());
                    let _ = std::fs::write(&self.shows_path, crate::shop::shows_to_text(&self.shows));
                    self.notes.push(format!("store {store:?}: boards on show placed at {at} ({}), kept", if till { "behind the till" } else { "along a wall" }));
                    show
                }
            };
            // The store's complete boards (deck, trucks, wheels), new ones
            // each game day; the same all day.
            let completes: Vec<BoardChoice> = crate::shop::stock(library, tier)
                .iter()
                .flat_map(|section| section.offers.iter())
                .filter_map(|offer| match &offer.goods {
                    crate::shop::Goods::Complete(board) => Some(board.clone()),
                    crate::shop::Goods::Part(..) | crate::shop::Goods::Grip(_) | crate::shop::Goods::TruckPaint(..) => None,
                })
                .collect();
            let boards = crate::shop::show_picks(&completes, store, n::game_day(), ON_SHOW);
            self.show_at = Some(show.at);
            for (place, choice) in show.places(boards.len()).into_iter().zip(boards) {
                self.display.push((place, choice, Default::default()));
            }
        }
        for (place, choice, parts) in &mut self.display {
            parts.show(choice, &Default::default(), super::board_parts::Place::At(*place));
        }
    }

    /// Runs the counter for this frame. True when the board ridden changed.
    pub fn tick(&mut self, config: &Config, library: Option<&Library>, defaults: Option<&Board>, skating: bool) -> bool {
        self.previewed = false;
        let changed = self.counter(config, library, defaults, skating);
        if !self.previewed && self.parts.shown() {
            self.parts.clear();
        }
        changed
    }

    fn counter(&mut self, config: &Config, library: Option<&Library>, defaults: Option<&Board>, skating: bool) -> bool {
        let ped = n::player_ped();
        let here = (config.shops && !skating && !n::is_dead(ped) && !n::in_vehicle(ped))
            .then(|| Self::tier_here(config, ped))
            .flatten();
        let Some(tier) = here else {
            self.menu = None;
            self.clear_display();
            return false;
        };
        self.show_display(library, ped, tier);
        if self.menu.is_none() {
            // At the boards on show: GTA's own prompt, as at the store's
            // other goods, and only there. A store without them: a line at the
            // bottom, and the boards open anywhere in it.
            // G opens the boards anywhere in the store (a line at the
            // bottom says so); at the boards on show E does too, with GTA's
            // own prompt, as at the store's other goods.
            let at_show = self.show_at.is_some_and(|at| n::entity_coords(ped).truncate().distance(at.truncate()) < SHOW_REACH);
            if at_show {
                n::disable_control(0, BROWSE);
                n::help("~INPUT_CONTEXT~ Skate 3 boards: complete or build your own");
            } else {
                n::draw_text("G / D-pad left:  Skate 3 boards (complete or build your own)", 0.985, 0.93, 0.36, [255, 255, 255, 230], true);
            }
            let near = n::control_just_pressed(0, OPEN) || (at_show && n::disabled_control_just_pressed(0, BROWSE));
            if near {
                match library {
                    None => n::notify(
                        "Skate 3: the skateboard catalog is not prepared. Start the mashup once and accept the board catalog.",
                    ),
                    Some(library) => {
                        let sections = stock(library, tier);
                        if sections.is_empty() {
                            n::notify("Skate 3: nothing in stock.");
                        } else {
                            self.menu = Some(Menu::new(tier, sections));
                            self.opened = std::time::Instant::now();
                            n::sound("SELECT", "HUD_FRONTEND_DEFAULT_SOUNDSET");
                        }
                    }
                }
            }
            return false;
        }
        n::disable_player_controls();
        // GTA's idle camera turned the view (and the preview with it) while
        // the player stood choosing.
        n::invalidate_idle_camera();
        let input = [
            (UP, Input::Up),
            (DOWN, Input::Down),
            (LEFT, Input::Left),
            (RIGHT, Input::Right),
            (ACCEPT, Input::Accept),
            (CANCEL, Input::Back),
            (RESET, Input::Reset),
        ]
        .into_iter()
        .find(|(control, _)| n::control_just_pressed(2, *control))
        .map(|(_, input)| input);
        if n::control_just_pressed(2, VIEW) {
            self.skater_view = !self.skater_view;
            n::sound("NAV_LEFT_RIGHT", "HUD_FRONTEND_DEFAULT_SOUNDSET");
        }
        let mut changed = false;
        if let Some(input) = input {
            let action = self.menu.as_mut().map_or(Action::None, |menu| menu.handle(input, &self.garage));
            if matches!(input, Input::Up | Input::Down | Input::Left | Input::Right) {
                n::sound("NAV_UP_DOWN", "HUD_FRONTEND_DEFAULT_SOUNDSET");
            }
            changed = self.act(action);
        }
        self.draw(library, defaults);
        changed
    }

    fn act(&mut self, action: Action) -> bool {
        match action {
            Action::None => false,
            Action::Close => {
                self.menu = None;
                n::sound("BACK", "HUD_FRONTEND_DEFAULT_SOUNDSET");
                false
            }
            Action::Ride(offer) => {
                self.garage.ride(&offer.goods);
                self.save();
                n::sound("SELECT", "HUD_FRONTEND_DEFAULT_SOUNDSET");
                n::notify(&format!("Skate 3: riding {}.", offer.title));
                true
            }
            Action::RideDefault => {
                self.garage.ride_default();
                self.save();
                n::notify("Skate 3: riding the default board.");
                true
            }
            Action::Buy(offer) => self.buy(&offer),
        }
    }

    fn buy(&mut self, offer: &Offer) -> bool {
        let Some(stat) = cash_stat(n::entity_model(n::player_ped())) else {
            n::notify("Skate 3: only Michael, Franklin and Trevor can pay here.");
            return false;
        };
        let cash = n::stat_int(stat).unwrap_or(0);
        if i64::from(cash) < i64::from(offer.price) {
            n::sound("ERROR", "HUD_FRONTEND_DEFAULT_SOUNDSET");
            n::notify(&format!("Skate 3: {} costs {}. You have {}.", offer.title, money(offer.price), money(cash.max(0) as u32)));
            return false;
        }
        n::set_stat_int(stat, cash - offer.price as i32);
        // A new one of a part ridden: its wear starts over.
        match &offer.goods {
            Goods::Grip(_) | Goods::TruckPaint(..) => {}
            Goods::Part(_, value) => self.wear.renew(value),
            Goods::Complete(board) => {
                for value in Part::ALL.iter().filter_map(|&part| board.get(part)) {
                    self.wear.renew(value);
                }
            }
        }
        self.save_wear();
        self.garage.add(&offer.goods);
        self.garage.ride(&offer.goods);
        self.save();
        n::sound("PURCHASE", "HUD_LIQUOR_STORE_SOUNDSET");
        n::notify(&format!("Skate 3: bought {} for {}. Riding it now.", offer.title, money(offer.price)));
        true
    }

    /// The board the menu shows: the one ridden with the hovered part on
    /// it, or the hovered complete board, as worn as the parts ridden are.
    fn shown(&self, offer: &Offer, library: &Library) -> BoardChoice {
        let mut board = library.fresh(&self.garage.riding);
        match &offer.goods {
            Goods::Part(part, value) => board.set(*part, Some(value.clone())),
            Goods::Complete(complete) => board = complete.clone(),
            Goods::Grip(_) | Goods::TruckPaint(..) => {}
        }
        self.wear.look(&board, library)
    }

    /// The griptape the menu shows: the hovered one, else the one ridden.
    fn shown_grip(&self, offer: &Offer) -> Option<String> {
        match &offer.goods {
            Goods::Grip(id) => Some(id.clone()),
            _ => self.garage.grip.clone(),
        }
    }

    /// What the menu shows laid over the board now: the hovered griptape or
    /// truck paint, else those ridden.
    pub fn extras_shown(&self) -> super::board_parts::Extras {
        let mut extras = self.extras();
        match self.menu.as_ref().and_then(|m| m.current()).map(|o| &o.goods) {
            Some(Goods::Grip(id)) => extras.grip = Some(id.clone()),
            Some(Goods::TruckPaint(skate_boards::paints::TruckPart::Base, id)) => extras.truck_base = Some(id.clone()),
            Some(Goods::TruckPaint(skate_boards::paints::TruckPart::Hanger, id)) => extras.truck_hanger = Some(id.clone()),
            // A catalog truck hovered shows as it is.
            Some(Goods::Part(Part::Trucks, _)) => {
                extras.truck_base = None;
                extras.truck_hanger = None;
            }
            _ => {}
        }
        extras
    }

    /// What is laid over the board ridden.
    pub fn extras(&self) -> super::board_parts::Extras {
        super::board_parts::Extras {
            grip: self.garage.grip.clone(),
            truck_base: self.garage.truck_base.clone(),
            truck_hanger: self.garage.truck_hanger.clone(),
            scuff: self.scuff,
        }
    }

    /// The ridden deck's scuff stage, from its wear (see `wear::scuff_stage`);
    /// set whenever the wear changes.
    pub fn update_scuff(&mut self, library: Option<&Library>) {
        let deck = self.garage.riding.get(Part::Deck).map(|d| library.map_or_else(|| d.clone(), |l| l.fresh_of(Part::Deck, d)));
        self.scuff = deck.and_then(|d| skate_boards::wear::scuff_stage(self.wear.of(&d)));
    }

    /// The griptape on the board ridden (none: Skate's own).
    pub fn grip(&self) -> Option<&str> {
        self.garage.grip.as_deref()
    }

    /// Draws the shown board in 3D beside the menu, turning over.
    fn draw_preview(&mut self, offer: &Offer, library: Option<&Library>, defaults: Option<&Board>) {
        let (Some(library), Some(defaults)) = (library, defaults) else { return };
        let wanted = self.shown(offer, library);
        // Turned with the mouse while the left button is held (GTA's look
        // input and attack, read while the store holds the controls): a
        // mouse moved by accident leaves it be.
        let (sideways, up_down) = if n::disabled_control_pressed(0, 24) {
            (n::disabled_control_normal(0, 1), n::disabled_control_normal(0, 2))
        } else {
            (0., 0.)
        };
        self.turn += sideways * 0.25;
        self.orient = crate::shop::trackball(self.orient, sideways, up_down);
        if super::board_parts::available() {
            // GTA's own models of the parts: the board turns over about its
            // length (across the view), from a three-quarter view of its
            // graphic. The models lie along Z from the grip, the deck facing
            // -X; their middle is 0.35 m along.
            let eye = n::camera_coord();
            let (forward, right, up) = n::camera_axes();
            // Beside the character, clear of it, far enough not to fill the
            // view when the store's camera comes close.
            let origin = eye + forward * 1.6 + right * 0.5 + up * 0.05;
            let turn = bevy::math::Mat3::from_cols(forward, up, right) * bevy::math::Mat3::from_quat(self.orient);
            let at = bevy::math::Mat4::from_translation(origin) * bevy::math::Mat4::from_mat3(turn) * bevy::math::Mat4::from_translation(bevy::math::Vec3::new(0., 0., -0.35));
            let extras = self.extras_shown();
            self.parts.show(&wanted, &extras, super::board_parts::Place::At(at));
            self.previewed = true;
            return;
        }
        if self.preview.as_ref().is_none_or(|(shown, _)| *shown != wanted) {
            let built = skate_boards::assemble(Some(library), defaults, &wanted, 256).ok().map(|board| {
                let look = BoardLook::new(&board, 24, 1800);
                let rest = look.rest(defaults);
                (look, rest)
            });
            self.preview = Some((wanted, built));
        }
        let Some((_, Some((look, rest)))) = self.preview.as_ref() else { return };
        let eye = n::camera_coord();
        let (forward, right, up) = n::camera_axes();
        // To the right of the menu, a little over a metre away.
        let origin = eye + forward * 1.25 + right * 0.18 + up * 0.04;
        let angle = skate_boards::preview::angle_at(0.) + self.turn;
        let bones = showcase(rest, to_skate(right), to_skate(up), to_skate(-forward), angle);
        n::backface_culling(false);
        for poly in look.polys(&bones, &rest.bound, origin, eye, 1.) {
            n::draw_poly(poly.points, poly.rgba);
        }
    }

    fn draw(&mut self, library: Option<&Library>, defaults: Option<&Board>) {
        let Some(menu) = self.menu.as_ref() else { return };
        let (left, width) = (0.03, 0.30);
        let centre = left + width / 2.;
        let row_h = 0.034;
        n::draw_rect(centre, 0.115, width, 0.07, [12, 12, 12, 235]);
        n::draw_text(menu.tier.store(), left + 0.01, 0.087, 0.7, [255, 255, 255, 255], false);
        let section = menu.sections.get(menu.section);
        let lines = menu.lines();
        let count = lines.len();
        let title = match &menu.group {
            Some((group, _)) => format!("{} / {}", section.map_or("", |s| s.title.as_str()), group),
            None => section.map_or(String::new(), |s| s.title.clone()),
        };
        let subtitle = format!("SKATEBOARDS  <  {}  >   {}/{}", title.to_uppercase(), if count == 0 { 0 } else { menu.row + 1 }, count);
        n::draw_rect(centre, 0.167, width, row_h, [0, 0, 0, 220]);
        n::draw_text(&subtitle, left + 0.01, 0.153, 0.32, [93, 182, 229, 255], false);
        let first = menu.first_visible();
        for (i, line) in lines.iter().enumerate().skip(first).take(VISIBLE) {
            let y = 0.167 + row_h * (i - first + 1) as f32;
            let selected = i == menu.row;
            let (back, fore) = if selected {
                ([240, 240, 240, 235], [0, 0, 0, 255])
            } else {
                ([0, 0, 0, 170], [245, 245, 245, 255])
            };
            n::draw_rect(centre, y, width, row_h, back);
            let offer = match line {
                crate::shop::Line::Group(name, count) => {
                    n::draw_text(name, left + 0.01, y - 0.013, 0.32, fore, false);
                    n::draw_text(&format!("{count}  >"), left + width - 0.008, y - 0.013, 0.32, fore, true);
                    continue;
                }
                crate::shop::Line::Offer(offer) => offer,
            };
            n::draw_text(&offer.title, left + 0.01, y - 0.013, 0.32, fore, false);
            let tag = if self.garage.rides(&offer.goods) {
                "Riding".to_owned()
            } else if self.garage.owns(&offer.goods) {
                "Owned".to_owned()
            } else {
                money(offer.price)
            };
            n::draw_text(&tag, left + width - 0.008, y - 0.013, 0.32, fore, true);
        }
        let footer_y = 0.167 + row_h * (VISIBLE.min(count) + 1) as f32;
        n::draw_rect(centre, footer_y, width, row_h, [0, 0, 0, 220]);
        let cash = cash_stat(n::entity_model(n::player_ped())).and_then(n::stat_int).unwrap_or(0);
        n::draw_text(
            &format!("Cash {}   Enter: open/buy/ride  Back: up  Space: default  Tab: view  Hold LMB + mouse: turn", money(cash.max(0) as u32)),
            left + 0.01,
            footer_y - 0.013,
            0.28,
            [200, 200, 200, 255],
            false,
        );
        let hovered = menu.current().cloned();
        let ridden = hovered.as_ref().is_some_and(|offer| self.garage.rides(&offer.goods));
        if let Some(offer) = hovered.as_ref() {
            // How worn the part ridden is, under the list.
            if ridden && let Goods::Part(_, value) = &offer.goods {
                let wear = self.wear.of(value);
                n::draw_text(
                    &format!("Worn {:.0}%: buy it again for a new one", wear.min(1.) * 100.),
                    left + 0.01,
                    footer_y + row_h - 0.013,
                    0.28,
                    [255, 210, 120, 255],
                    false,
                );
            }
            if !self.posed {
                self.draw_preview(offer, library, defaults);
            }
        }
    }
}
