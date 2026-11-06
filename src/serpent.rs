use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::board::{Board, Ground};

const DIRECTIONS: [IVec2; 4] = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y];
const LENGTH: usize = 3;
const STEPS_PER_TURN: usize = 2;

pub struct Serpent {
    body: Vec<IVec2>,
    poison_age: Option<u8>,
    straight_frame: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerpentVisual {
    pub cells: Vec<IVec2>,
    pub venomous: bool,
    pub straight_frame: usize,
}

pub struct SerpentStep {
    pub visual: SerpentVisual,
    pub hit: Option<(usize, u8)>,
    pub impact: Option<IVec2>,
}

impl Serpent {
    pub fn new(origin: IVec2, board: &Board, players: [Option<IVec2>; 2], health: [u8; 2]) -> Self {
        let safe = |cell| {
            board
                .ground(cell)
                .is_some_and(|ground| ground != Ground::Hole)
                && !board.is_acidic(cell)
        };
        let mut candidates = Vec::new();
        for direction in DIRECTIONS {
            let middle = origin + direction;
            if board.ground(middle).is_none() {
                continue;
            }

            for next in DIRECTIONS {
                let tail = middle + next;
                if tail != origin && board.ground(tail).is_some() {
                    candidates.push(vec![origin, middle, tail]);
                }
            }
        }
        let body = candidates
            .into_iter()
            .min_by_key(|body| {
                let hazards = body[1..].iter().filter(|&&cell| !safe(cell)).count();
                let overlaps = body[1..]
                    .iter()
                    .filter(|cell| players.contains(&Some(**cell)))
                    .count();
                let snake = Self {
                    body: body.clone(),
                    poison_age: None,
                    straight_frame: 0,
                };
                let routes = players.map(|cell| {
                    cell.and_then(|cell| snake.route(board, cell))
                        .map_or(usize::MAX, |route| route.0)
                });
                let distance = if health[0] == health[1] {
                    routes[0].min(routes[1])
                } else {
                    routes[usize::from(health[1] > health[0])]
                };
                (hazards, overlaps, distance)
            })
            .expect("board has room for a three-cell serpent");
        Self {
            body,
            poison_age: None,
            straight_frame: 0,
        }
    }

    #[cfg(test)]
    pub fn cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.body.iter().copied()
    }

    pub fn venomous(&self) -> bool {
        self.poison_age.is_some_and(|age| age <= 1)
    }

    pub fn visual(&self) -> SerpentVisual {
        SerpentVisual {
            cells: self.body.clone(),
            venomous: self.venomous(),
            straight_frame: self.straight_frame,
        }
    }

    #[cfg(test)]
    pub fn advance(
        &mut self,
        board: &Board,
        players: [Option<IVec2>; 2],
        health: &mut [u8; 2],
    ) -> bool {
        self.advance_animated(board, players, health, &mut Vec::new())
    }

    pub fn advance_animated(
        &mut self,
        board: &Board,
        players: [Option<IVec2>; 2],
        health: &mut [u8; 2],
        steps: &mut Vec<SerpentStep>,
    ) -> bool {
        if let Some(&cell) = self
            .body
            .iter()
            .find(|&&cell| board.ground(cell) == Some(Ground::Hole) || board.is_acidic(cell))
        {
            steps.push(SerpentStep {
                visual: self.visual(),
                hit: None,
                impact: Some(cell),
            });
            return false;
        }
        if let Some(age) = &mut self.poison_age {
            *age = age.saturating_add(1);
        }
        self.touch_poison(board);

        let routes: [Option<(usize, IVec2)>; 2] =
            players.map(|target| target.and_then(|target| self.route(board, target)));
        let target = match (players[0], players[1]) {
            (None, None) => return true,
            (Some(_), None) => 0,
            (None, Some(_)) => 1,
            (Some(_), Some(_)) if health[0] != health[1] => usize::from(health[1] > health[0]),
            (Some(_), Some(_)) => {
                let distance = |index: usize| routes[index].map_or(usize::MAX, |route| route.0);
                usize::from(distance(1) < distance(0))
            }
        };
        let Some(destination) = players[target] else {
            return true;
        };
        if self.strike_at(self.body[0], target, players, health, steps) {
            return false;
        }
        for _ in 0..STEPS_PER_TURN {
            let Some((_, next)) = self.route(board, destination) else {
                break;
            };
            self.body.insert(0, next);
            self.body.truncate(LENGTH);
            if self.body[0] - self.body[1] == self.body[1] - self.body[2] {
                self.straight_frame ^= 1;
            }
            if board.ground(next) == Some(Ground::Hole) || board.is_acidic(next) {
                steps.push(SerpentStep {
                    visual: self.visual(),
                    hit: None,
                    impact: Some(next),
                });
                return false;
            }
            self.touch_poison(board);
            if self.strike_at(next, target, players, health, steps) {
                return false;
            }
            steps.push(SerpentStep {
                visual: self.visual(),
                hit: None,
                impact: None,
            });
        }
        true
    }

