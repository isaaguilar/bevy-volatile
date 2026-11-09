use bevy::prelude::IVec2;
use rand::{Rng, RngExt};

use crate::{
    board::{Board, COLUMNS, Ground, ROWS},
    ingredients,
};

pub const POOL_SIZE: usize = 64;
pub const MIN_COPIES: u8 = 2;
pub const DROP_INTERVAL: u8 = 3;
pub const DROP_BATCH: usize = 3;
pub const RESPAWN_DELAY: u8 = 5;
const WEIGHTS: [u32; ingredients::COUNT] = [0, 4, 4, 2, 4, 0, 4, 0, 4, 4, 4, 1, 4, 2];

pub fn candidates(board: &Board, occupied: &[IVec2], swept: &[IVec2]) -> Vec<IVec2> {
    (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
        .filter(|cell| {
            board.ground(*cell) == Some(Ground::Solid)
                && !board.is_wet(*cell)
                && !board.has_oil(*cell)
                && !board.is_poisoned(*cell)
                && !board.is_acidic(*cell)
                && !board.has_thorns(*cell)
                && !board.aquatic_cells().any(|occupied| occupied == *cell)
                && !swept.contains(cell)
                && !occupied.contains(cell)
        })
        .collect()
}

pub fn random_ingredient(rng: &mut impl Rng, spawned: &[u8; ingredients::COUNT]) -> usize {
    let needs_copies = WEIGHTS
        .iter()
        .enumerate()
        .any(|(index, &weight)| weight > 0 && spawned[index] < MIN_COPIES);
    let weights = std::array::from_fn::<_, { ingredients::COUNT }, _>(|index| {
        if needs_copies && spawned[index] >= MIN_COPIES {
            0
        } else {
            WEIGHTS[index]
        }
    });
    let mut choice = rng.random_range(0..weights.iter().sum());
    for (index, weight) in weights.into_iter().enumerate() {
        if choice < weight {
            return index;
        }
        choice -= weight;
    }
    unreachable!("ingredient weights must contain a positive value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rand::prelude::WyRand;
    use rand::SeedableRng;

    #[test]
    fn spawn_candidates_exclude_occupied_swept_and_hazardous_cells() {
        let mut board = Board::default();
        let occupied = IVec2::new(1, 1);
        let swept = IVec2::new(2, 1);
        let acid = IVec2::new(3, 1);
        board.add_acid(acid);
        let cells = candidates(&board, &[occupied], &[swept]);
        assert!(!cells.contains(&occupied));
        assert!(!cells.contains(&swept));
        assert!(!cells.contains(&acid));
        assert!(cells.contains(&IVec2::ZERO));
    }

    #[test]
    fn ingredient_selection_prioritizes_underrepresented_types() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut spawned = [MIN_COPIES; ingredients::COUNT];
        spawned[ingredients::COATL_EGG] = 1;
        for _ in 0..10 {
            assert_eq!(
                random_ingredient(&mut rng, &spawned),
                ingredients::COATL_EGG
            );
        }
    }
}
