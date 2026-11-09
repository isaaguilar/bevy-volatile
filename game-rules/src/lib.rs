#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Walkable,
    Ridge,
    Hole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Walk,
    Jump,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementError {
    OutOfRange,
    Blocked,
    Occupied,
    InsufficientEnergy,
    UnsafeHover,
    RidgeBlocksJump,
}

pub type Cell = (i32, i32);

pub fn validate_movement(
    ground: impl Fn(Cell) -> Option<Ground>,
    origin: Cell,
    destination: Cell,
    opponent: Option<Cell>,
    energy: u8,
    hovering: bool,
    magic: u8,
) -> Result<Movement, MovementError> {
    if ground(origin).is_none() || ground(destination).is_none() {
        return Err(MovementError::OutOfRange);
    }
    let dx = i64::from(destination.0) - i64::from(origin.0);
    let dy = i64::from(destination.1) - i64::from(origin.1);
    let movement = match dx.abs().max(dy.abs()) {
        1 => Movement::Walk,
        2 => Movement::Jump,
        _ => return Err(MovementError::OutOfRange),
    };
    if opponent == Some(destination) {
        return Err(MovementError::Occupied);
    }
    if ground(destination) != Some(Ground::Walkable) {
        if !hovering {
            return Err(MovementError::Blocked);
        }
        let safe_exits = (-1i32..=1)
            .flat_map(|x| (-1i32..=1).map(move |y| (x, y)))
            .filter(|offset| *offset != (0, 0))
            .filter_map(|(x, y)| {
                Some((destination.0.checked_add(x)?, destination.1.checked_add(y)?))
            })
            .filter(|cell| ground(*cell) == Some(Ground::Walkable) && opponent != Some(*cell))
            .count();
        if magic < 2 || safe_exits < 2 {
            return Err(MovementError::UnsafeHover);
        }
    }
    if movement == Movement::Jump && energy == 0 {
        return Err(MovementError::InsufficientEnergy);
    }
    if movement == Movement::Jump && !hovering {
        let halfway = (
            (i64::from(origin.0) + dx / 2) as i32,
            (i64::from(origin.1) + dy / 2) as i32,
        );
        let other_halfway = (
            (i64::from(destination.0) - dx / 2) as i32,
            (i64::from(destination.1) - dy / 2) as i32,
        );
        if [halfway, other_halfway]
            .into_iter()
            .any(|cell| ground(cell) == Some(Ground::Ridge))
        {
            return Err(MovementError::RidgeBlocksJump);
        }
    }
    Ok(movement)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(cell: Cell) -> Option<Ground> {
        if !(0..5).contains(&cell.0) || !(0..5).contains(&cell.1) {
            return None;
        }
        Some(match cell {
            (2, 1) => Ground::Ridge,
            (2, 2) => Ground::Hole,
            _ => Ground::Walkable,
        })
    }

    #[test]
    fn validates_walk_jump_obstacles_and_energy() {
        let check = |destination, opponent, energy, hovering, magic| {
            validate_movement(
                board,
                (1, 1),
                destination,
                opponent,
                energy,
                hovering,
                magic,
            )
        };
        assert_eq!(check((1, 2), None, 0, false, 0), Ok(Movement::Walk));
        assert_eq!(check((1, 3), None, 1, false, 0), Ok(Movement::Jump));
        assert_eq!(
            check((1, 3), None, 0, false, 0),
            Err(MovementError::InsufficientEnergy)
        );
        assert_eq!(
            check((3, 1), None, 1, false, 0),
            Err(MovementError::RidgeBlocksJump)
        );
        assert_eq!(check((3, 1), None, 1, true, 2), Ok(Movement::Jump));
        assert_eq!(
            check((2, 2), None, 1, false, 2),
            Err(MovementError::Blocked)
        );
        assert_eq!(
            check((1, 2), Some((1, 2)), 1, false, 2),
            Err(MovementError::Occupied)
        );
        assert_eq!(
            check((1, 1), None, 1, false, 2),
            Err(MovementError::OutOfRange)
        );
        assert_eq!(
            check((4, 4), None, 1, false, 2),
            Err(MovementError::OutOfRange)
        );
    }

    #[test]
    fn unsafe_hover_requires_magic_and_two_unoccupied_exits() {
        assert_eq!(
            validate_movement(board, (1, 1), (2, 2), None, 1, true, 1),
            Err(MovementError::UnsafeHover)
        );
        assert_eq!(
            validate_movement(board, (1, 1), (2, 2), None, 1, true, 2),
            Ok(Movement::Walk)
        );
        let narrow = |cell| match cell {
            (0, 0) | (1, 0) => Some(Ground::Walkable),
            (1, 1) => Some(Ground::Hole),
            _ => None,
        };
        assert_eq!(
            validate_movement(narrow, (0, 0), (1, 1), Some((1, 0)), 1, true, 2),
            Err(MovementError::UnsafeHover)
        );
    }

    #[test]
    fn rejects_off_board_and_extreme_coordinates_without_overflow() {
        for destination in [(-1, 1), (5, 1), (i32::MIN, i32::MAX)] {
            assert_eq!(
                validate_movement(board, (1, 1), destination, None, 5, true, 5),
                Err(MovementError::OutOfRange)
            );
        }
        assert_eq!(
            validate_movement(board, (i32::MIN, i32::MAX), (1, 1), None, 5, true, 5),
            Err(MovementError::OutOfRange)
        );
    }
}
pub mod aquatic;
pub mod board;
pub mod environment;
pub mod ingredients;
pub mod initial;
pub mod placement;
pub mod serpent;
pub mod spawning;
pub mod thorn;
pub mod void;
