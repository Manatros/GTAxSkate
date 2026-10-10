//! GTA V's surface materials as Skate 3 ground: the material hash a probe
//! reports (`GET_SHAPE_TEST_RESULT_INCLUDING_MATERIAL`) to the board's
//! rolling surface (see `skate_rules::Ground`).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Ground surfaces a skater can roll on, by their `materials.dat` name and
/// the hash the game reports for them (these are not `GET_HASH_KEY` of the
/// names). Glass, body parts and the like are left out: they skate as
/// smooth.
pub const MATERIALS: &[(&str, u32)] = &[
    ("CONCRETE", 0x46ca81e8),
    ("CONCRETE_POTHOLE", 0x1567bf52),
    ("CONCRETE_DUSTY", 0xbf59b491),
    ("TARMAC", 0x10dd5498),
    ("TARMAC_PAINTED", 0xb26eefb0),
    ("TARMAC_POTHOLE", 0x70726a55),
    ("RUMBLE_STRIP", 0xf116bc2d),
    ("BREEZE_BLOCK", 0xc72165d6),
    ("ROCK", 0xcdeb5023),
    ("ROCK_MOSSY", 0xf8902ac8),
    ("STONE", 0x2d9c1e0d),
    ("COBBLESTONE", 0x2257a573),
    ("BRICK", 0x61b1f936),
    ("MARBLE", 0x73ef7697),
    ("PAVING_SLAB", 0x71ab3fee),
    ("SANDSTONE_SOLID", 0x23500534),
    ("SANDSTONE_BRITTLE", 0x7209440e),
    ("SAND_LOOSE", 0xa0ebf7e4),
    ("SAND_COMPACT", 0x1e6d775e),
    ("SAND_WET", 0x363cbcd5),
    ("SAND_TRACK", 0x8e4d8aff),
    ("SAND_UNDERWATER", 0xbc4922a4),
    ("SAND_DRY_DEEP", 0x1e5e7a48),
    ("SAND_WET_DEEP", 0x4ccc2aff),
    ("ICE", 0xd125aa55),
    ("ICE_TARMAC", 0x8ce6e7d9),
    ("SNOW_LOOSE", 0x8c8308ca),
    ("SNOW_COMPACT", 0xcba23987),
    ("SNOW_DEEP", 0x608abc80),
    ("SNOW_TARMAC", 0x5c67c62a),
    ("GRAVEL_SMALL", 0x38bbd00c),
    ("GRAVEL_LARGE", 0x7edc5571),
    ("GRAVEL_DEEP", 0xeabd174e),
    ("GRAVEL_TRAIN_TRACK", 0x72c668b6),
    ("DIRT_TRACK", 0x8f9cd58f),
    ("MUD_HARD", 0x8c31b7ea),
    ("MUD_POTHOLE", 0x129eca2a),
    ("MUD_SOFT", 0x61826e7a),
    ("MUD_UNDERWATER", 0xefb2df09),
    ("MUD_DEEP", 0x42251dc0),
    ("MARSH", 0x0d4c07e2),
    ("MARSH_DEEP", 0x5e73a22e),
    ("SOIL", 0xd63ccddb),
    ("CLAY_HARD", 0x4434dfe7),
    ("CLAY_SOFT", 0x216ff3f0),
    ("GRASS_LONG", 0xe47a3e41),
    ("GRASS", 0x4f747b87),
    ("GRASS_SHORT", 0xb34e900d),
    ("HAY", 0x92b69883),
    ("BUSHES", 0x22ad7b72),
    ("TWIGS", 0xc98f5b61),
    ("LEAVES", 0x8653c6cd),
    ("WOODCHIPS", 0xed932e53),
    ("TREE_BARK", 0x8dd4ebb9),
    ("METAL_SOLID_SMALL", 0xa9bc4217),
    ("METAL_SOLID_MEDIUM", 0xea34e8f8),
    ("METAL_SOLID_LARGE", 0x2cd49bd1),
    ("METAL_HOLLOW_SMALL", 0x00f3b93b),
    ("METAL_HOLLOW_MEDIUM", 0x6e3dbfb8),
    ("METAL_HOLLOW_LARGE", 0xdd3cdcf9),
    ("METAL_CHAINLINK_SMALL", 0x2d6e26cd),
    ("METAL_CHAINLINK_LARGE", 0x0781fa34),
    ("METAL_CORRUGATED_IRON", 0x31b80ad6),
    ("METAL_GRILLE", 0xe699f485),
    ("METAL_RAILING", 0x7d368d93),
    ("METAL_DUCT", 0x68feb9fd),
    ("METAL_GARAGE_DOOR", 0xf2373de9),
    ("METAL_MANHOLE", 0xd2ffa63d),
    ("WOOD_SOLID_SMALL", 0xe82a6f1c),
    ("WOOD_SOLID_MEDIUM", 0x2114b37d),
    ("WOOD_SOLID_LARGE", 0x309f8bb7),
    ("WOOD_SOLID_POLISHED", 0x0789c7ab),
    ("WOOD_FLOOR_DUSTY", 0xd35443de),
    ("WOOD_HOLLOW_SMALL", 0x76d9ac2f),
    ("WOOD_HOLLOW_MEDIUM", 0xea3746bd),
    ("WOOD_HOLLOW_LARGE", 0xc8d738e7),
    ("WOOD_CHIPBOARD", 0x461d0e9b),
    ("WOOD_OLD_CREAKY", 0x2b13503d),
    ("WOOD_HIGH_DENSITY", 0x981e5200),
    ("WOOD_LATTICE", 0x77e08a22),
    ("CERAMIC", 0xb94a2eb5),
    ("ROOF_TILE", 0x689e0e75),
    ("ROOF_FELT", 0xab87c845),
    ("FIBREGLASS", 0x50b728db),
    ("TARPAULIN", 0xd9b1cde0),
    ("PLASTIC", 0x846bc4ff),
    ("PLASTIC_HOLLOW", 0x25612338),
    ("PLASTIC_HIGH_DENSITY", 0x9f154729),
    ("RUBBER", 0xf7503f13),
    ("RUBBER_HOLLOW", 0xd1461b30),
    ("LINOLEUM", 0x11436942),
    ("LAMINATE", 0x6e02c9aa),
    ("CARPET_SOLID", 0x27e49616),
    ("CARPET_SOLID_DUSTY", 0x0973ae44),
    ("CARPET_FLOORBOARD", 0xacc354b1),
    ("CLOTH", 0x07519e5d),
    ("PLASTER_SOLID", 0xddc7963f),
    ("CARDBOARD_SHEET", 0x0e18dff5),
    ("CARDBOARD_BOX", 0xac038918),
    ("FOAM", 0x30341454),
    ("WATER", 0x19f81600),
    ("OIL", 0xda2e9567),
    ("PUDDLE", 0x3b982e13),
    ("CONCRETE_PAVEMENT", 0x78239b1a),
    ("BRICK_PAVEMENT", 0xbb9ca6d8),
    ("WOOD_HIGH_FRICTION", 0x8070dcf9),
    ("ROCK_NOINST", 0x079e4953),
    ("BUSHES_NOINST", 0x55e5aaee),
    ("METAL_SOLID_ROAD_SURFACE", 0xd48aa0f2),
    ("STUNT_RAMP_SURFACE", 0x8388fa6c),
    ("PHYS_NO_FRICTION", 0x63545f03),
];

