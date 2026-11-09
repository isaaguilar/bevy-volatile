use std::time::Duration;

use volatile_online_protocol::{
    Animation, AnimationEvent, AquaticAnimationView, AquaticView, GameAction, Growth, HistoryEntry,
    ItemView, MatchView, PlayerView, Seat, SerpentView, SoundEffect, Terrain, TileView,
    WeatherEffect,
};

use crate::*;

pub struct HeadlessMatch {
    app: bevy::app::SubApp,
    broken: bool,
    history: Vec<HistoryEntry>,
}

#[derive(Resource, Default)]
pub(crate) struct MatchEvents {
    entries: Vec<HistoryEntry>,
    health: [u8; 2],
    weather_notices: Vec<String>,
    tiles: Vec<HistoryTile>,
}

#[derive(Clone, PartialEq, Eq)]
struct HistoryTile {
    ground: Ground,
    oil: bool,
    frozen: bool,
    poison: bool,
    acid: bool,
    electrified: bool,
    life: bool,
    thorns: bool,
    growth: Option<TerrainGrowthKind>,
}

fn history_tiles(board: &Board) -> Vec<HistoryTile> {
    (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
        .map(|cell| HistoryTile {
            ground: board.ground(cell).expect("history cell is in bounds"),
            oil: board.has_oil(cell),
            frozen: board.is_frozen(cell),
            poison: board.is_poisoned(cell),
            acid: board.is_acidic(cell),
            electrified: board.is_electrified_acid(cell),
            life: board.has_life_essence(cell),
            thorns: board.has_thorns(cell),
            growth: board.terrain_growth_kind_at(cell),
        })
        .collect()
}

fn ground_name(ground: Ground) -> &'static str {
    match ground {
        Ground::Solid => "solid ground",
        Ground::Water => "water",
        Ground::Ridge => "ridge",
        Ground::Hole => "hole",
    }
}

impl MatchEvents {
    pub(crate) fn add(&mut self, seat: Option<Seat>, text: String, item: Option<usize>) {
        self.entries.push(HistoryEntry { seat, text, item });
    }
}

#[derive(Resource, Default)]
pub(crate) struct AnimationLog {
    start: Duration,
    events: Vec<AnimationEvent>,
    last_flight: Option<Entity>,
    serpents: Option<(Vec<serpent::SerpentVisual>, Option<IVec2>)>,
    aquatic: Option<Vec<aquatic::AquaticVisual>>,
    weather: Option<WeatherKind>,
}

impl AnimationLog {
    pub(crate) fn record(&mut self, time: Duration, animation: Animation) {
        let millis = time.saturating_sub(self.start).as_millis();
        self.events.push(AnimationEvent {
            at_ms: u32::try_from(millis).expect("animation duration exceeds protocol limit"),
            animation,
        });
    }
}

#[derive(PartialEq, Eq)]
struct ActionState {
    active: PlayerId,
    actions_used: u8,
    strength: [u8; 2],
    magic: [u8; 2],
    hovering: [bool; 2],
    inventory: [[usize; 14]; 2],
    positions: [Option<IVec2>; 2],
}

