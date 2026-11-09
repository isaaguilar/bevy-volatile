use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use rand::{RngExt, SeedableRng};

use crate::board::Board;

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
pub const ACTIVE_TURNS: u8 = 7;

#[derive(Clone)]
struct Segment {
    cell: IVec2,
    present: bool,
    detached: bool,
    detached_this_turn: bool,
}

#[derive(Clone)]
struct Growth {
    base: IVec2,
    base_present: bool,
    paths: [Vec<Segment>; 2],
    directions: [IVec2; 2],
    remaining: u8,
    fresh: bool,
    stopped: bool,
    missed_growth: [usize; 2],
    retraction_steps: [usize; 2],
    life_owed: bool,
    thorn_returned: bool,
    rng: WyRand,
}

#[derive(Clone, Default)]
pub struct ThornGarden {
    growths: Vec<Growth>,
}

impl ThornGarden {
    pub fn activate(&mut self, base: IVec2, seed: u64, life_owed: bool, active_turns: u8) {
        let mut rng = WyRand::seed_from_u64(seed);
        let primary = rng.random_range(0..DIRECTIONS.len());
        let adjacent = if rng.random_bool(0.5) {
            (primary + 1) % DIRECTIONS.len()
        } else {
            (primary + DIRECTIONS.len() - 1) % DIRECTIONS.len()
        };
        self.growths.push(Growth {
            base,
            base_present: true,
            paths: [Vec::new(), Vec::new()],
            directions: [DIRECTIONS[primary], DIRECTIONS[adjacent]],
            remaining: active_turns,
            fresh: true,
            stopped: false,
            missed_growth: [0; 2],
            retraction_steps: [0; 2],
            life_owed,
            thorn_returned: false,
            rng,
        });
    }

    pub fn contains(&self, cell: IVec2) -> bool {
        self.growths.iter().any(|growth| growth.contains(cell))
    }

    pub fn burn(&mut self, cells: &[IVec2]) {
        for growth in &mut self.growths {
            let hit_base = growth.base_present && cells.contains(&growth.base);
            let hit_path = growth
                .paths
                .iter()
                .flatten()
                .any(|segment| segment.present && cells.contains(&segment.cell));
            if !hit_base && !hit_path {
                continue;
            }
            growth.stopped = true;
            if hit_base {
                growth.base_present = false;
            }
            for path in &mut growth.paths {
                let mut disconnected = hit_base;
                for segment in path {
                    if segment.present && cells.contains(&segment.cell) {
                        segment.present = false;
                        disconnected = true;
                    } else if segment.present && disconnected {
                        if !segment.detached {
                            segment.detached_this_turn = true;
                        }
                        segment.detached = true;
                    }
                }
            }
        }
    }

    pub fn dissolve(&mut self, cells: &[IVec2]) {
        for growth in &mut self.growths {
            if cells.iter().any(|cell| growth.contains(*cell)) {
                growth.dissolve();
            }
        }
    }

    pub fn finish_turn(&mut self, board: &Board) -> (u32, u32) {
        let mut returned_thorns = 0;
        let mut returned_life = 0;
        for growth in &mut self.growths {
            for segment in growth.paths.iter_mut().flatten() {
                if segment.detached && !segment.detached_this_turn {
                    segment.present = false;
                }
                segment.detached_this_turn = false;
            }
            if growth.remaining > 0 {
                if !growth.fresh {
                    if growth.stopped {
                        // Reserve the growth opportunities lost to the blast so earlier
                        // segments retain their original tip-to-base retraction slots.
                        growth.missed_growth[0] += 1;
                        if board.is_water(growth.base) {
                            growth.missed_growth[1] += 1;
                        }
                    } else {
                        growth.grow(board, 0);
                        if board.is_water(growth.base) {
                            growth.grow(board, 1);
                        }
                        if (growth.base_present && board.is_acidic(growth.base))
                            || growth
                                .paths
                                .iter()
                                .flatten()
                                .any(|segment| segment.present && board.is_acidic(segment.cell))
                        {
                            growth.dissolve();
                        }
                    }
                }
                growth.fresh = false;
                growth.remaining -= 1;
                if growth.remaining == 0 {
                    growth.retraction_steps = std::array::from_fn(|index| {
                        growth.paths[index].len() + growth.missed_growth[index]
                    });
                    if growth.life_owed {
                        growth.life_owed = false;
                        returned_life += 1;
                    }
                }
            } else {
                if growth.retraction_steps == [0, 0] {
                    growth.base_present = false;
                } else {
                    for (path, steps) in growth.paths.iter_mut().zip(&mut growth.retraction_steps) {
                        if *steps > 0 {
                            *steps -= 1;
                            if let Some(segment) = path.get_mut(*steps) {
                                segment.present = false;
                            }
                        }
                    }
                }
            }
            if !growth.thorn_returned && !growth.any_present() {
                growth.thorn_returned = true;
                returned_thorns += 1;
            }
        }
        self.growths
            .retain(|growth| !growth.thorn_returned || growth.remaining > 0);
        (returned_thorns, returned_life)
    }
}

