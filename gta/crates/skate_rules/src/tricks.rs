//! The names Skate 3's HUD gives its tricks, from the engine's scorable
//! identifiers (`kickflip2`, `n_360flip`, `bs_crook_tailgrab`...): the
//! game's own names come from its localisation tables, which the converted
//! data does not carry, so the HUD would otherwise show the identifiers.

/// A grab's name.
fn grab(id: &str) -> Option<&'static str> {
    Some(match id {
        "fsgrab" => "Indy",
        "bsgrab" => "Melon",
        "mutegrab" => "Mute",
        "stalegrab" => "Stalefish",
        "nosegrab" => "Nosegrab",
        "tailgrab" => "Tailgrab",
        "dblgrab" => "Double Grab",
        "crailgrab" | "crail" => "Crail Grab",
        "seatbeltgrab" | "seatbelt" => "Seatbelt",
        "christ_air" => "Christ Air",
        "coffin" => "Coffin",
        "rocketair" => "Rocket Air",
        "superman" => "Superman",
        "fsgrabvarial" => "Indy Varial",
        "bsgrabvarial" => "Melon Varial",
        "stale" => "Stalefish",
        "mute" => "Mute",
        "nose" => "Nosegrab",
        "tail" => "Tailgrab",
        _ => return None,
    })
}

/// A grind or slide's name, without its side.
fn grind(id: &str) -> Option<&'static str> {
    Some(match id {
        "5_o" | "grind5_o" => "5-0 Grind",
        "50_50" | "grind5050" => "50-50 Grind",
        "blunt" | "slideblunt" => "Bluntslide",
        "board" | "slideboard" => "Boardslide",
        "crook" | "grindcrooks" => "Crooked Grind",
        "feeble" | "grindfeeble" => "Feeble Grind",
        "lip" | "slidelip" => "Lipslide",
        "noseblunt" | "slidenblunt" => "Noseblunt Slide",
        "nosegrind" | "grindnose" => "Nosegrind",
        "noseslide" | "slidenose" => "Noseslide",
        "overcrook" | "grindovercrooks" => "Overcrook",
        "overwilly" | "grindoverwilly" => "Overwilly",
        "salad" | "grindsalad" => "Salad Grind",
        "smith" | "grindsmith" => "Smith Grind",
        "tailslide" | "slidetail" => "Tailslide",
        "willy" | "grindwilly" => "Willy Grind",
        "darkslide" => "Darkslide",
        _ => return None,
    })
}

/// A flip trick's name, without nollie.
fn flip(id: &str) -> Option<String> {
    let counted = |base: &str, name: &str| -> Option<String> {
        let rest = id.strip_prefix(base)?;
        let times = match rest {
            "" => "",
            "2" => "Double ",
            "3" => "Triple ",
            "4" => "Quad ",
            _ => return None,
        };
        Some(format!("{times}{name}"))
    };
    if let Some(name) = counted("kickflip", "Kickflip").or_else(|| counted("heelflip", "Heelflip")) {
        return Some(name);
    }
    Some(
        match id {
            "360flip" => "360 Flip",
            "360hardflip" => "360 Hardflip",
            "360inwardheelflip" => "360 Inward Heelflip",
            "360popshuvit" => "360 Shove-it",
            "fs360popshuvit" => "Frontside 360 Shove-it",
            "fspopshuvit" => "Frontside Shove-it",
            "popshuvit" => "Pop Shove-it",
            "hardflip" => "Hardflip",
            "inwardheelflip" => "Inward Heelflip",
            "laserflip" => "Laser Flip",
            "latebackfootheelflip" => "Late Backfoot Heelflip",
            "latebackfootkickflip" => "Late Backfoot Kickflip",
            "latebsshuv" => "Late Shove-it",
            "latefsshuv" => "Late Frontside Shove-it",
            "lateheelflip" => "Late Heelflip",
            "latekickflip" => "Late Kickflip",
            "lateflip" => "Late Flip",
            "varialheelflip" => "Varial Heelflip",
            "varialkickflip" => "Varial Kickflip",
            "darktolightbsshuv" => "Dark to Light Shove-it",
            "darktolightfsshuv" => "Dark to Light Frontside Shove-it",
            "darktolightheelflip" => "Dark to Light Heelflip",
            "darktolightkickflip" => "Dark to Light Kickflip",
            _ => return None,
        }
        .to_owned(),
    )
}

/// "fs" or "bs" as a side.
fn side(id: &str) -> Option<&'static str> {
    match id {
        "fs" => Some("Frontside"),
        "bs" => Some("Backside"),
        _ => None,
    }
}

/// A grab's tweak direction.
fn tweak(name: &str, direction: &str) -> Option<String> {
    Some(match direction {
        "up" => format!("Stiffy {name}"),
        "down" => format!("Tuck Knee {name}"),
        "left" | "right" => format!("Tweaked {name}"),
        _ => return None,
    })
}

