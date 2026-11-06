use crate::*;

const ACTION_DELAY: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BotAction {
    Move(IVec2),
    Place { item: usize, cell: IVec2 },
    Pickup,
    Rest,
}

#[derive(Resource)]
pub(crate) struct BotController {
    pub(crate) computer_mode: bool,
    pub(crate) bot_turn: bool,
    pub(crate) action: Option<BotAction>,
    timer: Timer,
}

impl Default for BotController {
    fn default() -> Self {
        Self {
            computer_mode: false,
            bot_turn: false,
            action: None,
            timer: Timer::from_seconds(ACTION_DELAY, TimerMode::Once),
        }
    }
}

pub(crate) fn plan(
    time: Res<Time>,
    frontend: Res<menu::Frontend>,
    turn: Res<TurnState>,
    game: (Res<Board>, Res<PlayerHealth>),
    players: Query<(&Player, &Transform)>,
    collectibles: Query<&Collectible>,
    mut bot: ResMut<BotController>,
) {
    let (board, health) = game;
    let computer_mode = frontend.computer_game();
    let bot_turn = computer_mode && turn.active == PlayerId::Two;
    if bot.bot_turn != bot_turn {
        bot.timer.reset();
    }
    bot.computer_mode = computer_mode;
    bot.bot_turn = bot_turn;
    bot.action = None;
    if !bot_turn
        || turn.animation_active()
        || turn.pending_placement.as_ref().is_some_and(|p| p.ready)
    {
        return;
    }
    if !bot.timer.tick(time.delta()).just_finished() {
        return;
    }
    bot.timer.reset();

    let Some(player) = players.iter().find_map(|(id, transform)| {
        (id.0 == PlayerId::Two).then_some(transform.translation.truncate())
    }) else {
        error!("Computer player is missing");
        return;
    };
    let Some(opponent) = players.iter().find_map(|(id, transform)| {
        (id.0 == PlayerId::One).then_some(transform.translation.truncate())
    }) else {
        error!("Computer opponent is missing");
        return;
    };
    let items: Vec<_> = collectibles.iter().collect();
    bot.action = Some(choose_action(
        &board, &turn, &health, player, opponent, &items,
    ));
}

fn danger(board: &Board, cell: IVec2, hovering: bool) -> i32 {
    let mut damage = i32::from(board.has_thorns(cell)) * 2;
    if !hovering {
        damage += i32::from(board.is_frozen(cell));
        damage += i32::from(board.is_poisoned(cell) && !board.is_frozen(cell));
        damage += i32::from(board.is_acidic(cell));
        damage += i32::from(board.is_electrified_acid(cell)) * 2;
    }
    damage + i32::from(board.aquatic_contacts(cell).next().is_some()) * 2
}

fn available(turn: &TurnState, item: usize) -> bool {
    is_unlimited(item) || turn.inventory[PlayerId::Two.index()][item] > 0
}

fn can_place(board: &Board, turn: &TurnState, player: Vec2, item: usize, cell: IVec2) -> bool {
    available(turn, item)
        && item_target(player, cell_center(cell)).is_some_and(|target| {
            turn.strength[PlayerId::Two.index()] as u32 >= item_energy_cost(&target)
                && queued_placement_error(board, item, cell, turn.pending_placement.as_ref(), false)
                    .is_none()
        })
}

