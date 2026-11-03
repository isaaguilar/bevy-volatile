use bevy::prelude::*;

use crate::board::{Board, Ground};

const DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y];
const GROWTH_TURNS: u8 = 3;
const TOTAL_TURNS: u8 = 5;

struct Arm {
    direction: IVec2,
    segments: Vec<Segment>,
    stopped: bool,
}

struct Segment {
    cell: IVec2,
    rotation_progress: u8,
}

pub struct AquaticImpact {
    pub cells: Vec<IVec2>,
    pub damage: u8,
    pub initial: Vec<IVec2>,
    pub final_cells: Vec<IVec2>,
    pub frames: Vec<AquaticFrame>,
}

pub struct AquaticFrame {
    pub cells: Vec<IVec2>,
    pub swept: Vec<IVec2>,
    pub flattened: Vec<IVec2>,
}

pub struct AquaticCreature {
    origin: IVec2,
    arms: [Arm; 4],
    age: u8,
}

impl AquaticCreature {
    pub fn new(origin: IVec2) -> Self {
        Self {
            origin,
            arms: DIRECTIONS.map(|direction| Arm {
                direction,
                segments: Vec::new(),
                stopped: false,
            }),
            age: 0,
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        std::iter::once(self.origin).chain(
            self.arms
                .iter()
                .flat_map(|arm| arm.segments.iter().map(|segment| segment.cell)),
        )
    }

    pub fn advance(&mut self, board: &mut Board) -> Option<AquaticImpact> {
        if self.age == TOTAL_TURNS {
            return None;
        }
        self.age += 1;
        let initial: Vec<_> = self.cells().collect();
        let mut cells = initial.clone();
        let mut frames = Vec::new();
        if self.age <= GROWTH_TURNS {
            for arm in &mut self.arms {
                if arm.stopped {
                    continue;
                }
                let next = arm
                    .segments
                    .last()
                    .map_or(self.origin, |segment| segment.cell)
                    + arm.direction;
                match board.ground(next) {
                    None | Some(Ground::Ridge) => arm.stopped = true,
                    Some(_) => {
                        arm.segments.push(Segment {
                            cell: next,
                            rotation_progress: 0,
                        });
                        cells.push(next);
                    }
                }
            }
        } else {
            let mut swept = initial.clone();
            let mut flattened = Vec::new();
            for &cell in &initial {
                if board.ground(cell) == Some(Ground::Ridge) {
                    board.set(cell, Ground::Solid);
                    flattened.push(cell);
                }
            }
            for index in 0..usize::from(GROWTH_TURNS) {
                let radius = u8::try_from(index + 1).expect("arm length is at most three");
                for _ in 0..radius {
                    for arm in &mut self.arms {
                        let Some(segment) = arm.segments.get_mut(index) else {
                            continue;
                        };
                        let clockwise = IVec2::new(-arm.direction.y, arm.direction.x);
                        let sweep = if segment.rotation_progress < radius {
                            clockwise
                        } else {
                            -arm.direction
                        };
                        let next = segment.cell + sweep;
                        match board.ground(next) {
                            None => continue,
                            Some(Ground::Ridge) => {
                                board.set(next, Ground::Solid);
                                flattened.push(next);
                            }
                            Some(_) => {}
                        }
                        segment.cell = next;
                        segment.rotation_progress += 1;
                        cells.push(next);
                        swept.push(next);
                    }
                    if !swept.is_empty() || !flattened.is_empty() {
                        frames.push(AquaticFrame {
                            cells: self.cells().collect(),
                            swept: std::mem::take(&mut swept),
                            flattened: std::mem::take(&mut flattened),
                        });
                    }
                }
            }
        }
        Some(AquaticImpact {
            cells,
            damage: if self.age <= GROWTH_TURNS { 2 } else { 3 },
            initial,
            final_cells: self.cells().collect(),
            frames,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_across_holes_and_rotates_entire_arms_for_two_turns() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        board.set(origin + IVec2::X, Ground::Hole);
        let mut creature = AquaticCreature::new(origin);
        for distance in 1..=3 {
            assert!(creature.advance(&mut board).is_some());
            for direction in DIRECTIONS {
                assert!(
                    creature
                        .cells()
                        .any(|cell| cell == origin + direction * distance)
                );
            }
        }
        let impact = creature.advance(&mut board).unwrap();
        assert_eq!(impact.damage, 3);
        assert!(impact.cells.contains(&(origin + IVec2::new(3, 1))));
        assert!(
            creature
                .cells()
                .any(|cell| cell == origin + IVec2::new(1, 1))
        );
        assert!(
            creature
                .cells()
                .any(|cell| cell == origin + IVec2::new(2, 2))
        );
        assert!(!creature.cells().any(|cell| cell == origin + IVec2::X));
        assert!(
            creature
                .cells()
                .any(|cell| cell == origin + IVec2::new(3, 3))
        );
        assert!(creature.advance(&mut board).is_some());
        assert!(
            creature
                .cells()
                .any(|cell| cell == origin + IVec2::new(0, 3))
        );
        assert!(creature.advance(&mut board).is_none());
        assert_eq!(board.ground(origin + IVec2::X), Some(Ground::Hole));
    }

    #[test]
    fn rotation_frames_move_from_base_to_tip_and_record_intermediate_cells() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        let mut creature = AquaticCreature::new(origin);
        for _ in 0..3 {
            creature.advance(&mut board);
        }
        let impact = creature.advance(&mut board).unwrap();
        assert_eq!(impact.frames.len(), 6);
        let first = &impact.frames[0];
        assert!(first.cells.contains(&(origin + IVec2::new(1, 1))));
        assert!(first.cells.contains(&(origin + IVec2::new(2, 0))));
        assert!(first.cells.contains(&(origin + IVec2::new(3, 0))));
        assert!(
            impact.frames[1]
                .swept
                .contains(&(origin + IVec2::new(2, 1)))
        );
        assert!(
            impact.frames[1]
                .cells
                .contains(&(origin + IVec2::new(3, 0)))
        );
        assert!(
            impact.frames[2]
                .cells
                .contains(&(origin + IVec2::new(2, 2)))
        );
        assert_eq!(impact.frames.last().unwrap().cells, impact.final_cells);
    }

    #[test]
    fn blocked_arms_stop_growing_but_all_segments_level_ridges_while_swinging() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        board.set(origin + IVec2::new(-2, 0), Ground::Ridge);
        board.set(origin + IVec2::new(3, 1), Ground::Ridge);
        board.set(origin + IVec2::new(2, 1), Ground::Ridge);
        board.set(origin + IVec2::new(-1, -1), Ground::Ridge);
        board.set(origin + IVec2::new(1, 0), Ground::Hole);
        let mut creature = AquaticCreature::new(origin);
        for _ in 0..3 {
            creature.advance(&mut board);
        }
        assert_eq!(creature.arms[2].segments.len(), 1);
        assert_eq!(
            board.ground(origin + IVec2::new(-2, 0)),
            Some(Ground::Ridge)
        );
        creature.advance(&mut board);
        assert_eq!(board.ground(origin + IVec2::new(3, 1)), Some(Ground::Solid));
        assert_eq!(board.ground(origin + IVec2::new(2, 1)), Some(Ground::Solid));
        assert_eq!(
            board.ground(origin + IVec2::new(-1, -1)),
            Some(Ground::Solid)
        );
        assert_eq!(board.ground(origin + IVec2::new(1, 0)), Some(Ground::Hole));
    }

    #[test]
    fn blocked_growth_does_not_restart_but_short_arms_rotate_through_ridges() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        let obstruction = origin + IVec2::new(2, 0);
        board.set(obstruction, Ground::Ridge);
        let mut creature = AquaticCreature::new(origin);
        creature.advance(&mut board);
        creature.advance(&mut board);
        board.set(obstruction, Ground::Solid);
        creature.advance(&mut board);
        assert_eq!(creature.arms[0].segments.len(), 1);
        let rotation_obstruction = origin + IVec2::new(1, 1);
        board.set(rotation_obstruction, Ground::Ridge);
        creature.advance(&mut board);
        assert_eq!(creature.arms[0].segments[0].cell, rotation_obstruction);
        assert_eq!(board.ground(rotation_obstruction), Some(Ground::Solid));
        creature.advance(&mut board);
        assert_eq!(creature.arms[0].segments[0].cell, origin + IVec2::Y);
    }

    #[test]
    fn board_edges_stop_growth_and_clip_arm_rotation() {
        let origin = IVec2::ZERO;
        let mut board = Board::default();
        let mut creature = AquaticCreature::new(origin);
        for _ in 0..5 {
            assert!(creature.advance(&mut board).is_some());
            assert!(creature.cells().all(|cell| board.ground(cell).is_some()));
        }
        assert_eq!(creature.arms[2].segments.len(), 0);
        assert_eq!(creature.arms[3].segments.len(), 0);
    }
}