/// Skate's ground (1 to 5) for a probed material hash; 0 when unknown,
/// which skates as smooth.
pub fn ground_of(material: u32) -> u8 {
    static TABLE: OnceLock<HashMap<u32, u8>> = OnceLock::new();
    TABLE
        .get_or_init(|| MATERIALS.iter().map(|(name, hash)| (*hash, skate_rules::named_ground(name) as u8)).collect())
        .get(&material)
        .copied()
        .unwrap_or(0)
}

/// Which of Skate 3's rolling recordings (`crate::rolling`) a GTA material
/// sounds like: wood (ramps, floors), metal, smooth or rough concrete, the
/// stony aggregate of cobbles and bricks, smooth or rough asphalt.
pub fn rolling_sound_of(material: u32) -> &'static str {
    let name = MATERIALS.iter().find(|(_, hash)| *hash == material).map_or("", |(name, _)| *name);
    match name {
        n if n.starts_with("WOOD") || matches!(n, "STUNT_RAMP_SURFACE" | "LAMINATE" | "LINOLEUM" | "CARPET_FLOORBOARD") => "wood_ramp",
        n if n.starts_with("METAL") => "metal_smooth",
        "CONCRETE_POTHOLE" | "CONCRETE_DUSTY" | "STONE" | "ROCK" | "ROCK_MOSSY" | "ROCK_NOINST" | "SANDSTONE_BRITTLE" => "concrete_rough",
        "COBBLESTONE" | "BRICK" | "BRICK_PAVEMENT" | "RUMBLE_STRIP" | "ROOF_TILE" => "concrete_aggregate",
        "TARMAC" | "TARMAC_PAINTED" | "ICE_TARMAC" | "SNOW_TARMAC" => "asphalt_smooth",
        n if n.starts_with("TARMAC") || n.starts_with("GRAVEL") || n.starts_with("SAND") || n.starts_with("MUD") || n.starts_with("DIRT") => {
            "asphalt_rough"
        }
        _ => "concrete_smooth",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate_rules::Ground;

    fn ground(name: &str) -> Ground {
        let hash = MATERIALS.iter().find(|(n, _)| *n == name).unwrap().1;
        Ground::from_index(ground_of(hash))
    }

    #[test]
    fn gta_materials_roll_with_their_own_sound() {
        let of = |name: &str| rolling_sound_of(MATERIALS.iter().find(|(n, _)| *n == name).unwrap().1);
        assert_eq!(of("WOOD_SOLID_POLISHED"), "wood_ramp");
        assert_eq!(of("STUNT_RAMP_SURFACE"), "wood_ramp");
        assert_eq!(of("METAL_SOLID_ROAD_SURFACE"), "metal_smooth");
        assert_eq!(of("TARMAC"), "asphalt_smooth");
        assert_eq!(of("TARMAC_POTHOLE"), "asphalt_rough");
        assert_eq!(of("COBBLESTONE"), "concrete_aggregate");
        assert_eq!(of("PAVING_SLAB"), "concrete_smooth");
        assert_eq!(rolling_sound_of(0), "concrete_smooth", "unknown: concrete");
    }

    #[test]
    fn gta_materials_skate_like_their_surfaces() {
        assert_eq!(ground("CONCRETE"), Ground::Smooth);
        assert_eq!(ground("PAVING_SLAB"), Ground::Smooth);
        assert_eq!(ground("WOOD_SOLID_POLISHED"), Ground::Smooth);
        assert_eq!(ground("TARMAC"), Ground::Rough);
        assert_eq!(ground("COBBLESTONE"), Ground::Rough);
        assert_eq!(ground("BRICK_PAVEMENT"), Ground::Rough);
        assert_eq!(ground("GRASS_SHORT"), Ground::Slow);
        assert_eq!(ground("CARPET_SOLID"), Ground::Slow);
        assert_eq!(ground("ICE_TARMAC"), Ground::Slippery);
        assert_eq!(ground("SAND_LOOSE"), Ground::VerySlow);
        assert_eq!(ground("SANDSTONE_SOLID"), Ground::Rough);
        assert_eq!(ground_of(0x1234_5678), 0);
        let mut hashes: Vec<u32> = MATERIALS.iter().map(|m| m.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), MATERIALS.len(), "every hash once");
    }
}
