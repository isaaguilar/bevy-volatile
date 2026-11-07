use bevy::prelude::*;

use crate::board::{Board, Ground};

const DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y];
const GROWTH_TURNS: u8 = 3;
const TOTAL_TURNS: u8 = 4;

struct Arm {
    direction: IVec2,
    segments: Vec<Segment>,
    stopped: bool,
}

struct Segment {
    cell: IVec2,
}

pub struct AquaticImpact {
    pub cells: Vec<IVec2>,
    pub damage: u8,
    pub initial: Vec<IVec2>,
    pub final_cells: Vec<IVec2>,
    pub frames: Vec<AquaticFrame>,
    pub initial_visual: AquaticVisual,
    pub final_visual: AquaticVisual,
}

pub struct AquaticFrame {
    pub cells: Vec<IVec2>,
    pub swept: Vec<IVec2>,
    pub flattened: Vec<IVec2>,
    pub visual: AquaticVisual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AquaticVisual {
    pub id: u64,
    pub origin: IVec2,
    pub lengths: [usize; 4],
    pub sweep_frame: Option<usize>,
}

pub struct AquaticCreature {
    id: u64,
    origin: IVec2,
    arms: [Arm; 4],
    age: u8,
}

impl AquaticCreature {
    pub fn new(origin: IVec2, id: u64) -> Self {
        Self {
            id,
            origin,
            arms: DIRECTIONS.map(|direction| Arm {
                direction,
                segments: Vec::new(),
                stopped: false,
            }),
            age: 0,
        }
    }