impl HeadlessMatch {
    pub fn new(seed: [u8; 8]) -> Self {
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(seed))
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<TurnState>()
            .init_resource::<PlayerHealth>()
            .init_resource::<Weather>()
            .init_resource::<Time>()
            .init_resource::<InventoryImages>()
            .init_resource::<effects::Effects>()
            .init_resource::<sound::SoundCues>()
            .init_resource::<AnimationLog>()
            .init_resource::<MatchEvents>()
            .add_systems(
                Startup,
                (
                    generate_board,
                    initialize_wind,
                    initialize_weather,
                    spawn_initial_collectibles,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    (
                        effects::advance,
                        advance_item_flight,
                        begin_turn_frame,
                        effects::capture,
                        advance_serpent_animation,
                        advance_aquatic_animation,
                        rest,
                        effects::capture,
                    )
                        .chain(),
                    (undo_action, handle_actions, effects::capture).chain(),
                    (
                        handle_selection,
                        effects::capture,
                        handle_selection.run_if(pending_placement_ready),
                        effects::capture,
                        clear_hole_items,
                        handle_selection.run_if(pending_placement_ready),
                        effects::capture,
                        apply_terrain_changes,
                        trigger_placed_frost,
                        effects::capture,
                    )
                        .chain(),
                    (finalize_undo, clear_hole_items).chain(),
                    advance_wind,
                    respawn_ingredients,
                    queue_weather,
                    advance_weather,
                    effects::capture,
                    sound::health_changes,
                    sound::aquatic_growth,
                )
                    .chain(),
            );
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud));
        app.world_mut().spawn((DropFeedback, Text::default()));
        for (index, player) in [PlayerId::One, PlayerId::Two].into_iter().enumerate() {
            let position = cell_center(PLAYER_STARTS[index]);
            app.world_mut().spawn((
                Player(player),
                Transform::from_xyz(position.x, position.y, 0.0),
            ));
        }
        app.world_mut().resource_mut::<TurnState>().history_causes = Some(Vec::new());
        app.update();
        Self {
            app: std::mem::take(app.main_mut()),
            broken: false,
            history: Vec::new(),
        }
    }

    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    pub fn is_broken(&self) -> bool {
        self.broken
    }

    pub fn take_sounds(&mut self) -> Vec<SoundEffect> {
        self.app
            .world_mut()
            .resource_mut::<sound::SoundCues>()
            .take_gameplay_events()
    }

    pub fn take_animations(&mut self) -> Vec<AnimationEvent> {
        let mut events =
            std::mem::take(&mut self.app.world_mut().resource_mut::<AnimationLog>().events);
        events.sort_by_key(|event| event.at_ms);
        events
    }

    pub fn view(&mut self, seat: Seat) -> MatchView {
        let world = self.app.world_mut();
        let mut positions = [None; 2];
        for (player, transform) in world.query::<(&Player, &Transform)>().iter(world) {
            positions[player.0.index()] = board_cell(transform.translation.truncate());
        }
        let items = world
            .query::<(&Collectible, Has<PlacedItem>)>()
            .iter(world)
            .map(|(item, placed)| ItemView {
                ingredient: item.index,
                x: item.cell.x,
                y: item.cell.y,
                placed,
            })
            .collect();
        let board = world.resource::<Board>();
        let tiles = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .map(|cell| TileView {
                terrain: match board.ground(cell).expect("view cell is in bounds") {
                    Ground::Solid => Terrain::Solid,
                    Ground::Water => Terrain::Water,
                    Ground::Ridge => Terrain::Ridge,
                    Ground::Hole => Terrain::Hole,
                },
                wet: board.water_percentage(cell),
                oil: board.has_oil(cell),
                frozen: board.is_frozen(cell),
                poison: board.is_poisoned(cell),
                acid: board.is_acidic(cell),
                electrified: board.is_electrified_acid(cell),
                life: board.has_life_essence(cell),
                thorns: board.has_thorns(cell),
                growth: board.terrain_growth_kind_at(cell).map(|kind| match kind {
                    TerrainGrowthKind::Void => Growth::Void,
                    TerrainGrowthKind::Stone => Growth::Stone,
                }),
            })
            .collect();
        let aquatic = board
            .aquatic_visuals()
            .map(|visual| AquaticView {
                id: visual.id,
                x: visual.origin.x,
                y: visual.origin.y,
                lengths: visual.lengths,
            })
            .collect();
        let health = world.resource::<PlayerHealth>();
        let turn = world.resource::<TurnState>();
        let players = [PlayerId::One, PlayerId::Two].map(|player| {
            let index = player.index();
            let cell = positions[index].expect("headless match player must be on the board");
            PlayerView {
                x: cell.x,
                y: cell.y,
                health: health.0[index],
                energy: turn.strength[index],
                magic: turn.magic[index],
                hovering: turn.hovering[index],
                frozen: turn.is_frozen(player),
            }
        });
        MatchView {
            active: if turn.active == PlayerId::One {
                Seat::Host
            } else {
                Seat::Guest
            },
            actions_used: turn.actions_used,
            can_undo: turn.undo.is_some(),
            wind: turn.wind.label().into(),
            players,
            own_inventory: turn.inventory[match seat {
                Seat::Host => 0,
                Seat::Guest => 1,
            }],
            tiles,
            items,
            pending_item: turn.pending_placement.as_ref().map(|pending| ItemView {
                ingredient: pending.index,
                x: pending.target.cell.x,
                y: pending.target.cell.y,
                placed: true,
            }),
            serpents: turn
                .serpents
                .iter()
                .map(|serpent| {
                    let visual = serpent.visual();
                    SerpentView {
                        cells: visual
                            .cells
                            .into_iter()
                            .map(|cell| (cell.x, cell.y))
                            .collect(),
                        venomous: visual.venomous,
                        straight_frame: visual.straight_frame,
                    }
                })
                .collect(),
            aquatic,
        }
    }

    pub fn perform(&mut self, seat: Seat, action: GameAction) -> Result<(), String> {
        let world = self.app.world_mut();
        let start = world.resource::<Time>().elapsed();
        world.resource_mut::<Time>().advance_by(Duration::ZERO);
        let mut log = world.resource_mut::<AnimationLog>();
        log.start = start;
        log.events.clear();
        log.last_flight = None;
        log.serpents = None;
        log.aquatic = None;
        log.weather = None;
        self.app
            .world_mut()
            .resource_mut::<sound::SoundCues>()
            .clear_pending();
        if self.broken {
            return Err("The match cannot continue".into());
        }
        let player = match seat {
            Seat::Host => PlayerId::One,
            Seat::Guest => PlayerId::Two,
        };
        let turn = self.app.world().resource::<TurnState>();
        if turn.active != player {
            return Err("It is not your turn".into());
        }
        if self.app.world().resource::<PlayerHealth>().0.contains(&0) {
            return Err("The match is over".into());
        }
        if let GameAction::Move { x, y } | GameAction::Place { x, y, .. } = &action {
            self.checked_cell(*x, *y)?;
        }
        let before = self.action_state();
        let before_view = self.view(seat);
        let tiles = history_tiles(self.app.world().resource::<Board>());
        {
            let mut events = self.app.world_mut().resource_mut::<MatchEvents>();
            events.entries.clear();
            events.health = before_view.players.each_ref().map(|player| player.health);
            events.weather_notices.clear();
            events.tiles = tiles;
        }
        let position = before.positions[player.index()].expect("player must be on the board");
        let (description, item) = match &action {
            GameAction::Move { x, y } => (
                format!(
                    "moved from {} to {}",
                    history::cell_label(position),
                    history::cell_label(IVec2::new(*x, *y))
                ),
                None,
            ),
            GameAction::Place { ingredient, x, y } => (
                format!(
                    "queued {} at {}",
                    INVENTORY_ITEMS
                        .get(*ingredient)
                        .map_or("an ingredient", |item| item.0),
                    history::cell_label(IVec2::new(*x, *y))
                ),
                Some(*ingredient),
            ),
            GameAction::Pickup => {
                let items: Vec<_> = before_view
                    .items
                    .iter()
                    .filter(|item| {
                        item.x == position.x
                            && item.y == position.y
                            && !is_unlimited(item.ingredient)
                    })
                    .collect();
                (
                    format!(
                        "picked up {} at {}",
                        items
                            .iter()
                            .map(|item| INVENTORY_ITEMS[item.ingredient].0)
                            .collect::<Vec<_>>()
                            .join(", "),
                        history::cell_label(position)
                    ),
                    (items.len() == 1).then(|| items[0].ingredient),
                )
            }
            GameAction::Rest => (format!("rested at {}", history::cell_label(position)), None),
            GameAction::Undo => ("undid the previous action".into(), None),
            GameAction::ToggleHover => (
                format!(
                    "turned hover {} at {}",
                    if before.hovering[player.index()] {
                        "off"
                    } else {
                        "on"
                    },
                    history::cell_label(position)
                ),
                None,
            ),
        };
        let move_destination = match &action {
            GameAction::Move { x, y } => Some((*x, *y)),
            _ => None,
        };
        match action {
            GameAction::Move { x, y } => {
                let cell = self.checked_cell(x, y)?;
                self.click(action_position(player, Action::Move));
                self.click(cell_center(cell));
            }
            GameAction::Place { ingredient, x, y } => {
                let cell = self.checked_cell(x, y)?;
                let slot = INVENTORY_ORDER
                    .iter()
                    .position(|&index| index == ingredient)
                    .ok_or_else(|| "Invalid ingredient".to_owned())?;
                let left = match player {
                    PlayerId::One => -RESOLUTION_WIDTH / 2.0,
                    PlayerId::Two => RESOLUTION_WIDTH / 2.0 - 2.0 * GRID_BLOCK_SIZE,
                };
                self.click(Vec2::new(
                    left + (slot % 2) as f32 * GRID_BLOCK_SIZE + GRID_BLOCK_SIZE / 2.0,
                    BOARD_TOP - (slot / 2) as f32 * GRID_BLOCK_SIZE - GRID_BLOCK_SIZE / 2.0,
                ));
                self.click(cell_center(cell));
            }
            GameAction::Pickup => self.click(action_position(player, Action::Pickup)),
            GameAction::Rest => self.click(action_position(player, Action::Rest)),
            GameAction::Undo => self.click(action_position(player, Action::Undo)),
            GameAction::ToggleHover => self.click(action_position(player, Action::Hover)),
        }
        let result = self.settle();
        self.app
            .world_mut()
            .run_system_once(|mut commands: Commands, mut selection: ResMut<Selection>| {
                clear_selection(&mut commands, &mut selection);
            })
            .expect("headless match selection cleanup failed");
        if let Err(error) = result {
            self.broken = true;
            return Err(error);
        }
        if self.action_state() == before {
            let world = self.app.world_mut();
            let feedback = world
                .query_filtered::<&Text, With<DropFeedback>>()
                .single(world)
                .expect("headless match feedback must exist")
                .0
                .clone();
            return Err(feedback);
        }
        if let Some((x, y)) = move_destination {
            let world = self.app.world_mut();
            world
                .resource_mut::<sound::SoundCues>()
                .request(sound::Cue::Whoosh);
            // The final position may include later terrain displacement.
            world.resource_mut::<AnimationLog>().events.insert(
                0,
                AnimationEvent {
                    at_ms: 0,
                    animation: Animation::MovePlayer { seat, x, y },
                },
            );
        }
        let after = self.view(seat);
        let events = &mut self.app.world_mut().resource_mut::<MatchEvents>().entries;
        events.insert(
            0,
            HistoryEntry {
                seat: Some(seat),
                text: description,
                item,
            },
        );
        if after.wind != before_view.wind {
            events.push(HistoryEntry {
                seat: None,
                text: format!("Wind changed to {}", after.wind),
                item: None,
            });
        }
        if after.players.iter().any(|player| player.health == 0) {
            let winner = match (after.players[0].health == 0, after.players[1].health == 0) {
                (true, false) => Some(Seat::Guest),
                (false, true) => Some(Seat::Host),
                _ => None,
            };
            events.push(HistoryEntry {
                seat: winner,
                text: if winner.is_some() {
                    "won the match"
                } else {
                    "Match ended in a draw"
                }
                .into(),
                item: None,
            });
        } else if after.active != before_view.active {
            events.push(HistoryEntry {
                seat: Some(after.active),
                text: "started a turn".into(),
                item: None,
            });
        }
        self.history.append(events);
        Ok(())
    }

    fn checked_cell(&self, x: i32, y: i32) -> Result<IVec2, String> {
        let cell = IVec2::new(x, y);
        self.app
            .world()
            .resource::<Board>()
            .ground(cell)
            .map(|_| cell)
            .ok_or_else(|| "Cell is outside the board".into())
    }

    fn click(&mut self, position: Vec2) {
        self.app.world_mut().resource_mut::<BoardCursor>().0 = Some(position);
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        self.update();
        let mut mouse = self
            .app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(MouseButton::Left);
        self.update();
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    }

    fn settle(&mut self) -> Result<(), String> {
        for _ in 0..5120 {
            let turn = self.app.world().resource::<TurnState>();
            if !turn.animation_active()
                && !turn
                    .pending_placement
                    .as_ref()
                    .is_some_and(|pending| pending.ready)
            {
                return Ok(());
            }
            self.app
                .world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            self.update();
        }
        Err("Could not resolve the action".into())
    }

    fn update(&mut self) {
        self.app.update();
        let world = self.app.world_mut();
        let turn = world.resource::<TurnState>();
        let serpents = turn
            .serpent_animation
            .as_ref()
            .map(|animation| (animation.visuals.clone(), animation.impact));
        let aquatic = turn
            .aquatic_animation
            .as_ref()
            .map(|animation| animation.creatures.clone());
        let weather = turn.weather_animation.as_ref().and_then(|animation| {
            if let WeatherPhase::Playing { kind, .. } = animation.phase {
                Some(kind)
            } else {
                None
            }
        });
        let flight = turn.item_flight.as_ref().and_then(|flight| {
            let pending = turn.pending_placement.as_ref()?;
            let from = board_cell(flight.origin)?;
            let to = pending.target.cell;
            Some((
                flight.entity,
                Animation::Throw {
                    ingredient: pending.index,
                    from: (from.x, from.y),
                    to: (to.x, to.y),
                    corner: flight.destination != cell_center(to),
                },
            ))
        });
        let time = world.resource::<Time>().elapsed();
        let health = world.resource::<PlayerHealth>().0;
        let tiles = history_tiles(world.resource::<Board>());
        let notices = world
            .resource::<TurnState>()
            .weather_animation
            .as_ref()
            .map(|animation| animation.notices.clone())
            .unwrap_or_default();
        let mut positions = [None; 2];
        for (player, transform) in world.query::<(&Player, &Transform)>().iter(world) {
            positions[player.0.index()] = board_cell(transform.translation.truncate());
        }
        if let Some((entity, animation)) = flight
            && world.resource::<AnimationLog>().last_flight != Some(entity)
        {
            if let Animation::Throw { ingredient, to, .. } = &animation {
                let actor = if world.resource::<TurnState>().active == PlayerId::One {
                    Seat::Host
                } else {
                    Seat::Guest
                };
                world.resource_mut::<MatchEvents>().add(
                    Some(actor),
                    format!(
                        "threw {} at {}",
                        INVENTORY_ITEMS[*ingredient].0,
                        history::cell_label(IVec2::new(to.0, to.1))
                    ),
                    Some(*ingredient),
                );
            }
            let mut log = world.resource_mut::<AnimationLog>();
            log.last_flight = Some(entity);
            log.record(time, animation);
        }
        let mut log = world.resource_mut::<AnimationLog>();
        if log.serpents != serpents {
            let (visuals, impact) = serpents.as_ref().map_or((None, None), |(visuals, impact)| {
                (
                    Some(
                        visuals
                            .iter()
                            .map(|visual| SerpentView {
                                cells: visual.cells.iter().map(|cell| (cell.x, cell.y)).collect(),
                                venomous: visual.venomous,
                                straight_frame: visual.straight_frame,
                            })
                            .collect(),
                    ),
                    impact.map(|cell| (cell.x, cell.y)),
                )
            });
            log.record(time, Animation::Serpents { visuals, impact });
            log.serpents = serpents;
        }
        if log.aquatic != aquatic {
            let visuals = aquatic.as_ref().map(|creatures| {
                creatures
                    .iter()
                    .map(|visual| AquaticAnimationView {
                        id: visual.id,
                        x: visual.origin.x,
                        y: visual.origin.y,
                        lengths: visual.lengths,
                        sweep_frame: visual.sweep_frame,
                    })
                    .collect()
            });
            log.record(time, Animation::Aquatic { visuals });
            log.aquatic = aquatic;
        }
        if log.weather != weather {
            if let Some(kind) = weather {
                log.record(
                    time,
                    Animation::Weather {
                        kind: match kind {
                            WeatherKind::Rain => WeatherEffect::Rain,
                            WeatherKind::Earthquake => WeatherEffect::Earthquake,
                        },
                    },
                );
            }
            log.weather = weather;
        }
        let causes = world
            .resource_mut::<TurnState>()
            .history_causes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        let mut events = world.resource_mut::<MatchEvents>();
        events.entries.extend(causes);
        if notices != events.weather_notices {
            for notice in notices.iter().skip(events.weather_notices.len()) {
                events.add(None, notice.clone(), None);
            }
            events.weather_notices = notices;
        }
        let mut tile_changes = Vec::new();
        for (index, (old, new)) in events.tiles.iter().zip(&tiles).enumerate() {
            if old == new {
                continue;
            }
            let mut changes = Vec::new();
            if old.ground != new.ground {
                changes.push(format!(
                    "{} became {}",
                    ground_name(old.ground),
                    ground_name(new.ground)
                ));
            }
            for (name, before, after) in [
                ("ember", old.oil, new.oil),
                ("ice", old.frozen, new.frozen),
                ("poison", old.poison, new.poison),
                ("acid", old.acid, new.acid),
                ("electrification", old.electrified, new.electrified),
                ("Life Essence", old.life, new.life),
                ("thorns", old.thorns, new.thorns),
            ] {
                if before != after {
                    changes.push(format!(
                        "{name} {}",
                        if after { "appeared" } else { "cleared" }
                    ));
                }
            }
            if old.growth != new.growth {
                changes.push(
                    match new.growth {
                        Some(TerrainGrowthKind::Void) => "void growth began",
                        Some(TerrainGrowthKind::Stone) => "stone growth began",
                        None => "terrain growth ended",
                    }
                    .into(),
                );
            }
            if !changes.is_empty() {
                let cell = IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32);
                let entry = HistoryEntry {
                    seat: None,
                    text: format!("At {}: {}", history::cell_label(cell), changes.join(", ")),
                    item: None,
                };
                tile_changes.push(entry);
            }
        }
        events.entries.extend(tile_changes);
        events.tiles = tiles;
        for index in 0..2 {
            if health[index] != events.health[index] {
                let difference = health[index].abs_diff(events.health[index]);
                let verb = if health[index] < events.health[index] {
                    "lost"
                } else {
                    "recovered"
                };
                let location = positions[index]
                    .map(history::cell_label)
                    .unwrap_or_else(|| "the board".into());
                events.add(
                    Some(if index == 0 { Seat::Host } else { Seat::Guest }),
                    format!("{verb} {difference} half-heart(s) at {location} (health: {}/10 half-hearts)", health[index]),
                    None,
                );
            }
        }
        events.health = health;
    }

    fn action_state(&mut self) -> ActionState {
        let world = self.app.world_mut();
        let mut positions = [None; 2];
        for (player, transform) in world.query::<(&Player, &Transform)>().iter(world) {
            positions[player.0.index()] = board_cell(transform.translation.truncate());
        }
        let turn = world.resource::<TurnState>();
        ActionState {
            active: turn.active,
            actions_used: turn.actions_used,
            strength: turn.strength,
            magic: turn.magic,
            hovering: turn.hovering,
            inventory: turn.inventory,
            positions,
        }
    }
}