    fn strike_at(
        &self,
        cell: IVec2,
        target: usize,
        players: [Option<IVec2>; 2],
        health: &mut [u8; 2],
        steps: &mut Vec<SerpentStep>,
    ) -> bool {
        let victim = if players[target] == Some(cell) {
            Some(target)
        } else {
            players.iter().position(|player| *player == Some(cell))
        };
        let Some(victim) = victim else {
            return false;
        };
        self.hit(&mut health[victim]);
        steps.push(SerpentStep {
            visual: self.visual(),
            hit: Some((victim, health[victim])),
            impact: Some(cell),
        });
        true
    }

    fn touch_poison(&mut self, board: &Board) {
        if self
            .body
            .iter()
            .any(|&cell| board.is_poisoned(cell) && !board.is_frozen(cell))
        {
            self.poison_age = Some(0);
        }
    }

    fn hit(&self, health: &mut u8) {
        if self.venomous() {
            *health = 0;
        } else if *health > 0 {
            *health = health.saturating_sub(3).max(1);
        }
    }

    fn route(&self, board: &Board, destination: IVec2) -> Option<(usize, IVec2)> {
        if self.body[0] == destination {
            return Some((0, destination));
        }
        let mut visited = HashSet::from([self.body.clone()]);
        let mut queue = VecDeque::from([(self.body.clone(), 0, None)]);
        while let Some((body, distance, first)) = queue.pop_front() {
            for direction in DIRECTIONS {
                let next = body[0] + direction;
                if board.ground(next).is_none()
                    || body[..body.len().min(LENGTH - 1)].contains(&next)
                {
                    continue;
                }
                let first = first.unwrap_or(next);
                if next == destination {
                    return Some((distance + 1, first));
                }
                let mut next_body = body.clone();
                next_body.insert(0, next);
                next_body.truncate(LENGTH);
                if visited.insert(next_body.clone()) {
                    queue.push_back((next_body, distance + 1, Some(first)));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snake(body: &[IVec2]) -> Serpent {
        Serpent {
            body: body.to_vec(),
            poison_age: None,
            straight_frame: 0,
        }
    }

    #[test]
    fn straight_frames_alternate_on_steps_and_persist_without_movement() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let mut health = [10, 10];
        let mut steps = Vec::new();
        assert!(serpent.advance_animated(
            &board,
            [Some(IVec2::new(9, 3)), None],
            &mut health,
            &mut steps
        ));
        assert_eq!(
            steps
                .iter()
                .map(|step| step.visual.straight_frame)
                .collect::<Vec<_>>(),
            [1, 0]
        );
        assert_eq!(serpent.visual().straight_frame, 0);
        steps.clear();
        assert!(serpent.advance_animated(&board, [None, None], &mut health, &mut steps));
        assert!(steps.is_empty());
        assert_eq!(serpent.visual().straight_frame, 0);
    }

    #[test]
    fn turning_uses_the_corner_without_advancing_the_straight_frame() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let mut steps = Vec::new();
        serpent.advance_animated(
            &board,
            [Some(IVec2::new(3, 0)), None],
            &mut [10, 10],
            &mut steps,
        );
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].visual.straight_frame, 0);
        assert_eq!(steps[1].visual.straight_frame, 1);
    }

