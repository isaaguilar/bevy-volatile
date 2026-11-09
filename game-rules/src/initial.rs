use bevy::prelude::IVec2;
use rand::{Rng, RngExt};

use crate::{
    board::{Board, PLAYER_STARTS},
    environment::{EARTHQUAKE_INTERVAL, RAIN_INTERVAL, WindDirection},
    ingredients, spawning,
};

pub const MAX_HEALTH: u8 = 10;
pub const MAX_STRENGTH: u8 = 5;
pub const MAX_MAGIC: u8 = 5;

pub struct InitialMatch {
    pub board: Board,
    pub wind: WindDirection,
    pub rain: u16,
    pub earthquake: u16,
    pub field_items: [(usize, IVec2); 2],
}

pub fn initial_inventory() -> ([[usize; ingredients::COUNT]; 2], [u8; ingredients::COUNT]) {
    let mut inventory = [[0; ingredients::COUNT]; 2];
    let mut spawned = [0; ingredients::COUNT];
    for player in &mut inventory {
        for index in [
            ingredients::SUNSTONE,
            ingredients::LIFE_ESSENCE,
            ingredients::OIL,
        ] {
            player[index] = 1;
            spawned[index] += 1;
        }
    }
    (inventory, spawned)
}

impl InitialMatch {
    pub fn generate(rng: &mut impl Rng) -> Self {
        let board = Board::generate(rng);
        let wind = WindDirection::random(rng);
        let rain = rng.random_range(RAIN_INTERVAL);
        let earthquake = rng.random_range(EARTHQUAKE_INTERVAL);
        let mut candidates = spawning::candidates(&board, &PLAYER_STARTS, &[]);
        let field_items = [ingredients::SUNSTONE, ingredients::LIFE_ESSENCE].map(|index| {
            assert!(
                !candidates.is_empty(),
                "not enough space for initial ingredients"
            );
            let cell = candidates.swap_remove(rng.random_range(0..candidates.len()));
            (index, cell)
        });
        Self {
            board,
            wind,
            rain,
            earthquake,
            field_items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rand::prelude::WyRand;
    use rand::SeedableRng;

    #[test]
    fn generated_match_has_valid_initial_items_and_private_inventories() {
        let initial = InitialMatch::generate(&mut WyRand::seed_from_u64(42));
        let candidates = spawning::candidates(&initial.board, &PLAYER_STARTS, &[]);
        assert_ne!(initial.field_items[0].1, initial.field_items[1].1);
        assert!(
            initial
                .field_items
                .iter()
                .all(|(_, cell)| candidates.contains(cell))
        );
        assert!(RAIN_INTERVAL.contains(&initial.rain));
        assert!(EARTHQUAKE_INTERVAL.contains(&initial.earthquake));
        let (inventory, spawned) = initial_inventory();
        assert_eq!(inventory[0], inventory[1]);
        assert_eq!(inventory[0][ingredients::SUNSTONE], 1);
        assert_eq!(inventory[0][ingredients::LIFE_ESSENCE], 1);
        assert_eq!(inventory[0][ingredients::OIL], 1);
        assert_eq!(spawned[ingredients::SUNSTONE], 2);
    }
}