    pub fn visual(&self, sweep_frame: Option<usize>) -> AquaticVisual {
        AquaticVisual {
            id: self.id,
            origin: self.origin,
            lengths: std::array::from_fn(|index| self.arms[index].segments.len()),
            sweep_frame,
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        std::iter::once(self.origin).chain(
            self.arms
                .iter()
                .flat_map(|arm| arm.segments.iter().map(|segment| segment.cell)),
        )
    }

    pub fn finished(&self) -> bool {
        self.age == TOTAL_TURNS
    }

    pub fn advance(&mut self, board: &mut Board) -> Option<AquaticImpact> {
        if self.finished() {
            return None;
        }
        self.age += 1;
        let initial_visual = self.visual(None);
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
                        arm.segments.push(Segment { cell: next });
                        cells.push(next);
                    }
                }
            }
        } else {
            let lengths = initial_visual.lengths;
            let mut swept = initial.clone();
            let mut flattened = Vec::new();
            for &cell in &initial {
                if board.ground(cell) == Some(Ground::Ridge) {
                    board.set(cell, Ground::Solid);
                    flattened.push(cell);
                }
            }
            for step in 0..5 {
                for arm in &mut self.arms {
                    let indices: &[usize] = match (arm.segments.len(), step) {
                        (3, 0) => &[0],
                        (2, 0) => &[0, 1],
                        (3, 1) => &[1],
                        (3, 2) => &[2],
                        (3, 3) => &[1, 2],
                        (3, 4) => &[2],
                        (2, 2) => &[1],
                        (1, 4) => &[0],
                        _ => &[],
                    };
                    for &index in indices {
                        let segment = &mut arm.segments[index];
                        let clockwise = IVec2::new(-arm.direction.y, arm.direction.x);
                        let next = segment.cell + clockwise;
                        match board.ground(next) {
                            None => continue,
                            Some(Ground::Ridge) => {
                                board.set(next, Ground::Solid);
                                flattened.push(next);
                            }
                            Some(_) => {}
                        }
                        segment.cell = next;
                        cells.push(next);
                        swept.push(next);
                    }
                }
                frames.push(AquaticFrame {
                    cells: self.cells().collect(),
                    swept: std::mem::take(&mut swept),
                    flattened: std::mem::take(&mut flattened),
                    visual: AquaticVisual {
                        sweep_frame: Some(step + 3),
                        lengths,
                        ..initial_visual.clone()
                    },
                });
            }
            for arm in &mut self.arms {
                arm.segments.truncate(2);
            }
            frames.push(AquaticFrame {
                cells: self.cells().collect(),
                swept: Vec::new(),
                flattened: Vec::new(),
                visual: AquaticVisual {
                    sweep_frame: Some(8),
                    lengths,
                    ..initial_visual.clone()
                },
            });
            for arm in &mut self.arms {
                arm.segments.clear();
            }
            frames.push(AquaticFrame {
                cells: self.cells().collect(),
                swept: Vec::new(),
                flattened: Vec::new(),
                visual: AquaticVisual {
                    sweep_frame: Some(9),
                    lengths,
                    ..initial_visual.clone()
                },
            });
        }
        Some(AquaticImpact {
            cells,
            damage: if self.age <= GROWTH_TURNS { 2 } else { 3 },
            initial,
            final_cells: self.cells().collect(),
            frames,
            initial_visual,
            final_visual: self.visual(None),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn grows_across_holes_then_sweeps_and_retracts() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        board.set(origin + IVec2::X, Ground::Hole);
        let mut creature = AquaticCreature::new(origin, 0);
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
            impact.frames[4]
                .cells
                .contains(&(origin + IVec2::new(3, 3)))
        );
        assert_eq!(impact.frames[5].cells.len(), 9);
        assert_eq!(impact.frames[6].cells, vec![origin]);
        assert!(
            impact.frames[5..]
                .iter()
                .all(|frame| frame.swept.is_empty())
        );
        assert!(creature.finished());
        assert!(creature.advance(&mut board).is_none());
        assert_eq!(board.ground(origin + IVec2::X), Some(Ground::Hole));
    }

    #[test]
    fn sweep_skips_frame_four_but_preserves_every_swept_cell() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        let mut creature = AquaticCreature::new(origin, 0);
        for _ in 0..3 {
            creature.advance(&mut board);
        }
        let expected = [
            [(1, 1), (2, 0), (3, 0)],
            [(1, 1), (2, 1), (3, 0)],
            [(1, 1), (2, 1), (3, 1)],
            [(1, 1), (2, 2), (3, 2)],
            [(1, 1), (2, 2), (3, 3)],
        ];
        let impact = creature.advance(&mut board).unwrap();
        assert_eq!(impact.frames.len(), 7);
        for (frame, expected_arm) in impact.frames.iter().zip(expected) {
            let expected_cells: Vec<_> = expected_arm
                .into_iter()
                .map(|(x, y)| origin + IVec2::new(x, y))
                .collect();
            assert_eq!(&frame.cells[1..4], expected_cells);
        }
        assert!(
            impact.frames[3]
                .swept
                .contains(&(origin + IVec2::new(2, 2)))
        );
        assert!(
            impact.frames[3]
                .swept
                .contains(&(origin + IVec2::new(3, 2)))
        );
        assert_eq!(impact.frames.last().unwrap().cells, impact.final_cells);
        let mut swept = HashSet::from([origin]);
        for direction in DIRECTIONS {
            let clockwise = IVec2::new(-direction.y, direction.x);
            for radius in 1..=3 {
                for step in 0..=radius {
                    swept.insert(origin + direction * radius + clockwise * step);
                }
            }
        }
        assert_eq!(impact.cells.into_iter().collect::<HashSet<_>>(), swept);
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
        let mut creature = AquaticCreature::new(origin, 0);
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
    fn shorter_arms_sweep_when_their_artwork_moves() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        board.set(origin + IVec2::new(2, 0), Ground::Ridge);
        board.set(origin + IVec2::new(0, 3), Ground::Ridge);
        let mut creature = AquaticCreature::new(origin, 0);
        for _ in 0..3 {
            creature.advance(&mut board);
        }
        let impact = creature.advance(&mut board).unwrap();
        let short_diagonal = origin + IVec2::new(1, 1);
        let medium_intermediate = origin + IVec2::new(-1, 2);
        let medium_diagonal = origin + IVec2::new(-2, 2);
        assert!(impact.frames[0].swept.contains(&medium_intermediate));
        assert!(!impact.frames[1].swept.contains(&medium_diagonal));
        assert!(impact.frames[2].swept.contains(&medium_diagonal));
        assert!(
            impact.frames[..4]
                .iter()
                .all(|frame| !frame.swept.contains(&short_diagonal))
        );
        assert!(impact.frames[4].swept.contains(&short_diagonal));
    }

    #[test]
    fn blocked_growth_does_not_restart_but_short_arms_rotate_through_ridges() {
        let origin = IVec2::new(4, 3);
        let mut board = Board::default();
        let obstruction = origin + IVec2::new(2, 0);
        board.set(obstruction, Ground::Ridge);
        let mut creature = AquaticCreature::new(origin, 0);
        creature.advance(&mut board);
        creature.advance(&mut board);
        board.set(obstruction, Ground::Solid);
        creature.advance(&mut board);
        assert_eq!(creature.arms[0].segments.len(), 1);
        let rotation_obstruction = origin + IVec2::new(1, 1);
        board.set(rotation_obstruction, Ground::Ridge);
        creature.advance(&mut board);
        assert_eq!(board.ground(rotation_obstruction), Some(Ground::Solid));
        assert!(creature.finished());
    }

    #[test]
    fn board_edges_stop_growth_and_clip_arm_rotation() {
        let origin = IVec2::ZERO;
        let mut board = Board::default();
        let mut creature = AquaticCreature::new(origin, 0);
        for _ in 0..4 {
            assert!(creature.advance(&mut board).is_some());
            assert!(creature.cells().all(|cell| board.ground(cell).is_some()));
        }
        assert_eq!(creature.arms[2].segments.len(), 0);
        assert_eq!(creature.arms[3].segments.len(), 0);
    }
}