fn action_position(player: PlayerId, action: Action) -> Vec2 {
    let x = match player {
        PlayerId::One => BOARD_LEFT - GRID_BLOCK_SIZE / 2.0,
        PlayerId::Two => BOARD_RIGHT + GRID_BLOCK_SIZE / 2.0,
    };
    Vec2::new(x, BOARD_TOP - (action.row() as f32 + 0.5) * GRID_BLOCK_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_match_can_be_owned_by_the_server() {
        fn assert_send<T: Send>() {}
        assert_send::<HeadlessMatch>();
    }

    #[test]
    fn headless_match_validates_turns_and_completes_actions() {
        let mut game = HeadlessMatch::new([7; 8]);
        assert!(game.perform(Seat::Guest, GameAction::Rest).is_err());
        assert!(
            game.perform(Seat::Host, GameAction::Move { x: -1, y: 0 })
                .is_err()
        );
        assert!(
            game.perform(Seat::Host, GameAction::Move { x: 3, y: 3 })
                .is_ok()
        );
        assert!(game.perform(Seat::Host, GameAction::Rest).is_ok());
        assert!(game.perform(Seat::Host, GameAction::Rest).is_err());
        assert!(game.perform(Seat::Guest, GameAction::Rest).is_ok());
    }

    #[test]
    fn headless_match_resolves_queued_water_using_existing_rules() {
        let mut game = HeadlessMatch::new([7; 8]);
        let cell = PLAYER_STARTS[0];
        assert!(
            game.perform(
                Seat::Host,
                GameAction::Place {
                    ingredient: WATER_ITEM_INDEX,
                    x: cell.x,
                    y: cell.y,
                }
            )
            .is_ok()
        );
        assert!(game.perform(Seat::Host, GameAction::Rest).is_ok());
        assert!(game.app.world().resource::<Board>().is_water(cell));
        let sounds = game.take_sounds();
        assert!(sounds.contains(&SoundEffect::Water));
        assert!(sounds.contains(&SoundEffect::Chime));
        assert!(game.take_sounds().is_empty());
    }

    #[test]
    fn movement_precedes_the_queued_throw_and_uses_the_moved_position() {
        let mut game = HeadlessMatch::new([7; 8]);
        let origin = PLAYER_STARTS[0];
        let destination = IVec2::new(3, 3);
        game.perform(
            Seat::Host,
            GameAction::Place {
                ingredient: WATER_ITEM_INDEX,
                x: origin.x,
                y: origin.y,
            },
        )
        .unwrap();
        game.take_animations();
        game.perform(
            Seat::Host,
            GameAction::Move {
                x: destination.x,
                y: destination.y,
            },
        )
        .unwrap();
        let events = game.take_animations();
        assert!(matches!(
            events.first().map(|event| &event.animation),
            Some(Animation::MovePlayer {
                seat: Seat::Host,
                x: 3,
                y: 3,
            })
        ));
        assert!(
            events
                .iter()
                .any(|event| matches!(event.animation, Animation::Throw { from: (3, 3), .. }))
        );
        assert!(
            game.perform(Seat::Guest, GameAction::Move { x: -1, y: 0 })
                .is_err()
        );
        assert!(game.take_animations().is_empty());
        let history = game.history();
        let queued = history
            .iter()
            .position(|event| event.text.contains("queued Tlaloc's Water"))
            .unwrap();
        let moved = history
            .iter()
            .position(|event| event.text.contains("moved from"))
            .unwrap();
        let thrown = history
            .iter()
            .position(|event| event.text.contains("threw Tlaloc's Water"))
            .unwrap();
        assert!(queued < moved && moved < thrown);
        assert_eq!(history[queued].seat, Some(Seat::Host));
        assert!(history[queued].text.contains(&history::cell_label(origin)));
        assert!(
            history[moved]
                .text
                .contains(&history::cell_label(destination))
        );
        let len = history.len();
        assert!(game.perform(Seat::Host, GameAction::Rest).is_err());
        assert_eq!(game.history().len(), len);
    }

    #[test]
    fn reactions_record_throw_and_transient_effects_in_order() {
        use volatile_online_protocol::VisualEffect;

        for (ground, ingredient, expected) in [
            (Ground::Water, OIL_ITEM_INDEX, VisualEffect::Steam),
            (Ground::Solid, SUNSTONE_ITEM_INDEX, VisualEffect::Explosion),
        ] {
            let mut game = HeadlessMatch::new([7; 8]);
            let cell = PLAYER_STARTS[0];
            {
                let mut board = game.app.world_mut().resource_mut::<Board>();
                board.set(cell, ground);
                if ground == Ground::Solid {
                    board.add_oil(cell);
                }
            }
            game.app.update();
            game.settle().unwrap();
            game.perform(
                Seat::Host,
                GameAction::Place {
                    ingredient,
                    x: cell.x,
                    y: cell.y,
                },
            )
            .unwrap();
            assert!(game.take_animations().is_empty());
            game.perform(Seat::Host, GameAction::Rest).unwrap();
            let events = game.take_animations();
            let throw = events
                .iter()
                .find(|event| matches!(event.animation, Animation::Throw { ingredient: item, .. } if item == ingredient))
                .expect("queued item must be thrown");
            let effect = events
                .iter()
                .find(|event| matches!(event.animation, Animation::Effect { effect, .. } if effect == expected))
                .expect("reaction must produce a visible effect");
            assert!(effect.at_ms >= throw.at_ms + 400);
            assert!(game.take_animations().is_empty());
            let history = game.history();
            let thrown = history
                .iter()
                .position(|entry| entry.text.starts_with("threw "))
                .unwrap();
            let reaction = history
                .iter()
                .position(|entry| {
                    entry.text.contains(if expected == VisualEffect::Steam {
                        "Steam formed when water met Tlexochtli Ember"
                    } else {
                        "Sunstone ignited Tlexochtli Ember"
                    })
                })
                .unwrap();
            let damage = history
                .iter()
                .position(|entry| entry.text.contains("lost ") && entry.text.contains("half-heart"))
                .unwrap();
            assert!(thrown < reaction && reaction < damage);
            assert!(history[reaction].text.contains(&history::cell_label(cell)));
        }
    }

    #[test]
    fn history_explains_hazards_and_undo_without_recording_rejected_actions() {
        let mut game = HeadlessMatch::new([7; 8]);
        let origin = PLAYER_STARTS[0];
        game.perform(Seat::Host, GameAction::Move { x: 3, y: 3 })
            .unwrap();
        game.perform(Seat::Host, GameAction::Undo).unwrap();
        assert!(
            game.history()
                .iter()
                .any(|entry| entry.text == "undid the previous action")
        );
        game.app
            .world_mut()
            .resource_mut::<Board>()
            .add_poison(origin);
        game.app.world_mut().resource_mut::<PlayerHealth>().0[0] = 1;
        game.app.update();
        game.settle().unwrap();
        game.perform(Seat::Host, GameAction::Rest).unwrap();
        let history = game.history();
        let cause = history
            .iter()
            .position(|entry| entry.text.contains("hurt by poison"))
            .unwrap();
        let loss = history
            .iter()
            .position(|entry| entry.text.contains("lost 1 half-heart"))
            .unwrap();
        assert!(cause < loss);
        assert!(history[cause].text.contains(&history::cell_label(origin)));
        assert_eq!(history.last().unwrap().seat, Some(Seat::Guest));
        assert_eq!(history.last().unwrap().text, "won the match");
        let len = history.len();
        assert!(
            game.perform(Seat::Host, GameAction::Move { x: i32::MIN, y: 0 })
                .is_err()
        );
        assert_eq!(game.history().len(), len);
    }

    #[test]
    fn history_records_public_ingredient_drops() {
        let mut game = HeadlessMatch::new([7; 8]);
        game.app
            .world_mut()
            .resource_mut::<TurnState>()
            .turns_until_drop = 1;
        game.perform(Seat::Host, GameAction::Rest).unwrap();
        let view = game.view(Seat::Host);
        let drops: Vec<_> = game
            .history()
            .iter()
            .filter(|entry| entry.text.contains("appeared at"))
            .collect();
        assert!(!drops.is_empty());
        for drop in drops {
            assert!(view.items.iter().any(|item| {
                Some(item.ingredient) == drop.item
                    && drop
                        .text
                        .contains(&history::cell_label(IVec2::new(item.x, item.y)))
            }));
        }
    }

    #[test]
    fn turn_records_creature_frames_and_weather() {
        let mut game = HeadlessMatch::new([7; 8]);
        let origin = IVec2::new(4, 3);
        {
            let world = game.app.world_mut();
            let mut board = world.resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            for _ in 0..3 {
                board.finish_aquatic_turn();
            }
            let serpent =
                serpent::Serpent::new(IVec2::new(5, 1), &board, PLAYER_STARTS.map(Some), [10, 10]);
            drop(board);
            world.resource_mut::<TurnState>().serpents.push(serpent);
            let mut weather = world.resource_mut::<Weather>();
            weather.rain = 1;
            weather.earthquake = 1;
        }
        game.app.update();
        game.settle().unwrap();
        game.perform(Seat::Host, GameAction::Rest).unwrap();
        let events = game.take_animations();
        assert!(
            events
                .iter()
                .any(|event| matches!(event.animation, Animation::Aquatic { visuals: Some(_) }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event.animation, Animation::Aquatic { visuals: None }))
        );
        assert!(events.iter().any(|event| matches!(
            event.animation,
            Animation::Serpents {
                visuals: Some(_),
                ..
            }
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event.animation, Animation::Serpents { visuals: None, .. }))
        );
        for kind in [WeatherEffect::Rain, WeatherEffect::Earthquake] {
            assert!(events.iter().any(|event| matches!(
                event.animation,
                Animation::Weather { kind: recorded } if recorded == kind
            )));
        }
    }

    #[test]
    fn rejected_placement_does_not_interfere_with_a_later_action() {
        let mut game = HeadlessMatch::new([7; 8]);
        let cell = PLAYER_STARTS[0];
        game.app
            .world_mut()
            .resource_mut::<Board>()
            .make_water(cell);
        assert!(
            game.perform(
                Seat::Host,
                GameAction::Place {
                    ingredient: WATER_ITEM_INDEX,
                    x: cell.x,
                    y: cell.y,
                },
            )
            .is_err()
        );
        assert!(
            game.perform(
                Seat::Host,
                GameAction::Place {
                    ingredient: STONE_DUST_ITEM_INDEX,
                    x: cell.x,
                    y: cell.y,
                },
            )
            .is_ok()
        );
    }

    #[test]
    fn match_view_contains_only_the_receiving_players_inventory() {
        let mut game = HeadlessMatch::new([7; 8]);
        game.app.world_mut().resource_mut::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX] = 3;
        game.app.world_mut().resource_mut::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX] = 7;
        let host = game.view(Seat::Host);
        let guest = game.view(Seat::Guest);
        assert_eq!(host.own_inventory[SUNSTONE_ITEM_INDEX], 3);
        assert_eq!(guest.own_inventory[SUNSTONE_ITEM_INDEX], 7);
        assert_eq!(host.tiles.len(), COLUMNS * ROWS);
        assert_eq!(host.items.len(), 2);
    }
}