fn choose_action(
    board: &Board,
    turn: &TurnState,
    health: &PlayerHealth,
    player: Vec2,
    opponent: Vec2,
    items: &[&Collectible],
) -> BotAction {
    let Some(cell) = board_cell(player) else {
        error!("Computer player is outside the board");
        return BotAction::Rest;
    };
    let Some(enemy) = board_cell(opponent) else {
        error!("Computer opponent is outside the board");
        return BotAction::Rest;
    };
    let index = PlayerId::Two.index();
    let hovering = turn.hovering[index];
    let frozen = turn.frozen_turns[index] > 0;
    let current_danger = danger(board, cell, hovering);

    if frozen
        && turn.pending_placement.is_none()
        && !board.has_oil(cell)
        && (!board.is_water(cell) || board.is_frozen(cell))
        && !items.iter().any(|item| item.cell == cell)
        && can_place(board, turn, player, SUNSTONE_ITEM_INDEX, cell)
    {
        return BotAction::Place {
            item: SUNSTONE_ITEM_INDEX,
            cell,
        };
    }

    let best_move = if frozen {
        None
    } else {
        choose_move(board, turn, player, opponent, items)
    };
    if let Some((destination, _)) = best_move
        && (must_land(turn, board, player)
            || (current_danger > 0 && danger(board, destination, hovering) < current_danger))
    {
        return BotAction::Move(destination);
    }

    if let Some(pending) = &turn.pending_placement {
        if pending.index == WATER_ITEM_INDEX
            && pending.target.cell == enemy
            && !board.has_oil(enemy)
            && !steam_cells(board, enemy, turn.wind).contains(&cell)
            && can_place(board, turn, player, SUNSTONE_ITEM_INDEX, enemy)
        {
            return BotAction::Place {
                item: SUNSTONE_ITEM_INDEX,
                cell: enemy,
            };
        }
        return BotAction::Rest;
    }

    if !must_land(turn, board, player)
        && items
            .iter()
            .any(|item| item.cell == cell && !is_unlimited(item.index))
    {
        return BotAction::Pickup;
    }
    if health.0[index] < (HEART_COUNT * 2) as u8
        && !hovering
        && board.is_water(cell)
        && !board.is_frozen(cell)
        && !board.has_life_essence(cell)
        && current_danger == 0
        && can_place(board, turn, player, LIFE_ESSENCE_ITEM_INDEX, cell)
    {
        return BotAction::Place {
            item: LIFE_ESSENCE_ITEM_INDEX,
            cell,
        };
    }

    if !must_land(turn, board, player) && !items.iter().any(|item| item.cell == enemy) {
        let water = board.is_water(enemy);
        let electric_water = board.connected_water(enemy);
        let shock_safe = hovering
            || electric_water
                .iter()
                .all(|water| (*water - cell).abs().max_element() > 1);
        if shock_safe && can_place(board, turn, player, STORM_SHARD_ITEM_INDEX, enemy) {
            return BotAction::Place {
                item: STORM_SHARD_ITEM_INDEX,
                cell: enemy,
            };
        }
        if water
            && !board.is_frozen(enemy)
            && !board.has_oil(enemy)
            && !steam_cells(board, enemy, turn.wind).contains(&cell)
            && can_place(board, turn, player, SUNSTONE_ITEM_INDEX, enemy)
        {
            return BotAction::Place {
                item: SUNSTONE_ITEM_INDEX,
                cell: enemy,
            };
        }
        if !turn.hovering[PlayerId::One.index()]
            && turn.frozen_turns[PlayerId::One.index()] == 0
            && (!water
                || hovering
                || (enemy - cell).abs().max_element() > 1
                || !board.is_water(cell))
            && can_place(board, turn, player, FROST_CRYSTAL_ITEM_INDEX, enemy)
        {
            return BotAction::Place {
                item: FROST_CRYSTAL_ITEM_INDEX,
                cell: enemy,
            };
        }
        if !board.is_poisoned(enemy) && can_place(board, turn, player, ACID_ITEM_INDEX, enemy) {
            return BotAction::Place {
                item: ACID_ITEM_INDEX,
                cell: enemy,
            };
        }
        if !board.is_frozen(enemy)
            && !board.is_acidic(enemy)
            && (!water || !board.connected_water(enemy).contains(&cell))
            && can_place(board, turn, player, POISON_ITEM_INDEX, enemy)
        {
            return BotAction::Place {
                item: POISON_ITEM_INDEX,
                cell: enemy,
            };
        }
        if board.ground(enemy) == Some(Ground::Solid)
            && !board.is_wet(enemy)
            && !board.has_oil(enemy)
            && !board.is_acidic(enemy)
            && !board.is_poisoned(enemy)
            && available(turn, SUNSTONE_ITEM_INDEX)
            && !steam_cells(board, enemy, turn.wind).contains(&cell)
            && can_place(board, turn, player, WATER_ITEM_INDEX, enemy)
        {
            return BotAction::Place {
                item: WATER_ITEM_INDEX,
                cell: enemy,
            };
        }
    }

    if let Some((destination, improvement)) = best_move
        && (improvement > 0 || must_land(turn, board, player))
    {
        return BotAction::Move(destination);
    }
    BotAction::Rest
}

fn choose_move(
    board: &Board,
    turn: &TurnState,
    player: Vec2,
    opponent: Vec2,
    items: &[&Collectible],
) -> Option<(IVec2, i32)> {
    let origin = board_cell(player)?;
    let enemy = board_cell(opponent)?;
    let index = PlayerId::Two.index();
    let hovering = turn.hovering[index];
    let current_danger = danger(board, origin, hovering);
    let distance = |from: IVec2| {
        items
            .iter()
            .filter(|item| !is_unlimited(item.index))
            .map(|item| (from - item.cell).abs().max_element())
            .min()
            .unwrap_or_else(|| ((from - enemy).abs().max_element() - 3).abs())
    };
    let initial_distance = distance(origin);
    (-2..=2)
        .flat_map(|y| (-2..=2).map(move |x| origin + IVec2::new(x, y)))
        .filter_map(|cell| {
            let target = drop_target(player, cell_center(cell))?;
            let movement = movement_target(
                board,
                &target,
                opponent,
                turn.strength[index],
                hovering,
                turn.magic[index],
            )
            .ok()?;
            let improvement = (current_danger - danger(board, cell, hovering)) * 100
                + (initial_distance - distance(cell)) * 10
                - i32::from(movement == Movement::Jump);
            Some((cell, improvement))
        })
        .max_by_key(|(_, score)| *score)
}