/// The HUD name of a trick, from its scorable identifier; `None` for the
/// air metrics the game scores without naming, and identifiers it does not
/// know.
pub fn trick_name(id: &str) -> Option<String> {
    let id = id.trim().to_ascii_lowercase();
    let id = id.as_str();
    if id.is_empty() || id.ends_with("_metric") || id.starts_with("air_") && id != "air_player_spin" && id != "air_player_flip" {
        return None;
    }
    let fixed = match id {
        "fspowerslide" => "Frontside Powerslide",
        "bspowerslide" => "Backside Powerslide",
        "nosemanual" => "Nose Manual",
        "tailmanual" => "Manual",
        "fsrevert" => "Frontside Revert",
        "bsrevert" => "Backside Revert",
        "ollie" => "Ollie",
        "nollie" => "Nollie",
        "air_player_spin" => "Spin",
        "air_player_flip" => "Flip",
        "nofoot_air" => "No-Footer",
        "fs_rock_n_roll" => "Frontside Rock 'n' Roll",
        "bs_rock_n_roll" => "Backside Rock 'n' Roll",
        "fakie_rock" => "Fakie Rock",
        "dropin" => "Drop In",
        "dropin_fs_revert" => "Drop In Frontside Revert",
        "dropin_bs_revert" => "Drop In Backside Revert",
        "fsboneless" => "Frontside Boneless",
        "bsboneless" => "Backside Boneless",
        "fsfastplant" => "Frontside Fastplant",
        "bsfastplant" => "Backside Fastplant",
        "nocomply" => "No Comply",
        "no_comply_fs180" => "Frontside 180 No Comply",
        "no_comply_bs180" => "Backside 180 No Comply",
        "no_comply_fs360" => "Frontside 360 No Comply",
        "no_comply_bs360" => "Backside 360 No Comply",
        "hippyjump" => "Hippie Jump",
        "hippyjump_fs180" => "Frontside 180 Hippie Jump",
        "hippyjump_bs180" => "Backside 180 Hippie Jump",
        "tailgrab_airwalk" => "Airwalk",
        "nosegrab_airwalk" => "Nosegrab Airwalk",
        "fs_onefoot_right" => "Frontside One-Footer",
        "fs_backfoot" => "Frontside Back-Footer",
        "bs_onefoot_front" => "Backside One-Footer",
        "bs_onefoot_behind" => "Backside One-Footer Behind",
        "gnd_fsgrab" => "Ground Indy",
        "gnd_bsgrab" => "Ground Melon",
        "gnd_dblgrab" => "Ground Double Grab",
        _ => "",
    };
    if !fixed.is_empty() {
        return Some(fixed.to_owned());
    }
    // Endings that wrap another trick.
    for (ending, word) in [("_underflip", "Underflip"), ("_darkcatch", "Dark Catch")] {
        if let Some(base) = id.strip_suffix(ending) {
            return trick_name(base).map(|name| format!("{name} {word}"));
        }
    }
    if let Some(rest) = id.strip_prefix("darkslideout_") {
        return side(rest.get(..2)?).map(|s| format!("{s} Darkslide Out"));
    }
    if let Some(base) = id.strip_prefix("n_") {
        return flip(base).map(|name| format!("Nollie {name}"));
    }
    if let Some(name) = flip(id).or_else(|| grind(id).map(str::to_owned)) {
        return Some(name);
    }
    if let Some(name) = grab(id) {
        return Some(name.to_owned());
    }
    if let Some(base) = id.strip_suffix("_fingerflip") {
        // `fsgrabtostalegrab`: one grab caught into another.
        if let Some((from, to)) = base.split_once("to")
            && let (Some(from), Some(to)) = (grab(from), grab(to))
        {
            return Some(format!("{from} to {to} Fingerflip"));
        }
        return grab(base).map(|name| format!("{name} Fingerflip"));
    }
    let words: Vec<&str> = id.split('_').collect();
    // Grabs with a tweak or one foot off.
    if let [g, rest @ ..] = words.as_slice()
        && let Some(name) = grab(g)
    {
        return match rest {
            [direction] => tweak(name, direction),
            ["onefoot", _] => Some(format!("One-Foot {name}")),
            [s, "footplant"] => side(s).map(|s| format!("{name} {s} Footplant")),
            _ => None,
        };
    }
    if let ["ollie", "onefoot", _] = words.as_slice() {
        return Some("One-Foot Ollie".into());
    }
    if let ["ollie", s, "footplant"] = words.as_slice() {
        return side(s).map(|s| format!("{s} Footplant"));
    }
    let (Some(&first), Some(rest)) = (words.first(), words.get(1..)) else {
        return None;
    };
    let s = side(first)?;
    // Handplants and their variations.
    if rest.first() == Some(&"handplant") {
        let base = format!("{s} Handplant");
        return Some(match &rest[1..] {
            [] | ["mount"] | ["dismount"] | ["to", "flat"] => base,
            ["nofoot"] => format!("No-Foot {base}"),
            [direction] => tweak(&base, direction)?,
            ["onefoot", _] => format!("One-Foot {base}"),
            ["to", "fakie"] => format!("{base} to Fakie"),
            ["to", "revert"] => format!("{base} Revert"),
            _ => return None,
        });
    }
    // Grinds, sometimes with a grab: `fs_50_50_fsgrab`, `bs_board_tail`.
    for split in (1..=rest.len()).rev() {
        let Some(name) = grind(&rest[..split].join("_")) else { continue };
        let base = format!("{s} {name}");
        return match &rest[split..] {
            [] => Some(base),
            ["double"] => Some(format!("{base} Double Grab")),
            [g] => grab(g).map(|g| format!("{base} {g}")),
            _ => None,
        };
    }
    None
}

