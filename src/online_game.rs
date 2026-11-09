use bevy::ecs::system::SystemParam;
use std::collections::VecDeque;

use volatile_online_protocol::{
    Animation, AnimationEvent, ClientMessage, GameAction, Growth, MatchView, Seat, Terrain,
    WeatherEffect,
};

use crate::*;

#[derive(Resource, Default)]
pub(crate) struct AppliedMatch {
    room_id: Option<String>,
    revision: Option<u64>,
}

impl AppliedMatch {
    pub(crate) fn is_current(&self, online: &online::Online) -> bool {
        self.room_id == online.room_id
            && self.revision.is_some()
            && self.revision == online.game.as_ref().map(|game| game.revision)
    }
}

#[derive(Resource, Default)]
pub(crate) struct AnimationPlayback {
    room_id: Option<String>,
    elapsed: f32,
    until: f32,
    scheduled: VecDeque<(f32, Animation)>,
    pub(crate) serpents: Option<(Vec<serpent::SerpentVisual>, Option<IVec2>)>,
    pub(crate) aquatic: Option<Vec<aquatic::AquaticVisual>>,
    earthquake: Option<f32>,
}

impl AnimationPlayback {
    pub(super) fn active(&self) -> bool {
        self.elapsed < self.until || !self.scheduled.is_empty()
    }

    pub(crate) fn earthquake_elapsed(&self) -> Option<f32> {
        self.earthquake
            .map(|start| self.elapsed - start)
            .filter(|elapsed| *elapsed < WEATHER_ANIMATION_SECONDS)
    }
}

#[derive(Component)]
pub(crate) struct OnlineAnimation {
    start: f32,
    kind: Animation,
}

type AnimationSpriteParts = (
    Entity,
    &'static OnlineAnimation,
    &'static mut Sprite,
    &'static mut Transform,
);

#[derive(SystemParam)]
pub(crate) struct AnimationVisuals<'w, 's> {
    players: Query<'w, 's, (&'static Player, &'static mut Transform), Without<OnlineAnimation>>,
    sprites: Query<'w, 's, AnimationSpriteParts, Without<Player>>,
}

fn valid_cell((x, y): (i32, i32)) -> bool {
    (0..COLUMNS as i32).contains(&x) && (0..ROWS as i32).contains(&y)
}

fn valid_animation(event: &AnimationEvent) -> bool {
    event.at_ms <= 256_000
        && match &event.animation {
            Animation::MovePlayer { x, y, .. } => valid_cell((*x, *y)),
            Animation::Throw {
                ingredient,
                from,
                to,
                ..
            } => *ingredient < INVENTORY_ITEMS.len() && valid_cell(*from) && valid_cell(*to),
            Animation::Effect { x, y, .. } => valid_cell((*x, *y)),
            Animation::Serpents { visuals, impact } => {
                impact.is_none_or(valid_cell)
                    && visuals.as_ref().is_none_or(|visuals| {
                        visuals.iter().all(|visual| {
                            serpent::Serpent::from_visual(&serpent::SerpentVisual {
                                cells: visual
                                    .cells
                                    .iter()
                                    .map(|&(x, y)| IVec2::new(x, y))
                                    .collect(),
                                venomous: visual.venomous,
                                straight_frame: visual.straight_frame,
                            })
                            .is_ok()
                        })
                    })
            }
            Animation::Aquatic { visuals } => visuals.as_ref().is_none_or(|visuals| {
                visuals.iter().all(|visual| {
                    valid_cell((visual.x, visual.y))
                        && visual.lengths.iter().all(|length| *length <= 3)
                        && visual.sweep_frame.is_none_or(|frame| frame <= 9)
                })
            }),
            Animation::Weather { .. } => true,
        }
}

pub(crate) fn play_animations(
    mut commands: Commands,
    time: Res<Time>,
    mut online: ResMut<online::Online>,
    applied: Res<AppliedMatch>,
    mut playback: ResMut<AnimationPlayback>,
    assets: (Res<InventoryImages>, Res<effects::Effects>),
    visuals: AnimationVisuals,
) {
    let (images, effects) = assets;
    let AnimationVisuals {
        mut players,
        mut sprites,
    } = visuals;
    if online.game.is_none() || online.room_id != playback.room_id {
        for (entity, ..) in &mut sprites {
            commands.entity(entity).despawn();
        }
        *playback = AnimationPlayback {
            room_id: online.room_id.clone(),
            ..default()
        };
        if online.game.is_none() {
            online.animations.clear();
            return;
        }
    }
    if (applied.room_id != online.room_id
        || applied.revision != online.game.as_ref().map(|game| game.revision))
        && let Some(game) = online.game.as_ref()
        && let Err(error) = validate_view(&game.state)
    {
        online.animations.clear();
        online.status = error.into();
        return;
    }
    playback.elapsed += time.delta_secs();
    for mut events in std::mem::take(&mut online.animations) {
        if events.iter().any(|event| !valid_animation(event)) {
            online.status = "Invalid match animation".into();
            break;
        }
        events.sort_by_key(|event| event.at_ms);
        let start = playback.elapsed.max(playback.until);
        for event in events {
            let at = start + event.at_ms as f32 / 1000.0;
            playback.until = playback
                .until
                .max(at + headless::animation_duration(&event.animation));
            playback.scheduled.push_back((at, event.animation));
        }
    }
    while playback
        .scheduled
        .front()
        .is_some_and(|(at, _)| *at <= playback.elapsed)
    {
        let (start, animation) = playback
            .scheduled
            .pop_front()
            .expect("scheduled event exists");
        let (sprite, transform) = match &animation {
            Animation::MovePlayer { seat, x, y } => {
                let player = if *seat == Seat::Host {
                    PlayerId::One
                } else {
                    PlayerId::Two
                };
                if let Some((_, mut transform)) =
                    players.iter_mut().find(|(owner, _)| owner.0 == player)
                {
                    let position = cell_center(IVec2::new(*x, *y));
                    transform.translation.x = position.x;
                    transform.translation.y = position.y;
                } else {
                    online.status = "Match player is missing".into();
                }
                // Render the move before starting same-time reactions.
                break;
            }
            Animation::Serpents { visuals, impact } => {
                playback.serpents = visuals.as_ref().map(|visuals| {
                    (
                        visuals
                            .iter()
                            .map(|visual| serpent::SerpentVisual {
                                cells: visual
                                    .cells
                                    .iter()
                                    .map(|&(x, y)| IVec2::new(x, y))
                                    .collect(),
                                venomous: visual.venomous,
                                straight_frame: visual.straight_frame,
                            })
                            .collect(),
                        impact.map(|(x, y)| IVec2::new(x, y)),
                    )
                });
                continue;
            }
            Animation::Aquatic { visuals } => {
                playback.aquatic = visuals.as_ref().map(|visuals| {
                    visuals
                        .iter()
                        .map(|visual| aquatic::AquaticVisual {
                            id: visual.id,
                            origin: IVec2::new(visual.x, visual.y),
                            lengths: visual.lengths,
                            sweep_frame: visual.sweep_frame,
                        })
                        .collect()
                });
                continue;
            }
            Animation::Weather { kind } => {
                if *kind == WeatherEffect::Earthquake {
                    playback.earthquake = Some(start);
                    continue;
                }
                (
                    Sprite {
                        color: Color::srgba(0.1, 0.4, 1.0, 0.0),
                        custom_size: Some(Vec2::new(
                            BOARD_RIGHT - BOARD_LEFT,
                            BOARD_TOP - BOARD_BOTTOM,
                        )),
                        ..default()
                    },
                    Transform::from_xyz(
                        (BOARD_LEFT + BOARD_RIGHT) / 2.0,
                        (BOARD_TOP + BOARD_BOTTOM) / 2.0,
                        2.0,
                    ),
                )
            }
            Animation::Throw {
                ingredient, from, ..
            } => {
                let center = cell_center(IVec2::new(from.0, from.1));
                (
                    images.sprite(*ingredient, GRID_BLOCK_SIZE / 2.0),
                    Transform::from_xyz(center.x, center.y, 1.8),
                )
            }
            Animation::Effect { effect, x, y } => {
                let center = cell_center(IVec2::new(*x, *y));
                let effect = (*effect).into();
                (
                    effects::visual_sprite(&effects, effect),
                    Transform::from_xyz(center.x, center.y, effects::visual_layer(effect)),
                )
            }
        };
        commands.spawn((
            OnlineAnimation {
                start,
                kind: animation,
            },
            sprite,
            transform,
        ));
    }
    for (entity, animation, mut sprite, mut transform) in &mut sprites {
        let elapsed = (playback.elapsed - animation.start).max(0.0);
        if elapsed >= headless::animation_duration(&animation.kind) {
            commands.entity(entity).despawn();
            continue;
        }
        match animation.kind {
            Animation::Throw {
                from, to, corner, ..
            } => {
                let origin = cell_center(IVec2::new(from.0, from.1));
                let destination = cell_center(IVec2::new(to.0, to.1))
                    + if corner {
                        Vec2::splat(ITEM_CORNER_OFFSET)
                    } else {
                        Vec2::ZERO
                    };
                let progress = elapsed / ITEM_FLIGHT_SECONDS;
                let position = origin.lerp(destination, progress)
                    + Vec2::Y * (4.0 * ITEM_FLIGHT_HEIGHT * progress * (1.0 - progress));
                transform.translation.x = position.x;
                transform.translation.y = position.y;
            }
            Animation::Effect { effect, .. } => {
                effects::visual_frame(&mut sprite, effect.into(), elapsed);
            }
            Animation::Weather { .. } => {
                let progress = (elapsed / WEATHER_ANIMATION_SECONDS).clamp(0.0, 1.0);
                sprite.color =
                    Color::srgba(0.1, 0.4, 1.0, 0.4 * (1.0 - (2.0 * progress - 1.0).abs()));
            }
            Animation::MovePlayer { .. }
            | Animation::Serpents { .. }
            | Animation::Aquatic { .. } => unreachable!(),
        }
    }
}

