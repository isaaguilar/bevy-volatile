use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use rand::{RngExt, SeedableRng};

use crate::board::{COLUMNS, ROWS};

const DIRECTIONS: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
];
fn in_bounds(cell: IVec2) -> bool {
    cell.x >= 0 && cell.y >= 0 && cell.x < COLUMNS as i32 && cell.y < ROWS as i32
}

#[derive(Clone)]
pub struct VoidGrowth {
    base: IVec2,
    paths: [Vec<IVec2>; 2],
    directions: [IVec2; 2],
    cleared: Vec<IVec2>,
    fresh: bool,
    rng: WyRand,
}

impl VoidGrowth {
    pub fn new(base: IVec2, seed: u64) -> Self {
        let mut rng = WyRand::seed_from_u64(seed);
        let primary = rng.random_range(0..DIRECTIONS.len());
        let adjacent = if rng.random_bool(0.5) {
            (primary + 1) % DIRECTIONS.len()
        } else {
            (primary + DIRECTIONS.len() - 1) % DIRECTIONS.len()
        };
        Self {
            base,
            paths: [Vec::new(), Vec::new()],
            directions: [DIRECTIONS[primary], DIRECTIONS[adjacent]],
            cleared: Vec::new(),
            fresh: true,
            rng,
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.occupied_cells()
            .filter(|cell| !self.cleared.contains(cell))
    }

    fn occupied_cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        std::iter::once(self.base).chain(self.paths.iter().flatten().copied())
    }

    pub fn clear_cell(&mut self, cell: IVec2) {
        if self.occupied_cells().any(|occupied| occupied == cell) && !self.cleared.contains(&cell) {
            self.cleared.push(cell);
        }
    }

    pub fn advance(&mut self) -> Option<Vec<IVec2>> {
        if self.fresh {
            self.fresh = false;
            return None;
        }
        let side = if self.paths[0].len() < 2 {
            0
        } else if self.paths[1].len() < 2 {
            1
        } else {
            return Some(self.outcome());
        };
        let current = self.paths[side].last().copied().unwrap_or(self.base);
        let sign = if side == 0 { 1 } else { -1 };
        let mut candidates: Vec<_> = self
            .directions
            .iter()
            .map(|direction| current + *direction * sign)
            .filter(|cell| {
                in_bounds(*cell) && !self.occupied_cells().any(|occupied| occupied == *cell)
            })
            .collect();
        if candidates.is_empty() {
            candidates = DIRECTIONS
                .iter()
                .map(|direction| current + *direction)
                .filter(|cell| {
                    in_bounds(*cell) && !self.occupied_cells().any(|occupied| occupied == *cell)
                })
                .collect();
        }
        if self.paths[side].is_empty() {
            candidates.retain(|candidate| {
                DIRECTIONS.iter().any(|direction| {
                    let next = *candidate + *direction;
                    in_bounds(next)
                        && next != *candidate
                        && !self.occupied_cells().any(|cell| cell == next)
                })
            });
        }
        if candidates.is_empty() {
            return Some(self.outcome());
        }
        let cell = candidates[self.rng.random_range(0..candidates.len())];
        self.paths[side].push(cell);
        None
    }

    fn outcome(&self) -> Vec<IVec2> {
        self.paths
            .iter()
            .flatten()
            .copied()
            .chain(std::iter::once(self.base))
            .filter(|cell| !self.cleared.contains(cell))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleared_growth_cell_stays_empty_without_canceling_other_cells() {
        let mut growth = VoidGrowth::new(IVec2::new(4, 3), 42);
        assert!(growth.advance().is_none());
        assert!(growth.advance().is_none());
        let cleared = growth.paths[0][0];
        growth.clear_cell(cleared);
        assert!(!growth.cells().any(|cell| cell == cleared));
        let outcome = loop {
            if let Some(outcome) = growth.advance() {
                break outcome;
            }
        };
        assert!(!outcome.contains(&cleared));
        assert_eq!(outcome.len(), 4);
    }

    #[test]
    fn cleared_origin_is_not_changed_when_growth_completes() {
        let center = IVec2::new(4, 3);
        let mut growth = VoidGrowth::new(center, 42);
        growth.clear_cell(center);
        let outcome = loop {
            if let Some(outcome) = growth.advance() {
                break outcome;
            }
        };
        assert_eq!(outcome.len(), 4);
        assert!(!outcome.contains(&center));
    }

    #[test]
    fn growth_completes_at_corners_and_edges_without_overlap() {
        for center in [IVec2::ZERO, IVec2::new(9, 6), IVec2::new(4, 3)] {
            for seed in 0..1000 {
                let mut growth = VoidGrowth::new(center, seed);
                let mut outcome = None;
                for _ in 0..6 {
                    if let Some(result) = growth.advance() {
                        outcome = Some(result);
                        break;
                    }
                }
                let all = outcome.expect("growth must finish");
                assert_eq!(all.len(), 5);
                assert!(all.contains(&center));
                for (index, cell) in all.iter().enumerate() {
                    assert!(in_bounds(*cell));
                    assert!(!all[..index].contains(cell));
                }
            }
        }
    }
}