/// What the HUD shows for a trick: its name from the identifier, else the
/// label the game's data gives it made readable.
pub fn trick_title(id: &str, label: &str) -> String {
    if let Some(name) = trick_name(id) {
        return name;
    }
    readable(label)
}

/// A label that is a key (`STR_TRICK_KICK_FLIP`, `kickflip_2`) made into
/// words.
pub fn readable(label: &str) -> String {
    let mut text = label.trim();
    for prefix in ["STR_", "str_", "TRICK_", "trick_", "TRICKNAME_", "tricknames_"] {
        text = text.strip_prefix(prefix).unwrap_or(text);
    }
    if text.contains(' ') && text.chars().any(char::is_lowercase) {
        return text.to_owned();
    }
    text.split(['_', '-', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            let mut chars = lower.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tricks_have_their_names() {
        for (id, name) in [
            ("kickflip", "Kickflip"),
            ("kickflip2", "Double Kickflip"),
            ("n_heelflip3", "Nollie Triple Heelflip"),
            ("n_360flip", "Nollie 360 Flip"),
            ("360inwardheelflip_darkcatch", "360 Inward Heelflip Dark Catch"),
            ("n_kickflip4_underflip", "Nollie Quad Kickflip Underflip"),
            ("fs_50_50", "Frontside 50-50 Grind"),
            ("bs_crook_tailgrab", "Backside Crooked Grind Tailgrab"),
            ("fs_50_50_fsgrab", "Frontside 50-50 Grind Indy"),
            ("bs_board_double", "Backside Boardslide Double Grab"),
            ("fs_darkslide_nose", "Frontside Darkslide Nosegrab"),
            ("grind5050", "50-50 Grind"),
            ("slidenblunt", "Noseblunt Slide"),
            ("bsgrab_down", "Tuck Knee Melon"),
            ("stalegrab_up", "Stiffy Stalefish"),
            ("nosegrab_onefoot_front", "One-Foot Nosegrab"),
            ("tailgrab_bs_footplant", "Tailgrab Backside Footplant"),
            ("ollie_fs_footplant", "Frontside Footplant"),
            ("fsgrabtostalegrab_fingerflip", "Indy to Stalefish Fingerflip"),
            ("fsgrabvarial_fingerflip", "Indy Varial Fingerflip"),
            ("bs_handplant_to_fakie", "Backside Handplant to Fakie"),
            ("fs_handplant_left", "Tweaked Frontside Handplant"),
            ("fspopshuvit", "Frontside Shove-it"),
            ("tailmanual", "Manual"),
            ("darktolightkickflip", "Dark to Light Kickflip"),
            ("lateflip_darkcatch", "Late Flip Dark Catch"),
        ] {
            assert_eq!(trick_name(id).as_deref(), Some(name), "{id}");
        }
        assert_eq!(trick_name("air_height_to_peak"), None);
        assert_eq!(trick_name("generic_metric"), None);
        assert_eq!(trick_name("fs_nonsense"), None);
    }

    #[test]
    fn every_scorable_but_the_metrics_is_named() {
        // The identifiers of Skate 3's scorable enum that are not metrics.
        let all = include_str!("trick_ids.txt");
        let unnamed: Vec<&str> = all
            .split_whitespace()
            .filter(|id| !id.ends_with("_metric") && !id.starts_with("air_") && trick_name(id).is_none())
            .collect();
        assert!(unnamed.is_empty(), "{unnamed:?}");
    }

    #[test]
    fn labels_that_are_keys_read_as_words() {
        assert_eq!(readable("STR_TRICK_KICK_FLIP"), "Kick Flip");
        assert_eq!(readable("Kickflip to Fakie"), "Kickflip to Fakie");
        assert_eq!(trick_title("unknown_thing", "TRICK_BIG_SPIN"), "Big Spin");
        assert_eq!(trick_title("ollie", "STR_OLLIE"), "Ollie");
    }
}