#[derive(Component)]
struct PendingMarker;

#[derive(Component)]
pub(crate) struct DisabledControls(PlayerId);

#[derive(Component)]
pub(crate) struct TurnIndicator;

#[derive(Component)]
pub(crate) struct SideAvatar;

#[derive(Component)]
pub(crate) struct TurnTimer(pub PlayerId);

#[derive(Default)]
pub(crate) struct DisplayClock {
    generation: Option<u64>,
    observed: std::time::Duration,
}

pub(crate) fn update_turn_timer(
    time: Res<Time<Real>>,
    frontend: Res<menu::Frontend>,
    online: Res<online::Online>,
    applied: Res<AppliedMatch>,
    playback: Res<AnimationPlayback>,
    mut display: Local<DisplayClock>,
    mut timers: Query<(&TurnTimer, &mut Text, &mut Visibility)>,
) {
    if display.generation != Some(online.clock_generation) {
        display.generation = Some(online.clock_generation);
        display.observed = time.elapsed();
    }
    let remaining = online.turn_clock.as_ref().map(|clock| {
        let elapsed = if clock.running {
            time.elapsed()
                .saturating_sub(display.observed)
                .as_millis()
                .saturating_sub(u128::from(clock.starts_in_ms))
        } else {
            0
        };
        u128::from(clock.remaining_ms).saturating_sub(elapsed)
    });
    for (timer, mut text, mut visibility) in &mut timers {
        let active = online.game.as_ref().is_some_and(|game| {
            timer.0
                == if game.state.active == Seat::Host {
                    PlayerId::One
                } else {
                    PlayerId::Two
                }
        });
        if frontend.online_game()
            && online.connection == online::Connection::Connected
            && applied.is_current(&online)
            && !playback.active()
            && active
            && let Some(remaining) = remaining
        {
            text.0 = format!("{}s", remaining.div_ceil(1000));
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(crate) fn update_side_avatars(
    frontend: Res<menu::Frontend>,
    mut avatars: Query<&mut Visibility, With<SideAvatar>>,
) {
    for mut visibility in &mut avatars {
        *visibility = if frontend.online_game() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

type IndicatorParts = (
    &'static mut Text,
    &'static mut TextColor,
    &'static mut Visibility,
);
type IndicatorFilter = (
    With<TurnIndicator>,
    Without<DisabledControls>,
    Without<DropFeedback>,
);

pub(crate) fn spawn_disabled_overlay(parent: &mut ChildSpawnerCommands, owner: PlayerId) {
    parent.spawn((
        DisabledControls(owner),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.35, 0.35, 0.35, 0.8)),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

pub(super) fn can_act(
    online: &online::Online,
    applied: &AppliedMatch,
    turn: &TurnState,
    playback: &AnimationPlayback,
) -> bool {
    let Some(game) = online.game.as_ref() else {
        return false;
    };
    online.connection == online::Connection::Connected
        && online.opponent_online
        && !online.pending
        && !playback.active()
        && applied.room_id == online.room_id
        && applied.revision == Some(game.revision)
        && online.seat
            == Some(if turn.active == PlayerId::One {
                Seat::Host
            } else {
                Seat::Guest
            })
}

pub(crate) fn update_controls(
    mut commands: Commands,
    context: (Res<menu::Frontend>, Res<online::Online>, Res<TurnState>),
    animation_state: (Res<AppliedMatch>, Res<AnimationPlayback>),
    mut selection: ResMut<Selection>,
    mut overlays: Query<(&DisabledControls, &mut Visibility), Without<TurnIndicator>>,
    mut indicator: Query<IndicatorParts, IndicatorFilter>,
    mut feedback: Query<&mut Text, (With<DropFeedback>, Without<TurnIndicator>)>,
) {
    let (frontend, online, turn) = context;
    let (applied, playback) = animation_state;
    let in_online = frontend.online_game();
    let enabled = in_online && can_act(&online, &applied, &turn, &playback);
    if in_online && !enabled && selection.0.is_some() {
        clear_selection(&mut commands, &mut selection);
        if let Ok(mut feedback) = feedback.single_mut() {
            feedback.0.clear();
        }
    }
    let local = match online.seat {
        Some(Seat::Host) => Some(PlayerId::One),
        Some(Seat::Guest) => Some(PlayerId::Two),
        None => None,
    };
    for (overlay, mut visibility) in &mut overlays {
        *visibility = if in_online && local == Some(overlay.0) && !enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, mut color, mut visibility) in &mut indicator {
        *visibility = if in_online {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if in_online {
            let your_turn = local == Some(turn.active);
            text.0 = if your_turn {
                "YOUR TURN"
            } else {
                "OPPONENT'S TURN"
            }
            .into();
            color.0 = if your_turn {
                Color::srgb(0.05, 0.35, 0.14)
            } else {
                Color::srgb(0.5, 0.16, 0.08)
            };
        }
    }
}

type SnapshotItems = Or<(With<BoardItem>, With<PendingMarker>)>;

struct ValidatedView {
    board: Board,
    wind: WindDirection,
    serpents: Vec<serpent::Serpent>,
}

fn validate_view(view: &MatchView) -> Result<ValidatedView, &'static str> {
    if view.actions_used > 2
        || view
            .own_inventory
            .iter()
            .any(|count| *count > INGREDIENT_POOL_SIZE)
        || view.players.iter().any(|player| {
            player.x < 0
                || player.y < 0
                || player.x >= COLUMNS as i32
                || player.y >= ROWS as i32
                || player.health > 10
                || player.energy > MAX_STRENGTH
                || player.magic > MAX_MAGIC
        })
    {
        return Err("Invalid match state");
    }
    if view
        .items
        .iter()
        .chain(view.pending_item.iter())
        .any(|item| {
            item.ingredient >= INVENTORY_ITEMS.len()
                || item.x < 0
                || item.y < 0
                || item.x >= COLUMNS as i32
                || item.y >= ROWS as i32
        })
    {
        return Err("Invalid match items");
    }
    let tiles: Vec<_> = view
        .tiles
        .iter()
        .map(|tile| board::VisualTile {
            ground: match tile.terrain {
                Terrain::Solid => Ground::Solid,
                Terrain::Water => Ground::Water,
                Terrain::Ridge => Ground::Ridge,
                Terrain::Hole => Ground::Hole,
            },
            wet: tile.wet,
            oil: tile.oil,
            frozen: tile.frozen,
            poison: tile.poison,
            acid: tile.acid,
            electrified: tile.electrified,
            life: tile.life,
            thorns: tile.thorns,
            growth: tile.growth.as_ref().map(|growth| match growth {
                Growth::Void => TerrainGrowthKind::Void,
                Growth::Stone => TerrainGrowthKind::Stone,
            }),
        })
        .collect();
    let aquatic: Vec<_> = view
        .aquatic
        .iter()
        .map(|visual| aquatic::AquaticVisual {
            id: visual.id,
            origin: IVec2::new(visual.x, visual.y),
            lengths: visual.lengths,
            sweep_frame: None,
        })
        .collect();
    let board = Board::from_visual(&tiles, &aquatic)?;
    let serpents = view
        .serpents
        .iter()
        .map(|visual| {
            serpent::Serpent::from_visual(&serpent::SerpentVisual {
                cells: visual
                    .cells
                    .iter()
                    .map(|&(x, y)| IVec2::new(x, y))
                    .collect(),
                venomous: visual.venomous,
                straight_frame: visual.straight_frame,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let wind = WindDirection::from_label(&view.wind).ok_or("Invalid wind direction")?;
    Ok(ValidatedView {
        board,
        wind,
        serpents,
    })
}

#[derive(SystemParam)]
pub(crate) struct SnapshotWorld<'w, 's> {
    frontend: ResMut<'w, menu::Frontend>,
    online: ResMut<'w, online::Online>,
    applied: ResMut<'w, AppliedMatch>,
    playback: Res<'w, AnimationPlayback>,
    board: ResMut<'w, Board>,
    turn: ResMut<'w, TurnState>,
    health: ResMut<'w, PlayerHealth>,
    selection: ResMut<'w, Selection>,
    images: Res<'w, InventoryImages>,
    effects: Option<ResMut<'w, effects::Effects>>,
    players: Query<'w, 's, (&'static Player, &'static mut Transform)>,
    feedback: Query<'w, 's, &'static mut Text, With<DropFeedback>>,
    items: Query<'w, 's, Entity, SnapshotItems>,
    temporary: Query<'w, 's, Entity, (TemporarySprites, Without<OnlineAnimation>)>,
}

pub(crate) fn apply_snapshot(mut commands: Commands, mut state: SnapshotWorld) {
    let Some(game) = state.online.game.as_ref() else {
        if state.frontend.has_online_match() {
            state
                .frontend
                .end_online_match(state.online.room_id.is_some());
            *state.applied = AppliedMatch::default();
        }
        return;
    };
    let Some(room_id) = state.online.room_id.clone() else {
        return;
    };
    let Some(seat) = state.online.seat else {
        state.online.status = "Match state arrived without a player seat".into();
        return;
    };
    let revision = game.revision;
    if state.applied.room_id.as_deref() == Some(&room_id)
        && state.applied.revision == Some(revision)
    {
        return;
    }
    if state.applied.room_id.as_deref() == Some(&room_id) && state.playback.active() {
        return;
    }
    let view = game.state.clone();
    let validated = match validate_view(&view) {
        Ok(validated) => validated,
        Err(error) => {
            state.online.status = error.into();
            return;
        }
    };
    let first = state.applied.room_id.as_deref() != Some(&room_id);
    if first {
        state.frontend.enter_online_match();
        *state.turn = TurnState::default();
        if let Some(effects) = &mut state.effects {
            effects.reset();
        }
    }
    let selected: Vec<_> = match &state.selection.0 {
        Some(SelectedAction::Item {
            icon, highlight, ..
        }) => vec![*icon, *highlight],
        Some(SelectedAction::Movement { highlights }) => highlights.clone(),
        None => Vec::new(),
    };
    clear_selection(&mut commands, &mut state.selection);
    for entity in &state.items {
        commands.entity(entity).despawn();
    }
    if first {
        for entity in &state.temporary {
            if state.items.get(entity).is_err() && !selected.contains(&entity) {
                commands.entity(entity).despawn();
            }
        }
    }
    *state.board = validated.board;
    state.turn.active = if view.active == Seat::Host {
        PlayerId::One
    } else {
        PlayerId::Two
    };
    if let Ok(mut feedback) = state.feedback.single_mut() {
        feedback.0 = if view.active == seat {
            "Click Move or an item to select it"
        } else {
            ""
        }
        .into();
    }
    state.turn.actions_used = view.actions_used;
    state.turn.wind = validated.wind;
    state.turn.serpents = validated.serpents;
    state.turn.inventory = [[0; INVENTORY_ITEMS.len()]; 2];
    state.turn.inventory[if seat == Seat::Host { 0 } else { 1 }] = view.own_inventory;
    for (player, mut transform) in &mut state.players {
        let index = player.0.index();
        let source = &view.players[index];
        let position = cell_center(IVec2::new(source.x, source.y));
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        state.health.0[index] = source.health;
        state.turn.strength[index] = source.energy;
        state.turn.magic[index] = source.magic;
        state.turn.hovering[index] = source.hovering;
        state.turn.frozen_turns[index] = u8::from(source.frozen);
    }
    for item in &view.items {
        let entity = spawn_board_item(
            &mut commands,
            &state.images,
            item.ingredient,
            IVec2::new(item.x, item.y),
        );
        if item.placed {
            commands.entity(entity).insert(PlacedItem);
        }
    }
    if let Some(item) = &view.pending_item {
        let center = cell_center(IVec2::new(item.x, item.y));
        commands.spawn((
            PendingMarker,
            Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                ..state.images.sprite(item.ingredient, 44.0)
            },
            Transform::from_xyz(center.x, center.y, 0.8),
        ));
    }
    state.applied.room_id = Some(room_id);
    state.applied.revision = Some(revision);
}

#[derive(SystemParam)]
pub(crate) struct OnlineInput<'w, 's> {
    online: ResMut<'w, online::Online>,
    applied: Res<'w, AppliedMatch>,
    playback: Res<'w, AnimationPlayback>,
    turn: Res<'w, TurnState>,
    board: Res<'w, Board>,
    selection: ResMut<'w, Selection>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    cursor: Res<'w, BoardCursor>,
    images: Res<'w, InventoryImages>,
    hud: Res<'w, Hud>,
    players: Query<'w, 's, (&'static Player, &'static Transform)>,
    items: Query<'w, 's, &'static Collectible>,
    feedback: Query<'w, 's, &'static mut Text, With<DropFeedback>>,
    sounds: Option<ResMut<'w, sound::SoundCues>>,
}

fn online_placement_error<'a>(
    board: &Board,
    index: usize,
    cell: IVec2,
    pending: bool,
    items: impl IntoIterator<Item = &'a Collectible>,
) -> Option<&'static str> {
    if pending {
        return None;
    }
    let mut reactive = false;
    let mut frost = false;
    for item in items {
        if item.cell == cell {
            reactive |= matches!(item.index, OBSIDIAN_ITEM_INDEX | LIFE_ESSENCE_ITEM_INDEX);
            frost |= item.index == FROST_CRYSTAL_ITEM_INDEX;
        }
    }
    item_placement_error(board, index, cell, reactive, frost)
}

pub(crate) fn handle_input(mut commands: Commands, mut input: OnlineInput) {
    if input.keyboard.just_pressed(KeyCode::Escape) && input.selection.0.is_some() {
        clear_selection(&mut commands, &mut input.selection);
        if let Ok(mut feedback) = input.feedback.single_mut() {
            feedback.0 = "Selection cancelled".into();
        }
        return;
    }
    if !input.mouse.just_pressed(MouseButton::Left) && !input.keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Some(game) = input.online.game.as_ref() else {
        return;
    };
    let revision = game.revision;
    let can_undo = game.state.can_undo;
    let pending_item = game.state.pending_item.is_some();
    if input.applied.revision != Some(revision) || input.applied.room_id != input.online.room_id {
        return;
    }
    let Some(seat) = input.online.seat else {
        return;
    };
    let player = if seat == Seat::Host {
        PlayerId::One
    } else {
        PlayerId::Two
    };
    if input.turn.active != player {
        return;
    }
    let message = if input.online.connection != online::Connection::Connected {
        Some("Disconnected from server")
    } else if !input.online.opponent_online {
        Some("Waiting for opponent to reconnect")
    } else if input.online.pending {
        Some("Waiting for the server")
    } else if input.playback.active() {
        Some("Waiting for animations")
    } else {
        None
    };
    if let Some(message) = message {
        if let Ok(mut feedback) = input.feedback.single_mut() {
            feedback.0 = message.into();
        }
        return;
    }
    let Some(position) = input
        .players
        .iter()
        .find_map(|(p, t)| (p.0 == player).then_some(t.translation.truncate()))
    else {
        return;
    };
    let Some(opponent) = input
        .players
        .iter()
        .find_map(|(p, t)| (p.0 == player.other()).then_some(t.translation.truncate()))
    else {
        return;
    };
    let clicked = input.mouse.just_pressed(MouseButton::Left);
    let cursor = input.cursor.0;
    let action_row = if input.keyboard.just_pressed(KeyCode::KeyR) {
        Some(Action::Rest.row())
    } else if clicked {
        cursor.and_then(|cursor| action_cell(cursor, player))
    } else {
        None
    };
    let mut error = None;
    let mut action = None;
    let mut cancelled = false;
    if let Some(row) = action_row {
        if row == Action::Move.row() {
            let selected = matches!(input.selection.0, Some(SelectedAction::Movement { .. }));
            clear_selection(&mut commands, &mut input.selection);
            cancelled = selected;
            if !selected && input.turn.actions_used < 2 && !input.turn.is_frozen(player) {
                input.selection.0 = Some(select_movement(
                    &mut commands,
                    position,
                    opponent,
                    &input.board,
                    input.turn.strength[player.index()],
                    input.turn.hovering[player.index()],
                    input.turn.magic[player.index()],
                ));
                sound::request(&mut input.sounds, sound::Cue::Select);
            } else if !selected {
                error = Some("Cannot move right now");
            }
        } else if row == Action::Pickup.row() {
            if available_pickup(&input.turn, &input.board, position, &input.items) {
                action = Some(GameAction::Pickup);
            } else {
                error = Some("No collectible ingredient available");
            }
        } else if row == Action::Rest.row() {
            if must_land(&input.turn, &input.board, position) {
                error = Some("Land on safe ground before magic runs out");
            } else {
                action = Some(GameAction::Rest);
            }
        } else if row == Action::Undo.row() {
            if can_undo {
                action = Some(GameAction::Undo);
            } else {
                error = Some("Nothing to undo");
            }
        } else if row == Action::Hover.row() {
            let index = player.index();
            if input.turn.hovering[index]
                && board_cell(position)
                    .is_some_and(|cell| !input.board.ground(cell).is_some_and(Ground::walkable))
            {
                error = Some("Land on safe ground before turning hover off");
            } else if !input.turn.hovering[index]
                && (input.turn.is_frozen(player) || input.turn.magic[index] == 0)
            {
                error = Some("Cannot start hovering");
            } else {
                action = Some(GameAction::ToggleHover);
            }
        }
    } else if clicked && let Some(cursor) = cursor {
        if let Some(index) = inventory_item(cursor, player) {
            let selected = matches!(input.selection.0, Some(SelectedAction::Item { index: current, .. }) if current == index);
            clear_selection(&mut commands, &mut input.selection);
            cancelled = selected;
            if !selected {
                if input.turn.actions_used >= 2 || must_land(&input.turn, &input.board, position) {
                    error = Some("Cannot place an ingredient right now");
                } else if !is_unlimited(index) && input.turn.inventory[player.index()][index] == 0 {
                    error = Some("No ingredients remaining - pick one up");
                } else {
                    let highlight = commands
                        .spawn((
                            Sprite {
                                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                                ..default()
                            },
                            Visibility::Hidden,
                        ))
                        .id();
                    let icon = commands
                        .spawn((
                            input.images.icon(index),
                            preview_node(Vec2::ZERO),
                            Visibility::Hidden,
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(input.hud.0).add_child(icon);
                    input.selection.0 = Some(SelectedAction::Item {
                        index,
                        icon,
                        highlight,
                    });
                    sound::request(&mut input.sounds, sound::Cue::Select);
                }
            }
        } else {
            match &input.selection.0 {
                Some(SelectedAction::Movement { .. }) => {
                    if let Some(target) = drop_target(position, cursor) {
                        if movement_target(
                            &input.board,
                            &target,
                            opponent,
                            input.turn.strength[player.index()],
                            input.turn.hovering[player.index()],
                            input.turn.magic[player.index()],
                        )
                        .is_ok()
                        {
                            action = Some(GameAction::Move {
                                x: target.cell.x,
                                y: target.cell.y,
                            });
                        } else {
                            error = Some("Invalid movement destination");
                        }
                    }
                }
                Some(SelectedAction::Item { index, .. }) => {
                    if let Some(target) = item_target(position, cursor) {
                        if item_energy_cost(&target)
                            > u32::from(input.turn.strength[player.index()])
                        {
                            error = Some("Not enough energy");
                        } else {
                            error = online_placement_error(
                                &input.board,
                                *index,
                                target.cell,
                                pending_item,
                                input.items.iter(),
                            );
                        }
                        if error.is_none() {
                            action = Some(GameAction::Place {
                                ingredient: *index,
                                x: target.cell.x,
                                y: target.cell.y,
                            });
                        }
                    }
                }
                None => {}
            }
        }
    }
    if let Some(message) = error
        && let Ok(mut feedback) = input.feedback.single_mut()
    {
        feedback.0 = message.into();
    }
    if cancelled && let Ok(mut feedback) = input.feedback.single_mut() {
        feedback.0 = "Selection cancelled".into();
    }
    if error.is_some() {
        sound::request(&mut input.sounds, sound::Cue::Reject);
    }
    if let Some(action) = action {
        clear_selection(&mut commands, &mut input.selection);
        if let Ok(mut feedback) = input.feedback.single_mut() {
            feedback.0 = "Click Move or an item to select it".into();
        }
        input
            .online
            .send(ClientMessage::GameAction { revision, action });
    }
}

pub(crate) fn update_status(
    online: Res<online::Online>,
    mut status: Query<&mut Text, With<TurnProgress>>,
) {
    let Ok(mut status) = status.single_mut() else {
        return;
    };
    let connection = if online.connection != online::Connection::Connected {
        Some("DISCONNECTED - RECONNECTING")
    } else if !online.opponent_online {
        Some("OPPONENT OFFLINE")
    } else if online.pending {
        Some("WAITING FOR SERVER")
    } else {
        None
    };
    if let Some(connection) = connection {
        status.0.push_str(" | ");
        status.0.push_str(connection);
    }
    if !online.status.is_empty() {
        status.0.push_str(" | ");
        status.0.push_str(&online.status);
    }
}

#[derive(SystemParam)]
pub(crate) struct OnlinePreview<'w, 's> {
    cursor: Res<'w, BoardCursor>,
    selection: Res<'w, Selection>,
    turn: Res<'w, TurnState>,
    board: Res<'w, Board>,
    online: Res<'w, online::Online>,
    players: Query<'w, 's, (&'static Player, &'static Transform)>,
    items: Query<'w, 's, &'static Collectible>,
    feedback: Query<'w, 's, &'static mut Text, With<DropFeedback>>,
}

pub(crate) fn update_selection_preview(mut commands: Commands, mut preview: OnlinePreview) {
    let Some(position) = preview.players.iter().find_map(|(player, transform)| {
        (player.0 == preview.turn.active).then_some(transform.translation.truncate())
    }) else {
        return;
    };
    let active = preview.turn.active.index();
    let message = match &preview.selection.0 {
        Some(SelectedAction::Item {
            index,
            icon,
            highlight,
        }) => {
            let cell = preview.cursor.0.and_then(board_cell);
            let target = preview
                .cursor
                .0
                .and_then(|cursor| item_target(position, cursor));
            let pending = preview
                .online
                .game
                .as_ref()
                .is_some_and(|game| game.state.pending_item.is_some());
            let error = cell.and_then(|cell| {
                online_placement_error(&preview.board, *index, cell, pending, preview.items.iter())
            });
            if let Some(cursor) = preview.cursor.0 {
                commands
                    .entity(*icon)
                    .insert((preview_node(cursor), Visibility::Visible));
            } else {
                commands.entity(*icon).insert(Visibility::Hidden);
            }
            if let Some(target) = &target {
                let can_use = error.is_none()
                    && item_energy_cost(target) <= u32::from(preview.turn.strength[active]);
                let color = if !can_use {
                    Color::srgba(0.9, 0.15, 0.1, 0.4)
                } else if target.distance <= 1 {
                    Color::srgba(0.2, 0.8, 0.3, 0.4)
                } else {
                    Color::srgba(1.0, 0.7, 0.1, 0.4)
                };
                commands.entity(*highlight).insert((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                        ..default()
                    },
                    Transform::from_translation(cell_center(target.cell).extend(1.0)),
                    Visibility::Visible,
                ));
            } else {
                commands.entity(*highlight).insert(Visibility::Hidden);
            }
            placement_feedback(
                *index,
                preview.cursor.0,
                cell,
                target.as_ref(),
                error,
                preview.turn.strength[active],
            )
        }
        Some(SelectedAction::Movement { .. }) => {
            let Some(opponent) = preview.players.iter().find_map(|(player, transform)| {
                (player.0 == preview.turn.active.other())
                    .then_some(transform.translation.truncate())
            }) else {
                return;
            };
            let target = preview
                .cursor
                .0
                .and_then(|cursor| drop_target(position, cursor));
            movement_feedback(target.as_ref().map(|target| {
                movement_target(
                    &preview.board,
                    target,
                    opponent,
                    preview.turn.strength[active],
                    preview.turn.hovering[active],
                    preview.turn.magic[active],
                )
            }))
            .into()
        }
        None => return,
    };
    if let Ok(mut feedback) = preview.feedback.single_mut() {
        feedback.0 = message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_avatars_use_player_frames_and_fit_the_hud_corners() {
        let mut app = App::new();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_side_avatars(&mut commands, hud, Handle::default());
            })
            .unwrap();
        let world = app.world_mut();
        let avatars: Vec<_> = world
            .query_filtered::<(&ImageNode, &Node), With<SideAvatar>>()
            .iter(world)
            .map(|(image, node)| (image.rect, node.clone()))
            .collect();
        assert_eq!(avatars.len(), 2);
        assert_eq!(
            avatars[0].0,
            Some(player_frame_rect(PlayerId::One, false, 0))
        );
        assert_eq!(
            avatars[1].0,
            Some(player_frame_rect(PlayerId::Two, false, 0))
        );
        assert_eq!(avatars[0].1.left, Val::Px(3.0 * GRID_BLOCK_SIZE + 2.0));
        assert_eq!(avatars[1].1.right, Val::Px(3.0 * GRID_BLOCK_SIZE + 2.0));
        assert_eq!(avatars[0].1.top, Val::Px(6.0));
        assert_eq!(avatars[1].1.top, avatars[0].1.top);
        assert_eq!(avatars[0].1.width, Val::Px(GRID_BLOCK_SIZE - 8.0));
        assert_eq!(avatars[0].1.height, avatars[0].1.width);
        assert_eq!(avatars[1].1.width, avatars[0].1.width);
        assert_eq!(avatars[1].1.height, avatars[0].1.height);
    }

    #[test]
    fn turn_timer_displays_only_the_active_seat_and_pauses() {
        use volatile_online_protocol::TurnClock;

        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.init_resource::<Time<Real>>()
            .add_systems(Update, update_turn_timer);
        let hud = app.world().resource::<Hud>().0;
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_side_avatars(&mut commands, hud, Handle::default());
            })
            .unwrap();
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.turn_clock = Some(TurnClock {
                remaining_ms: 30_000,
                starts_in_ms: 2_000,
                running: true,
            });
            online.clock_generation += 1;
        }
        app.update();
        let timers = |app: &mut App| {
            let world = app.world_mut();
            world
                .query::<(&TurnTimer, &Text, &Visibility, &Node)>()
                .iter(world)
                .map(|(timer, text, visibility, node)| {
                    (timer.0, text.0.clone(), *visibility, node.clone())
                })
                .collect::<Vec<_>>()
        };
        let initial = timers(&mut app);
        let host = initial
            .iter()
            .find(|timer| timer.0 == PlayerId::One)
            .unwrap();
        let guest = initial
            .iter()
            .find(|timer| timer.0 == PlayerId::Two)
            .unwrap();
        assert_eq!((host.1.as_str(), host.2), ("30s", Visibility::Visible));
        assert_eq!(guest.2, Visibility::Hidden);
        assert_eq!(host.3.left, Val::Px(4.0 * GRID_BLOCK_SIZE));
        assert_eq!(guest.3.right, Val::Px(4.0 * GRID_BLOCK_SIZE));

        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(3));
        app.update();
        assert_eq!(
            timers(&mut app)
                .iter()
                .find(|timer| timer.0 == PlayerId::One)
                .unwrap()
                .1,
            "29s"
        );
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.game.as_mut().unwrap().state.active = Seat::Guest;
            online.turn_clock = Some(TurnClock {
                remaining_ms: 12_000,
                starts_in_ms: 0,
                running: false,
            });
            online.clock_generation += 1;
        }
        app.update();
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(5));
        app.update();
        let paused = timers(&mut app);
        let host = paused
            .iter()
            .find(|timer| timer.0 == PlayerId::One)
            .unwrap();
        let guest = paused
            .iter()
            .find(|timer| timer.0 == PlayerId::Two)
            .unwrap();
        assert_eq!(host.2, Visibility::Hidden);
        assert_eq!((guest.1.as_str(), guest.2), ("12s", Visibility::Visible));
        app.world_mut().resource_mut::<online::Online>().connection =
            online::Connection::Disconnected;
        app.update();
        assert!(
            timers(&mut app)
                .iter()
                .all(|timer| timer.2 == Visibility::Hidden)
        );
    }

    fn client_unstarted(game: &mut headless::HeadlessMatch, seat: Seat) -> App {
        let mut app = App::new();
        let mut online = online::Online::default();
        online.connection = online::Connection::Connected;
        online.room_id = Some("room".into());
        online.seat = Some(seat);
        online.opponent_online = true;
        online.game = Some(online::OnlineMatch {
            revision: 0,
            state: game.view(seat),
        });
        app.insert_resource(online)
            .init_resource::<menu::Frontend>()
            .init_resource::<AppliedMatch>()
            .init_resource::<AnimationPlayback>()
            .init_resource::<Time>()
            .init_resource::<effects::Effects>()
            .init_resource::<Board>()
            .init_resource::<TurnState>()
            .init_resource::<PlayerHealth>()
            .init_resource::<Selection>()
            .init_resource::<InventoryImages>()
            .init_resource::<BoardCursor>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<sound::SoundCues>()
            .add_systems(Update, apply_snapshot.before(menu::GameplaySystems));
        for player in [PlayerId::One, PlayerId::Two] {
            app.world_mut()
                .spawn((Player(player), Transform::default(), PersistentWorld));
        }
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud));
        app.world_mut().spawn((DropFeedback, Text::default()));
        app
    }

    fn client(game: &mut headless::HeadlessMatch, seat: Seat) -> App {
        let mut app = client_unstarted(game, seat);
        app.update();
        app
    }

    fn feedback(app: &mut App) -> String {
        let world = app.world_mut();
        world
            .query_filtered::<&Text, With<DropFeedback>>()
            .single(world)
            .unwrap()
            .0
            .clone()
    }

    #[test]
    fn online_movement_preview_shows_costs_and_invalid_destinations() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.add_systems(Update, update_selection_preview);
        app.world_mut().resource_mut::<Selection>().0 =
            Some(SelectedAction::Movement { highlights: vec![] });
        for cell in [IVec2::new(3, 3), IVec2::new(4, 3), IVec2::new(5, 3)] {
            app.world_mut()
                .resource_mut::<Board>()
                .set(cell, Ground::Solid);
        }
        for (cell, expected) in [
            (IVec2::new(3, 3), "Click to walk - 1 grid step"),
            (IVec2::new(4, 3), "Click to jump - 1 energy"),
            (IVec2::new(5, 3), "Too far - jump up to 2 grid steps"),
        ] {
            app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(cell));
            app.update();
            assert_eq!(feedback(&mut app), expected);
        }
        app.world_mut()
            .resource_mut::<Board>()
            .set(IVec2::new(3, 3), Ground::Ridge);
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(IVec2::new(3, 3)));
        app.update();
        assert_eq!(feedback(&mut app), "Cannot land on a ridge or hole");
        app.world_mut()
            .resource_mut::<Board>()
            .set(IVec2::new(3, 3), Ground::Solid);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(IVec2::new(4, 3)));
        app.update();
        assert_eq!(
            feedback(&mut app),
            "Not enough energy - choose Move, a nearby tile, or rest"
        );
        {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &mut Transform)>();
            for (player, mut transform) in players.iter_mut(world) {
                if player.0 == PlayerId::Two {
                    transform.translation = cell_center(IVec2::new(3, 3)).extend(0.0);
                }
            }
        }
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(IVec2::new(3, 3)));
        app.update();
        assert_eq!(feedback(&mut app), "Cell occupied by opponent");
    }

    #[test]
    fn online_ingredient_preview_reports_cost_and_defers_pending_reactions() {
        use volatile_online_protocol::ItemView;

        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.add_systems(Update, update_selection_preview);
        let icon = app.world_mut().spawn_empty().id();
        let highlight = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Selection>().0 = Some(SelectedAction::Item {
            index: WATER_ITEM_INDEX,
            icon,
            highlight,
        });
        let target = IVec2::new(4, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Solid);
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(target));
        app.update();
        assert!(feedback(&mut app).contains("2 grid steps - 1 energy"));
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.update();
        assert_eq!(
            feedback(&mut app),
            "Cannot place water on a water tile - choose another tile"
        );
        assert_eq!(
            app.world().get::<Sprite>(highlight).unwrap().color,
            Color::srgba(0.9, 0.15, 0.1, 0.4)
        );
        app.world_mut()
            .resource_mut::<online::Online>()
            .game
            .as_mut()
            .unwrap()
            .state
            .pending_item = Some(ItemView {
            ingredient: STONE_DUST_ITEM_INDEX,
            x: target.x,
            y: target.y,
            placed: true,
        });
        app.update();
        assert!(feedback(&mut app).contains("2 grid steps - 1 energy"));
        assert_eq!(
            app.world().get::<Sprite>(highlight).unwrap().color,
            Color::srgba(1.0, 0.7, 0.1, 0.4)
        );
    }

    #[test]
    fn online_feedback_respects_turn_connection_and_board_hover() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.init_resource::<InventoryHover>().add_systems(
            Update,
            (
                update_controls,
                handle_input,
                update_selection_preview,
                highlight_inventory_hover,
                update_cell_feedback,
                update_turn_progress,
                update_status,
            )
                .chain(),
        );
        app.world_mut()
            .resource_mut::<online::Online>()
            .status
            .clear();
        let drop = app
            .world_mut()
            .query_filtered::<Entity, With<DropFeedback>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(drop)
            .insert(Visibility::Inherited);
        let cell_feedback = app
            .world_mut()
            .spawn((CellFeedback, Text::default(), Visibility::Hidden))
            .id();
        let progress = app.world_mut().spawn((TurnProgress, Text::default())).id();
        let water_slot = INVENTORY_ORDER
            .iter()
            .position(|index| *index == WATER_ITEM_INDEX)
            .unwrap();
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(
            -RESOLUTION_WIDTH / 2.0 + (water_slot % 2) as f32 * GRID_BLOCK_SIZE + 10.0,
            BOARD_TOP - (water_slot / 2) as f32 * GRID_BLOCK_SIZE - 10.0,
        ));
        app.update();
        assert_eq!(feedback(&mut app), "Tlaloc's Water - click to select");
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(IVec2::new(3, 3)));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(drop).unwrap(),
            Visibility::Hidden
        );
        assert!(
            app.world()
                .get::<Text>(cell_feedback)
                .unwrap()
                .0
                .starts_with("D4: ")
        );
        app.world_mut().resource_mut::<AnimationPlayback>().until = 1.0;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(cell_feedback).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<AnimationPlayback>().until = 0.0;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(cell_feedback).unwrap(),
            Visibility::Inherited
        );

        app.world_mut().resource_mut::<Selection>().0 =
            Some(SelectedAction::Movement { highlights: vec![] });
        app.world_mut()
            .resource_mut::<online::Online>()
            .opponent_online = false;
        app.update();
        assert!(app.world().resource::<Selection>().0.is_none());
        assert!(feedback(&mut app).is_empty());
        assert!(
            app.world()
                .get::<Text>(progress)
                .unwrap()
                .0
                .contains("OPPONENT OFFLINE")
        );
        app.world_mut().resource_mut::<online::Online>().status = "Action rejected".into();
        app.update();
        let status = &app.world().get::<Text>(progress).unwrap().0;
        assert!(status.contains("ACTIONS:") && status.contains("Action rejected"));
        assert_eq!(status.matches("OPPONENT OFFLINE").count(), 1);
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.opponent_online = true;
            online.status.clear();
        }
        app.world_mut().resource_mut::<TurnState>().active = PlayerId::Two;
        app.world_mut().resource_mut::<BoardCursor>().0 =
            Some(action_position_for_test(PlayerId::One, Action::Move));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(feedback(&mut app).is_empty());
        assert!(app.world().resource::<online::Online>().outgoing.is_empty());
    }

    #[test]
    fn history_waits_for_playback_and_retains_coalesced_revisions() {
        use volatile_online_protocol::HistoryEntry;

        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client_unstarted(&mut game, Seat::Host);
        app.add_message::<bevy::input::mouse::MouseWheel>()
            .add_plugins(history::plugin);
        app.update();
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.game.as_mut().unwrap().revision = 2;
            online.history = vec![
                HistoryEntry {
                    seat: Some(Seat::Host),
                    text: "moved to D4".into(),
                    item: None,
                },
                HistoryEntry {
                    seat: Some(Seat::Guest),
                    text: "rested at H4".into(),
                    item: None,
                },
            ];
        }
        app.world_mut().resource_mut::<AnimationPlayback>().until = 1.0;
        app.update();
        {
            let world = app.world_mut();
            assert!(
                !world
                    .query::<&Text>()
                    .iter(world)
                    .any(|text| text.0 == "You: moved to D4")
            );
        }
        app.world_mut().resource_mut::<AnimationPlayback>().until = 0.0;
        app.update();
        let world = app.world_mut();
        let texts: Vec<_> = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect();
        assert!(texts.contains(&"You: moved to D4".into()), "{texts:?}");
        assert!(
            texts.contains(&"Opponent: rested at H4".into()),
            "{texts:?}"
        );
    }

    #[test]
    fn snapshots_apply_private_state_and_reject_invalid_updates() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Guest);
        assert!(app.world().resource::<menu::Frontend>().online_game());
        let expected = game.view(Seat::Guest);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.inventory[1], expected.own_inventory);
        assert_eq!(turn.inventory[0], [0; 14]);
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(0));
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            let game = online.game.as_mut().unwrap();
            game.revision = 1;
            game.state.tiles.clear();
        }
        app.update();
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(0));
        assert_eq!(
            app.world().resource::<online::Online>().status,
            "Invalid board size"
        );
    }

    #[test]
    fn input_sends_only_legal_current_turn_actions() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.add_systems(Update, handle_input);
        let click = |app: &mut App, position: Vec2| {
            app.world_mut().resource_mut::<BoardCursor>().0 = Some(position);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            app.update();
            let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            mouse.clear();
            mouse.release(MouseButton::Left);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .clear();
        };
        click(
            &mut app,
            action_position_for_test(PlayerId::One, Action::Move),
        );
        assert!(
            app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Select)
        );
        click(
            &mut app,
            action_position_for_test(PlayerId::One, Action::Move),
        );
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(feedback(&mut app), "Selection cancelled");
        click(
            &mut app,
            action_position_for_test(PlayerId::One, Action::Move),
        );
        click(&mut app, cell_center(IVec2::new(3, 3)));
        assert!(matches!(
            app.world().resource::<online::Online>().outgoing.as_slice(),
            [ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Move { x: 3, y: 3 }
            }]
        ));
        click(
            &mut app,
            action_position_for_test(PlayerId::One, Action::Rest),
        );
        assert_eq!(app.world().resource::<online::Online>().outgoing.len(), 1);
    }

    #[test]
    fn inactive_player_cannot_select_or_send_actions() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Guest);
        app.add_systems(Update, handle_input);
        app.world_mut().resource_mut::<BoardCursor>().0 =
            Some(action_position_for_test(PlayerId::Two, Action::Move));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Selection>().0.is_none());
        assert!(app.world().resource::<online::Online>().outgoing.is_empty());
        assert!(
            !app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Select)
        );
    }

    #[test]
    fn animation_playback_precedes_snapshot_and_blocks_input() {
        use volatile_online_protocol::VisualEffect;

        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.add_systems(Update, play_animations.before(apply_snapshot))
            .add_systems(Update, handle_input.after(apply_snapshot));
        let from = (PLAYER_STARTS[0].x, PLAYER_STARTS[0].y);
        let to = (from.0 + 1, from.1);
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.game.as_mut().unwrap().revision = 1;
            online.animations.push(vec![
                AnimationEvent {
                    at_ms: 0,
                    animation: Animation::Throw {
                        ingredient: WATER_ITEM_INDEX,
                        from,
                        to,
                        corner: false,
                    },
                },
                AnimationEvent {
                    at_ms: 450,
                    animation: Animation::Effect {
                        effect: VisualEffect::Steam,
                        x: to.0,
                        y: to.1,
                    },
                },
            ]);
        }
        app.update();
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(0));
        app.world_mut().resource_mut::<BoardCursor>().0 =
            Some(action_position_for_test(PlayerId::One, Action::Move));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(225));
        app.update();
        assert!(app.world().resource::<Selection>().0.is_none());
        let world = app.world_mut();
        let mut query = world.query::<(&OnlineAnimation, &Transform)>();
        let (animation, transform) = query.single(world).unwrap();
        assert!(matches!(animation.kind, Animation::Throw { .. }));
        let expected = cell_center(IVec2::new(from.0, from.1))
            .lerp(cell_center(IVec2::new(to.0, to.1)), 0.5)
            + Vec2::Y * ITEM_FLIGHT_HEIGHT;
        assert!(transform.translation.truncate().distance(expected) < 0.01);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(225));
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<&OnlineAnimation>();
        assert!(query.iter(world).any(|animation| matches!(
            animation.kind,
            Animation::Effect {
                effect: VisualEffect::Steam,
                ..
            }
        )));
        assert_eq!(world.resource::<AppliedMatch>().revision, Some(0));
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(901));
        app.update();
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(1));
        assert!(!app.world().resource::<AnimationPlayback>().active());
        assert_eq!(
            app.world_mut()
                .query::<&OnlineAnimation>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn moved_player_is_visible_before_queued_effects_for_both_seats() {
        use volatile_online_protocol::VisualEffect;

        for (seat, actor) in [
            (Seat::Host, Seat::Host),
            (Seat::Guest, Seat::Host),
            (Seat::Host, Seat::Guest),
            (Seat::Guest, Seat::Guest),
        ] {
            let mut game = headless::HeadlessMatch::new([7; 8]);
            let mut app = client(&mut game, seat);
            app.add_systems(Update, play_animations.before(apply_snapshot));
            let (player, destination, final_destination) = if actor == Seat::Host {
                (PlayerId::One, IVec2::new(3, 3), IVec2::new(4, 3))
            } else {
                (PlayerId::Two, IVec2::new(6, 3), IVec2::new(5, 3))
            };
            {
                let mut online = app.world_mut().resource_mut::<online::Online>();
                let snapshot = online.game.as_mut().unwrap();
                snapshot.revision = 1;
                snapshot.state.players[player.index()].x = final_destination.x;
                snapshot.state.players[player.index()].y = final_destination.y;
                online.animations.push(vec![
                    AnimationEvent {
                        at_ms: 0,
                        animation: Animation::MovePlayer {
                            seat: actor,
                            x: destination.x,
                            y: destination.y,
                        },
                    },
                    AnimationEvent {
                        at_ms: 0,
                        animation: Animation::Effect {
                            effect: VisualEffect::Explosion,
                            x: destination.x,
                            y: destination.y,
                        },
                    },
                ]);
            }
            app.update();
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &Transform)>();
            let moved = players
                .iter(world)
                .find(|(owner, _)| owner.0 == player)
                .unwrap()
                .1;
            assert_eq!(moved.translation.truncate(), cell_center(destination));
            assert_eq!(world.query::<&OnlineAnimation>().iter(world).count(), 0);
            assert_eq!(world.resource::<AppliedMatch>().revision, Some(0));
            app.update();
            let world = app.world_mut();
            assert_eq!(world.query::<&OnlineAnimation>().iter(world).count(), 1);
            assert_eq!(world.resource::<AppliedMatch>().revision, Some(0));
            world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.update();
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &Transform)>();
            let final_position = players
                .iter(world)
                .find(|(owner, _)| owner.0 == player)
                .unwrap()
                .1;
            assert_eq!(
                final_position.translation.truncate(),
                cell_center(final_destination)
            );
            assert_eq!(world.resource::<AppliedMatch>().revision, Some(1));
        }
    }

    #[test]
    fn creature_frames_and_weather_are_replayed_and_cleared() {
        use volatile_online_protocol::{AquaticAnimationView, SerpentView};

        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Host);
        app.add_systems(Update, play_animations.before(apply_snapshot));
        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            online.game.as_mut().unwrap().revision = 1;
            online.animations.push(vec![
                AnimationEvent {
                    at_ms: 0,
                    animation: Animation::Serpents {
                        visuals: Some(vec![SerpentView {
                            cells: vec![(3, 3), (3, 4), (3, 5)],
                            venomous: false,
                            straight_frame: 0,
                        }]),
                        impact: Some((3, 3)),
                    },
                },
                AnimationEvent {
                    at_ms: 0,
                    animation: Animation::Aquatic {
                        visuals: Some(vec![AquaticAnimationView {
                            id: 1,
                            x: 4,
                            y: 3,
                            lengths: [1, 2, 3, 0],
                            sweep_frame: Some(3),
                        }]),
                    },
                },
                AnimationEvent {
                    at_ms: 0,
                    animation: Animation::Weather {
                        kind: WeatherEffect::Earthquake,
                    },
                },
                AnimationEvent {
                    at_ms: 500,
                    animation: Animation::Serpents {
                        visuals: None,
                        impact: None,
                    },
                },
                AnimationEvent {
                    at_ms: 500,
                    animation: Animation::Aquatic { visuals: None },
                },
            ]);
        }
        app.update();
        let playback = app.world().resource::<AnimationPlayback>();
        assert_eq!(
            playback.serpents.as_ref().unwrap().1,
            Some(IVec2::new(3, 3))
        );
        assert_eq!(playback.aquatic.as_ref().unwrap()[0].sweep_frame, Some(3));
        assert!(playback.earthquake_elapsed().is_some());

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(500));
        app.update();
        let playback = app.world().resource::<AnimationPlayback>();
        assert!(playback.serpents.is_none());
        assert!(playback.aquatic.is_none());
        assert!(playback.earthquake_elapsed().is_some());
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(0));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert!(
            app.world()
                .resource::<AnimationPlayback>()
                .earthquake_elapsed()
                .is_none()
        );
        assert_eq!(app.world().resource::<AppliedMatch>().revision, Some(1));
    }

    #[test]
    fn controls_and_indicator_follow_turn_and_connection_state() {
        let mut game = headless::HeadlessMatch::new([7; 8]);
        let mut app = client(&mut game, Seat::Guest);
        app.add_systems(Update, update_controls.after(apply_snapshot));
        let inventory = app
            .world_mut()
            .spawn((DisabledControls(PlayerId::Two), Visibility::Hidden))
            .id();
        let action = app
            .world_mut()
            .spawn((DisabledControls(PlayerId::Two), Visibility::Hidden))
            .id();
        let indicator = app
            .world_mut()
            .spawn((
                TurnIndicator,
                Text::default(),
                TextColor(Color::WHITE),
                Visibility::Hidden,
            ))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(inventory).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<Visibility>(action).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            app.world().get::<Text>(indicator).unwrap().0,
            "OPPONENT'S TURN"
        );

        {
            let mut online = app.world_mut().resource_mut::<online::Online>();
            let game = online.game.as_mut().unwrap();
            game.revision = 1;
            game.state.active = Seat::Guest;
        }
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(inventory).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(action).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(app.world().get::<Text>(indicator).unwrap().0, "YOUR TURN");
        assert_eq!(
            app.world_mut()
                .query_filtered::<&Text, With<DropFeedback>>()
                .single(app.world())
                .unwrap()
                .0,
            "Click Move or an item to select it"
        );
        app.world_mut()
            .resource_mut::<online::Online>()
            .opponent_online = false;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(inventory).unwrap(),
            Visibility::Visible
        );
        assert_eq!(app.world().get::<Text>(indicator).unwrap().0, "YOUR TURN");
    }

    fn action_position_for_test(player: PlayerId, action: Action) -> Vec2 {
        let x = if player == PlayerId::One {
            BOARD_LEFT - GRID_BLOCK_SIZE / 2.0
        } else {
            BOARD_RIGHT + GRID_BLOCK_SIZE / 2.0
        };
        Vec2::new(x, BOARD_TOP - (action.row() as f32 + 0.5) * GRID_BLOCK_SIZE)
    }
}