impl Growth {
    fn dissolve(&mut self) {
        self.stopped = true;
        self.base_present = false;
        for segment in self.paths.iter_mut().flatten() {
            segment.present = false;
        }
    }

    fn contains(&self, cell: IVec2) -> bool {
        (self.base_present && self.base == cell)
            || self
                .paths
                .iter()
                .flatten()
                .any(|segment| segment.present && segment.cell == cell)
    }

    fn any_present(&self) -> bool {
        self.base_present || self.paths.iter().flatten().any(|segment| segment.present)
    }

    fn grow(&mut self, board: &Board, path_index: usize) {
        let tip = self.paths[path_index]
            .last()
            .map_or(self.base, |segment| segment.cell);
        let sign = if path_index == 0 { 1 } else { -1 };
        let candidates = self.directions.map(|direction| tip + sign * direction);
        let in_bounds: Vec<_> = candidates
            .into_iter()
            .filter(|cell| board.ground(*cell).is_some())
            .collect();
        let at_edge = tip.x == 0
            || tip.y == 0
            || tip.x == crate::board::COLUMNS as i32 - 1
            || tip.y == crate::board::ROWS as i32 - 1;
        let choices: Vec<_> = if in_bounds.is_empty()
            || (at_edge && in_bounds.iter().all(|cell| self.contains(*cell)))
        {
            DIRECTIONS
                .into_iter()
                .map(|direction| tip + direction)
                .filter(|cell| board.ground(*cell).is_some())
                .filter(|cell| {
                    cell.x == 0
                        || cell.y == 0
                        || cell.x == crate::board::COLUMNS as i32 - 1
                        || cell.y == crate::board::ROWS as i32 - 1
                })
                .collect()
        } else {
            in_bounds
        };
        let choices: Vec<_> = choices
            .into_iter()
            .filter(|cell| !board.is_frozen(*cell) && !self.contains(*cell))
            .collect();
        if !choices.is_empty() {
            let cell = choices[self.rng.random_range(0..choices.len())];
            self.paths[path_index].push(Segment {
                cell,
                present: true,
                detached: false,
                detached_this_turn: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Ground;

    fn garden(base: IVec2) -> ThornGarden {
        let mut garden = ThornGarden::default();
        garden.activate(base, 42, true, ACTIVE_TURNS);
        garden.growths[0].directions = [IVec2::X, IVec2::new(1, -1)];
        garden
    }

    #[test]
    fn activation_chooses_adjacent_directions() {
        for seed in 0..100 {
            let mut garden = ThornGarden::default();
            garden.activate(IVec2::new(4, 3), seed, false, ACTIVE_TURNS);
            let [first, second] = garden.growths[0].directions;
            let index = DIRECTIONS
                .iter()
                .position(|direction| *direction == first)
                .unwrap();
            assert!(second == DIRECTIONS[(index + 1) % 8] || second == DIRECTIONS[(index + 7) % 8]);
        }
    }

    #[test]
    fn activation_grows_for_seven_turns_then_retracts_from_tips() {
        let board = Board::default();
        let base = IVec2::new(1, 4);
        let mut garden = garden(base);
        assert_eq!(garden.finish_turn(&board), (0, 0));
        assert!(garden.growths[0].paths[0].is_empty());
        for _ in 0..5 {
            assert_eq!(garden.finish_turn(&board), (0, 0));
        }
        assert_eq!(garden.finish_turn(&board), (0, 1));
        assert_eq!(garden.growths[0].paths[0].len(), 6);
        let cells: Vec<_> = garden.growths[0].paths[0].iter().map(|s| s.cell).collect();
        for &cell in cells.iter().rev() {
            assert_eq!(garden.finish_turn(&board), (0, 0));
            assert!(!garden.contains(cell));
            assert!(garden.contains(base));
        }
        assert_eq!(garden.finish_turn(&board), (1, 0));
        assert!(garden.growths.is_empty());
    }

    #[test]
    fn acid_dissolves_a_vine_when_it_touches_a_segment_or_grows_into_acid() {
        let base = IVec2::new(1, 4);
        let mut board = Board::default();
        let mut existing = garden(base);
        for _ in 0..4 {
            existing.finish_turn(&board);
        }
        let segment = existing.growths[0].paths[0][1].cell;
        existing.dissolve(&[segment]);
        assert!(!existing.contains(base));
        assert!(!existing.growths[0].any_present());
        assert_eq!(existing.finish_turn(&board), (1, 0));

        let mut growing = garden(base);
        board.add_acid(base + IVec2::X);
        board.add_acid(base + IVec2::new(1, -1));
        growing.finish_turn(&board);
        assert_eq!(growing.finish_turn(&board), (1, 0));
        assert!(!growing.contains(base));
    }

    #[test]
    fn explosion_does_not_advance_the_original_retraction_schedule() {
        let board = Board::default();
        let base = IVec2::new(1, 4);
        let mut garden = garden(base);
        for _ in 0..4 {
            garden.finish_turn(&board);
        }
        let cells: Vec<_> = garden.growths[0].paths[0].iter().map(|s| s.cell).collect();
        garden.burn(&[cells[2]]);
        for _ in 0..3 {
            garden.finish_turn(&board);
        }
        assert!(garden.contains(cells[1]));
        for _ in 0..4 {
            garden.finish_turn(&board);
            assert!(garden.contains(cells[1]));
        }
        garden.finish_turn(&board);
        assert!(!garden.contains(cells[1]));
        assert!(garden.contains(cells[0]));
        garden.finish_turn(&board);
        assert!(!garden.contains(cells[0]));
        assert!(garden.contains(base));
        assert_eq!(garden.finish_turn(&board), (1, 0));
    }

    #[test]
    fn explosion_during_retraction_does_not_skip_the_destroyed_tip_slot() {
        let board = Board::default();
        let mut garden = garden(IVec2::new(1, 4));
        for _ in 0..7 {
            garden.finish_turn(&board);
        }
        let cells: Vec<_> = garden.growths[0].paths[0].iter().map(|s| s.cell).collect();
        garden.burn(&[cells[5]]);
        garden.finish_turn(&board);
        assert!(garden.contains(cells[4]));
        garden.finish_turn(&board);
        assert!(!garden.contains(cells[4]));
    }

    #[test]
    fn wet_base_grows_opposite_branch_and_rewetting_resumes_it() {
        let base = IVec2::new(4, 3);
        let mut board = Board::default();
        let mut garden = garden(base);
        garden.finish_turn(&board);
        garden.finish_turn(&board);
        assert!(garden.growths[0].paths[1].is_empty());
        board.make_wet(base);
        garden.finish_turn(&board);
        let reverse = garden.growths[0].paths[1][0].cell - base;
        assert!(reverse == -IVec2::X || reverse == IVec2::new(-1, 1));
        board.dry(base);
        garden.finish_turn(&board);
        assert_eq!(garden.growths[0].paths[1].len(), 1);
        board.make_wet(base);
        garden.finish_turn(&board);
        assert_eq!(garden.growths[0].paths[1].len(), 2);
    }

    #[test]
    fn both_vines_retract_from_their_tips_before_the_base() {
        let base = IVec2::new(4, 3);
        let mut board = Board::default();
        let mut garden = garden(base);
        for _ in 0..3 {
            garden.finish_turn(&board);
        }
        board.make_wet(base);
        for _ in 0..4 {
            garden.finish_turn(&board);
        }
        let paths: [Vec<_>; 2] = std::array::from_fn(|index| {
            garden.growths[0].paths[index]
                .iter()
                .map(|segment| segment.cell)
                .collect()
        });
        assert_eq!([paths[0].len(), paths[1].len()], [6, 4]);
        for turn in 0..6 {
            garden.finish_turn(&board);
            assert!(garden.contains(base));
            for path in &paths {
                if turn < path.len() {
                    assert!(!garden.contains(path[path.len() - turn - 1]));
                }
            }
        }
        assert_eq!(garden.finish_turn(&board), (1, 0));
    }

    #[test]
    fn ice_blocks_growth_until_thawed_and_edge_uses_available_border() {
        let base = IVec2::new(5, 3);
        let mut board = Board::default();
        let mut garden = garden(base);
        for cell in [base + IVec2::X, base + IVec2::new(1, -1)] {
            board.set(cell, Ground::Water);
            assert!(board.freeze_water(cell));
        }
        garden.finish_turn(&board);
        garden.finish_turn(&board);
        assert!(garden.growths[0].paths[0].is_empty());
        board.thaw(base + IVec2::X);
        garden.finish_turn(&board);
        assert_eq!(garden.growths[0].paths[0][0].cell, base + IVec2::X);

        let mut edge = garden_at_edge();
        edge.finish_turn(&board);
        edge.finish_turn(&board);
        let cell = edge.growths[0].paths[0][0].cell;
        assert!(cell.x == 9 || cell.y == 0);
    }

    fn garden_at_edge() -> ThornGarden {
        let mut garden = garden(IVec2::new(9, 0));
        garden.growths[0].directions = [IVec2::X, IVec2::new(1, -1)];
        garden
    }

    #[test]
    fn explosion_severs_next_turn_even_if_another_blast_hits() {
        let board = Board::default();
        let base = IVec2::new(1, 4);
        let mut garden = garden(base);
        for _ in 0..5 {
            garden.finish_turn(&board);
        }
        let cells: Vec<_> = garden.growths[0].paths[0].iter().map(|s| s.cell).collect();
        garden.burn(&[cells[1]]);
        assert!(!garden.contains(cells[1]));
        assert!(garden.contains(cells[2]));
        garden.finish_turn(&board);
        assert!(garden.contains(cells[2]));
        garden.burn(&[cells[0]]);
        garden.finish_turn(&board);
        assert!(!garden.contains(cells[2]));
        assert!(garden.contains(base));
        assert!(garden.growths[0].stopped);
        assert_eq!(garden.growths[0].paths[0].len(), 4);
    }
}
