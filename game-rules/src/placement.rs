use bevy::prelude::IVec2;

use crate::{
    board::{Board, Ground},
    ingredients,
};

pub fn placement_error(
    board: &Board,
    index: usize,
    cell: IVec2,
    reactive_item: bool,
    has_frost: bool,
) -> Option<&'static str> {
    if index >= ingredients::COUNT {
        return Some("Invalid ingredient");
    }
    let Some(ground) = board.ground(cell) else {
        return Some("Cannot place an ingredient outside the board");
    };
    if index != ingredients::STONE_DUST && ground == Ground::Hole {
        return Some("Only Stone Dust can be placed in a hole");
    }
    if index == ingredients::THORNHEART && board.is_acidic(cell) {
        return Some("Cannot place Thornheart on acid - choose another tile");
    }
    if (index == ingredients::OIL && has_frost)
        || (index == ingredients::FROST_CRYSTAL && board.has_oil(cell))
    {
        return Some("Frost Crystal and Tlexochtli Ember cannot share a tile");
    }
    if index == ingredients::OIL && board.is_frozen(cell) {
        return Some("Cannot place Tlexochtli Ember on frozen water");
    }
    if index == ingredients::OIL && board.has_oil(cell) {
        return Some("Tile already has Tlexochtli Ember");
    }
    let reacts = reactive_item || board.has_life_essence(cell);
    if index == ingredients::VOID_DUST && ground == Ground::Water && !reacts {
        return Some("Void Dust needs Obsidian or Life Essence on water");
    }
    if index == ingredients::STONE_DUST
        && !reacts
        && !board.is_water(cell)
        && ground != Ground::Hole
        && !board.is_acidic(cell)
        && !board.is_poisoned(cell)
    {
        return Some("Stone Dust needs water, a hole, acid, poison, Obsidian, or Life Essence");
    }
    if index == ingredients::WATER && ground == Ground::Water {
        return Some("Cannot place water on a water tile - choose another tile");
    }
    None
}

pub fn queued_placement_error(
    board: &Board,
    index: usize,
    cell: IVec2,
    first: Option<(usize, IVec2)>,
    reactive_item: bool,
    has_frost: bool,
) -> Option<&'static str> {
    if index >= ingredients::COUNT || board.ground(cell).is_none() {
        return placement_error(board, index, cell, reactive_item, has_frost);
    }
    if first.is_some_and(|(first_index, first_cell)| {
        first_cell == cell
            && matches!(
                (first_index, index),
                (ingredients::FROST_CRYSTAL, ingredients::OIL)
                    | (ingredients::OIL, ingredients::FROST_CRYSTAL)
            )
    }) {
        return Some("Frost Crystal and Tlexochtli Ember cannot share a tile");
    }
    let error = placement_error(board, index, cell, reactive_item, has_frost)?;
    if let Some((first_index, first_cell)) = first
        && first_cell == cell
        && ((first_index == ingredients::STONE_DUST && board.ground(cell) == Some(Ground::Hole))
            || (first_index == ingredients::LIFE_ESSENCE
                && index == ingredients::THORNHEART
                && board.is_acidic(cell))
            || (matches!(index, ingredients::VOID_DUST | ingredients::STONE_DUST)
                && matches!(
                    first_index,
                    ingredients::OBSIDIAN | ingredients::LIFE_ESSENCE
                ))
            || (index == ingredients::STONE_DUST
                && matches!(
                    first_index,
                    ingredients::WATER | ingredients::ACID | ingredients::POISON
                )))
    {
        return None;
    }
    Some(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_ingredients_and_off_board_cells() {
        let board = Board::default();
        assert!(placement_error(&board, ingredients::COUNT, IVec2::ZERO, false, false).is_some());
        assert!(
            placement_error(&board, ingredients::WATER, IVec2::new(-1, 0), false, false).is_some()
        );
        assert!(
            queued_placement_error(
                &board,
                ingredients::STONE_DUST,
                IVec2::new(-1, 0),
                Some((ingredients::OBSIDIAN, IVec2::new(-1, 0))),
                false,
                false,
            )
            .is_some()
        );
    }

    #[test]
    fn queued_reactions_allow_compatible_placements_but_reject_frost_and_ember() {
        let mut board = Board::default();
        let cell = IVec2::new(3, 3);
        board.set(cell, Ground::Hole);
        assert!(placement_error(&board, ingredients::WATER, cell, false, false).is_some());
        assert_eq!(
            queued_placement_error(
                &board,
                ingredients::WATER,
                cell,
                Some((ingredients::STONE_DUST, cell)),
                false,
                false
            ),
            None
        );
        board.set(cell, Ground::Solid);
        assert!(
            queued_placement_error(
                &board,
                ingredients::OIL,
                cell,
                Some((ingredients::FROST_CRYSTAL, cell)),
                false,
                false
            )
            .is_some()
        );
    }
}