    #[test]
    fn pursues_healthiest_player_even_past_a_closer_opponent() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let players = [Some(IVec2::new(3, 4)), Some(IVec2::new(8, 3))];
        let mut health = [6, 10];
        assert!(serpent.advance(&board, players, &mut health));
        assert_eq!(serpent.body[0], IVec2::new(5, 3));
        assert_eq!(health, [6, 10]);
        assert!(serpent.advance(&board, players, &mut health));
        assert!(!serpent.advance(&board, players, &mut health));
        assert_eq!(health, [6, 7]);
    }

    #[test]
    fn non_target_intercepts_on_either_step_or_at_the_start() {
        let board = Board::default();
        for blocker_x in [3, 4, 5] {
            let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
            let blocker = IVec2::new(blocker_x, 3);
            let mut health = [6, 10];
            let mut steps = Vec::new();
            assert!(!serpent.advance_animated(
                &board,
                [Some(blocker), Some(IVec2::new(8, 3))],
                &mut health,
                &mut steps,
            ));
            assert_eq!(health, [3, 10]);
            assert_eq!(serpent.body[0], blocker);
            assert_eq!(steps.last().unwrap().hit, Some((0, 3)));
            assert_eq!(steps.last().unwrap().impact, Some(blocker));
        }
    }

    #[test]
    fn poisonous_interception_is_lethal_and_hazards_still_stop_coatl_first() {
        let mut board = Board::default();
        let blocker = IVec2::new(4, 3);
        board.add_poison(blocker);
        let players = [Some(blocker), Some(IVec2::new(8, 3))];
        let mut health = [6, 10];
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        assert!(!serpent.advance(&board, players, &mut health));
        assert_eq!(health, [0, 10]);

        assert!(board.make_hole(blocker));
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let mut health = [6, 10];
        assert!(!serpent.advance(&board, players, &mut health));
        assert_eq!(health, [6, 10]);
    }

    #[test]
    fn equal_health_targets_the_closest_player() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let mut health = [10, 10];
        assert!(!serpent.advance(
            &board,
            [Some(IVec2::new(8, 3)), Some(IVec2::new(3, 4))],
            &mut health,
        ));
        assert_eq!(health, [10, 7]);
    }

    #[test]
    fn retargets_when_the_other_player_becomes_healthier() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(2, 3), IVec2::new(1, 3), IVec2::new(0, 3)]);
        let players = [Some(IVec2::new(8, 3)), Some(IVec2::new(1, 6))];
        let mut health = [10, 9];
        assert!(serpent.advance(&board, players, &mut health));
        assert_eq!(serpent.body[0], IVec2::new(4, 3));
        health = [4, 9];
        assert!(serpent.advance(&board, players, &mut health));
        assert!(serpent.body[0].y > 3);
    }

    #[test]
    fn routes_around_its_body_and_moves_two_cells_each_turn() {
        let board = Board::default();
        let mut serpent = snake(&[IVec2::new(3, 3), IVec2::new(2, 3), IVec2::new(1, 3)]);
        let mut health = [10, 0];
        let players = [Some(IVec2::new(0, 3)), None];
        assert!(serpent.advance(&board, players, &mut health));
        assert_eq!(serpent.body.len(), LENGTH);
        assert_ne!(serpent.body[1], IVec2::new(2, 3));
        assert_eq!(
            serpent.body.iter().copied().collect::<HashSet<_>>().len(),
            3
        );
        assert_eq!(health[0], 10);
        assert!(serpent.advance(&board, players, &mut health));
        assert!(!serpent.advance(&board, players, &mut health));
        assert_eq!(health[0], 7);
    }

    #[test]
    fn unpoisoned_hit_leaves_at_least_half_a_heart() {
        let board = Board::default();
        for (starting, expected) in [(10, 7), (4, 1), (3, 1), (2, 1), (1, 1), (0, 0)] {
            let mut serpent = snake(&[IVec2::new(1, 3), IVec2::new(0, 3), IVec2::new(0, 2)]);
            let mut health = [starting, 0];
            assert!(!serpent.advance(&board, [Some(IVec2::new(2, 3)), None], &mut health));
            assert_eq!(health[0], expected);
        }
    }

    #[test]
    fn poison_is_lethal_on_the_hit_turn_and_the_following_turn_only() {
        for (target_x, lethal) in [(1, true), (4, true), (6, true), (8, false)] {
            let mut board = Board::default();
            board.add_poison(IVec2::new(1, 3));
            let mut serpent = snake(&[IVec2::new(0, 3), IVec2::new(0, 2), IVec2::new(0, 1)]);
            let mut health = [10, 0];
            let players = [Some(IVec2::new(target_x, 3)), None];
            for _ in 0..4 {
                if !serpent.advance(&board, players, &mut health) {
                    break;
                }
            }
            assert_eq!(health[0], if lethal { 0 } else { 7 });
        }
    }

    #[test]
    fn frozen_poison_does_not_make_the_serpent_lethal() {
        let mut board = Board::default();
        let target = IVec2::new(2, 3);
        board.make_water(target);
        board.add_poison(target);
        assert!(board.freeze_water(target));
        let mut serpent = snake(&[IVec2::new(1, 3), IVec2::new(0, 3), IVec2::new(0, 2)]);
        let mut health = [10, 0];
        assert!(!serpent.advance(&board, [Some(target), None], &mut health));
        assert_eq!(health[0], 7);
    }

    #[test]
    fn poison_touching_the_body_can_make_the_hit_lethal() {
        let mut board = Board::default();
        board.add_poison(IVec2::new(0, 3));
        let mut serpent = snake(&[IVec2::new(1, 3), IVec2::new(0, 3), IVec2::new(0, 2)]);
        let mut health = [10, 0];
        assert!(!serpent.advance(&board, [Some(IVec2::new(2, 3)), None], &mut health));
        assert_eq!(health[0], 0);
    }

    #[test]
    fn acid_and_holes_kill_a_serpent_on_its_direct_route() {
        for hole in [false, true] {
            let mut board = Board::default();
            let hazard = IVec2::new(2, 3);
            if hole {
                assert!(board.make_hole(hazard));
            } else {
                board.add_acid(hazard);
            }
            let mut serpent = snake(&[IVec2::new(1, 3), IVec2::new(0, 3), IVec2::new(0, 2)]);
            let mut health = [10, 0];
            assert!(!serpent.advance(&board, [Some(IVec2::new(3, 3)), None], &mut health));
            assert_eq!(health[0], 10);
        }
    }

    #[test]
    fn new_serpents_have_three_cells_and_die_if_trapped_in_hazards() {
        let mut board = Board::default();
        let origin = IVec2::ZERO;
        for cell in [IVec2::X, IVec2::Y] {
            board.add_acid(cell);
        }
        let mut serpent = Serpent::new(origin, &board, [Some(IVec2::new(9, 6)), None], [10, 0]);
        assert_eq!(serpent.cells().count(), LENGTH);
        let mut health = [10, 0];
        assert!(!serpent.advance(&board, [Some(IVec2::new(9, 6)), None], &mut health));
        assert_eq!(health, [10, 0]);
    }
}
