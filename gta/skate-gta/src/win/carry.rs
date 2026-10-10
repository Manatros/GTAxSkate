//! The board in hand on foot, as a melee weapon: `WEAPON_SKATEBOARD` from
//! the skate3weapon pack (`pack-skate-skater.ps1 -Mode weapon`: the board's
//! own model, the bat's moves, its own place in the weapon wheel's melee
//! slot), drawn by GTA. Without the pack the baseball bat stands in: it is
//! hidden and the script draws the ridden Skate 3 board in the hand. With
//! it out, F starts skate mode, vehicles close or not (another weapon out,
//! F gets into vehicles as always).

use super::natives as n;

/// WEAPON_BAT and WEAPON_UNARMED.
pub const BAT: u32 = 0x958A_4A8F;
pub const UNARMED: u32 = 0xA271_9263;
/// The skate3weapon pack's board.
pub fn skateboard() -> u32 {
    crate::coords::joaat("WEAPON_SKATEBOARD")
}

/// The board's weapon: the pack's when it is loaded, else the bat.
pub fn board_weapon() -> u32 {
    let board = skateboard();
    if n::weapon_valid(board) { board } else { BAT }
}

/// INPUT_ENTER (F).
pub const ENTER: i32 = 23;

#[derive(Default)]
pub struct Carry {
    /// The ped given the board's weapon.
    given_to: i32,
    /// The board's weapon is out: the board is in hand.
    holding: bool,
}

impl Carry {
    /// The board is out as the bat (no pack): for the script to draw.
    pub fn holding(&self) -> bool {
        self.holding && board_weapon() == BAT
    }

    /// The board's weapon is out (the pack's or the bat).
    pub fn holding_board(&self) -> bool {
        self.holding
    }

    /// One frame on foot, not skating. True when F asks to get on the board.
    pub fn tick(&mut self) -> bool {
        let ped = n::player_ped();
        if ped == 0 || n::is_dead(ped) {
            self.put_away();
            return false;
        }
        let weapon = board_weapon();
        if self.given_to != ped {
            if !n::has_weapon(ped, weapon) {
                n::give_weapon(ped, weapon);
            }
            self.given_to = ped;
        }
        if n::selected_weapon(ped) != weapon || !n::is_on_foot(ped) {
            if self.holding {
                n::set_weapon_visible(ped, true);
            }
            self.holding = false;
            return false;
        }
        // The bat never shows: the board is what is held (the pack's board
        // shows itself).
        if weapon == BAT {
            n::set_weapon_visible(ped, false);
            let held = n::weapon_entity(ped);
            if held != 0 {
                n::set_visible(held, false);
            }
        }
        self.holding = true;
        n::draw_text("F:  skate", 0.985, 0.93, 0.36, [255, 255, 255, 230], true);
        // With the board out F is always skating, cars close or not (GTA's
        // getting in won over it near parked cars); put the board away (Tab)
        // to get into a vehicle.
        n::disable_control(0, ENTER);
        if n::disabled_control_just_pressed(0, ENTER) {
            return true;
        }
        false
    }

    /// The board out of the hand (skating, a cutscene).
    pub fn put_away(&mut self) {
        self.holding = false;
    }
}
