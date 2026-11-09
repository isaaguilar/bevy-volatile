pub const COUNT: usize = 14;
pub const WATER: usize = 0;
pub const SUNSTONE: usize = 1;
pub const FROST_CRYSTAL: usize = 2;
pub const THORNHEART: usize = 3;
pub const STORM_SHARD: usize = 4;
pub const VOID_DUST: usize = 5;
pub const OBSIDIAN: usize = 6;
pub const STONE_DUST: usize = 7;
pub const OIL: usize = 8;
pub const ACID: usize = 9;
pub const POISON: usize = 10;
pub const COATL_EGG: usize = 11;
pub const LIFE_ESSENCE: usize = 12;
pub const AQUATIC: usize = 13;

pub fn is_unlimited(index: usize) -> bool {
    matches!(index, WATER | VOID_DUST | STONE_DUST)
}
