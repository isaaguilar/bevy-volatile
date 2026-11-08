use bevy::ecs::system::RunSystemOnce;
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    camera::ScalingMode,
    ecs::system::SystemParam,
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    text::TextLayoutInfo,
    window::WindowResolution,
};
use bevy_aspect_ratio_mask::{AspectRatioPlugin, Hud, Resolution};
use bevy_rand::prelude::{EntropyPlugin, GlobalRng, WyRand};
use rand::{Rng, RngExt};

mod ai;
mod aquatic;
mod audio_settings;
mod board;
mod effects;
mod ground_art;
mod help;
mod history;
mod menu;
mod music;
mod serpent;
mod sound;
mod thorn;
mod void;

use board::{Board, COLUMNS, Ground, PLAYER_STARTS, ROWS, TerrainGrowthKind};

const RESOLUTION_WIDTH: f32 = 960.0;
const RESOLUTION_HEIGHT: f32 = 540.0;
const GRID_COLUMNS: u32 = 16;
const GRID_BLOCK_SIZE: f32 = RESOLUTION_WIDTH / GRID_COLUMNS as f32;
const GRID_LINE_WIDTH: f32 = 2.0;
const BOARD_COLUMNS: u32 = COLUMNS as u32;
const BOARD_ROWS: u32 = ROWS as u32;
const BOARD_LEFT: f32 = -RESOLUTION_WIDTH / 2.0 + 3.0 * GRID_BLOCK_SIZE;
const BOARD_TOP: f32 = RESOLUTION_HEIGHT / 2.0 - GRID_BLOCK_SIZE;
const BOARD_RIGHT: f32 = BOARD_LEFT + BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE;
const BOARD_BOTTOM: f32 = BOARD_TOP - BOARD_ROWS as f32 * GRID_BLOCK_SIZE;
const HEALTH_PANEL_INSET: f32 = 6.0;
const HEALTH_PANEL_WIDTH: f32 = 3.0 * GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEALTH_PANEL_HEIGHT: f32 = GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEALTH_PANEL_BORDER: f32 = 3.0;
const ENERGY_BAR_SIDE_PADDING: f32 = 6.0;
const HEART_COUNT: usize = 5;
const MAX_STRENGTH: u8 = 5;
const PARTIAL_REST_RECOVERY: u8 = 2;
const MAX_MAGIC: u8 = 5;
const INGREDIENT_RESPAWN_DELAY: u8 = 5;
const INGREDIENT_POOL_SIZE: usize = 64;
const MIN_INGREDIENT_COPIES: u8 = 2;
const INGREDIENT_DROP_INTERVAL: u8 = 3;
const INGREDIENT_DROP_BATCH: usize = 3;
const RAIN_INTERVAL: std::ops::RangeInclusive<u16> = 10..=20;
const EARTHQUAKE_INTERVAL: std::ops::RangeInclusive<u16> = 20..=30;
const EARTHQUAKE_TILES: usize = 2;
const SERPENT_STEP_SECONDS: f32 = 0.35;
const SERPENT_IMPACT_SECONDS: f32 = 0.6;
const AQUATIC_STEP_SECONDS: f32 = 0.25;
const ITEM_FLIGHT_SECONDS: f32 = 0.45;
const ITEM_FLIGHT_HEIGHT: f32 = GRID_BLOCK_SIZE * 0.8;
const ITEM_CORNER_OFFSET: f32 = 15.0;
const PLAYER_FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(333);
const PLAYER_FRAME_SIZE: f32 = 48.0;
const FROZEN_FRAME_SIZE: f32 = 32.0;
const FROZEN_FRAME_DURATION: std::time::Duration = std::time::Duration::from_millis(250);
const FROZEN_LAST_FRAME: usize = 5;
const WATER_ITEM_INDEX: usize = 0;
const SUNSTONE_ITEM_INDEX: usize = 1;
const FROST_CRYSTAL_ITEM_INDEX: usize = 2;
const THORNHEART_ITEM_INDEX: usize = 3;
const STORM_SHARD_ITEM_INDEX: usize = 4;
const VOID_DUST_ITEM_INDEX: usize = 5;
const OBSIDIAN_ITEM_INDEX: usize = 6;
const STONE_DUST_ITEM_INDEX: usize = 7;
const OIL_ITEM_INDEX: usize = 8;
const ACID_ITEM_INDEX: usize = 9;
const POISON_ITEM_INDEX: usize = 10;
const COATL_EGG_ITEM_INDEX: usize = 11;
const LIFE_ESSENCE_ITEM_INDEX: usize = 12;
const AQUATIC_ITEM_INDEX: usize = 13;
const INGREDIENT_WEIGHTS: [u32; 14] = [0, 4, 4, 2, 4, 0, 4, 0, 4, 4, 4, 1, 4, 2];
const INVENTORY_ITEMS: [(&str, usize); 14] = [
    ("Tlaloc's Water", 0),
    ("Sunstone", 4),
    ("Frost Crystal", 6),
    ("Thorn Heart", 10),
    ("Storm Shard", 7),
    ("Void Dust", 2),
    ("Obsidian", 5),
    ("Stone Dust", 3),
    ("Tlexochtli Ember", 1),
    ("Toad Wart", 9),
    ("Poison Frog Sphere", 8),
    ("Coatl's Egg", 11),
    ("Life Essence", 13),
    ("Ahuizotl Totem", 12),
];
const INVENTORY_ORDER: [usize; 14] = [
    WATER_ITEM_INDEX,
    OIL_ITEM_INDEX,
    VOID_DUST_ITEM_INDEX,
    STONE_DUST_ITEM_INDEX,
    SUNSTONE_ITEM_INDEX,
    OBSIDIAN_ITEM_INDEX,
    FROST_CRYSTAL_ITEM_INDEX,
    STORM_SHARD_ITEM_INDEX,
    POISON_ITEM_INDEX,
    ACID_ITEM_INDEX,
    THORNHEART_ITEM_INDEX,
    COATL_EGG_ITEM_INDEX,
    AQUATIC_ITEM_INDEX,
    LIFE_ESSENCE_ITEM_INDEX,
];

#[derive(Component)]
struct Player(PlayerId);

#[derive(Component)]
struct PlayerAnimation {
    timer: Timer,
    hovering: bool,
    frame: usize,
}

impl Default for PlayerAnimation {
    fn default() -> Self {
        Self {
            timer: Timer::new(PLAYER_FRAME_DURATION, TimerMode::Repeating),
            hovering: false,
            frame: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerId {
    One,
    Two,
}

impl PlayerId {
    fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }

    fn other(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

#[derive(Resource)]
struct TurnState {
    active: PlayerId,
    strength: [u8; 2],
    magic: [u8; 2],
    hovering: [bool; 2],
    frozen_turns: [u8; 2],
    inventory: [[usize; 14]; 2],
    ingredient_respawns: Vec<(usize, u8)>,
    spawned_ingredients: [u8; 14],
    turns_until_drop: u8,
    serpents: Vec<serpent::Serpent>,
    serpent_animation: Option<SerpentAnimation>,
    aquatic_animation: Option<AquaticAnimation>,
    effects_active: bool,
    item_flight: Option<ItemFlight>,
    weather_animation: Option<WeatherAnimation>,
    aquatic_contacts: Vec<(usize, PlayerId)>,
    terrain_displacements: Vec<(PlayerId, IVec2)>,
    terrain_limit_hit: bool,
    terrain_rejection: Option<(usize, IVec2)>,
    actions_used: u8,
    undo: Option<UndoAction>,
    free_action: bool,
    moved_while_hovering: bool,
    pending_placement: Option<PendingPlacement>,
    next_placement: Option<PendingPlacement>,
    pending_effects: TurnEffects,
    pending_direct_hit: Option<DeferredDirectHit>,
    existing_respawns: Option<usize>,
    partial_rest: bool,
    acted_this_frame: bool,
    completed_turn: bool,
    wind: WindDirection,
}

#[derive(Resource)]
struct Weather {
    rain: u16,
    earthquake: u16,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WeatherKind {
    Rain,
    Earthquake,
}

enum WeatherPhase {
    Waiting,
    Playing {
        kind: WeatherKind,
        elapsed: f32,
        overlay: Option<Entity>,
    },
    Resolving,
}

struct WeatherAnimation {
    pending: std::collections::VecDeque<WeatherKind>,
    phase: WeatherPhase,
    notices: Vec<String>,
}

#[derive(Component)]
struct RainOverlay;

const WEATHER_ANIMATION_SECONDS: f32 = 2.5;
const EARTHQUAKE_FRAME_SECONDS: f32 = 0.15;
const EARTHQUAKE_OFFSET: f32 = 4.0;

impl Default for Weather {
    fn default() -> Self {
        Self {
            rain: *RAIN_INTERVAL.start(),
            earthquake: *EARTHQUAKE_INTERVAL.start(),
        }
    }
}

struct SerpentAnimationFrame {
    visuals: Vec<serpent::SerpentVisual>,
    hit: Option<(usize, u8)>,
    impact: Option<IVec2>,
}

struct SerpentAnimation {
    visuals: Vec<serpent::SerpentVisual>,
    impact: Option<IVec2>,
    frames: std::collections::VecDeque<SerpentAnimationFrame>,
    timer: Timer,
}

struct AquaticAnimationFrame {
    visuals: Vec<IVec2>,
    creatures: Vec<aquatic::AquaticVisual>,
    swept: Vec<IVec2>,
    flattened: Vec<IVec2>,
}

struct AquaticAnimation {
    visuals: Vec<IVec2>,
    creatures: Vec<aquatic::AquaticVisual>,
    frames: std::collections::VecDeque<AquaticAnimationFrame>,
    swept: std::collections::HashSet<IVec2>,
    unflattened: std::collections::HashSet<IVec2>,
    timer: Timer,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum WindDirection {
    #[default]
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl WindDirection {
    fn random(rng: &mut impl Rng) -> Self {
        const DIRECTIONS: [WindDirection; 8] = [
            WindDirection::North,
            WindDirection::NorthEast,
            WindDirection::East,
            WindDirection::SouthEast,
            WindDirection::South,
            WindDirection::SouthWest,
            WindDirection::West,
            WindDirection::NorthWest,
        ];
        DIRECTIONS[rng.random_range(0..DIRECTIONS.len())]
    }

    fn offset(self) -> IVec2 {
        match self {
            Self::North => IVec2::new(0, -1),
            Self::NorthEast => IVec2::new(1, -1),
            Self::East => IVec2::new(1, 0),
            Self::SouthEast => IVec2::new(1, 1),
            Self::South => IVec2::new(0, 1),
            Self::SouthWest => IVec2::new(-1, 1),
            Self::West => IVec2::new(-1, 0),
            Self::NorthWest => IVec2::new(-1, -1),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::North => "N",
            Self::NorthEast => "NE",
            Self::East => "E",
            Self::SouthEast => "SE",
            Self::South => "S",
            Self::SouthWest => "SW",
            Self::West => "W",
            Self::NorthWest => "NW",
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        let mut inventory = [[0; INVENTORY_ITEMS.len()]; 2];
        let mut spawned_ingredients = [0; INVENTORY_ITEMS.len()];
        for player in &mut inventory {
            for index in [SUNSTONE_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX, OIL_ITEM_INDEX] {
                player[index] = 1;
                spawned_ingredients[index] += 1;
            }
        }
        Self {
            active: PlayerId::One,
            strength: [MAX_STRENGTH; 2],
            magic: [MAX_MAGIC; 2],
            hovering: [false; 2],
            frozen_turns: [0; 2],
            inventory,
            ingredient_respawns: Vec::new(),
            spawned_ingredients,
            turns_until_drop: INGREDIENT_DROP_INTERVAL,
            serpents: Vec::new(),
            serpent_animation: None,
            aquatic_animation: None,
            effects_active: false,
            item_flight: None,
            weather_animation: None,
            aquatic_contacts: Vec::new(),
            terrain_displacements: Vec::new(),
            terrain_limit_hit: false,
            terrain_rejection: None,
            actions_used: 0,
            undo: None,
            free_action: false,
            moved_while_hovering: false,
            pending_placement: None,
            next_placement: None,
            pending_effects: TurnEffects::default(),
            pending_direct_hit: None,
            existing_respawns: None,
            partial_rest: false,
            acted_this_frame: false,
            completed_turn: false,
            wind: WindDirection::default(),
        }
    }
}

impl TurnState {
    fn is_frozen(&self, player: PlayerId) -> bool {
        self.frozen_turns[player.index()] > 0
            || self.pending_effects.frozen_players[player.index()] > 0
    }

    fn animation_active(&self) -> bool {
        self.item_flight.is_some()
            || self.aquatic_animation.is_some()
            || self.serpent_animation.is_some()
            || self.effects_active
            || self.weather_animation.is_some()
    }

    fn animate_aquatic(&mut self, impacts: &[Option<aquatic::AquaticImpact>]) {
        let mut visuals: Vec<_> = impacts
            .iter()
            .map(|impact| {
                impact.as_ref().map(|impact| {
                    if impact.frames.is_empty() {
                        impact.final_cells.clone()
                    } else {
                        impact.initial.clone()
                    }
                })
            })
            .collect();
        let initial = visuals.iter().flatten().flatten().copied().collect();
        let mut creatures: Vec<_> = impacts
            .iter()
            .map(|impact| {
                impact.as_ref().map(|impact| {
                    if impact.frames.is_empty() {
                        impact.final_visual.clone()
                    } else {
                        impact.initial_visual.clone()
                    }
                })
            })
            .collect();
        let initial_creatures = creatures.iter().flatten().cloned().collect();
        let mut frames = std::collections::VecDeque::new();
        let mut swept = std::collections::HashSet::new();
        let mut unflattened = std::collections::HashSet::new();
        for (index, impact) in impacts.iter().enumerate() {
            if let Some(impact) = impact {
                for frame in &impact.frames {
                    visuals[index] = Some(frame.cells.clone());
                    creatures[index] = Some(frame.visual.clone());
                    swept.extend(frame.swept.iter().copied());
                    unflattened.extend(frame.flattened.iter().copied());
                    frames.push_back(AquaticAnimationFrame {
                        visuals: visuals.iter().flatten().flatten().copied().collect(),
                        creatures: creatures.iter().flatten().cloned().collect(),
                        swept: frame.swept.clone(),
                        flattened: frame.flattened.clone(),
                    });
                }
                if !impact.frames.is_empty() {
                    creatures[index] = None;
                    visuals[index] = None;
                }
            }
        }
        if !frames.is_empty() {
            self.aquatic_animation = Some(AquaticAnimation {
                visuals: initial,
                creatures: initial_creatures,
                frames,
                swept,
                unflattened,
                timer: Timer::from_seconds(AQUATIC_STEP_SECONDS, TimerMode::Once),
            });
        }
    }

    fn spend_ingredient(&mut self, item: usize) {
        if !is_unlimited(item) {
            self.inventory[self.active.index()][item] -= 1;
        }
    }

    fn refund_ingredient(&mut self, item: usize) {
        if !is_unlimited(item) {
            self.inventory[self.active.index()][item] += 1;
        }
    }

    fn spend_action_energy(&mut self, cost: u8) {
        if cost == 0 {
            self.free_action = true;
        } else {
            self.strength[self.active.index()] -= cost;
        }
    }

    fn animate_serpents(&mut self, board: &Board, players: [Option<IVec2>; 2], health: [u8; 2]) {
        let serpents = std::mem::take(&mut self.serpents);
        let mut visuals: Vec<_> = serpents
            .iter()
            .map(|serpent| Some(serpent.visual()))
            .collect();
        let initial = visuals.iter().flatten().cloned().collect();
        let mut frames = std::collections::VecDeque::new();
        let mut projected_health = health;
        for (index, mut serpent) in serpents.into_iter().enumerate() {
            let mut steps = Vec::new();
            let alive = serpent.advance_animated(board, players, &mut projected_health, &mut steps);
            for step in steps {
                visuals[index] = Some(step.visual);
                frames.push_back(SerpentAnimationFrame {
                    visuals: visuals.iter().flatten().cloned().collect(),
                    hit: step.hit,
                    impact: step.impact,
                });
            }
            if alive {
                visuals[index] = Some(serpent.visual());
                self.serpents.push(serpent);
            } else {
                visuals[index] = None;
                frames.push_back(SerpentAnimationFrame {
                    visuals: visuals.iter().flatten().cloned().collect(),
                    hit: None,
                    impact: None,
                });
            }
        }
        if !frames.is_empty() {
            self.serpent_animation = Some(SerpentAnimation {
                visuals: initial,
                impact: None,
                frames,
                timer: Timer::from_seconds(SERPENT_STEP_SECONDS, TimerMode::Once),
            });
        }
    }

    fn finish_action(
        &mut self,
        board: &mut Board,
        health: &mut PlayerHealth,
        positions: impl IntoIterator<Item = (PlayerId, Vec2)>,
        effects: TurnEffects,
    ) {
        let mut combined = std::mem::take(&mut self.pending_effects);
        combined.merge(effects);
        let mut effects = combined;
        let mut health_changes = effects.health_changes;
        let mut positions: Vec<_> = positions
            .into_iter()
            .map(|(player, position)| {
                let cell = board_cell(position);
                let frozen = cell.is_some_and(|cell| board.is_frozen(cell));
                (player, cell, frozen)
            })
            .collect();
        board.finish_turn(effects.refreshed_water, effects.watered_poison);
        board.advance_terrain_growth();
        let aquatic_impacts = board.finish_aquatic_turn();
        let aquatic_contacts = std::mem::take(&mut self.aquatic_contacts);
        let (new_unsafe, limit_hit) = board.take_terrain_events();
        self.terrain_limit_hit |= limit_hit;
        for index in 0..positions.len() {
            let Some(origin) = positions[index].1 else {
                continue;
            };
            if !new_unsafe.contains(&origin) || board.ground(origin).is_some_and(Ground::walkable) {
                continue;
            }
            let player = positions[index].0;
            self.frozen_turns[player.index()] = 0;
            effects.frozen_players[player.index()] = 0;
            let hover_cost = u8::from(player == self.active);
            if self.magic[player.index()] > hover_cost {
                self.hovering[player.index()] = true;
                continue;
            }
            let destination = (1..=COLUMNS.max(ROWS) as i32).find_map(|distance| {
                (-distance..=distance).find_map(|y| {
                    (-distance..=distance).find_map(|x| {
                        let offset = IVec2::new(x, y);
                        let cell = origin + offset;
                        (offset.abs().max_element() == distance
                            && board.ground(cell).is_some_and(Ground::walkable)
                            && !positions
                                .iter()
                                .any(|(_, occupied, _)| *occupied == Some(cell)))
                        .then_some(cell)
                    })
                })
            });
            if let Some(destination) = destination {
                positions[index].1 = Some(destination);
                positions[index].2 = board.is_frozen(destination);
                self.hovering[player.index()] = false;
                self.terrain_displacements
                    .push((positions[index].0, destination));
                health_changes.damage(player, 1);
            } else {
                error!("No unoccupied safe tile for player displaced by terrain");
            }
        }
        let (expired_thorns, expired_thorn_life) = board.finish_thorn_turn();
        for &(player, cell, frozen) in &positions {
            if player == self.active
                && !self.hovering[player.index()]
                && cell.is_some_and(|cell| board.is_frozen(cell))
            {
                health_changes.damage(player, 1);
            }
            if !frozen
                && !self.hovering[player.index()]
                && cell.is_some_and(|cell| board.is_poisoned(cell))
            {
                health_changes.damage(player, 1);
            }
            if !self.hovering[player.index()] && cell.is_some_and(|cell| board.is_acidic(cell)) {
                health_changes.damage(player, 1);
                if cell.is_some_and(|cell| board.is_electrified_acid(cell)) {
                    health_changes.damage(player, 2);
                }
            }
            if !self.hovering[player.index()]
                && cell.is_some_and(|cell| board.has_life_essence(cell))
            {
                health_changes.heal(player, 1);
            }
            if cell.is_some_and(|cell| board.has_thorns(cell)) {
                health_changes.damage(player, 2);
            }
            for (index, impact) in aquatic_impacts.iter().enumerate() {
                if let Some(impact) = impact
                    && (cell.is_some_and(|cell| impact.cells.contains(&cell))
                        || aquatic_contacts.contains(&(index, player)))
                {
                    health_changes.damage(player, impact.damage);
                }
            }
        }
        self.animate_aquatic(&aquatic_impacts);
        health.apply(health_changes);
        let mut player_cells = [None; 2];
        for &(player, cell, _) in &positions {
            player_cells[player.index()] = cell;
        }
        self.animate_serpents(board, player_cells, health.0);
        board.finish_life_turn();
        let existing_respawns = self
            .existing_respawns
            .take()
            .unwrap_or(self.ingredient_respawns.len());
        for (_, remaining) in self.ingredient_respawns.iter_mut().take(existing_respawns) {
            *remaining = remaining.saturating_sub(1);
        }
        self.ingredient_respawns.extend(std::iter::repeat_n(
            (LIFE_ESSENCE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
            (board.take_expired_life_sources() + expired_thorn_life) as usize,
        ));
        self.ingredient_respawns.extend(std::iter::repeat_n(
            (THORNHEART_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
            expired_thorns as usize,
        ));
        self.ingredient_respawns.extend(std::iter::repeat_n(
            (OIL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
            board.take_expired_oil_sources() as usize,
        ));
        let index = self.active.index();
        self.frozen_turns[index] = self.frozen_turns[index].saturating_sub(1);
        for (remaining, new_freeze) in self.frozen_turns.iter_mut().zip(effects.frozen_players) {
            *remaining = (*remaining).max(new_freeze);
        }
        if effects.resting && self.actions_used == 0 {
            self.strength[index] = MAX_STRENGTH;
        } else {
            let recovery = if effects.resting || self.partial_rest {
                PARTIAL_REST_RECOVERY
            } else {
                u8::from(self.free_action)
            };
            self.strength[index] = self.strength[index]
                .saturating_add(recovery)
                .min(MAX_STRENGTH);
        }
        if self.hovering[index] || self.moved_while_hovering {
            let unsafe_hover = positions.iter().any(|(player, cell, _)| {
                player.index() == index
                    && cell.is_some_and(|cell| !board.ground(cell).is_some_and(Ground::walkable))
            });
            if !unsafe_hover || self.magic[index] > 1 {
                self.magic[index] = self.magic[index].saturating_sub(1);
            }
            if self.magic[index] == 0 {
                self.hovering[index] = false;
            }
        } else {
            self.magic[index] = (self.magic[index] + 1).min(MAX_MAGIC);
        }
        self.active = self.active.other();
        self.actions_used = 0;
        self.free_action = false;
        self.moved_while_hovering = false;
        self.partial_rest = false;
        self.acted_this_frame = true;
        self.completed_turn = true;
    }
}

#[derive(Default)]
struct TurnEffects {
    resting: bool,
    refreshed_water: Vec<IVec2>,
    watered_poison: Vec<IVec2>,
    frozen_players: [u8; 2],
    health_changes: HealthChanges,
}

impl TurnEffects {
    fn merge(&mut self, other: Self) {
        self.resting |= other.resting;
        self.refreshed_water.extend(other.refreshed_water);
        self.watered_poison.extend(other.watered_poison);
        for (remaining, added) in self.frozen_players.iter_mut().zip(other.frozen_players) {
            *remaining = (*remaining).max(added);
        }
        for (total, change) in self.health_changes.0.iter_mut().zip(other.health_changes.0) {
            *total += change;
        }
    }
}

#[derive(Default)]
struct HealthChanges([i16; 2]);

impl HealthChanges {
    fn damage(&mut self, player: PlayerId, half_hearts: u8) {
        self.0[player.index()] -= i16::from(half_hearts);
    }

    fn heal(&mut self, player: PlayerId, half_hearts: u8) {
        self.0[player.index()] += i16::from(half_hearts);
    }
}

struct DeferredDirectHit {
    cell: IVec2,
    frost: bool,
    item: Option<Entity>,
}

fn apply_direct_hit(
    positions: impl IntoIterator<Item = (PlayerId, Vec2)>,
    hit: &DeferredDirectHit,
    hovering: [bool; 2],
    health_changes: &mut HealthChanges,
    frozen_players: &mut [u8; 2],
) -> bool {
    let mut hit_player = false;
    for (player, position) in positions {
        if board_cell(position) == Some(hit.cell) {
            if hit.frost && hovering[player.index()] {
                continue;
            } else if hit.frost {
                frozen_players[player.index()] = frozen_players[player.index()].max(2);
            } else {
                health_changes.damage(player, 1);
            }
            hit_player = true;
        }
    }
    hit_player
}

fn resolve_deferred_hit(
    turn: &mut TurnState,
    commands: &mut Commands,
    positions: impl IntoIterator<Item = (PlayerId, Vec2)>,
    reacted: bool,
) -> bool {
    let Some(hit) = turn.pending_direct_hit.take() else {
        return false;
    };
    if reacted {
        return false;
    }
    let hit_player = apply_direct_hit(
        positions,
        &hit,
        turn.hovering,
        &mut turn.pending_effects.health_changes,
        &mut turn.pending_effects.frozen_players,
    );
    if hit_player {
        commands.spawn(effects::EffectSignal(
            hit.cell,
            if hit.frost {
                effects::Effect::Freeze
            } else {
                effects::Effect::Electric
            },
        ));
        commands
            .entity(hit.item.expect("deferred direct hit has a placed item"))
            .despawn();
        turn.ingredient_respawns.push((
            if hit.frost {
                FROST_CRYSTAL_ITEM_INDEX
            } else {
                STORM_SHARD_ITEM_INDEX
            },
            INGREDIENT_RESPAWN_DELAY,
        ));
    }
    hit_player
}

#[derive(Resource)]
struct PlayerHealth([u8; 2]);

impl Default for PlayerHealth {
    fn default() -> Self {
        Self([(HEART_COUNT * 2) as u8; 2])
    }
}

impl PlayerHealth {
    fn apply(&mut self, changes: HealthChanges) {
        for (health, change) in self.0.iter_mut().zip(changes.0) {
            let result = (i16::from(*health) + change).clamp(0, (HEART_COUNT * 2) as i16);
            *health = u8::try_from(result).expect("clamped health must fit in u8");
        }
    }
}

#[derive(Component)]
struct HealthPanel;

#[derive(Component)]
struct Heart {
    owner: PlayerId,
    index: usize,
}

#[derive(Component)]
struct EnergyFill(PlayerId);

#[derive(Component)]
struct MagicFill(PlayerId);

#[derive(Component)]
struct PickupHighlight;

#[derive(Component)]
struct InventoryPanel(PlayerId);

#[derive(Component)]
struct InventorySlot {
    index: usize,
    owner: PlayerId,
}

#[derive(Component)]
struct InventoryCount {
    owner: PlayerId,
    index: usize,
}

#[derive(Component)]
struct ActionSlot {
    owner: PlayerId,
    action: Action,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Move,
    Hover,
    Pickup,
    Rest,
    Undo,
}

impl Action {
    fn row(self) -> usize {
        match self {
            Self::Move => 0,
            Self::Hover => 1,
            Self::Pickup => 2,
            Self::Rest => 3,
            Self::Undo => 4,
        }
    }

    fn icon_rect(self) -> Option<Rect> {
        let frame = match self {
            Self::Move => 0,
            Self::Hover => 1,
            Self::Pickup => 2,
            Self::Rest => 3,
            Self::Undo => return None,
        };
        let y = frame as f32 * 32.0;
        Some(Rect::new(0.0, y, 32.0, y + 32.0))
    }
}

#[derive(Component)]
struct TurnProgress;

#[derive(Component)]
struct WindStatus;

#[derive(Component)]
struct DropFeedback;

#[derive(Component)]
struct CellFeedback;

#[derive(Component)]
struct FitStatusText {
    initial_size: f32,
    bounds: Vec2,
    last_text: String,
}

#[derive(Component)]
struct BoardItem;

#[derive(Component)]
struct GroundTile(IVec2);

#[derive(Component)]
struct PersistentWorld;

#[derive(Component)]
struct Collectible {
    index: usize,
    cell: IVec2,
}

#[derive(Component)]
struct PlacedItem;

struct ItemFlight {
    entity: Entity,
    origin: Vec2,
    destination: Vec2,
    elapsed: f32,
}

#[derive(Component)]
struct PickedUpForUndo;

struct UndoPickup {
    entity: Entity,
    index: usize,
    cell: IVec2,
    visibility: Option<Visibility>,
}

enum UndoAction {
    Movement {
        origin: Vec2,
        strength: u8,
        hovering: bool,
        moved_while_hovering: bool,
        triggered_frost: Vec<UndoPickup>,
        previous_pending_freeze: u8,
    },
    Placement {
        strength: u8,
    },
    Pickup {
        items: Vec<UndoPickup>,
    },
}

#[derive(Component)]
struct WaterIcon(IVec2);

#[derive(Component)]
struct WaterLabel(IVec2);

#[derive(Component)]
struct PoisonTile(IVec2);

#[derive(Component)]
struct AcidTile(IVec2);

#[derive(Component)]
struct VoidGrowthTile(IVec2);

#[derive(Component)]
struct LifeEssenceTile(IVec2);

#[derive(Component)]
struct ThornTile(IVec2);

#[derive(Component)]
struct PendingFrost(IVec2);

#[derive(Component)]
struct PendingStormShard(IVec2);

#[derive(Component)]
struct PendingSunstone(IVec2);

#[derive(Component)]
struct PendingLifeEssence(IVec2);

#[derive(Component)]
struct PendingThornheart(IVec2);

#[derive(Component)]
struct PendingObsidian(IVec2);

#[derive(Component)]
struct PendingStoneDust(IVec2);

#[derive(Component)]
struct PendingWaterStone(IVec2);

#[derive(Component)]
struct PendingVoidDust(IVec2);

#[derive(Component)]
struct PendingCoatlEgg(IVec2);

#[derive(Component)]
struct PendingAquaticCreature(IVec2);

#[derive(Component)]
struct SerpentPart;

#[derive(Resource, Default)]
struct SerpentSheet(Handle<Image>);

#[derive(Resource, Default)]
struct AquaticSheet(Handle<Image>);

#[derive(Component)]
struct AquaticTile {
    id: u64,
    part: usize,
    frame: usize,
    timer: Timer,
}

#[derive(SystemParam)]
struct SelectionInput<'w, 's> {
    mouse: Res<'w, ButtonInput<MouseButton>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    cursor: Res<'w, BoardCursor>,
    bot: Option<Res<'w, ai::BotController>>,
    rng: Single<'w, 's, &'static mut WyRand, With<GlobalRng>>,
}

#[derive(SystemParam)]
struct SelectionActors<'w, 's> {
    players: Query<'w, 's, (Entity, &'static Player, &'static mut Transform)>,
    crystals: Query<'w, 's, (Entity, &'static PendingFrost), Without<PickedUpForUndo>>,
    shards: Query<'w, 's, (Entity, &'static PendingStormShard), Without<PickedUpForUndo>>,
    sunstones: Query<'w, 's, (Entity, &'static PendingSunstone), Without<PickedUpForUndo>>,
    life_essences: Query<'w, 's, (Entity, &'static PendingLifeEssence), Without<PickedUpForUndo>>,
    thornhearts: Query<'w, 's, (Entity, &'static PendingThornheart), Without<PickedUpForUndo>>,
    obsidians: Query<'w, 's, (Entity, &'static PendingObsidian), Without<PickedUpForUndo>>,
    stones: Query<'w, 's, (Entity, &'static PendingStoneDust), Without<PickedUpForUndo>>,
    water_stones: Query<'w, 's, (Entity, &'static PendingWaterStone), Without<PickedUpForUndo>>,
    void_dust: Query<'w, 's, (Entity, &'static PendingVoidDust), Without<PickedUpForUndo>>,
    eggs: Query<'w, 's, (Entity, &'static PendingCoatlEgg), Without<PickedUpForUndo>>,
    aquatic: Query<'w, 's, (Entity, &'static PendingAquaticCreature), Without<PickedUpForUndo>>,
    collectibles: Query<'w, 's, &'static Collectible, Without<PickedUpForUndo>>,
}

fn is_unlimited(index: usize) -> bool {
    matches!(
        index,
        WATER_ITEM_INDEX | VOID_DUST_ITEM_INDEX | STONE_DUST_ITEM_INDEX
    )
}

#[derive(Component)]
struct ElectricFlash(Timer);

#[derive(Component)]
struct SteamFlash(Timer);

#[derive(Component)]
struct FireFlash(Timer);

#[derive(Component)]
struct FrozenTile(IVec2);

#[derive(Component)]
struct FrostImpact(PlayerId);

#[derive(Resource, Default)]
struct FrozenSheet(Handle<Image>);

#[derive(Component)]
struct FrozenAnimation {
    timer: Timer,
    frame: usize,
}

impl Default for FrozenAnimation {
    fn default() -> Self {
        Self {
            timer: Timer::new(FROZEN_FRAME_DURATION, TimerMode::Repeating),
            frame: 0,
        }
    }
}

#[derive(Resource, Default)]
struct BoardCursor(Option<Vec2>);

#[derive(Resource, Default)]
struct Selection(Option<SelectedAction>);

#[derive(Resource, Default)]
struct InventoryHover {
    hovered: Option<(PlayerId, usize)>,
    selected: Option<(PlayerId, usize)>,
}

#[derive(Resource, Default)]
struct InventoryImages(Handle<Image>);

impl InventoryImages {
    fn rect(index: usize) -> Rect {
        let frame = INVENTORY_ITEMS[index].1;
        let x = (frame % 2) as f32 * 32.0;
        let y = (frame / 2) as f32 * 32.0;
        Rect::new(x, y, x + 32.0, y + 32.0)
    }

    fn icon(&self, index: usize) -> ImageNode {
        ImageNode {
            image: self.0.clone(),
            rect: Some(Self::rect(index)),
            ..default()
        }
    }

    fn sprite(&self, index: usize, size: f32) -> Sprite {
        Sprite {
            image: self.0.clone(),
            rect: Some(Self::rect(index)),
            custom_size: Some(Vec2::splat(size)),
            ..default()
        }
    }
}

enum SelectedAction {
    Item {
        index: usize,
        icon: Entity,
        highlight: Entity,
    },
    Movement {
        highlights: Vec<Entity>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DropKind {
    Place,
    Distant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DropTarget {
    origin: IVec2,
    cell: IVec2,
    distance: u32,
    kind: DropKind,
}

struct PendingPlacement {
    index: usize,
    target: DropTarget,
    marker: Entity,
    ready: bool,
    previous_free_action: bool,
    landed: bool,
}

fn item_energy_cost(target: &DropTarget) -> u32 {
    target.distance.saturating_sub(1)
}

fn item_placement_error(
    board: &Board,
    index: usize,
    cell: IVec2,
    reactive_item: bool,
    has_frost: bool,
) -> Option<&'static str> {
    if index != STONE_DUST_ITEM_INDEX && board.ground(cell) == Some(Ground::Hole) {
        return Some("Only Stone Dust can be placed in a hole");
    }
    if index == THORNHEART_ITEM_INDEX && board.is_acidic(cell) {
        return Some("Cannot place Thornheart on acid - choose another tile");
    }
    if (index == OIL_ITEM_INDEX && has_frost)
        || (index == FROST_CRYSTAL_ITEM_INDEX && board.has_oil(cell))
    {
        return Some("Frost Crystal and Tlexochtli Ember cannot share a tile");
    }
    if index == OIL_ITEM_INDEX && board.is_frozen(cell) {
        return Some("Cannot place Tlexochtli Ember on frozen water");
    }
    if index == OIL_ITEM_INDEX && board.has_oil(cell) {
        return Some("Tile already has Tlexochtli Ember");
    }
    let reacts = reactive_item || board.has_life_essence(cell);
    if index == VOID_DUST_ITEM_INDEX && board.ground(cell) == Some(Ground::Water) && !reacts {
        return Some("Void Dust needs Obsidian or Life Essence on water");
    }
    if index == STONE_DUST_ITEM_INDEX
        && !reacts
        && !board.is_water(cell)
        && board.ground(cell) != Some(Ground::Hole)
        && !board.is_acidic(cell)
        && !board.is_poisoned(cell)
    {
        return Some("Stone Dust needs water, a hole, acid, poison, Obsidian, or Life Essence");
    }
    if index != WATER_ITEM_INDEX {
        return None;
    }
    match board.ground(cell) {
        Some(Ground::Water) => Some("Cannot place water on a water tile - choose another tile"),
        Some(Ground::Hole | Ground::Solid | Ground::Ridge) | None => None,
    }
}

fn queued_placement_error(
    board: &Board,
    index: usize,
    cell: IVec2,
    pending: Option<&PendingPlacement>,
    reactive_item: bool,
    has_frost: bool,
) -> Option<&'static str> {
    if pending.is_some_and(|first| {
        first.target.cell == cell
            && matches!(
                (first.index, index),
                (FROST_CRYSTAL_ITEM_INDEX, OIL_ITEM_INDEX)
                    | (OIL_ITEM_INDEX, FROST_CRYSTAL_ITEM_INDEX)
            )
    }) {
        return Some("Frost Crystal and Tlexochtli Ember cannot share a tile");
    }
    let error = item_placement_error(board, index, cell, reactive_item, has_frost)?;
    if let Some(first) = pending
        && first.target.cell == cell
        && ((first.index == STONE_DUST_ITEM_INDEX && board.ground(cell) == Some(Ground::Hole))
            || (first.index == LIFE_ESSENCE_ITEM_INDEX
                && index == THORNHEART_ITEM_INDEX
                && board.is_acidic(cell))
            || (matches!(index, VOID_DUST_ITEM_INDEX | STONE_DUST_ITEM_INDEX)
                && matches!(first.index, OBSIDIAN_ITEM_INDEX | LIFE_ESSENCE_ITEM_INDEX))
            || (index == STONE_DUST_ITEM_INDEX
                && matches!(
                    first.index,
                    WATER_ITEM_INDEX | ACID_ITEM_INDEX | POISON_ITEM_INDEX
                )))
    {
        return None;
    }
    Some(error)
}

fn projected_placement_error(
    state: (&Board, &TurnState, &PlayerHealth),
    players: &[(PlayerId, Vec2)],
    items: &[(usize, IVec2)],
    rng: &WyRand,
    first: &PendingPlacement,
    second_index: usize,
    second_target: &DropTarget,
) -> Option<String> {
    let (board, turn, health) = state;
    let frost_ember = match (first.index, second_index) {
        (FROST_CRYSTAL_ITEM_INDEX, OIL_ITEM_INDEX) => Some((first.target.cell, second_target.cell)),
        (OIL_ITEM_INDEX, FROST_CRYSTAL_ITEM_INDEX) => Some((second_target.cell, first.target.cell)),
        _ => None,
    };
    if let Some((frost, ember)) = frost_ember {
        if frost == ember {
            return Some("Frost Crystal and Tlexochtli Ember cannot share a tile".into());
        }
        if board.is_water(frost)
            && board.is_water(ember)
            && (frost - ember).abs().max_element() <= 1
        {
            return Some("Cannot combine Ember and Frost on affected water in one turn".into());
        }
    }

    let mut world = World::new();
    let hud = world.spawn_empty().id();
    let first_marker = world.spawn_empty().id();
    let second_marker = world.spawn_empty().id();
    world.insert_resource(Hud(hud));
    world.insert_resource(board.clone());
    world.insert_resource(PlayerHealth(health.0));
    world.insert_resource(InventoryImages::default());
    world.insert_resource(ButtonInput::<MouseButton>::default());
    world.insert_resource(ButtonInput::<KeyCode>::default());
    world.insert_resource(BoardCursor::default());
    world.insert_resource(Selection::default());
    world.spawn((rng.clone(), GlobalRng));
    world.spawn((DropFeedback, Text::default()));
    for &(player, position) in players {
        world.spawn((
            Player(player),
            Transform::from_xyz(position.x, position.y, 0.0),
        ));
    }
    for &(index, cell) in items {
        let mut item = world.spawn((Collectible { index, cell }, BoardItem));
        match index {
            WATER_ITEM_INDEX => {
                item.insert(PendingWaterStone(cell));
            }
            SUNSTONE_ITEM_INDEX => {
                item.insert(PendingSunstone(cell));
            }
            FROST_CRYSTAL_ITEM_INDEX => {
                item.insert(PendingFrost(cell));
            }
            THORNHEART_ITEM_INDEX => {
                item.insert(PendingThornheart(cell));
            }
            STORM_SHARD_ITEM_INDEX => {
                item.insert(PendingStormShard(cell));
            }
            VOID_DUST_ITEM_INDEX => {
                item.insert(PendingVoidDust(cell));
            }
            OBSIDIAN_ITEM_INDEX => {
                item.insert(PendingObsidian(cell));
            }
            STONE_DUST_ITEM_INDEX => {
                item.insert(PendingStoneDust(cell));
            }
            COATL_EGG_ITEM_INDEX => {
                item.insert(PendingCoatlEgg(cell));
            }
            LIFE_ESSENCE_ITEM_INDEX => {
                item.insert(PendingLifeEssence(cell));
            }
            AQUATIC_ITEM_INDEX => {
                item.insert(PendingAquaticCreature(cell));
            }
            _ => {}
        }
    }
    let projected = TurnState {
        active: turn.active,
        strength: turn.strength,
        magic: turn.magic,
        hovering: turn.hovering,
        frozen_turns: turn.frozen_turns,
        inventory: turn.inventory,
        wind: turn.wind,
        actions_used: 2,
        free_action: turn.free_action,
        pending_placement: Some(PendingPlacement {
            index: first.index,
            target: first.target.clone(),
            marker: first_marker,
            ready: true,
            previous_free_action: first.previous_free_action,
            landed: true,
        }),
        next_placement: Some(PendingPlacement {
            index: second_index,
            target: second_target.clone(),
            marker: second_marker,
            ready: true,
            previous_free_action: turn.free_action,
            landed: true,
        }),
        ..default()
    };
    world.insert_resource(projected);
    world
        .run_system_once(handle_selection)
        .expect("placement preview system failed");
    if world.resource::<TurnState>().pending_placement.is_none() {
        return Some("Queued placement cannot resolve - undo it and choose another action".into());
    }
    world
        .run_system_once(clear_hole_items)
        .expect("placement preview cleanup failed");
    world
        .run_system_once(handle_selection)
        .expect("placement preview system failed");
    if world.resource::<TurnState>().active == turn.active {
        let feedback = world
            .query_filtered::<&Text, With<DropFeedback>>()
            .single(&world)
            .expect("placement preview feedback missing");
        Some(
            feedback
                .0
                .split(" - first item resolved")
                .next()
                .unwrap_or(&feedback.0)
                .to_owned(),
        )
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Movement {
    Walk,
    Jump,
}

#[derive(Debug, PartialEq, Eq)]
enum MovementError {
    OutOfRange,
    Blocked,
    Occupied,
    InsufficientEnergy,
    UnsafeHover,
    RidgeBlocksJump,
}

fn movement_target(
    board: &Board,
    target: &DropTarget,
    opponent: Vec2,
    energy: u8,
    hover: bool,
    magic: u8,
) -> Result<Movement, MovementError> {
    let movement = match target.distance {
        1 => Movement::Walk,
        2 => Movement::Jump,
        _ => return Err(MovementError::OutOfRange),
    };
    if board_cell(opponent) == Some(target.cell) {
        return Err(MovementError::Occupied);
    }
    if !board.ground(target.cell).is_some_and(Ground::walkable) {
        if !hover {
            return Err(MovementError::Blocked);
        }
        let safe_exits = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |y| IVec2::new(x, y)))
            .filter(|delta| *delta != IVec2::ZERO)
            .map(|delta| target.cell + delta)
            .filter(|cell| {
                board.ground(*cell).is_some_and(Ground::walkable)
                    && board_cell(opponent) != Some(*cell)
            })
            .count();
        if magic < 2 || safe_exits < 2 {
            return Err(MovementError::UnsafeHover);
        }
    }
    if movement == Movement::Jump && energy == 0 {
        return Err(MovementError::InsufficientEnergy);
    }
    if movement == Movement::Jump && !hover {
        let delta = target.cell - target.origin;
        let halfway = target.origin + delta / 2;
        let other_halfway = target.cell - delta / 2;
        if [halfway, other_halfway]
            .into_iter()
            .any(|cell| board.ground(cell) == Some(Ground::Ridge))
        {
            return Err(MovementError::RidgeBlocksJump);
        }
    }

    Ok(movement)
}

fn must_land(turn: &TurnState, board: &Board, position: Vec2) -> bool {
    let index = turn.active.index();
    !turn.is_frozen(turn.active)
        && turn.hovering[index]
        && turn.magic[index] == 1
        && board_cell(position)
            .is_some_and(|cell| !board.ground(cell).is_some_and(Ground::walkable))
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "VolaTile".into(),
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: if cfg!(target_family = "wasm") {
                        AssetMetaCheck::Never
                    } else {
                        AssetMetaCheck::default()
                    },
                    ..default()
                }),
        )
        .add_plugins(AspectRatioPlugin {
            resolution: Resolution {
                width: RESOLUTION_WIDTH,
                height: RESOLUTION_HEIGHT,
            },
            ..default()
        })
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .init_resource::<BoardCursor>()
        .init_resource::<Selection>()
        .init_resource::<InventoryHover>()
        .init_resource::<TurnState>()
        .init_resource::<PlayerHealth>()
        .init_resource::<Weather>()
        .init_resource::<ai::BotController>()
        .init_resource::<effects::Effects>()
        .add_plugins(help::plugin)
        .add_plugins(ground_art::plugin)
        .add_plugins(history::plugin)
        .add_plugins(menu::plugin)
        .add_plugins(audio_settings::plugin)
        .add_plugins(sound::plugin)
        .add_plugins(music::plugin)
        .add_systems(
            Startup,
            (
                setup,
                effects::load,
                spawn_board_grid,
                (initialize_wind, initialize_weather).chain(),
            ),
        )
        .add_systems(
            Startup,
            (
                generate_board,
                spawn_board_terrain,
                spawn_initial_collectibles.after(setup),
            )
                .chain(),
        )
        .add_systems(Update, reset_match.before(menu::GameplaySystems))
        .add_systems(
            Update,
            (
                (
                    effects::advance,
                    advance_item_flight,
                    begin_turn_frame,
                    effects::capture,
                    ai::plan,
                    advance_serpent_animation,
                    advance_aquatic_animation,
                    rest,
                    effects::capture,
                )
                    .chain(),
                update_board_cursor,
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
                (
                    finalize_undo,
                    clear_hole_items,
                    update_serpent_visuals,
                    update_collectible_positions,
                )
                    .chain(),
                advance_wind,
                respawn_ingredients,
                highlight_inventory_hover,
                queue_weather,
                advance_weather,
                (
                    effects::capture,
                    sound::health_changes,
                    sound::aquatic_growth,
                )
                    .chain(),
                update_cell_feedback,
                (
                    update_inventory_counts,
                    update_action_slots,
                    update_turn_progress,
                    update_wind_status,
                    update_energy_bars,
                    update_magic_bars,
                    update_private_panels,
                    animate_players,
                    update_pickup_highlight,
                    update_hearts,
                )
                    .chain(),
                fit_status_text,
                update_electric_flashes,
                update_reaction_flashes,
                update_frozen_tiles,
                update_ground_tiles,
                (
                    update_water_effects,
                    update_poison_visuals,
                    update_acid_visuals,
                    update_void_growth_visuals,
                    update_life_essence_visuals,
                    update_thorn_visuals,
                    update_aquatic_visuals,
                )
                    .chain(),
                shake_ground,
            )
                .chain()
                .in_set(menu::GameplaySystems)
                .run_if(menu::in_game)
                .run_if(help::closed),
        )
        .run();
}

#[derive(SystemParam)]
struct MatchReset<'w, 's> {
    frontend: ResMut<'w, menu::Frontend>,
    turn: ResMut<'w, TurnState>,
    board: ResMut<'w, Board>,
    health: ResMut<'w, PlayerHealth>,
    weather: ResMut<'w, Weather>,
    bot: ResMut<'w, ai::BotController>,
    help: ResMut<'w, help::Help>,
    history: ResMut<'w, history::ComputerHistory>,
    selection: ResMut<'w, Selection>,
    hover: ResMut<'w, InventoryHover>,
    cursor: ResMut<'w, BoardCursor>,
    effects: Option<ResMut<'w, effects::Effects>>,
    sounds: Option<ResMut<'w, sound::SoundCues>>,
    images: Res<'w, InventoryImages>,
    rng: Single<'w, 's, &'static mut WyRand, With<GlobalRng>>,
    players: Query<'w, 's, (&'static Player, &'static mut Transform)>,
    temporary: Query<'w, 's, Entity, TemporarySprites>,
}

type TemporarySprites = (With<Sprite>, Without<PersistentWorld>, Without<ChildOf>);

fn reset_match(mut commands: Commands, mut state: MatchReset) {
    if !state.frontend.take_restart_request() {
        return;
    }
    state.help.reset(&mut commands);
    if let Some(effects) = &mut state.effects {
        effects.reset();
    }
    if let Some(sounds) = &mut state.sounds {
        sounds.reset_match();
    }
    if let Some(SelectedAction::Item { icon, .. }) = state.selection.0.take() {
        commands.entity(icon).despawn();
    }
    for entity in &state.temporary {
        commands.entity(entity).despawn();
    }
    for (player, mut transform) in &mut state.players {
        let start = cell_center(PLAYER_STARTS[player.0.index()]);
        transform.translation.x = start.x;
        transform.translation.y = start.y;
    }
    *state.turn = TurnState::default();
    *state.health = PlayerHealth::default();
    *state.bot = ai::BotController::default();
    *state.history = history::ComputerHistory::default();
    *state.cursor = BoardCursor::default();
    state.hover.hovered = Some((PlayerId::One, usize::MAX));
    state.hover.selected = None;
    *state.board = Board::generate(state.rng.as_mut());
    state.turn.wind = WindDirection::random(state.rng.as_mut());
    state.weather.rain = state.rng.random_range(RAIN_INTERVAL);
    state.weather.earthquake = state.rng.random_range(EARTHQUAKE_INTERVAL);
    let mut candidates = ingredient_spawn_candidates(&state.board, &PLAYER_STARTS, None);
    for index in [SUNSTONE_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX] {
        assert!(
            !candidates.is_empty(),
            "not enough space for initial ingredients"
        );
        let cell = candidates.swap_remove(state.rng.random_range(0..candidates.len()));
        spawn_collectible(&mut commands, &state.images, index, cell);
        state.turn.spawned_ingredients[index] += 1;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, hud: Res<Hud>) {
    commands.spawn((
        Camera2d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: RESOLUTION_WIDTH,
                min_height: RESOLUTION_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    commands.entity(hud.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    top: Val::Px(8.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|parent| {
                parent.spawn((
                    WindStatus,
                    Text::new("Wind: N"),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.1, 0.07, 0.04)),
                ));
            });
        for cell_info in [false, true] {
            parent
                .spawn((
                    Node {
                        left: Val::Px(3.0 * GRID_BLOCK_SIZE),
                        width: Val::Px(BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE),
                        height: Val::Px(36.0),
                        bottom: Val::Px(21.0),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|parent| {
                    let mut text = parent.spawn((
                        Text::new(if cell_info {
                            ""
                        } else {
                            "Click Move or an item to select it"
                        }),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextLayout::justify(Justify::Center),
                        TextColor(Color::srgb(0.1, 0.07, 0.04)),
                        Node {
                            width: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        FitStatusText {
                            initial_size: 16.0,
                            bounds: Vec2::new(BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE, 36.0),
                            last_text: String::new(),
                        },
                        Pickable::IGNORE,
                    ));
                    if cell_info {
                        text.insert((CellFeedback, Visibility::Hidden));
                    } else {
                        text.insert(DropFeedback);
                    }
                });
        }
        parent
            .spawn((
                Node {
                    left: Val::Px(3.0 * GRID_BLOCK_SIZE),
                    width: Val::Px(BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE),
                    height: Val::Px(18.0),
                    bottom: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|parent| {
                parent.spawn((
                    TurnProgress,
                    Text::new("ACTIONS: 2 AVAILABLE | REST: FULL"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextLayout::no_wrap().with_justify(Justify::Center),
                    TextColor(Color::srgb(0.1, 0.07, 0.04)),
                    FitStatusText {
                        initial_size: 14.0,
                        bounds: Vec2::new(BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE, 18.0),
                        last_text: String::new(),
                    },
                    Pickable::IGNORE,
                ));
            });
    });

    let heart_sheet = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("hearts-Sheet.png");
    spawn_health_displays(&mut commands, hud.0, heart_sheet);
    let item_images = InventoryImages(
        asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load("inventory-Sheet.png"),
    );
    spawn_inventories(
        &mut commands,
        hud.0,
        &item_images,
        asset_server.load("infinity.png"),
    );
    let action_sheet = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("actions-Sheet.png");
    spawn_actions(&mut commands, hud.0, action_sheet);
    commands.insert_resource(item_images);

    commands.spawn((
        PersistentWorld,
        Sprite {
            image: asset_server.load("backgroundboard.png"),
            custom_size: Some(Vec2::new(RESOLUTION_WIDTH, RESOLUTION_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -2.0),
    ));

    let player_sheet = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("players-animated-Sheet.png");
    spawn_players(&mut commands, player_sheet);
    commands.insert_resource(FrozenSheet(
        asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load("frozen-Sheet.png"),
    ));
    let serpent_sheet = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("quezacuatl-Sheet.png");
    commands.insert_resource(SerpentSheet(serpent_sheet));
    let aquatic_sheet = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("aquatic2-Sheet.png");
    commands.insert_resource(AquaticSheet(aquatic_sheet));
}

fn spawn_players(commands: &mut Commands, sheet: Handle<Image>) {
    for (owner, start) in [PlayerId::One, PlayerId::Two]
        .into_iter()
        .zip(PLAYER_STARTS)
    {
        commands.spawn((
            PersistentWorld,
            Sprite {
                image: sheet.clone(),
                rect: Some(player_frame_rect(owner, false, 0)),
                custom_size: Some(Vec2::splat(PLAYER_FRAME_SIZE * 1.5)),
                ..default()
            },
            Transform::from_translation(cell_center(start).extend(0.0)),
            Player(owner),
            PlayerAnimation::default(),
        ));
    }
}

fn player_frame_rect(owner: PlayerId, hovering: bool, frame: usize) -> Rect {
    let x = (usize::from(hovering) * 2 + frame) as f32 * PLAYER_FRAME_SIZE;
    let y = owner.index() as f32 * PLAYER_FRAME_SIZE;
    Rect::new(x, y, x + PLAYER_FRAME_SIZE, y + PLAYER_FRAME_SIZE)
}

fn animate_players(
    time: Res<Time>,
    turn: Res<TurnState>,
    mut players: Query<(&Player, &mut PlayerAnimation, &mut Sprite)>,
) {
    for (player, mut animation, mut sprite) in &mut players {
        if turn.is_frozen(player.0) {
            continue;
        }
        let hovering = turn.hovering[player.0.index()];
        if animation.hovering != hovering {
            animation.hovering = hovering;
            animation.frame = 0;
            animation.timer.reset();
        } else {
            animation.timer.tick(time.delta());
            animation.frame =
                (animation.frame + animation.timer.times_finished_this_tick() as usize) % 2;
        }
        sprite.rect = Some(player_frame_rect(player.0, hovering, animation.frame));
    }
}

fn spawn_inventories(
    commands: &mut Commands,
    hud: Entity,
    images: &InventoryImages,
    infinity: Handle<Image>,
) {
    commands.entity(hud).with_children(|parent| {
        for is_left in [true, false] {
            let mut node = Node {
                position_type: PositionType::Absolute,
                top: Val::Px(GRID_BLOCK_SIZE),
                width: Val::Px(2.0 * GRID_BLOCK_SIZE),
                height: Val::Px(BOARD_ROWS as f32 * GRID_BLOCK_SIZE),
                ..default()
            };
            if is_left {
                node.left = Val::Px(0.0);
            } else {
                node.right = Val::Px(0.0);
            }
            parent
                .spawn((
                    Name::new(if is_left {
                        "Left inventory"
                    } else {
                        "Right inventory"
                    }),
                    InventoryPanel(if is_left {
                        PlayerId::One
                    } else {
                        PlayerId::Two
                    }),
                    if is_left {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    node,
                    Pickable::IGNORE,
                ))
                .with_children(|inventory| {
                    inventory.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(HEALTH_PANEL_INSET),
                            right: Val::Px(HEALTH_PANEL_INSET),
                            top: Val::Px(HEALTH_PANEL_INSET),
                            bottom: Val::Px(HEALTH_PANEL_INSET),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.06, 0.04, 0.03, 0.82)),
                        BorderColor::all(Color::srgb(0.86, 0.68, 0.32)),
                        Pickable::IGNORE,
                    ));
                    for (position, &index) in INVENTORY_ORDER.iter().enumerate() {
                        let (label, _) = INVENTORY_ITEMS[index];
                        let column = position % 2;
                        let row = position / 2;
                        let left_inset = if column == 0 { HEALTH_PANEL_INSET } else { 0.0 };
                        let top_inset = if row == 0 { HEALTH_PANEL_INSET } else { 0.0 };
                        let bottom_inset = if row == BOARD_ROWS as usize - 1 {
                            HEALTH_PANEL_INSET
                        } else {
                            0.0
                        };
                        inventory
                            .spawn((
                                Name::new(label),
                                InventorySlot {
                                    index,
                                    owner: if is_left {
                                        PlayerId::One
                                    } else {
                                        PlayerId::Two
                                    },
                                },
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(column as f32 * GRID_BLOCK_SIZE + left_inset),
                                    top: Val::Px(row as f32 * GRID_BLOCK_SIZE + top_inset),
                                    width: Val::Px(GRID_BLOCK_SIZE - HEALTH_PANEL_INSET),
                                    height: Val::Px(GRID_BLOCK_SIZE - top_inset - bottom_inset),
                                    border: UiRect::all(Val::Px(0.5)),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                BorderColor::all(Color::srgba(0.86, 0.68, 0.32, 0.4)),
                                Pickable::IGNORE,
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    images.icon(index),
                                    Node {
                                        width: Val::Px(25.0),
                                        height: Val::Px(25.0),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                slot.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: FontSize::Px(9.0),
                                        ..default()
                                    },
                                    TextLayout::justify(Justify::Center),
                                    Node {
                                        width: Val::Percent(100.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.94, 0.78)),
                                    Pickable::IGNORE,
                                ));
                                let count = InventoryCount {
                                    owner: if is_left {
                                        PlayerId::One
                                    } else {
                                        PlayerId::Two
                                    },
                                    index,
                                };
                                let mut node = Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(2.0),
                                    right: Val::Px(3.0),
                                    ..default()
                                };
                                if is_unlimited(index) {
                                    node.width = Val::Px(12.0);
                                    node.height = Val::Px(6.0);
                                    slot.spawn((
                                        count,
                                        node,
                                        ImageNode::new(infinity.clone()),
                                        Pickable::IGNORE,
                                    ));
                                } else {
                                    slot.spawn((
                                        count,
                                        Text::new("0"),
                                        TextFont {
                                            font_size: FontSize::Px(11.0),
                                            ..default()
                                        },
                                        node,
                                        TextColor(Color::WHITE),
                                        Pickable::IGNORE,
                                    ));
                                }
                            });
                    }
                });
        }
    });
}

fn spawn_actions(commands: &mut Commands, hud: Entity, sheet: Handle<Image>) {
    commands.entity(hud).with_children(|parent| {
        for owner in [PlayerId::One, PlayerId::Two] {
            let left = match owner {
                PlayerId::One => 2.0 * GRID_BLOCK_SIZE,
                PlayerId::Two => RESOLUTION_WIDTH - 3.0 * GRID_BLOCK_SIZE,
            };
            for (row, (action, label)) in [
                (Action::Move, "Move"),
                (Action::Hover, "Hover"),
                (Action::Pickup, "Pick up"),
                (Action::Rest, "Rest"),
                (Action::Undo, "Undo"),
            ]
            .into_iter()
            .enumerate()
            {
                parent
                    .spawn((
                        ActionSlot { owner, action },
                        if owner == PlayerId::One {
                            Visibility::Visible
                        } else {
                            Visibility::Hidden
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left + HEALTH_PANEL_INSET),
                            top: Val::Px((row + 1) as f32 * GRID_BLOCK_SIZE + HEALTH_PANEL_INSET),
                            width: Val::Px(GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                            height: Val::Px(GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                            border: UiRect::all(Val::Px(2.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.06, 0.04, 0.03, 0.82)),
                        BorderColor::all(Color::srgb(0.86, 0.68, 0.32)),
                        Pickable::IGNORE,
                    ))
                    .with_children(|slot| {
                        if let Some(rect) = action.icon_rect() {
                            slot.spawn((
                                ImageNode {
                                    image: sheet.clone(),
                                    rect: Some(rect),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(26.0),
                                    height: Val::Px(26.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        }
                        slot.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(9.0),
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.94, 0.78)),
                            Pickable::IGNORE,
                        ));
                    });
            }
        }
    });
}

fn update_private_panels(
    turn: Res<TurnState>,
    bot: Option<Res<ai::BotController>>,
    mut panels: Query<(&InventoryPanel, &mut Visibility), Without<ActionSlot>>,
    mut actions: Query<(&ActionSlot, &mut Visibility), Without<InventoryPanel>>,
) {
    let visible_player = if bot.as_ref().is_some_and(|bot| bot.computer_mode) {
        PlayerId::One
    } else {
        turn.active
    };
    for (panel, mut visibility) in &mut panels {
        *visibility = if panel.0 == visible_player {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (action, mut visibility) in &mut actions {
        *visibility = if action.owner == visible_player {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_inventory_counts(turn: Res<TurnState>, mut counts: Query<(&InventoryCount, &mut Text)>) {
    for (count, mut text) in &mut counts {
        text.0 = turn.inventory[count.owner.index()][count.index].to_string();
    }
}

fn available_pickup(
    turn: &TurnState,
    board: &Board,
    position: Vec2,
    items: &Query<&Collectible>,
) -> bool {
    let Some(cell) = board_cell(position) else {
        return false;
    };
    turn.actions_used < 2
        && !turn.animation_active()
        && !must_land(turn, board, position)
        && items
            .iter()
            .any(|item| item.cell == cell && !is_unlimited(item.index))
}

fn update_action_slots(
    turn: Res<TurnState>,
    board: Res<Board>,
    cursor: Res<BoardCursor>,
    selection: Res<Selection>,
    players: Query<(&Player, &Transform)>,
    items: Query<&Collectible>,
    mut slots: Query<(&ActionSlot, &mut BackgroundColor, &mut BorderColor)>,
) {
    let active_position = players.iter().find_map(|(player, transform)| {
        (player.0 == turn.active).then_some(transform.translation.truncate())
    });
    for (slot, mut background, mut border) in &mut slots {
        let is_active = slot.owner == turn.active;
        let pickup_available = is_active
            && slot.action == Action::Pickup
            && active_position
                .is_some_and(|position| available_pickup(&turn, &board, position, &items));
        let undo_available = is_active
            && !turn.animation_active()
            && slot.action == Action::Undo
            && turn.undo.is_some();
        let hover_on = slot.action == Action::Hover && turn.hovering[slot.owner.index()];
        let move_selected = is_active
            && slot.action == Action::Move
            && matches!(selection.0, Some(SelectedAction::Movement { .. }));
        let hovered = is_active
            && cursor
                .0
                .and_then(|position| action_cell(position, slot.owner))
                == Some(slot.action.row());
        background.0 = if pickup_available || undo_available {
            Color::srgba(0.15, 0.55, 0.25, 0.9)
        } else if hover_on {
            Color::srgba(0.12, 0.35, 0.5, 0.9)
        } else if hovered || move_selected {
            Color::srgba(0.45, 0.34, 0.16, 0.9)
        } else {
            Color::srgba(0.06, 0.04, 0.03, 0.82)
        };
        *border = BorderColor::all(if pickup_available || undo_available {
            Color::srgb(0.4, 1.0, 0.5)
        } else {
            Color::srgb(0.86, 0.68, 0.32)
        });
    }
}

fn spawn_health_displays(commands: &mut Commands, hud: Entity, heart: Handle<Image>) {
    commands.entity(hud).with_children(|parent| {
        for is_left in [true, false] {
            let mut node = Node {
                position_type: PositionType::Absolute,
                top: Val::Px(HEALTH_PANEL_INSET),
                width: Val::Px(HEALTH_PANEL_WIDTH),
                height: Val::Px(HEALTH_PANEL_HEIGHT),
                border: UiRect::all(Val::Px(HEALTH_PANEL_BORDER)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            };
            if is_left {
                node.left = Val::Px(HEALTH_PANEL_INSET);
            } else {
                node.right = Val::Px(HEALTH_PANEL_INSET);
            }
            parent
                .spawn((
                    Name::new(if is_left {
                        "Left health"
                    } else {
                        "Right health"
                    }),
                    HealthPanel,
                    node,
                    BackgroundColor(Color::srgba(0.06, 0.04, 0.03, 0.82)),
                    BorderColor::all(Color::srgb(0.86, 0.68, 0.32)),
                    Pickable::IGNORE,
                ))
                .with_children(|panel| {
                    panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(28.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceEvenly,
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|row| {
                            for index in 0..HEART_COUNT {
                                row.spawn((
                                    Heart {
                                        owner: if is_left {
                                            PlayerId::One
                                        } else {
                                            PlayerId::Two
                                        },
                                        index,
                                    },
                                    ImageNode {
                                        image: heart.clone(),
                                        rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                            }
                        });
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(
                                    HEALTH_PANEL_WIDTH
                                        - 2.0 * (HEALTH_PANEL_BORDER + ENERGY_BAR_SIDE_PADDING),
                                ),
                                height: Val::Px(5.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.08, 0.18, 0.09)),
                            Pickable::IGNORE,
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                EnergyFill(if is_left {
                                    PlayerId::One
                                } else {
                                    PlayerId::Two
                                }),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.22, 0.85, 0.3)),
                                Pickable::IGNORE,
                            ));
                        });
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(
                                    HEALTH_PANEL_WIDTH
                                        - 2.0 * (HEALTH_PANEL_BORDER + ENERGY_BAR_SIDE_PADDING),
                                ),
                                height: Val::Px(5.0),
                                margin: UiRect::top(Val::Px(1.0)),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.06, 0.12, 0.24)),
                            Pickable::IGNORE,
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                MagicFill(if is_left {
                                    PlayerId::One
                                } else {
                                    PlayerId::Two
                                }),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.18, 0.55, 1.0)),
                                Pickable::IGNORE,
                            ));
                        });
                });
        }
    });
}

fn generate_board(mut commands: Commands, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    commands.insert_resource(Board::generate(rng.as_mut()));
}

fn spawn_board_terrain(mut commands: Commands, board: Res<Board>) {
    for y in 0..BOARD_ROWS {
        for x in 0..BOARD_COLUMNS {
            let cell = IVec2::new(x as i32, y as i32);
            let center = cell_center(cell);
            commands.spawn((
                GroundTile(cell),
                PersistentWorld,
                Sprite {
                    color: board
                        .ground(cell)
                        .expect("board cell must be in bounds")
                        .color(),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, -1.5),
            ));
        }
    }
}

fn spawn_board_grid(mut commands: Commands) {
    let width = BOARD_RIGHT - BOARD_LEFT;
    let height = BOARD_TOP - BOARD_BOTTOM;
    for column in 0..=BOARD_COLUMNS {
        let x = (BOARD_LEFT + column as f32 * GRID_BLOCK_SIZE).clamp(
            BOARD_LEFT + GRID_LINE_WIDTH / 2.0,
            BOARD_RIGHT - GRID_LINE_WIDTH / 2.0,
        );
        commands.spawn((
            PersistentWorld,
            Sprite {
                color: Color::srgba(0.25, 0.25, 0.25, 0.5),
                custom_size: Some(Vec2::new(GRID_LINE_WIDTH, height)),
                ..default()
            },
            Transform::from_xyz(x, (BOARD_TOP + BOARD_BOTTOM) / 2.0, -1.0),
        ));
    }

    for row in 0..=BOARD_ROWS {
        let y = (BOARD_BOTTOM + row as f32 * GRID_BLOCK_SIZE).clamp(
            BOARD_BOTTOM + GRID_LINE_WIDTH / 2.0,
            BOARD_TOP - GRID_LINE_WIDTH / 2.0,
        );
        commands.spawn((
            PersistentWorld,
            Sprite {
                color: Color::srgba(0.25, 0.25, 0.25, 0.5),
                custom_size: Some(Vec2::new(width, GRID_LINE_WIDTH)),
                ..default()
            },
            Transform::from_xyz((BOARD_LEFT + BOARD_RIGHT) / 2.0, y, -1.0),
        ));
    }
}

fn board_cell(position: Vec2) -> Option<IVec2> {
    if !position.is_finite()
        || !(BOARD_LEFT..BOARD_RIGHT).contains(&position.x)
        || position.y > BOARD_TOP
        || position.y <= BOARD_BOTTOM
    {
        return None;
    }
    Some(IVec2::new(
        ((position.x - BOARD_LEFT) / GRID_BLOCK_SIZE).floor() as i32,
        ((BOARD_TOP - position.y) / GRID_BLOCK_SIZE).floor() as i32,
    ))
}

fn cell_center(cell: IVec2) -> Vec2 {
    Vec2::new(
        BOARD_LEFT + (cell.x as f32 + 0.5) * GRID_BLOCK_SIZE,
        BOARD_TOP - (cell.y as f32 + 0.5) * GRID_BLOCK_SIZE,
    )
}

fn terrain_cross(board: &Board, center: IVec2) -> Vec<IVec2> {
    [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y]
        .into_iter()
        .map(|offset| center + offset)
        .filter(|&cell| board.ground(cell).is_some())
        .collect()
}

fn steam_cells(board: &Board, source: IVec2, wind: WindDirection) -> Vec<IVec2> {
    let direction = wind.offset();
    let perpendicular = IVec2::new(-direction.y, direction.x);
    let mut cells = Vec::new();
    for y in -1..=1 {
        for x in -1..=1 {
            let cell = source + IVec2::new(x, y);
            if board
                .ground(cell)
                .is_some_and(|ground| ground != Ground::Ridge)
            {
                cells.push(cell);
            }
        }
    }
    let leading = if direction.x != 0 && direction.y != 0 {
        [
            direction,
            IVec2::new(direction.x, 0),
            IVec2::new(0, direction.y),
        ]
    } else {
        [
            direction,
            direction + perpendicular,
            direction - perpendicular,
        ]
    };
    for offset in leading {
        let edge = source + offset;
        if !board
            .ground(edge)
            .is_some_and(|ground| ground != Ground::Ridge)
        {
            continue;
        }
        let downwind = edge + direction;
        let destinations = if board.ground(downwind) == Some(Ground::Ridge) {
            [Some(edge + perpendicular), Some(edge - perpendicular)]
        } else {
            [Some(downwind), None]
        };
        for cell in destinations.into_iter().flatten() {
            if board
                .ground(cell)
                .is_some_and(|ground| ground != Ground::Ridge)
                && !cells.contains(&cell)
            {
                cells.push(cell);
            }
        }
    }
    cells
}

#[derive(Clone, Copy)]
enum BlastKind {
    Oil,
    Dust,
    Diagonal,
    Cross,
}

struct Blast {
    source: IVec2,
    kind: BlastKind,
}

impl Blast {
    fn cells(&self, board: &Board) -> Vec<IVec2> {
        let mut cells = vec![self.source];
        match self.kind {
            BlastKind::Oil | BlastKind::Dust => {
                for x in -1..=1 {
                    for y in -1..=1 {
                        if x != 0 || y != 0 {
                            cells.push(self.source + IVec2::new(x, y));
                        }
                    }
                }
            }
            BlastKind::Diagonal | BlastKind::Cross => {
                let directions = match self.kind {
                    BlastKind::Diagonal => [
                        IVec2::new(1, 1),
                        IVec2::new(1, -1),
                        IVec2::new(-1, 1),
                        IVec2::new(-1, -1),
                    ],
                    _ => [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y],
                };
                let source_on_ridge = board.ground(self.source) == Some(Ground::Ridge);
                for direction in directions {
                    for distance in 1..=2 {
                        let cell = self.source + direction * distance;
                        let Some(ground) = board.ground(cell) else {
                            break;
                        };
                        if !source_on_ridge && ground == Ground::Ridge {
                            break;
                        }
                        cells.push(cell);
                    }
                }
            }
        }
        cells.retain(|cell| board.ground(*cell).is_some());
        cells
    }
}

struct BlastResult {
    blasts: Vec<Blast>,
    consumed_obsidian: Vec<Option<Entity>>,
    refreshed_water: Vec<IVec2>,
}

struct ObsidianIgnition {
    index: usize,
    diagonal: bool,
    cross: bool,
}

fn resolve_blasts(
    board: &mut Board,
    obsidians: &[(Option<Entity>, IVec2)],
    initial_obsidian: &[ObsidianIgnition],
    initial_oil: Option<IVec2>,
    initial_dust: Option<IVec2>,
) -> BlastResult {
    let mut queue = std::collections::VecDeque::new();
    let mut triggered = vec![false; obsidians.len()];
    let mut consumed_obsidian = Vec::new();
    if let Some(source) = initial_dust {
        queue.push_back(Blast {
            source,
            kind: BlastKind::Dust,
        });
    }
    for ignition in initial_obsidian {
        let index = ignition.index;
        if !triggered[index] {
            triggered[index] = true;
            consumed_obsidian.push(obsidians[index].0);
        }
        if ignition.diagonal {
            queue.push_back(Blast {
                source: obsidians[index].1,
                kind: BlastKind::Diagonal,
            });
        }
        if ignition.cross {
            queue.push_back(Blast {
                source: obsidians[index].1,
                kind: BlastKind::Cross,
            });
        }
    }
    if let Some(source) = initial_oil {
        queue.extend(board.connected_oil(source).into_iter().map(|source| Blast {
            source,
            kind: BlastKind::Oil,
        }));
    }
    let mut blasts = Vec::new();
    let mut refreshed_water = Vec::new();
    let mut thawed = Vec::new();
    while let Some(blast) = queue.pop_front() {
        if matches!(blast.kind, BlastKind::Dust) {
            for cell in blast.cells(board) {
                if board.ground(cell) == Some(Ground::Ridge) {
                    board.set(cell, Ground::Solid);
                }
            }
        }
        if matches!(blast.kind, BlastKind::Oil) {
            if !board.has_oil(blast.source) {
                continue;
            }
            board.remove_oil(blast.source);
            if board.is_frozen(blast.source) {
                board.thaw(blast.source);
                thawed.push(blast.source);
                if board.is_wet(blast.source) {
                    refreshed_water.push(blast.source);
                }
            } else if board.is_water(blast.source) && !thawed.contains(&blast.source) {
                board.burn_water(blast.source);
            } else if board.ground(blast.source) == Some(Ground::Solid)
                && !thawed.contains(&blast.source)
            {
                board.dry(blast.source);
            }
        }
        for cell in blast.cells(board) {
            if board.is_frozen(cell) {
                board.thaw(cell);
                thawed.push(cell);
                if board.is_wet(cell) {
                    refreshed_water.push(cell);
                }
            }
            if board.has_oil(cell) {
                queue.push_back(Blast {
                    source: cell,
                    kind: BlastKind::Oil,
                });
            }
            for (index, &(entity, position)) in obsidians.iter().enumerate() {
                if position == cell && !triggered[index] {
                    triggered[index] = true;
                    consumed_obsidian.push(entity);
                    queue.push_back(Blast {
                        source: position,
                        kind: BlastKind::Cross,
                    });
                }
            }
        }
        blasts.push(blast);
    }
    BlastResult {
        blasts,
        consumed_obsidian,
        refreshed_water,
    }
}

fn drop_target(player: Vec2, position: Vec2) -> Option<DropTarget> {
    let player_cell = board_cell(player)?;
    let cell = board_cell(position)?;
    let delta = (cell - player_cell).abs();
    let distance = delta.x.max(delta.y) as u32;
    let kind = match distance {
        0 => return None,
        1 => DropKind::Place,
        _ => DropKind::Distant,
    };
    Some(DropTarget {
        origin: player_cell,
        cell,
        distance,
        kind,
    })
}

fn item_target(player: Vec2, position: Vec2) -> Option<DropTarget> {
    let origin = board_cell(player)?;
    let cell = board_cell(position)?;
    if cell == origin {
        Some(DropTarget {
            origin,
            cell,
            distance: 0,
            kind: DropKind::Place,
        })
    } else {
        drop_target(player, position)
    }
}

fn inventory_item(position: Vec2, owner: PlayerId) -> Option<usize> {
    let left = match owner {
        PlayerId::One => -RESOLUTION_WIDTH / 2.0,
        PlayerId::Two => RESOLUTION_WIDTH / 2.0 - 2.0 * GRID_BLOCK_SIZE,
    };
    let x = position.x - left;
    let y = BOARD_TOP - position.y;
    if !position.is_finite()
        || !(0.0..2.0 * GRID_BLOCK_SIZE).contains(&x)
        || !(0.0..BOARD_ROWS as f32 * GRID_BLOCK_SIZE).contains(&y)
    {
        return None;
    }
    let column = (x / GRID_BLOCK_SIZE).floor() as usize;
    let row = (y / GRID_BLOCK_SIZE).floor() as usize;
    Some(INVENTORY_ORDER[row * 2 + column])
}

fn action_cell(position: Vec2, owner: PlayerId) -> Option<usize> {
    let left = match owner {
        PlayerId::One => BOARD_LEFT - GRID_BLOCK_SIZE,
        PlayerId::Two => BOARD_RIGHT,
    };
    let x = position.x - left;
    let y = BOARD_TOP - position.y;
    if !position.is_finite()
        || !(HEALTH_PANEL_INSET..GRID_BLOCK_SIZE - HEALTH_PANEL_INSET).contains(&x)
        || !(0.0..5.0 * GRID_BLOCK_SIZE).contains(&y)
        || !(HEALTH_PANEL_INSET..GRID_BLOCK_SIZE - HEALTH_PANEL_INSET)
            .contains(&(y % GRID_BLOCK_SIZE))
    {
        return None;
    }
    Some((y / GRID_BLOCK_SIZE) as usize)
}

fn undo_action(
    mut commands: Commands,
    input: (
        Res<ButtonInput<MouseButton>>,
        Res<BoardCursor>,
        Option<Res<ai::BotController>>,
    ),
    mut turn: ResMut<TurnState>,
    board: Res<Board>,
    mut selection: ResMut<Selection>,
    mut players: Query<(&Player, &mut Transform)>,
    output: (
        Query<&mut Text, With<DropFeedback>>,
        Option<ResMut<sound::SoundCues>>,
    ),
) {
    let (mouse, cursor, bot) = input;
    let (mut feedback, mut sounds) = output;
    if bot.as_ref().is_some_and(|bot| bot.bot_turn)
        || !mouse.just_pressed(MouseButton::Left)
        || turn.acted_this_frame
        || turn.animation_active()
        || cursor
            .0
            .and_then(|position| action_cell(position, turn.active))
            != Some(Action::Undo.row())
    {
        return;
    }
    let Ok(mut feedback) = feedback.single_mut() else {
        return;
    };
    let Some(undo) = turn.undo.take() else {
        feedback.0 = "No action available to undo".into();
        sound::request(&mut sounds, sound::Cue::Reject);
        return;
    };
    clear_selection(&mut commands, &mut selection);
    let index = turn.active.index();
    match undo {
        UndoAction::Movement {
            origin,
            strength,
            hovering,
            moved_while_hovering,
            triggered_frost,
            previous_pending_freeze,
        } => {
            let (_, mut transform) = players
                .iter_mut()
                .find(|(player, _)| player.0 == turn.active)
                .expect("active player must exist");
            transform.translation.x = origin.x;
            transform.translation.y = origin.y;
            turn.strength[index] = strength;
            turn.moved_while_hovering = moved_while_hovering;
            turn.pending_effects.frozen_players[index] = previous_pending_freeze;
            for item in triggered_frost {
                let mut entity = commands.entity(item.entity);
                entity.remove::<PickedUpForUndo>();
                entity.insert(Collectible {
                    index: item.index,
                    cell: item.cell,
                });
                if let Some(visibility) = item.visibility {
                    entity.insert(visibility);
                } else {
                    entity.remove::<Visibility>();
                }
            }
            turn.aquatic_contacts.clear();
            if board_cell(origin)
                .is_some_and(|cell| !board.ground(cell).is_some_and(Ground::walkable))
            {
                turn.hovering[index] = hovering;
            }
        }
        UndoAction::Placement { strength } => {
            let pending = turn
                .pending_placement
                .take()
                .expect("queued placement exists");
            commands.entity(pending.marker).despawn();
            turn.strength[index] = strength;
            turn.refund_ingredient(pending.index);
        }
        UndoAction::Pickup { items } => {
            for item in items {
                turn.inventory[index][item.index] -= 1;
                let mut entity = commands.entity(item.entity);
                entity.remove::<PickedUpForUndo>();
                entity.insert(Collectible {
                    index: item.index,
                    cell: item.cell,
                });
                if let Some(visibility) = item.visibility {
                    entity.insert(visibility);
                } else {
                    entity.remove::<Visibility>();
                }
            }
        }
    }
    turn.actions_used = 0;
    turn.free_action = false;
    turn.terrain_rejection = None;
    feedback.0 = "Action undone - choose an action".into();
    sound::request(&mut sounds, sound::Cue::Pop);
}

fn finalize_undo(mut commands: Commands, mut turn: ResMut<TurnState>) {
    if turn.acted_this_frame
        && let Some(undo) = turn.undo.take()
    {
        match undo {
            UndoAction::Pickup { items } => {
                for item in items {
                    commands.entity(item.entity).despawn();
                }
            }
            UndoAction::Movement {
                triggered_frost, ..
            } => {
                for item in triggered_frost {
                    commands.entity(item.entity).despawn();
                    turn.ingredient_respawns
                        .push((FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
            }
            UndoAction::Placement { .. } => {}
        }
    }
}

#[derive(SystemParam)]
struct ActionOutput<'w, 's> {
    feedback: Query<'w, 's, &'static mut Text, With<DropFeedback>>,
    history: Option<ResMut<'w, history::ComputerHistory>>,
    sounds: Option<ResMut<'w, sound::SoundCues>>,
}

fn handle_actions(
    mut commands: Commands,
    input: (
        Res<ButtonInput<MouseButton>>,
        Res<BoardCursor>,
        Option<Res<ai::BotController>>,
    ),
    game: (ResMut<TurnState>, ResMut<Board>, ResMut<PlayerHealth>),
    mut selection: ResMut<Selection>,
    players: Query<(&Player, &Transform)>,
    collectibles: Query<(Entity, &Collectible, Option<&Visibility>)>,
    output: ActionOutput,
) {
    let (mouse, cursor, bot) = input;
    let (mut turn, mut board, mut health) = game;
    let ActionOutput {
        mut feedback,
        mut history,
        mut sounds,
    } = output;
    if turn.acted_this_frame
        || turn.animation_active()
        || turn.pending_placement.as_ref().is_some_and(|p| p.ready)
    {
        return;
    }
    let bot_turn = bot.as_ref().is_some_and(|bot| bot.bot_turn);
    let action = if bot_turn {
        match bot.as_ref().and_then(|bot| bot.action) {
            Some(ai::BotAction::Pickup) => Some(Action::Pickup.row()),
            Some(ai::BotAction::Rest) => Some(Action::Rest.row()),
            _ => None,
        }
    } else if mouse.just_pressed(MouseButton::Left) {
        cursor
            .0
            .and_then(|position| action_cell(position, turn.active))
    } else {
        None
    };
    let Some(action) = action else {
        return;
    };
    let Some(position) = players.iter().find_map(|(player, transform)| {
        (player.0 == turn.active).then_some(transform.translation.truncate())
    }) else {
        return;
    };
    let Ok(mut feedback) = feedback.single_mut() else {
        return;
    };
    let index = turn.active.index();
    if action == Action::Undo.row() {
        return;
    }
    if action == Action::Move.row() {
        let already_selected = matches!(selection.0, Some(SelectedAction::Movement { .. }));
        clear_selection(&mut commands, &mut selection);
        if already_selected {
            feedback.0 = "Selection cancelled".into();
        } else if turn.actions_used >= 2 {
            feedback.0 = "Both actions have been used".into();
            sound::request(&mut sounds, sound::Cue::Reject);
        } else if turn.is_frozen(turn.active) {
            feedback.0 = "Frozen in place - use an item, pick up, or rest".into();
            sound::request(&mut sounds, sound::Cue::Reject);
        } else if let Some(opponent) = players.iter().find_map(|(player, transform)| {
            (player.0 == turn.active.other()).then_some(transform.translation.truncate())
        }) {
            selection.0 = Some(select_movement(
                &mut commands,
                position,
                opponent,
                &board,
                turn.strength[index],
                turn.hovering[index],
                turn.magic[index],
            ));
            feedback.0 = "Green: walk | yellow: jump (1 energy) | click Move to cancel".into();
            sound::request(&mut sounds, sound::Cue::Select);
        }
        return;
    }
    if action == Action::Hover.row() {
        if turn.hovering[index] {
            if board_cell(position)
                .is_some_and(|cell| !board.ground(cell).is_some_and(Ground::walkable))
            {
                feedback.0 = "Land on safe ground before turning hover off".into();
                sound::request(&mut sounds, sound::Cue::Reject);
                return;
            }
            turn.hovering[index] = false;
        } else if turn.is_frozen(turn.active) {
            feedback.0 = "Frozen players cannot start hovering".into();
            sound::request(&mut sounds, sound::Cue::Reject);
            return;
        } else if turn.magic[index] > 0 {
            turn.hovering[index] = true;
        } else {
            feedback.0 = "No magic - complete a turn to recover".into();
            sound::request(&mut sounds, sound::Cue::Reject);
            return;
        }
        sound::request(&mut sounds, sound::Cue::Whoosh);
        if matches!(selection.0, Some(SelectedAction::Movement { .. }))
            && let Some(opponent) = players.iter().find_map(|(player, transform)| {
                (player.0 == turn.active.other()).then_some(transform.translation.truncate())
            })
        {
            clear_selection(&mut commands, &mut selection);
            selection.0 = Some(select_movement(
                &mut commands,
                position,
                opponent,
                &board,
                turn.strength[index],
                turn.hovering[index],
                turn.magic[index],
            ));
        }
        feedback.0 = if turn.hovering[index] {
            "Hover on - choose an action"
        } else {
            "Hover off - choose an action"
        }
        .into();
        return;
    }
    if action == Action::Rest.row() {
        match finish_rest(&mut turn, &mut board, &mut health, &players) {
            Ok(()) => {
                if bot_turn && let Some(history) = &mut history {
                    history.add("Computer rested".into(), None);
                }
                clear_selection(&mut commands, &mut selection);
                feedback.0 = "Rested - next player's turn".into();
                sound::request(&mut sounds, sound::Cue::Chime);
            }
            Err(message) => {
                feedback.0 = message.into();
                sound::request(&mut sounds, sound::Cue::Reject);
            }
        }
        return;
    }
    if turn.actions_used >= 2 {
        feedback.0 = "Both actions have been used".into();
        sound::request(&mut sounds, sound::Cue::Reject);
        return;
    }
    if must_land(&turn, &board, position) {
        feedback.0 = "Land on safe ground before magic runs out".into();
        sound::request(&mut sounds, sound::Cue::Reject);
        return;
    }
    let Some(cell) = board_cell(position) else {
        return;
    };
    let mut picked_up = Vec::new();
    let mut picked_up_indices = Vec::new();
    let mut undo_items = Vec::new();
    for (entity, item, visibility) in &collectibles {
        if item.cell != cell || is_unlimited(item.index) {
            continue;
        }
        turn.inventory[index][item.index] += 1;
        if turn.actions_used == 0 {
            undo_items.push(UndoPickup {
                entity,
                index: item.index,
                cell: item.cell,
                visibility: visibility.copied(),
            });
            commands
                .entity(entity)
                .remove::<Collectible>()
                .insert((PickedUpForUndo, Visibility::Hidden));
        } else {
            commands.entity(entity).despawn();
        }
        picked_up.push(INVENTORY_ITEMS[item.index].0);
        picked_up_indices.push(item.index);
    }
    if picked_up.is_empty() {
        feedback.0 = "No collectible ingredient on this tile".into();
        sound::request(&mut sounds, sound::Cue::Reject);
        return;
    }
    if bot_turn && let Some(history) = &mut history {
        history.add(
            format!(
                "Computer picked up {} at {}",
                picked_up.join(", "),
                history::cell_label(cell)
            ),
            (picked_up_indices.len() == 1).then_some(picked_up_indices[0]),
        );
    }
    clear_selection(&mut commands, &mut selection);
    if turn.actions_used == 0 {
        turn.undo = Some(UndoAction::Pickup { items: undo_items });
    }
    turn.spend_action_energy(0);
    turn.actions_used += 1;
    if let Some(pending) = &mut turn.pending_placement {
        pending.ready = true;
    } else if turn.actions_used == 2 {
        let positions = players
            .iter()
            .map(|(player, transform)| (player.0, transform.translation.truncate()));
        turn.finish_action(&mut board, &mut health, positions, TurnEffects::default());
    }
    feedback.0 = format!("Picked up {}", picked_up.join(", "));
    sound::request(&mut sounds, sound::Cue::Pop);
}

fn apply_terrain_changes(
    mut turn: ResMut<TurnState>,
    mut players: Query<(&Player, &mut Transform)>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    for (owner, cell) in turn.terrain_displacements.drain(..) {
        if let Some((_, mut transform)) = players.iter_mut().find(|(player, _)| player.0 == owner) {
            let center = cell_center(cell);
            transform.translation.x = center.x;
            transform.translation.y = center.y;
        } else {
            error!("Cannot move displaced player: player entity is missing");
        }
    }
    if std::mem::take(&mut turn.terrain_limit_hit)
        && let Ok(mut feedback) = feedback.single_mut()
    {
        feedback.0 = "Terrain growth blocked by the 60% ridge and hole limit".into();
    }
}

type WaitingFrostFilter = (With<PlacedItem>, Without<PickedUpForUndo>);
type WaitingFrost = (
    Entity,
    &'static PendingFrost,
    &'static Collectible,
    Option<&'static Visibility>,
);

fn trigger_placed_frost(
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    players: Query<(&Player, &Transform)>,
    crystals: Query<WaitingFrost, WaitingFrostFilter>,
    mut selection: ResMut<Selection>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    for (player, transform) in &players {
        let index = player.0.index();
        if turn.hovering[index] {
            continue;
        }
        let Some(cell) = board_cell(transform.translation.truncate()) else {
            continue;
        };
        let mut triggered = false;
        for (entity, frost, item, visibility) in &crystals {
            if frost.0 != cell
                || turn
                    .pending_direct_hit
                    .as_ref()
                    .is_some_and(|hit| hit.item == Some(entity))
            {
                continue;
            }
            let reversible = player.0 == turn.active
                && !turn.completed_turn
                && matches!(turn.undo, Some(UndoAction::Movement { .. }));
            if reversible {
                if let Some(UndoAction::Movement {
                    triggered_frost, ..
                }) = &mut turn.undo
                {
                    triggered_frost.push(UndoPickup {
                        entity,
                        index: item.index,
                        cell: item.cell,
                        visibility: visibility.copied(),
                    });
                }
                commands
                    .entity(entity)
                    .remove::<Collectible>()
                    .insert((PickedUpForUndo, Visibility::Hidden));
            } else {
                if !turn.completed_turn && turn.existing_respawns.is_none() {
                    turn.existing_respawns = Some(turn.ingredient_respawns.len());
                }
                commands.entity(entity).despawn();
                turn.ingredient_respawns
                    .push((FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
            }
            triggered = true;
        }
        if triggered {
            if player.0 == turn.active && !turn.completed_turn {
                turn.pending_effects.frozen_players[index] =
                    turn.pending_effects.frozen_players[index].max(2);
            } else {
                turn.frozen_turns[index] = turn.frozen_turns[index].max(2);
            }
            commands.spawn(effects::EffectSignal(cell, effects::Effect::Freeze));
            if player.0 == turn.active {
                clear_selection(&mut commands, &mut selection);
                if let Ok(mut feedback) = feedback.single_mut() {
                    feedback.0 = "Frost Crystal triggered - frozen in place".into();
                }
            }
        }
    }
}

fn serpent_sprite(visual: &serpent::SerpentVisual, image: Handle<Image>) -> (Sprite, Transform) {
    let [head, middle, tail] = visual.cells.as_slice() else {
        unreachable!("serpents must occupy three cells");
    };
    let heading = *head - *middle;
    let tail_direction = *tail - *middle;
    let straight = heading == -tail_direction;
    let flipped = !straight && tail_direction != IVec2::new(-heading.y, heading.x);
    let angle = match heading {
        IVec2::X => 0.0,
        IVec2::Y => -std::f32::consts::FRAC_PI_2,
        IVec2::NEG_X => std::f32::consts::PI,
        IVec2::NEG_Y => std::f32::consts::FRAC_PI_2,
        _ => unreachable!("serpent cells must be cardinally adjacent"),
    };
    let rotation = Quat::from_rotation_z(angle);
    // Both poses use 32-pixel grid cells, with artwork extending below the grid.
    let (frame, width, height, grid_height) = if straight {
        (
            2 + usize::from(visual.venomous) * 2 + visual.straight_frame,
            96.0,
            40.0,
            32.0,
        )
    } else {
        (usize::from(visual.venomous), 64.0, 68.0, 64.0)
    };
    let scale = GRID_BLOCK_SIZE / 32.0;
    let source_x = frame as f32 * 96.0;
    let min = cell_center(*head)
        .min(cell_center(*tail))
        .min(cell_center(*middle));
    let max = cell_center(*head)
        .max(cell_center(*tail))
        .max(cell_center(*middle));
    let offset = Vec3::new(
        0.0,
        -(height - grid_height) * scale / 2.0 * if flipped { -1.0 } else { 1.0 },
        0.0,
    );
    let center = ((min + max) / 2.0).extend(0.7) + rotation * offset;
    (
        Sprite {
            image,
            rect: Some(Rect::new(source_x, 0.0, source_x + width, height)),
            custom_size: Some(Vec2::new(width, height) * scale),
            flip_y: flipped,
            ..default()
        },
        Transform {
            translation: center,
            rotation,
            ..default()
        },
    )
}

fn update_serpent_visuals(
    mut commands: Commands,
    turn: Res<TurnState>,
    sheet: Res<SerpentSheet>,
    parts: Query<Entity, With<SerpentPart>>,
    mut displayed: Local<(Vec<serpent::SerpentVisual>, Option<IVec2>)>,
) {
    let current = if let Some(animation) = &turn.serpent_animation {
        (animation.visuals.clone(), animation.impact)
    } else {
        (
            turn.serpents
                .iter()
                .map(|serpent| serpent.visual())
                .collect(),
            None,
        )
    };
    if *displayed == current {
        return;
    }
    for entity in &parts {
        commands.entity(entity).despawn();
    }
    for visual in &current.0 {
        let (sprite, transform) = serpent_sprite(visual, sheet.0.clone());
        commands.spawn((SerpentPart, sprite, transform));
    }
    if let Some(cell) = current.1 {
        let center = cell_center(cell);
        commands.spawn((
            SerpentPart,
            Sprite {
                color: Color::srgba(1.0, 0.1, 0.05, 0.65),
                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                ..default()
            },
            Transform::from_xyz(center.x, center.y, 1.8),
        ));
    }
    *displayed = current;
}

fn clear_hole_items(
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    mut board: ResMut<Board>,
    items: Query<(Entity, &Collectible)>,
) {
    if !turn.acted_this_frame
        && !(turn.existing_respawns.is_some()
            && turn.pending_placement.as_ref().is_some_and(|p| p.ready))
    {
        return;
    }
    let holes = board.take_new_holes();
    for (entity, item) in &items {
        if holes.contains(&item.cell) {
            commands.entity(entity).despawn();
            if !is_unlimited(item.index) {
                turn.ingredient_respawns
                    .push((item.index, INGREDIENT_RESPAWN_DELAY));
            }
        }
    }
}

fn respawn_ingredients(
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    board: Res<Board>,
    assets: (Res<InventoryImages>, Option<ResMut<sound::SoundCues>>),
    players: Query<(&Player, &Transform)>,
    collectibles: Query<&Collectible>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let (images, mut sounds) = assets;
    if !turn.completed_turn {
        return;
    }
    turn.turns_until_drop -= 1;
    if turn.turns_until_drop != 0 {
        return;
    }
    turn.turns_until_drop = INGREDIENT_DROP_INTERVAL;
    let mut occupied: Vec<_> = collectibles.iter().map(|item| item.cell).collect();
    occupied.extend(
        players
            .iter()
            .filter_map(|(_, transform)| board_cell(transform.translation.truncate())),
    );
    let mut candidates =
        ingredient_spawn_candidates(&board, &occupied, turn.aquatic_animation.as_ref());
    for _ in 0..INGREDIENT_DROP_BATCH {
        if candidates.is_empty() {
            break;
        }
        let index = if let Some(position) = turn
            .ingredient_respawns
            .iter()
            .position(|&(_, timer)| timer == 0)
        {
            turn.ingredient_respawns.remove(position).0
        } else if turn
            .spawned_ingredients
            .iter()
            .map(|&count| usize::from(count))
            .sum::<usize>()
            < INGREDIENT_POOL_SIZE
        {
            let index = random_ingredient(&mut *rng, &turn.spawned_ingredients);
            turn.spawned_ingredients[index] += 1;
            index
        } else {
            break;
        };
        let cell = candidates.swap_remove(rng.random_range(0..candidates.len()));
        spawn_collectible(&mut commands, &images, index, cell);
        sound::request(&mut sounds, sound::Cue::Pop);
    }
}

fn ingredient_spawn_candidates(
    board: &Board,
    occupied: &[IVec2],
    animation: Option<&AquaticAnimation>,
) -> Vec<IVec2> {
    (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
        .filter(|cell| {
            board.ground(*cell) == Some(Ground::Solid)
                && !board.is_wet(*cell)
                && !board.has_oil(*cell)
                && !board.is_poisoned(*cell)
                && !board.is_acidic(*cell)
                && !board.has_thorns(*cell)
                && !board.aquatic_cells().any(|occupied| occupied == *cell)
                && !animation.is_some_and(|animation| animation.swept.contains(cell))
                && !occupied.contains(cell)
        })
        .collect()
}

fn random_ingredient(rng: &mut impl Rng, spawned: &[u8; 14]) -> usize {
    let needs_copies = INGREDIENT_WEIGHTS
        .iter()
        .enumerate()
        .any(|(index, &weight)| weight > 0 && spawned[index] < MIN_INGREDIENT_COPIES);
    let weights = std::array::from_fn::<_, 14, _>(|index| {
        if needs_copies && spawned[index] >= MIN_INGREDIENT_COPIES {
            0
        } else {
            INGREDIENT_WEIGHTS[index]
        }
    });
    let mut choice = rng.random_range(0..weights.iter().sum());
    for (index, weight) in weights.into_iter().enumerate() {
        if choice < weight {
            return index;
        }
        choice -= weight;
    }
    unreachable!("ingredient weights must contain a positive value")
}

fn spawn_initial_collectibles(
    mut commands: Commands,
    board: Res<Board>,
    images: Res<InventoryImages>,
    players: Query<&Transform, With<Player>>,
    mut turn: ResMut<TurnState>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let occupied: Vec<_> = players
        .iter()
        .filter_map(|transform| board_cell(transform.translation.truncate()))
        .collect();
    let mut candidates = ingredient_spawn_candidates(&board, &occupied, None);
    for index in [SUNSTONE_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX] {
        assert!(
            !candidates.is_empty(),
            "not enough space for initial ingredients"
        );
        let cell = candidates.swap_remove(rng.random_range(0..candidates.len()));
        spawn_collectible(&mut commands, &images, index, cell);
        turn.spawned_ingredients[index] += 1;
    }
}

fn update_collectible_positions(
    players: Query<&Transform, With<Player>>,
    mut items: Query<(&Collectible, Has<PlacedItem>, &mut Transform, &mut Sprite), Without<Player>>,
) {
    let player_cells: Vec<_> = players
        .iter()
        .filter_map(|transform| board_cell(transform.translation.truncate()))
        .collect();
    for (item, placed, mut transform, mut sprite) in &mut items {
        let occupied = placed || player_cells.contains(&item.cell);
        let center = cell_center(item.cell);
        let offset = if occupied { ITEM_CORNER_OFFSET } else { 0.0 };
        transform.translation.x = center.x + offset;
        transform.translation.y = center.y + offset;
        sprite.custom_size = Some(Vec2::splat(if occupied { 24.0 } else { 44.0 }));
    }
}

fn advance_item_flight(
    time: Res<Time>,
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    mut sprites: Query<&mut Transform, Without<Player>>,
    mut sounds: Option<ResMut<sound::SoundCues>>,
) {
    let Some(flight) = &mut turn.item_flight else {
        return;
    };
    flight.elapsed += time.delta_secs();
    let progress = (flight.elapsed / ITEM_FLIGHT_SECONDS).clamp(0.0, 1.0);
    if progress >= 1.0 {
        commands.entity(flight.entity).despawn();
        turn.item_flight = None;
        sound::request(&mut sounds, sound::Cue::Pop);
        return;
    }
    let mut transform = sprites
        .get_mut(flight.entity)
        .expect("in-flight item sprite must exist");
    let position = flight.origin.lerp(flight.destination, progress)
        + Vec2::Y * (4.0 * ITEM_FLIGHT_HEIGHT * progress * (1.0 - progress));
    transform.translation.x = position.x;
    transform.translation.y = position.y;
}

fn spawn_collectible(commands: &mut Commands, images: &InventoryImages, index: usize, cell: IVec2) {
    debug_assert!(!is_unlimited(index));
    let center = cell_center(cell);
    let mut item = commands.spawn((
        Name::new(INVENTORY_ITEMS[index].0),
        BoardItem,
        Collectible { index, cell },
        images.sprite(index, 44.0),
        Transform::from_xyz(center.x, center.y, 0.1),
    ));
    match index {
        WATER_ITEM_INDEX | ACID_ITEM_INDEX | POISON_ITEM_INDEX => {}
        LIFE_ESSENCE_ITEM_INDEX => {
            item.insert(PendingLifeEssence(cell));
        }
        FROST_CRYSTAL_ITEM_INDEX => {
            item.insert(PendingFrost(cell));
        }
        STORM_SHARD_ITEM_INDEX => {
            item.insert(PendingStormShard(cell));
        }
        SUNSTONE_ITEM_INDEX => {
            item.insert(PendingSunstone(cell));
        }
        THORNHEART_ITEM_INDEX => {
            item.insert(PendingThornheart(cell));
        }
        OBSIDIAN_ITEM_INDEX => {
            item.insert(PendingObsidian(cell));
        }
        STONE_DUST_ITEM_INDEX => {
            item.insert(PendingStoneDust(cell));
        }
        VOID_DUST_ITEM_INDEX => {
            item.insert(PendingVoidDust(cell));
        }
        COATL_EGG_ITEM_INDEX => {
            item.insert(PendingCoatlEgg(cell));
        }
        AQUATIC_ITEM_INDEX => {
            item.insert(PendingAquaticCreature(cell));
        }
        _ => {}
    }
}

fn update_board_cursor(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor: ResMut<BoardCursor>,
) {
    cursor.0 = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .and_then(|position| {
            let (camera, transform) = cameras.single().ok()?;
            camera.viewport_to_world_2d(transform, position).ok()
        });
}

fn highlight_inventory_hover(
    cursor: Res<BoardCursor>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    selection: Res<Selection>,
    game: (Res<TurnState>, Option<Res<ai::BotController>>),
    mut hover: ResMut<InventoryHover>,
    mut slots: Query<(
        &InventorySlot,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
    )>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    let (turn, bot) = game;
    if bot.as_ref().is_some_and(|bot| bot.bot_turn) {
        return;
    }
    let hovered = cursor
        .0
        .and_then(|position| inventory_item(position, turn.active))
        .map(|index| (turn.active, index));
    let selected = match &selection.0 {
        Some(SelectedAction::Item { index, .. }) => Some((turn.active, *index)),
        _ => None,
    };
    let hover_changed = hover.hovered != hovered;
    if !hover_changed && hover.selected == selected {
        return;
    }

    for (slot, mut background, mut border, mut node) in &mut slots {
        let is_selected = selected == Some((slot.owner, slot.index));
        let is_hovered = hovered == Some((slot.owner, slot.index));
        background.0 = if is_selected {
            Color::srgba(0.86, 0.68, 0.32, 0.6)
        } else if is_hovered {
            Color::srgba(0.86, 0.68, 0.32, 0.35)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(if is_selected || is_hovered {
            Color::srgb(1.0, 0.85, 0.45)
        } else {
            Color::srgba(0.86, 0.68, 0.32, 0.4)
        });
        node.border = UiRect::all(Val::Px(if is_selected {
            3.0
        } else if is_hovered {
            2.0
        } else {
            0.5
        }));
    }

    if hover_changed
        && selection.0.is_none()
        && !mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left))
        && let Ok(mut feedback) = feedback.single_mut()
    {
        feedback.0 = match hovered {
            Some((_, index)) => format!("{} - click to select", INVENTORY_ITEMS[index].0),
            None => "Click Move or an item to select it".into(),
        };
    }
    hover.hovered = hovered;
    hover.selected = selected;
}

fn cell_feedback(board: &Board, cell: IVec2) -> Option<String> {
    let ground = match board.ground(cell)? {
        Ground::Solid => "Ground",
        Ground::Water => "Water",
        Ground::Ridge => "Ridge",
        Ground::Hole => "Hole",
    };
    let mut effects = Vec::new();
    if board.is_wet(cell) {
        effects.push("Wet");
    }
    if board.is_frozen(cell) {
        effects.push("Frozen");
    }
    if board.is_poisoned(cell) {
        effects.push(if board.is_frozen(cell) {
            "Poison (paused)"
        } else {
            "Poison"
        });
    }
    if board.is_electrified_acid(cell) {
        effects.push("Electrified acid");
    } else if board.is_acidic(cell) {
        effects.push("Acid");
    }
    if board.has_oil(cell) {
        effects.push("Ember");
    }
    if board.has_thorns(cell) {
        effects.push("Thorns");
    }
    if board.has_life_essence(cell) {
        effects.push("Healing water");
    }
    if board.aquatic_contacts(cell).next().is_some() {
        effects.push("Ahuizotl");
    }
    if let Some(growth) = board.terrain_growth_kind_at(cell) {
        effects.push(match growth {
            TerrainGrowthKind::Void => "Void growth",
            TerrainGrowthKind::Stone => "Ridge growth",
        });
    }
    let mut message = format!("{}: {ground}", history::cell_label(cell));
    if !effects.is_empty() {
        message.push_str(" | ");
        message.push_str(&effects.join(", "));
    }
    Some(message)
}

type CellFeedbackFilter = (With<CellFeedback>, Without<DropFeedback>);

fn update_cell_feedback(
    board: Res<Board>,
    cursor: Res<BoardCursor>,
    selection: Res<Selection>,
    turn: Res<TurnState>,
    mut cells: Query<(&mut Text, &mut Visibility), CellFeedbackFilter>,
    mut feedback: Query<&mut Visibility, (With<DropFeedback>, Without<CellFeedback>)>,
) {
    let message = if selection.0.is_none() && !turn.animation_active() {
        cursor
            .0
            .and_then(board_cell)
            .and_then(|cell| cell_feedback(&board, cell))
    } else {
        None
    };
    for (mut text, mut visibility) in &mut cells {
        *visibility = if let Some(message) = &message {
            if text.0 != *message {
                text.0.clone_from(message);
            }
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut feedback {
        *visibility = if message.is_some() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

fn fit_status_text(mut texts: Query<(&Text, &TextLayoutInfo, &mut TextFont, &mut FitStatusText)>) {
    for (text, layout, mut font, mut fit) in &mut texts {
        if fit.last_text != text.0 {
            fit.last_text.clone_from(&text.0);
            font.font_size = FontSize::Px(fit.initial_size);
            continue;
        }
        if layout.scale_factor <= 0.0 {
            continue;
        }
        let size = layout.size / layout.scale_factor;
        let scale = (fit.bounds.x / size.x).min(fit.bounds.y / size.y);
        if scale < 1.0
            && let FontSize::Px(current) = font.font_size
        {
            font.font_size = FontSize::Px(current * scale * 0.98);
        }
    }
}

fn preview_node(position: Vec2) -> Node {
    const SIZE: f32 = 44.0;
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(position.x + RESOLUTION_WIDTH / 2.0 - SIZE / 2.0),
        top: Val::Px(RESOLUTION_HEIGHT / 2.0 - position.y - SIZE / 2.0),
        width: Val::Px(SIZE),
        height: Val::Px(SIZE),
        ..default()
    }
}

fn clear_selection(commands: &mut Commands, selection: &mut Selection) {
    match selection.0.take() {
        Some(SelectedAction::Item {
            icon, highlight, ..
        }) => {
            commands.entity(icon).despawn();
            commands.entity(highlight).despawn();
        }
        Some(SelectedAction::Movement { highlights }) => {
            for entity in highlights {
                commands.entity(entity).despawn();
            }
        }
        None => {}
    }
}

fn select_movement(
    commands: &mut Commands,
    player: Vec2,
    opponent: Vec2,
    board: &Board,
    energy: u8,
    hover: bool,
    magic: u8,
) -> SelectedAction {
    let mut highlights = Vec::new();
    if let Some(cell) = board_cell(player) {
        for x in -2..=2 {
            for y in -2..=2 {
                if x == 0 && y == 0 {
                    continue;
                }
                let center = cell_center(cell + IVec2::new(x, y));
                let Some(target) = drop_target(player, center) else {
                    continue;
                };
                let Ok(movement) = movement_target(board, &target, opponent, energy, hover, magic)
                else {
                    continue;
                };
                highlights.push(
                    commands
                        .spawn((
                            Sprite {
                                color: match movement {
                                    Movement::Walk => Color::srgba(0.2, 0.8, 0.3, 0.45),
                                    Movement::Jump => Color::srgba(1.0, 0.7, 0.1, 0.45),
                                },
                                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                                ..default()
                            },
                            Transform::from_xyz(center.x, center.y, 0.5),
                        ))
                        .id(),
                );
            }
        }
    }
    SelectedAction::Movement { highlights }
}

fn handle_selection(
    mut commands: Commands,
    input: SelectionInput,
    assets: (
        Res<InventoryImages>,
        Res<Hud>,
        ResMut<Board>,
        ResMut<PlayerHealth>,
    ),
    mut selection: ResMut<Selection>,
    mut turn: ResMut<TurnState>,
    actors: SelectionActors,
    output: ActionOutput,
) {
    let ActionOutput {
        mut feedback,
        mut history,
        mut sounds,
    } = output;
    if turn.animation_active() {
        return;
    }
    let SelectionInput {
        mouse,
        keyboard,
        cursor,
        bot,
        mut rng,
    } = input;
    let (images, hud, mut board, mut health) = assets;
    let SelectionActors {
        mut players,
        crystals,
        shards,
        sunstones,
        life_essences,
        thornhearts,
        obsidians,
        stones,
        water_stones,
        void_dust,
        eggs,
        aquatic,
        collectibles,
    } = actors;
    let has_dust_reaction = |cell| {
        obsidians.iter().any(|(_, item)| item.0 == cell)
            || life_essences.iter().any(|(_, item)| item.0 == cell)
    };
    let has_frost = |cell| crystals.iter().any(|(_, item)| item.0 == cell);
    let Some((player_entity, player)) = players.iter().find_map(|(entity, player, transform)| {
        (player.0 == turn.active).then_some((entity, transform.translation.truncate()))
    }) else {
        return;
    };
    let Some(opponent) = players.iter().find_map(|(_, player, transform)| {
        (player.0 == turn.active.other()).then_some(transform.translation.truncate())
    }) else {
        return;
    };
    let Ok(mut feedback) = feedback.single_mut() else {
        return;
    };

    let resolving_pending = turn.pending_placement.as_ref().is_some_and(|p| p.ready);
    let bot_turn = bot.as_ref().is_some_and(|bot| bot.bot_turn);
    let bot_action = bot.as_ref().and_then(|bot| bot.action);
    if bot_turn && !resolving_pending && bot_action.is_none() {
        return;
    }
    if !bot_turn
        && !resolving_pending
        && keyboard.just_pressed(KeyCode::Escape)
        && selection.0.is_some()
    {
        clear_selection(&mut commands, &mut selection);
        feedback.0 = "Selection cancelled".into();
        return;
    }

    let input_position = if bot_turn {
        match bot_action {
            Some(ai::BotAction::Move(cell) | ai::BotAction::Place { cell, .. }) => {
                Some(cell_center(cell))
            }
            _ => None,
        }
    } else {
        cursor.0
    };
    let cell = input_position.and_then(board_cell);
    let mut target = input_position.and_then(|position| {
        if matches!(bot_action, Some(ai::BotAction::Place { .. }))
            || (!bot_turn && matches!(selection.0, Some(SelectedAction::Item { .. })))
        {
            item_target(player, position)
        } else {
            drop_target(player, position)
        }
    });
    let mut pending_index = None;
    let mut resolving_placement = None;
    let mut previous_free_action = false;
    if resolving_pending {
        if matches!(turn.undo, Some(UndoAction::Placement { .. })) {
            turn.undo = None;
        }
        let pending = turn
            .pending_placement
            .take()
            .expect("pending placement exists");
        if let Some(message) = item_placement_error(
            &board,
            pending.index,
            pending.target.cell,
            has_dust_reaction(pending.target.cell),
            has_frost(pending.target.cell),
        ) {
            let cost = u8::try_from(item_energy_cost(&pending.target)).expect("validated cost");
            let player_index = turn.active.index();
            turn.strength[player_index] += cost;
            turn.free_action = pending.previous_free_action;
            turn.actions_used -= 1;
            turn.refund_ingredient(pending.index);
            commands.entity(pending.marker).despawn();
            resolve_deferred_hit(
                &mut turn,
                &mut commands,
                players
                    .iter()
                    .map(|(_, player, transform)| (player.0, transform.translation.truncate())),
                false,
            );
            feedback.0 = format!("{message} - first item resolved; choose another action or rest");
            sound::request(&mut sounds, sound::Cue::Reject);
            return;
        }
        target = Some(pending.target.clone());
        pending_index = Some(pending.index);
        previous_free_action = pending.previous_free_action;
        resolving_placement = Some(pending);
    }
    let human_click = !bot_turn && mouse.just_pressed(MouseButton::Left);
    let bot_selection = bot_turn
        && matches!(
            bot_action,
            Some(ai::BotAction::Move(_) | ai::BotAction::Place { .. })
        );
    if (human_click || bot_selection || resolving_pending)
        && !turn.acted_this_frame
        && (resolving_pending || !bot_turn || turn.actions_used < 2)
    {
        if human_click
            && !resolving_pending
            && let Some(index) = cursor
                .0
                .and_then(|position| inventory_item(position, turn.active))
        {
            turn.terrain_rejection = None;
            let already_selected = matches!(selection.0, Some(SelectedAction::Item { index: current, .. }) if current == index);
            clear_selection(&mut commands, &mut selection);
            if already_selected {
                feedback.0 = "Selection cancelled".into();
            } else if turn.actions_used >= 2 {
                feedback.0 = "Both actions have been used".into();
                sound::request(&mut sounds, sound::Cue::Reject);
            } else if must_land(&turn, &board, player) {
                feedback.0 = "Land on safe ground before magic runs out".into();
                sound::request(&mut sounds, sound::Cue::Reject);
            } else if !is_unlimited(index) && turn.inventory[turn.active.index()][index] == 0 {
                feedback.0 = "No ingredients remaining - pick one up".into();
                sound::request(&mut sounds, sound::Cue::Reject);
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
                        images.icon(index),
                        preview_node(Vec2::ZERO),
                        Visibility::Hidden,
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(hud.0).add_child(icon);
                selection.0 = Some(SelectedAction::Item {
                    index,
                    icon,
                    highlight,
                });
                feedback.0 = format!("{} selected - click a board cell", INVENTORY_ITEMS[index].0);
                sound::request(&mut sounds, sound::Cue::Select);
            }
            return;
        }
        enum ActiveSelection {
            Item(usize),
            Movement,
        }
        let active_selection = pending_index.map(ActiveSelection::Item).or_else(|| {
            if bot_turn {
                match bot_action {
                    Some(ai::BotAction::Place { item, .. }) => Some(ActiveSelection::Item(item)),
                    Some(ai::BotAction::Move(_)) => Some(ActiveSelection::Movement),
                    _ => None,
                }
            } else {
                match &selection.0 {
                    Some(SelectedAction::Item { index, .. }) => Some(ActiveSelection::Item(*index)),
                    Some(SelectedAction::Movement { .. }) => Some(ActiveSelection::Movement),
                    None => None,
                }
            }
        });
        let placement_error = match (&active_selection, &target) {
            (Some(ActiveSelection::Item(index)), Some(target)) => {
                if !resolving_pending && let Some(first) = turn.pending_placement.as_ref() {
                    let positions: Vec<_> = players
                        .iter()
                        .map(|(_, player, transform)| (player.0, transform.translation.truncate()))
                        .collect();
                    let items: Vec<_> = collectibles
                        .iter()
                        .map(|item| (item.index, item.cell))
                        .collect();
                    projected_placement_error(
                        (&board, &turn, &health),
                        &positions,
                        &items,
                        &rng,
                        first,
                        *index,
                        target,
                    )
                } else {
                    item_placement_error(
                        &board,
                        *index,
                        target.cell,
                        has_dust_reaction(target.cell),
                        has_frost(target.cell),
                    )
                    .map(str::to_owned)
                }
            }
            _ => None,
        };
        match (&active_selection, &target) {
            (Some(ActiveSelection::Item(index)), Some(target))
                if placement_error.is_none()
                    && (resolving_pending
                        || (!must_land(&turn, &board, player)
                            && (is_unlimited(*index)
                                || turn.inventory[turn.active.index()][*index] > 0)))
                    && (resolving_pending
                        || u32::from(turn.strength[turn.active.index()])
                            >= item_energy_cost(target)) =>
            {
                if !resolving_pending && turn.pending_placement.is_some() {
                    let cost = u8::try_from(item_energy_cost(target)).expect("validated cost");
                    let previous_free_action = turn.free_action;
                    turn.spend_action_energy(cost);
                    turn.spend_ingredient(*index);
                    let marker = commands
                        .spawn((
                            Sprite {
                                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                                ..images.sprite(*index, 44.0)
                            },
                            Transform::from_xyz(
                                cell_center(target.cell).x,
                                cell_center(target.cell).y,
                                0.8,
                            ),
                        ))
                        .id();
                    turn.next_placement = Some(PendingPlacement {
                        index: *index,
                        target: target.clone(),
                        marker,
                        ready: true,
                        previous_free_action,
                        landed: false,
                    });
                    turn.actions_used += 1;
                    turn.pending_placement
                        .as_mut()
                        .expect("first item queued")
                        .ready = true;
                    clear_selection(&mut commands, &mut selection);
                    feedback.0 = "Resolving both items".into();
                    sound::request(&mut sounds, sound::Cue::Pop);
                    return;
                }
                let mut health_changes = HealthChanges::default();
                let mut frozen_players = [0; 2];
                let is_water = *index == WATER_ITEM_INDEX;
                let is_sunstone = *index == SUNSTONE_ITEM_INDEX;
                let is_frost = *index == FROST_CRYSTAL_ITEM_INDEX;
                let is_storm = *index == STORM_SHARD_ITEM_INDEX;
                let is_void = *index == VOID_DUST_ITEM_INDEX;
                let is_obsidian = *index == OBSIDIAN_ITEM_INDEX;
                let is_stone = *index == STONE_DUST_ITEM_INDEX;
                let is_oil = *index == OIL_ITEM_INDEX;
                let is_poison = *index == POISON_ITEM_INDEX;
                let is_acid = *index == ACID_ITEM_INDEX;
                let consumed_index = *index;
                let is_life_essence = *index == LIFE_ESSENCE_ITEM_INDEX;
                let is_thornheart = *index == THORNHEART_ITEM_INDEX;
                let is_egg = *index == COATL_EGG_ITEM_INDEX;
                let is_aquatic = *index == AQUATIC_ITEM_INDEX;
                let defer_direct_hit = turn.next_placement.as_ref().is_some_and(|next| {
                    let same_cell = next.target.cell == target.cell;
                    let acid_spreads_to_hit = next.index == ACID_ITEM_INDEX
                        && (same_cell
                            || ((next.target.cell - target.cell).abs() == IVec2::ONE
                                && board.is_poisoned(next.target.cell)
                                && !board.is_frozen(next.target.cell)));
                    (next.index == WATER_ITEM_INDEX && next.target.cell == target.cell)
                        || (is_storm
                            && (acid_spreads_to_hit
                                || (next.index == OBSIDIAN_ITEM_INDEX && same_cell)))
                });
                let water_on_solid = is_water && board.ground(target.cell) == Some(Ground::Solid);
                let water_on_ridge = is_water && board.ground(target.cell) == Some(Ground::Ridge);
                let corrodes_ridge = water_on_ridge && board.is_acidic(target.cell);
                let existing_frost = if water_on_solid {
                    crystals
                        .iter()
                        .find_map(|(entity, frost)| (frost.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let existing_shard = if water_on_solid || is_obsidian {
                    shards
                        .iter()
                        .find_map(|(entity, shard)| (shard.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let existing_sunstone = if water_on_solid || is_oil || is_obsidian {
                    sunstones.iter().find_map(|(entity, sunstone)| {
                        (sunstone.0 == target.cell).then_some(entity)
                    })
                } else {
                    None
                };
                let mut existing_life_essences: Vec<_> = if water_on_solid {
                    life_essences
                        .iter()
                        .filter_map(|(entity, essence)| {
                            (essence.0 == target.cell).then_some(entity)
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let pending_thorn = if is_life_essence {
                    thornhearts
                        .iter()
                        .find_map(|(entity, thorn)| (thorn.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let pending_thorn_life = if is_thornheart {
                    life_essences
                        .iter()
                        .find_map(|(entity, essence)| (essence.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let pending_obsidian = obsidians
                    .iter()
                    .find_map(|(entity, obsidian)| (obsidian.0 == target.cell).then_some(entity));
                let pending_stone = if is_obsidian || is_life_essence || water_on_solid {
                    stones
                        .iter()
                        .find_map(|(entity, stone)| (stone.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let pending_void = if is_obsidian || is_life_essence {
                    void_dust
                        .iter()
                        .find_map(|(entity, dust)| (dust.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let creates_ridge = (is_stone && pending_obsidian.is_some())
                    || (is_obsidian && pending_stone.is_some());
                let void_obsidian = !creates_ridge
                    && ((is_void && pending_obsidian.is_some())
                        || (is_obsidian && pending_void.is_some()));
                let pending_dust_life =
                    if is_void || is_stone || is_egg || is_aquatic || water_on_solid {
                        life_essences.iter().find_map(|(entity, essence)| {
                            (essence.0 == target.cell).then_some(entity)
                        })
                    } else {
                        None
                    };
                let void_life = !void_obsidian
                    && ((is_void
                        && (pending_dust_life.is_some() || board.has_life_essence(target.cell)))
                        || (is_life_essence && pending_void.is_some()));
                let void_reaction = void_obsidian || void_life;
                let stone_life = !creates_ridge
                    && !void_reaction
                    && ((is_stone
                        && (pending_dust_life.is_some() || board.has_life_essence(target.cell)))
                        || (is_life_essence && pending_stone.is_some()));
                let pending_egg = if is_life_essence {
                    eggs.iter()
                        .find_map(|(entity, egg)| (egg.0 == target.cell).then_some(entity))
                } else {
                    None
                };
                let egg_life = !void_reaction
                    && !stone_life
                    && ((is_egg
                        && (pending_dust_life.is_some() || board.has_life_essence(target.cell)))
                        || (is_life_essence && pending_egg.is_some()));
                let pending_aquatic = if is_life_essence || water_on_solid {
                    aquatic.iter().find_map(|(entity, creature)| {
                        (creature.0 == target.cell).then_some(entity)
                    })
                } else {
                    None
                };
                let aquatic_life = !void_reaction
                    && !stone_life
                    && !egg_life
                    && (board.is_water(target.cell) || (water_on_solid && pending_stone.is_none()))
                    && ((is_aquatic
                        && (pending_dust_life.is_some() || board.has_life_essence(target.cell)))
                        || (is_life_essence && pending_aquatic.is_some())
                        || (water_on_solid
                            && pending_aquatic.is_some()
                            && pending_dust_life.is_some()));
                let void_hole =
                    is_void && !void_reaction && board.ground(target.cell) == Some(Ground::Solid);
                let void_levels_ridge =
                    is_void && !void_reaction && board.ground(target.cell) == Some(Ground::Ridge);
                let hole_cells = if void_obsidian {
                    terrain_cross(&board, target.cell)
                } else if void_hole {
                    vec![target.cell]
                } else {
                    Vec::new()
                };
                let ridge_cells = if creates_ridge {
                    terrain_cross(&board, target.cell)
                } else {
                    Vec::new()
                };
                if !board.can_create_unsafe(&hole_cells) || !board.can_create_unsafe(&ridge_cells) {
                    if resolving_pending {
                        commands
                            .entity(
                                resolving_placement
                                    .as_ref()
                                    .expect("resolving placement")
                                    .marker,
                            )
                            .despawn();
                        let cost = u8::try_from(item_energy_cost(target)).expect("validated cost");
                        let player_index = turn.active.index();
                        turn.strength[player_index] += cost;
                        turn.free_action = previous_free_action;
                        turn.actions_used -= 1;
                        turn.refund_ingredient(*index);
                        resolve_deferred_hit(
                            &mut turn,
                            &mut commands,
                            players.iter().map(|(_, player, transform)| {
                                (player.0, transform.translation.truncate())
                            }),
                            false,
                        );
                    }
                    turn.terrain_rejection = Some((*index, target.cell));
                    feedback.0 = if resolving_pending {
                        "Cannot create terrain: 60% ridge and hole limit - first item resolved; choose another action or rest"
                    } else {
                        "Cannot create terrain: 60% ridge and hole limit"
                    }
                    .into();
                    sound::request(&mut sounds, sound::Cue::Reject);
                    return;
                }
                turn.terrain_rejection = None;
                if let Some(mut pending) = resolving_placement.take() {
                    if !pending.landed {
                        let player_on_target = players.iter().any(|(_, _, transform)| {
                            board_cell(transform.translation.truncate()) == Some(target.cell)
                        });
                        let grounded_player_on_target =
                            players.iter().any(|(_, player, transform)| {
                                board_cell(transform.translation.truncate()) == Some(target.cell)
                                    && !turn.hovering[player.0.index()]
                            });
                        let frozen_player_on_target =
                            players.iter().any(|(_, player, transform)| {
                                board_cell(transform.translation.truncate()) == Some(target.cell)
                                    && (turn.frozen_turns[player.0.index()] > 0
                                        || turn.pending_effects.frozen_players[player.0.index()]
                                            > 0)
                            });
                        let direct_hit = player_on_target && !defer_direct_hit;
                        let is_effect = match *index {
                            WATER_ITEM_INDEX => water_on_solid || corrodes_ridge,
                            ACID_ITEM_INDEX
                            | OIL_ITEM_INDEX
                            | STONE_DUST_ITEM_INDEX
                            | VOID_DUST_ITEM_INDEX => true,
                            POISON_ITEM_INDEX => {
                                !board.is_frozen(target.cell)
                                    && board.ground(target.cell).is_some_and(Ground::walkable)
                            }
                            FROST_CRYSTAL_ITEM_INDEX => {
                                board.is_water(target.cell)
                                    || (grounded_player_on_target && !defer_direct_hit)
                            }
                            STORM_SHARD_ITEM_INDEX => {
                                board.is_water(target.cell)
                                    || board.is_acidic(target.cell)
                                    || pending_obsidian.is_some()
                                    || direct_hit
                            }
                            SUNSTONE_ITEM_INDEX => {
                                board.is_frozen(target.cell)
                                    || board.has_oil(target.cell)
                                    || pending_obsidian.is_some()
                                    || frozen_player_on_target
                            }
                            OBSIDIAN_ITEM_INDEX => {
                                creates_ridge
                                    || void_reaction
                                    || existing_shard.is_some()
                                    || existing_sunstone.is_some()
                            }
                            LIFE_ESSENCE_ITEM_INDEX => {
                                board.is_water(target.cell)
                                    || board.is_acidic(target.cell)
                                    || void_life
                                    || stone_life
                                    || egg_life
                                    || aquatic_life
                                    || pending_thorn.is_some()
                            }
                            THORNHEART_ITEM_INDEX => {
                                pending_thorn_life.is_some() || board.has_life_essence(target.cell)
                            }
                            COATL_EGG_ITEM_INDEX => egg_life,
                            AQUATIC_ITEM_INDEX => aquatic_life,
                            _ => unreachable!("invalid ingredient index"),
                        };
                        let destination = cell_center(target.cell)
                            + if is_effect {
                                Vec2::ZERO
                            } else {
                                Vec2::splat(ITEM_CORNER_OFFSET)
                            };
                        commands.entity(pending.marker).insert(Visibility::Hidden);
                        let entity = commands
                            .spawn((
                                images.sprite(*index, GRID_BLOCK_SIZE / 2.0),
                                Transform::from_xyz(player.x, player.y, 1.8),
                            ))
                            .id();
                        turn.item_flight = Some(ItemFlight {
                            entity,
                            origin: player,
                            destination,
                            elapsed: 0.0,
                        });
                        sound::request(&mut sounds, sound::Cue::Whoosh);
                        pending.landed = true;
                        turn.pending_placement = Some(pending);
                        return;
                    }
                    commands.entity(pending.marker).despawn();
                }
                if !resolving_pending {
                    let cost =
                        u8::try_from(item_energy_cost(target)).expect("energy was validated");
                    let previous_free_action = turn.free_action;
                    if turn.actions_used == 0 {
                        turn.undo = Some(UndoAction::Placement {
                            strength: turn.strength[turn.active.index()],
                        });
                    }
                    turn.spend_action_energy(cost);
                    turn.actions_used += 1;
                    turn.spend_ingredient(*index);
                    let center = cell_center(target.cell);
                    let marker = commands
                        .spawn((
                            Sprite {
                                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                                ..images.sprite(*index, 44.0)
                            },
                            Transform::from_xyz(center.x, center.y, 0.8),
                        ))
                        .id();
                    turn.pending_placement = Some(PendingPlacement {
                        index: *index,
                        target: target.clone(),
                        marker,
                        ready: turn.actions_used == 2,
                        previous_free_action,
                        landed: false,
                    });
                    clear_selection(&mut commands, &mut selection);
                    feedback.0 = if turn.actions_used == 1 {
                        "Item queued - choose another action or rest"
                    } else {
                        "Resolving placement"
                    }
                    .into();
                    sound::request(&mut sounds, sound::Cue::Pop);
                    return;
                }
                if bot_turn && let Some(history) = &mut history {
                    history.add(
                        format!(
                            "Computer used {} at {}",
                            INVENTORY_ITEMS[*index].0,
                            history::cell_label(target.cell)
                        ),
                        Some(*index),
                    );
                }
                let stone_absorbs_acid = is_stone && !stone_life && board.is_acidic(target.cell);
                let fills_ground = (is_stone
                    && !stone_life
                    && (board.is_water(target.cell)
                        || board.ground(target.cell) == Some(Ground::Hole)))
                    || (water_on_solid && pending_stone.is_some());
                let stone_clears_poison = is_stone && board.is_poisoned(target.cell);
                let uses_stone = creates_ridge
                    || fills_ground
                    || stone_absorbs_acid
                    || stone_clears_poison
                    || stone_life;
                if let Some(entity) = pending_void
                    && void_reaction
                {
                    commands.entity(entity).despawn();
                }
                if is_void
                    && void_obsidian
                    && let Some(entity) = pending_obsidian
                {
                    commands.entity(entity).despawn();
                }
                if (is_void && void_life)
                    || (is_stone && stone_life)
                    || (is_egg && egg_life)
                    || ((is_aquatic || water_on_solid) && aquatic_life)
                {
                    if let Some(entity) = pending_dust_life {
                        commands.entity(entity).despawn();
                        existing_life_essences.retain(|other| *other != entity);
                    } else {
                        board.clear_life_essence(target.cell);
                    }
                }
                if void_life {
                    board.start_terrain_growth(target.cell, rng.random(), TerrainGrowthKind::Void);
                } else if stone_life {
                    board.start_terrain_growth(target.cell, rng.random(), TerrainGrowthKind::Stone);
                }
                if let Some(entity) = pending_stone
                    && uses_stone
                {
                    commands.entity(entity).despawn();
                }
                if creates_ridge
                    && is_stone
                    && let Some(entity) = pending_obsidian
                {
                    commands.entity(entity).despawn();
                }
                if void_levels_ridge {
                    board.set(target.cell, Ground::Solid);
                }
                let direct_storm = !creates_ridge
                    && !void_reaction
                    && ((is_storm && pending_obsidian.is_some())
                        || (is_obsidian && existing_shard.is_some()));
                let direct_sunstone = !creates_ridge
                    && !void_reaction
                    && ((is_sunstone && pending_obsidian.is_some())
                        || (is_obsidian && existing_sunstone.is_some()));
                let was_frozen = board.is_frozen(target.cell);
                let newly_wet = water_on_solid && !board.is_water(target.cell);
                let watered_poison = newly_wet && board.is_poisoned(target.cell);
                let new_oil = is_oil && !board.has_oil(target.cell);
                if water_on_solid {
                    board.make_water(target.cell);
                    commands.spawn(effects::EffectSignal(target.cell, effects::Effect::Water));
                }
                if corrodes_ridge {
                    board.set(target.cell, Ground::Solid);
                }
                if stone_absorbs_acid {
                    board.absorb_acid(target.cell);
                }
                if stone_clears_poison {
                    board.clear_poison(target.cell);
                }
                let cleared_acid = is_life_essence && board.is_acidic(target.cell);
                if cleared_acid {
                    board.clear_acid(target.cell);
                }
                if egg_life {
                    if let Some(entity) = pending_egg {
                        commands.entity(entity).despawn();
                    }
                    let mut player_cells = [None; 2];
                    for (_, player, transform) in &players {
                        player_cells[player.0.index()] =
                            board_cell(transform.translation.truncate());
                    }
                    turn.serpents.push(serpent::Serpent::new(
                        target.cell,
                        &board,
                        player_cells,
                        health.0,
                    ));
                }
                if aquatic_life {
                    if let Some(entity) = pending_aquatic {
                        commands.entity(entity).despawn();
                    }
                    board.activate_aquatic(target.cell);
                }
                let activated_life_essence = !void_life
                    && !stone_life
                    && !egg_life
                    && ((!aquatic_life && is_life_essence) || !existing_life_essences.is_empty())
                    && board.is_water(target.cell);
                if activated_life_essence {
                    if is_life_essence && !aquatic_life {
                        board.add_life_essence(target.cell);
                    }
                    for entity in existing_life_essences {
                        board.add_life_essence(target.cell);
                        commands.entity(entity).despawn();
                    }
                }
                let activated_thornheart = !void_life
                    && !stone_life
                    && !egg_life
                    && !aquatic_life
                    && (pending_thorn.is_some()
                        || (is_thornheart
                            && (pending_thorn_life.is_some()
                                || board.has_life_essence(target.cell))));
                if activated_thornheart {
                    if let Some(entity) = pending_thorn {
                        commands.entity(entity).despawn();
                    }
                    if let Some(entity) = pending_thorn_life {
                        commands.entity(entity).despawn();
                        if board.is_water(target.cell) {
                            board.add_life_essence(target.cell);
                        }
                    }
                    let life_owed = !board.is_water(target.cell)
                        && (is_life_essence || pending_thorn_life.is_some());
                    board.activate_thornheart(target.cell, rng.random(), life_owed);
                }
                if is_oil {
                    board.add_oil(target.cell);
                    commands.spawn(effects::EffectSignal(target.cell, effects::Effect::Fuel));
                }
                let poison_acid_reaction = !board.is_frozen(target.cell)
                    && ((is_poison
                        && board.is_acidic(target.cell)
                        && board.ground(target.cell).is_some_and(Ground::walkable))
                        || (is_acid && board.is_poisoned(target.cell)));
                let mut acid_cells = Vec::new();
                if is_acid || poison_acid_reaction {
                    let offsets: &[IVec2] = if poison_acid_reaction {
                        &[
                            IVec2::ZERO,
                            IVec2::new(1, 1),
                            IVec2::new(1, -1),
                            IVec2::new(-1, 1),
                            IVec2::new(-1, -1),
                        ]
                    } else {
                        &[IVec2::ZERO]
                    };
                    for offset in offsets {
                        let cell = target.cell + *offset;
                        if board
                            .ground(cell)
                            .is_none_or(|ground| ground == Ground::Hole)
                        {
                            continue;
                        }
                        board.add_acid(cell);
                        commands.spawn(effects::EffectSignal(cell, effects::Effect::Acid));
                        if poison_acid_reaction && !board.is_frozen(cell) {
                            board.add_poison(cell);
                            commands.spawn(effects::EffectSignal(cell, effects::Effect::Poison));
                        }
                        acid_cells.push(cell);
                    }
                }
                let mut acid_thorns = 0;
                let mut acid_shards = 0;
                if !acid_cells.is_empty() {
                    board.dissolve_thorns(&acid_cells);
                    for (entity, thorn) in &thornhearts {
                        if acid_cells.contains(&thorn.0) {
                            commands.entity(entity).despawn();
                            acid_thorns += 1;
                        }
                    }
                    for (entity, stone) in &stones {
                        if acid_cells.contains(&stone.0) {
                            commands.entity(entity).despawn();
                            board.absorb_acid(stone.0);
                            if board.is_water(stone.0) {
                                board.thaw(stone.0);
                                board.dry(stone.0);
                                board.set(stone.0, Ground::Solid);
                            }
                        }
                    }
                    for (entity, shard) in &shards {
                        if acid_cells.contains(&shard.0) {
                            commands.entity(entity).despawn();
                            board.electrify_acid(shard.0);
                            commands
                                .spawn(effects::EffectSignal(shard.0, effects::Effect::Electric));
                            acid_shards += 1;
                        }
                    }
                    for (entity, stone) in &water_stones {
                        if acid_cells.contains(&stone.0)
                            && board.ground(stone.0) == Some(Ground::Ridge)
                        {
                            commands.entity(entity).despawn();
                            board.set(stone.0, Ground::Solid);
                        }
                    }
                }
                if is_poison
                    && !poison_acid_reaction
                    && !board.is_frozen(target.cell)
                    && board
                        .ground(target.cell)
                        .is_some_and(|ground| ground.walkable())
                {
                    board.add_poison(target.cell);
                    commands.spawn(effects::EffectSignal(target.cell, effects::Effect::Poison));
                }
                let electrified =
                    if is_storm || (existing_shard.is_some() && !creates_ridge && !void_reaction) {
                        board.connected_water(target.cell)
                    } else {
                        Vec::new()
                    };
                let storm_hits_acid = is_storm && board.is_acidic(target.cell);
                if storm_hits_acid {
                    board.electrify_acid(target.cell);
                    commands.spawn(effects::EffectSignal(
                        target.cell,
                        effects::Effect::Electric,
                    ));
                }
                for &cell in &electrified {
                    board.electrify_acid(cell);
                }
                let mut frozen_cells = Vec::new();
                let mut newly_frozen = Vec::new();
                let froze = (is_frost || (water_on_solid && existing_frost.is_some()))
                    && board.is_water(target.cell);
                if froze {
                    for y in -1..=1 {
                        for x in -1..=1 {
                            let cell = target.cell + IVec2::new(x, y);
                            let was_frozen = board.is_frozen(cell);
                            if board.freeze_water(cell) {
                                frozen_cells.push(cell);
                                if !was_frozen {
                                    newly_frozen.push(cell);
                                }
                            }
                        }
                    }
                }
                if let Some(entity) = existing_frost
                    && froze
                {
                    commands.entity(entity).despawn();
                }
                if let Some(entity) = existing_shard
                    && (!electrified.is_empty() || direct_storm)
                {
                    commands.entity(entity).despawn();
                }
                let reacts_as_sunstone =
                    is_sunstone || (existing_sunstone.is_some() && (is_oil || direct_sunstone));
                let ignites_oil = reacts_as_sunstone
                    && board.has_oil(target.cell)
                    && !(is_oil && board.is_water(target.cell) && !board.is_frozen(target.cell));
                let thaws_ice = reacts_as_sunstone && was_frozen && !ignites_oil;
                if thaws_ice {
                    board.thaw(target.cell);
                }
                let makes_steam = (is_oil || water_on_solid)
                    && board.has_oil(target.cell)
                    && board.is_water(target.cell)
                    && !board.is_frozen(target.cell)
                    && !ignites_oil;
                if makes_steam {
                    board.remove_oil(target.cell);
                }
                let mut refreshed_water = Vec::new();
                if water_on_solid {
                    refreshed_water.push(target.cell);
                }
                if let Some(entity) = existing_sunstone
                    && (ignites_oil || direct_sunstone)
                {
                    commands.entity(entity).despawn();
                }
                let mut obsidian_sources: Vec<_> = obsidians
                    .iter()
                    .filter(|(entity, _)| {
                        !(creates_ridge && is_stone && Some(*entity) == pending_obsidian)
                            && !(void_obsidian && is_void && Some(*entity) == pending_obsidian)
                    })
                    .map(|(entity, obsidian)| (Some(entity), obsidian.0))
                    .collect();
                if is_obsidian && !creates_ridge && !void_obsidian {
                    obsidian_sources.push((None, target.cell));
                }
                let mut initial_obsidian = Vec::new();
                for (source_index, &(entity, cell)) in obsidian_sources.iter().enumerate() {
                    let is_direct = cell == target.cell
                        && ((!is_obsidian && entity.is_some())
                            || (is_obsidian && entity.is_none()));
                    let diagonal = (is_direct && direct_storm) || electrified.contains(&cell);
                    let cross = is_direct && direct_sunstone;
                    if diagonal || cross {
                        initial_obsidian.push(ObsidianIgnition {
                            index: source_index,
                            diagonal,
                            cross,
                        });
                    }
                }
                let blast_result = resolve_blasts(
                    &mut board,
                    &obsidian_sources,
                    &initial_obsidian,
                    ignites_oil.then_some(target.cell),
                    (void_obsidian || creates_ridge).then_some(target.cell),
                );
                for (_, player, transform) in &players {
                    if !turn.hovering[player.0.index()]
                        && board_cell(transform.translation.truncate()).is_some_and(|cell| {
                            frozen_cells.contains(&cell) && board.is_frozen(cell)
                        })
                    {
                        frozen_players[player.0.index()] = 2;
                    }
                }
                let direct_frost_hit = is_frost && !void_reaction && !froze;
                let direct_storm_hit =
                    is_storm && electrified.is_empty() && !direct_storm && !storm_hits_acid;
                let reacted_with_deferred = turn.pending_direct_hit.as_ref().is_some_and(|hit| {
                    if hit.frost {
                        target.cell == hit.cell && is_water && froze && existing_frost.is_some()
                    } else {
                        (target.cell == hit.cell
                            && existing_shard.is_some()
                            && (!electrified.is_empty() || direct_storm))
                            || (is_acid && acid_cells.contains(&hit.cell))
                    }
                });
                let mut hit_player = resolve_deferred_hit(
                    &mut turn,
                    &mut commands,
                    players
                        .iter()
                        .map(|(_, player, transform)| (player.0, transform.translation.truncate())),
                    reacted_with_deferred,
                );
                let mut direct_hit_landed = false;
                if direct_frost_hit || direct_storm_hit {
                    let hit = DeferredDirectHit {
                        cell: target.cell,
                        frost: direct_frost_hit,
                        item: None,
                    };
                    if defer_direct_hit {
                        turn.pending_direct_hit = Some(hit);
                    } else {
                        direct_hit_landed = apply_direct_hit(
                            players.iter().map(|(_, player, transform)| {
                                (player.0, transform.translation.truncate())
                            }),
                            &hit,
                            turn.hovering,
                            &mut health_changes,
                            &mut frozen_players,
                        );
                        hit_player |= direct_hit_landed;
                    }
                }
                if direct_hit_landed {
                    commands.spawn(effects::EffectSignal(
                        target.cell,
                        if direct_frost_hit {
                            effects::Effect::Freeze
                        } else {
                            effects::Effect::Electric
                        },
                    ));
                }
                if direct_storm {
                    commands.spawn(effects::EffectSignal(
                        target.cell,
                        effects::Effect::Electric,
                    ));
                }
                let mut thawed_player = false;
                if is_sunstone {
                    for (_, player, transform) in &players {
                        let index = player.0.index();
                        if board_cell(transform.translation.truncate()) == Some(target.cell)
                            && (turn.frozen_turns[index] > 0
                                || turn.pending_effects.frozen_players[index] > 0)
                        {
                            turn.frozen_turns[index] = 0;
                            turn.pending_effects.frozen_players[index] = 0;
                            thawed_player = true;
                        }
                    }
                }
                let consumed_new_obsidian = blast_result.consumed_obsidian.contains(&None);
                for entity in blast_result.consumed_obsidian.iter().flatten() {
                    commands.entity(*entity).despawn();
                }
                refreshed_water.extend(blast_result.refreshed_water);
                let mut blast_cells = Vec::new();
                let oil_sources: Vec<_> = blast_result
                    .blasts
                    .iter()
                    .filter(|blast| matches!(blast.kind, BlastKind::Oil))
                    .map(|blast| blast.source)
                    .collect();
                for blast in &blast_result.blasts {
                    let cells = blast.cells(&board);
                    for (_, player, transform) in &players {
                        if let Some(cell) = board_cell(transform.translation.truncate())
                            && cells.contains(&cell)
                        {
                            let damage = if matches!(blast.kind, BlastKind::Oil | BlastKind::Dust) {
                                if cell == blast.source { 2 } else { 1 }
                            } else {
                                3
                            };
                            health_changes.damage(player.0, damage);
                        }
                    }
                    for cell in cells {
                        if !blast_cells.contains(&cell) {
                            blast_cells.push(cell);
                        }
                        let center = cell_center(cell);
                        commands.spawn((
                            FireFlash(Timer::from_seconds(0.65, TimerMode::Once)),
                            Sprite {
                                color: Color::srgba(
                                    1.0,
                                    0.35,
                                    0.0,
                                    if oil_sources.contains(&cell) {
                                        0.65
                                    } else {
                                        0.35
                                    },
                                ),
                                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                                ..default()
                            },
                            Transform::from_xyz(center.x, center.y, 1.6),
                        ));
                    }
                }
                board.burn_thorns(&blast_cells);
                if makes_steam {
                    let cells = steam_cells(&board, target.cell, turn.wind);
                    for (_, player, transform) in &players {
                        if let Some(cell) = board_cell(transform.translation.truncate())
                            && cells.contains(&cell)
                        {
                            health_changes
                                .damage(player.0, if cell == target.cell { 2 } else { 1 });
                        }
                    }
                    for cell in cells {
                        let center = cell_center(cell);
                        commands.spawn((
                            SteamFlash(Timer::from_seconds(1.0, TimerMode::Once)),
                            Sprite {
                                color: Color::srgba(0.85, 0.9, 0.95, 0.65),
                                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                                ..default()
                            },
                            Transform::from_xyz(center.x, center.y, 1.5),
                        ));
                    }
                }
                if fills_ground && !creates_ridge {
                    board.thaw(target.cell);
                    board.dry(target.cell);
                    board.set(target.cell, Ground::Solid);
                }
                for cell in newly_frozen {
                    if !board.is_frozen(cell) {
                        continue;
                    }
                    let center = cell_center(cell);
                    commands.spawn((
                        FrozenTile(cell),
                        Sprite {
                            color: Color::srgba(0.7, 0.94, 1.0, 0.8),
                            custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                            ..default()
                        },
                        Transform::from_xyz(center.x, center.y, -0.5),
                    ));
                }
                if (!is_water || (water_on_ridge && !corrodes_ridge))
                    && !is_oil
                    && !is_poison
                    && !is_acid
                    && !direct_hit_landed
                    && !(is_oil && ignites_oil)
                    && (!is_storm || (electrified.is_empty() && !direct_storm && !storm_hits_acid))
                    && !(is_frost && froze)
                    && !(is_sunstone
                        && (ignites_oil || direct_sunstone || thaws_ice || thawed_player))
                    && !(is_obsidian && (creates_ridge || consumed_new_obsidian))
                    && !(is_stone && uses_stone)
                    && !(is_void && (void_hole || void_levels_ridge))
                    && !((is_void || is_obsidian || is_life_essence) && void_reaction)
                    && !(is_life_essence && stone_life)
                    && !((is_life_essence || is_egg) && egg_life)
                    && !((is_life_essence || is_aquatic) && aquatic_life)
                    && !(is_life_essence && activated_life_essence)
                    && !(activated_thornheart && (is_thornheart || is_life_essence))
                {
                    let center = cell_center(target.cell);
                    let mut item = commands.spawn((
                        Name::new(INVENTORY_ITEMS[*index].0),
                        BoardItem,
                        PlacedItem,
                        images.sprite(*index, 24.0),
                        Transform::from_xyz(
                            center.x + ITEM_CORNER_OFFSET,
                            center.y + ITEM_CORNER_OFFSET,
                            0.1,
                        ),
                    ));
                    item.insert(Collectible {
                        index: *index,
                        cell: target.cell,
                    });
                    if defer_direct_hit && (direct_frost_hit || direct_storm_hit) {
                        turn.pending_direct_hit
                            .as_mut()
                            .expect("deferred direct hit exists")
                            .item = Some(item.id());
                    }
                    if is_frost {
                        item.insert(PendingFrost(target.cell));
                    }
                    if is_storm {
                        item.insert(PendingStormShard(target.cell));
                    }
                    if is_sunstone {
                        item.insert(PendingSunstone(target.cell));
                    }
                    if is_life_essence && !board.is_water(target.cell) {
                        item.insert(PendingLifeEssence(target.cell));
                    }
                    if is_thornheart {
                        item.insert(PendingThornheart(target.cell));
                    }
                    if is_obsidian {
                        item.insert(PendingObsidian(target.cell));
                    }
                    if is_stone {
                        item.insert(PendingStoneDust(target.cell));
                    }
                    if water_on_ridge {
                        item.insert(PendingWaterStone(target.cell));
                    }
                    if is_void {
                        item.insert(PendingVoidDust(target.cell));
                    }
                    if is_egg {
                        item.insert(PendingCoatlEgg(target.cell));
                    }
                    if is_aquatic {
                        item.insert(PendingAquaticCreature(target.cell));
                    }
                }
                let mut hit_count = 0;
                if !electrified.is_empty() {
                    let mut adjacent = Vec::new();
                    for &cell in &electrified {
                        for y in -1..=1 {
                            for x in -1..=1 {
                                let neighbor = cell + IVec2::new(x, y);
                                if board.ground(neighbor).is_some()
                                    && !electrified.contains(&neighbor)
                                    && !adjacent.contains(&neighbor)
                                {
                                    adjacent.push(neighbor);
                                }
                            }
                        }
                    }
                    for (_, player, transform) in &players {
                        if !turn.hovering[player.0.index()]
                            && let Some(cell) = board_cell(transform.translation.truncate())
                        {
                            let damage = if electrified.contains(&cell) {
                                2
                            } else if adjacent.contains(&cell) {
                                1
                            } else {
                                0
                            };
                            if damage > 0 {
                                health_changes.damage(player.0, damage);
                                hit_count += 1;
                            }
                        }
                    }
                    for (cell, opacity) in electrified
                        .iter()
                        .map(|&cell| (cell, 0.65))
                        .chain(adjacent.into_iter().map(|cell| (cell, 0.5)))
                    {
                        let center = cell_center(cell);
                        commands.spawn((
                            ElectricFlash(Timer::from_seconds(0.65, TimerMode::Once)),
                            Sprite {
                                color: Color::srgba(1.0, 0.9, 0.1, opacity),
                                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                                ..default()
                            },
                            Transform::from_xyz(center.x, center.y, 1.5),
                        ));
                    }
                }
                clear_selection(&mut commands, &mut selection);
                for cell in ridge_cells {
                    assert!(board.make_ridge(cell), "ridge creation was validated");
                }
                for cell in hole_cells {
                    assert!(board.make_hole(cell), "hole creation was validated");
                }
                let positions = [(turn.active, player), (turn.active.other(), opponent)];
                let watered_poison =
                    (watered_poison && board.is_water(target.cell)).then_some(target.cell);
                let effects = TurnEffects {
                    resting: false,
                    refreshed_water,
                    watered_poison: watered_poison.into_iter().collect(),
                    frozen_players,
                    health_changes,
                };
                if turn.next_placement.is_some() {
                    turn.existing_respawns = Some(turn.ingredient_respawns.len());
                    turn.pending_effects.merge(effects);
                    turn.pending_placement = turn.next_placement.take();
                } else {
                    turn.finish_action(&mut board, &mut health, positions, effects);
                }
                if is_acid || is_poison {
                    turn.ingredient_respawns
                        .push((consumed_index, INGREDIENT_RESPAWN_DELAY));
                }
                if direct_hit_landed {
                    turn.ingredient_respawns
                        .push((*index, INGREDIENT_RESPAWN_DELAY));
                }
                if froze && (is_frost || existing_frost.is_some()) {
                    turn.ingredient_respawns
                        .push((FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
                if (!electrified.is_empty() || direct_storm || storm_hits_acid)
                    && (is_storm || existing_shard.is_some())
                {
                    turn.ingredient_respawns
                        .push((STORM_SHARD_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
                if (ignites_oil || direct_sunstone || thaws_ice || thawed_player)
                    && (is_sunstone || existing_sunstone.is_some())
                {
                    turn.ingredient_respawns
                        .push((SUNSTONE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
                if creates_ridge {
                    turn.ingredient_respawns
                        .push((OBSIDIAN_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
                if void_reaction {
                    if void_obsidian {
                        turn.ingredient_respawns
                            .push((OBSIDIAN_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    } else if void_life && (is_life_essence || pending_dust_life.is_some()) {
                        turn.ingredient_respawns
                            .push((LIFE_ESSENCE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    }
                }
                if stone_life && (is_life_essence || pending_dust_life.is_some()) {
                    turn.ingredient_respawns
                        .push((LIFE_ESSENCE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                }
                if egg_life {
                    turn.ingredient_respawns
                        .push((COATL_EGG_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    if is_life_essence || pending_dust_life.is_some() {
                        turn.ingredient_respawns
                            .push((LIFE_ESSENCE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    }
                }
                if aquatic_life {
                    turn.ingredient_respawns
                        .push((AQUATIC_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    if is_life_essence || pending_dust_life.is_some() {
                        turn.ingredient_respawns
                            .push((LIFE_ESSENCE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY));
                    }
                }
                turn.ingredient_respawns.extend(std::iter::repeat_n(
                    (THORNHEART_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
                    acid_thorns,
                ));
                turn.ingredient_respawns.extend(std::iter::repeat_n(
                    (STORM_SHARD_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
                    acid_shards,
                ));
                turn.ingredient_respawns.extend(std::iter::repeat_n(
                    (OBSIDIAN_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
                    blast_result.consumed_obsidian.len(),
                ));
                feedback.0 = if !electrified.is_empty() {
                    format!("Electricity hits {hit_count} player(s) - next player's turn")
                } else if void_obsidian {
                    "Void explosion creates holes - next player's turn".into()
                } else if void_life {
                    "Void and Life Essence begin growing - next player's turn".into()
                } else if stone_life {
                    "Stone Dust and Life Essence begin growing - next player's turn".into()
                } else if egg_life {
                    "Coatl awakens - next player's turn".into()
                } else if aquatic_life {
                    "Ahuizotl awakens - next player's turn".into()
                } else if creates_ridge {
                    "Stone explosion creates ridges - next player's turn".into()
                } else if void_hole {
                    "Void Dust creates a hole - next player's turn".into()
                } else if void_levels_ridge {
                    "Void Dust levels a ridge - next player's turn".into()
                } else if fills_ground {
                    "Stone Dust creates solid ground - next player's turn".into()
                } else if !blast_result.consumed_obsidian.is_empty() {
                    "Obsidian exploded - next player's turn".into()
                } else if makes_steam {
                    "Steam released - next player's turn".into()
                } else if ignites_oil {
                    "Tlexochtli Ember ignited - next player's turn".into()
                } else if froze {
                    "Water frozen - next player's turn".into()
                } else if hit_player {
                    "Direct hit - next player's turn".into()
                } else if is_poison {
                    "Poison Frog Sphere used - next player's turn".into()
                } else if is_acid {
                    "Toad Wart used - next player's turn".into()
                } else if cleared_acid {
                    "Life Essence removes acid - next player's turn".into()
                } else if activated_life_essence {
                    "Water is healing - next player's turn".into()
                } else if water_on_solid {
                    "Tlaloc's Water creates water - next player's turn".into()
                } else if corrodes_ridge {
                    "Tlaloc's Water and acid corrode the ridge - next player's turn".into()
                } else if water_on_ridge {
                    "Tlaloc's Water placed on ridge - next player's turn".into()
                } else if is_oil && new_oil {
                    "Tlexochtli Ember coats the tile - next player's turn".into()
                } else if is_oil {
                    "Tlexochtli Ember refreshed - next player's turn".into()
                } else {
                    "Item used - next player's turn".into()
                };
                return;
            }
            (Some(ActiveSelection::Movement), Some(target))
                if !turn.is_frozen(turn.active)
                    && movement_target(
                        &board,
                        target,
                        opponent,
                        turn.strength[turn.active.index()],
                        turn.hovering[turn.active.index()],
                        turn.magic[turn.active.index()],
                    )
                    .is_ok() =>
            {
                let movement = movement_target(
                    &board,
                    target,
                    opponent,
                    turn.strength[turn.active.index()],
                    turn.hovering[turn.active.index()],
                    turn.magic[turn.active.index()],
                )
                .expect("movement was validated");
                if let Ok((_, _, mut transform)) = players.get_mut(player_entity) {
                    if bot_turn && let Some(history) = &mut history {
                        history.add(
                            format!("Computer moved to {}", history::cell_label(target.cell)),
                            None,
                        );
                    }
                    let active = turn.active;
                    for cell in [board_cell(player), Some(target.cell)]
                        .into_iter()
                        .flatten()
                    {
                        for creature in board.aquatic_contacts(cell) {
                            let contact = (creature, active);
                            if !turn.aquatic_contacts.contains(&contact) {
                                turn.aquatic_contacts.push(contact);
                            }
                        }
                    }
                    if turn.actions_used == 0 {
                        turn.undo = Some(UndoAction::Movement {
                            origin: player,
                            strength: turn.strength[turn.active.index()],
                            hovering: turn.hovering[turn.active.index()],
                            moved_while_hovering: turn.moved_while_hovering,
                            triggered_frost: Vec::new(),
                            previous_pending_freeze: turn.pending_effects.frozen_players
                                [turn.active.index()],
                        });
                    }
                    turn.moved_while_hovering |= turn.hovering[turn.active.index()];
                    let center = cell_center(target.cell);
                    transform.translation.x = center.x;
                    transform.translation.y = center.y;
                    clear_selection(&mut commands, &mut selection);
                    turn.spend_action_energy(u8::from(movement == Movement::Jump));
                    turn.actions_used += 1;
                    if let Some(pending) = &mut turn.pending_placement {
                        pending.ready = true;
                        feedback.0 = "Movement complete - resolving placement".into();
                    } else if turn.actions_used == 2 {
                        let positions = [(turn.active, center), (turn.active.other(), opponent)];
                        turn.finish_action(
                            &mut board,
                            &mut health,
                            positions,
                            TurnEffects::default(),
                        );
                        feedback.0 = "Movement complete - next player's turn".into();
                    } else {
                        feedback.0 = "Movement complete - choose another action or rest".into();
                    }
                }
                return;
            }
            _ => {}
        }
        if human_click && cell.is_some() && active_selection.is_some() {
            sound::request(&mut sounds, sound::Cue::Reject);
        }
    }

    match &selection.0 {
        Some(SelectedAction::Item {
            index,
            icon,
            highlight,
        }) => {
            let placement_error = cell.and_then(|cell| {
                if let Some(first) = turn.pending_placement.as_ref()
                    && let Some(target) = target.as_ref()
                {
                    let positions: Vec<_> = players
                        .iter()
                        .map(|(_, player, transform)| (player.0, transform.translation.truncate()))
                        .collect();
                    let items: Vec<_> = collectibles
                        .iter()
                        .map(|item| (item.index, item.cell))
                        .collect();
                    projected_placement_error(
                        (&board, &turn, &health),
                        &positions,
                        &items,
                        &rng,
                        first,
                        *index,
                        target,
                    )
                } else {
                    item_placement_error(
                        &board,
                        *index,
                        cell,
                        has_dust_reaction(cell),
                        has_frost(cell),
                    )
                    .map(str::to_owned)
                }
                .or_else(|| {
                    (turn.terrain_rejection == Some((*index, cell)))
                        .then(|| "Cannot create terrain: 60% ridge and hole limit".to_owned())
                })
            });
            let can_use = target.as_ref().is_some_and(|target| {
                placement_error.is_none()
                    && u32::from(turn.strength[turn.active.index()]) >= item_energy_cost(target)
            });
            if let Some(position) = cursor.0 {
                commands
                    .entity(*icon)
                    .insert((preview_node(position), Visibility::Visible));
            } else {
                commands.entity(*icon).insert(Visibility::Hidden);
            }
            if let Some(cell) = cell {
                let center = cell_center(cell);
                let color = match target.as_ref().map(|target| &target.kind) {
                    Some(DropKind::Place) if can_use => Color::srgba(0.2, 0.8, 0.3, 0.4),
                    Some(DropKind::Distant) if can_use => Color::srgba(1.0, 0.7, 0.1, 0.4),
                    _ => Color::srgba(0.9, 0.15, 0.1, 0.4),
                };
                commands.entity(*highlight).insert((
                    Sprite {
                        color,
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, 1.0),
                    Visibility::Visible,
                ));
            } else {
                commands.entity(*highlight).insert(Visibility::Hidden);
            }
            feedback.0 = if cursor.0.is_some_and(help::contains_target) {
                format!("Click ? to see {} reactions", INVENTORY_ITEMS[*index].0)
            } else {
                match (&target, placement_error) {
                    (_, Some(message)) => message,
                    (Some(target), None) if !can_use => format!(
                        "Not enough energy - needs {} - choose Move, a nearby tile, or rest",
                        item_energy_cost(target)
                    ),
                    (Some(target), None) => format!(
                        "{}: PLACE - {} grid {} - {} energy - click to confirm",
                        INVENTORY_ITEMS[*index].0,
                        target.distance,
                        if target.distance == 1 {
                            "step"
                        } else {
                            "steps"
                        },
                        item_energy_cost(target),
                    ),
                    (None, None) if cell.is_some() => "Click Move to select movement".into(),
                    (None, None) => "Click a board cell, or click the item again to cancel".into(),
                }
            };
        }
        Some(SelectedAction::Movement { .. }) => {
            feedback.0 = match &target {
                Some(target) => match movement_target(
                    &board,
                    target,
                    opponent,
                    turn.strength[turn.active.index()],
                    turn.hovering[turn.active.index()],
                    turn.magic[turn.active.index()],
                ) {
                    Ok(Movement::Walk) => "Click to walk - 1 grid step".into(),
                    Ok(Movement::Jump) => "Click to jump - 1 energy".into(),
                    Err(MovementError::OutOfRange) => "Too far - jump up to 2 grid steps".into(),
                    Err(MovementError::Blocked) => "Cannot land on a ridge or hole".into(),
                    Err(MovementError::Occupied) => "Cell occupied by opponent".into(),
                    Err(MovementError::InsufficientEnergy) => {
                        "Not enough energy - choose Move, a nearby tile, or rest".into()
                    }
                    Err(MovementError::RidgeBlocksJump) => "Cannot jump over a ridge".into(),
                    Err(MovementError::UnsafeHover) => {
                        "Need magic and two safe exits to hover here".into()
                    }
                },
                None => "Green: walk | yellow: jump (1 energy) | click Move to cancel".into(),
            };
        }
        None => {}
    }
}

fn pending_placement_ready(turn: Res<TurnState>) -> bool {
    turn.pending_placement.as_ref().is_some_and(|p| p.ready)
}

fn begin_turn_frame(mut turn: ResMut<TurnState>) {
    turn.acted_this_frame = false;
    turn.completed_turn = false;
}

fn advance_serpent_animation(
    time: Res<Time>,
    mut turn: ResMut<TurnState>,
    mut health: ResMut<PlayerHealth>,
    mut sounds: Option<ResMut<sound::SoundCues>>,
) {
    if turn.aquatic_animation.is_some() || turn.effects_active {
        return;
    }
    let Some(animation) = &mut turn.serpent_animation else {
        return;
    };
    if !animation.timer.tick(time.delta()).just_finished() {
        return;
    }
    if let Some(frame) = animation.frames.pop_front() {
        if frame.hit.is_none() && !frame.visuals.is_empty() {
            sound::request(&mut sounds, sound::Cue::Organic);
        }
        animation.visuals = frame.visuals;
        animation.impact = frame.impact;
        if let Some((player, remaining)) = frame.hit {
            health.0[player] = remaining;
        }
        animation.timer = Timer::from_seconds(
            if animation.impact.is_some() {
                SERPENT_IMPACT_SECONDS
            } else {
                SERPENT_STEP_SECONDS
            },
            TimerMode::Once,
        );
    } else {
        turn.serpent_animation = None;
    }
}

fn advance_aquatic_animation(
    time: Res<Time>,
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    items: Query<(Entity, &Collectible)>,
    mut sounds: Option<ResMut<sound::SoundCues>>,
) {
    if turn.effects_active {
        return;
    }
    let Some(animation) = &mut turn.aquatic_animation else {
        return;
    };
    if !animation.timer.tick(time.delta()).just_finished() {
        return;
    }
    if let Some(frame) = animation.frames.pop_front() {
        if frame
            .creatures
            .iter()
            .any(|creature| creature.sweep_frame == Some(3))
        {
            sound::request(&mut sounds, sound::Cue::Whoosh);
        }
        animation.visuals = frame.visuals;
        animation.creatures = frame.creatures;
        for cell in frame.flattened {
            animation.unflattened.remove(&cell);
        }
        animation.timer = Timer::from_seconds(AQUATIC_STEP_SECONDS, TimerMode::Once);
        let swept = frame.swept;
        for (entity, item) in &items {
            if swept.contains(&item.cell) {
                commands.entity(entity).despawn();
                if !is_unlimited(item.index) {
                    turn.ingredient_respawns
                        .push((item.index, INGREDIENT_RESPAWN_DELAY));
                }
            }
        }
    } else {
        turn.aquatic_animation = None;
    }
}

fn initialize_weather(mut weather: ResMut<Weather>, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    weather.rain = rng.random_range(RAIN_INTERVAL);
    weather.earthquake = rng.random_range(EARTHQUAKE_INTERVAL);
}

fn queue_weather(
    mut turn: ResMut<TurnState>,
    mut weather: ResMut<Weather>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    if !turn.completed_turn {
        return;
    }
    weather.rain -= 1;
    weather.earthquake -= 1;
    if weather.rain != 0 && weather.earthquake != 0 {
        return;
    }
    let mut pending = std::collections::VecDeque::new();
    if weather.rain == 0 {
        weather.rain = rng.random_range(RAIN_INTERVAL);
        pending.push_back(WeatherKind::Rain);
    }
    if weather.earthquake == 0 {
        weather.earthquake = rng.random_range(EARTHQUAKE_INTERVAL);
        pending.push_back(WeatherKind::Earthquake);
    }
    turn.weather_animation = Some(WeatherAnimation {
        pending,
        phase: WeatherPhase::Waiting,
        notices: Vec::new(),
    });
}

#[derive(SystemParam)]
struct WeatherVisuals<'w, 's> {
    overlays: Query<'w, 's, &'static mut Sprite, With<RainOverlay>>,
    feedback: Query<'w, 's, &'static mut Text, With<DropFeedback>>,
    sounds: Option<ResMut<'w, sound::SoundCues>>,
}

fn advance_weather(
    mut commands: Commands,
    time: Res<Time>,
    mut turn: ResMut<TurnState>,
    mut board: ResMut<Board>,
    actors: (Query<&Transform, With<Player>>, Query<&Collectible>),
    visuals: WeatherVisuals,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let (players, collectibles) = actors;
    let WeatherVisuals {
        mut overlays,
        mut feedback,
        mut sounds,
    } = visuals;
    let Some(mut animation) = turn.weather_animation.take() else {
        return;
    };
    if turn.effects_active || turn.aquatic_animation.is_some() || turn.serpent_animation.is_some() {
        turn.weather_animation = Some(animation);
        return;
    }
    let kind = match &mut animation.phase {
        WeatherPhase::Waiting => {
            let kind = animation.pending.pop_front().expect("queued weather event");
            sound::request(
                &mut sounds,
                match kind {
                    WeatherKind::Rain => sound::Cue::Water,
                    WeatherKind::Earthquake => sound::Cue::Explosion,
                },
            );
            let overlay = if kind == WeatherKind::Rain {
                Some(
                    commands
                        .spawn((
                            RainOverlay,
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
                        ))
                        .id(),
                )
            } else {
                None
            };
            animation.phase = WeatherPhase::Playing {
                kind,
                elapsed: 0.0,
                overlay,
            };
            turn.weather_animation = Some(animation);
            return;
        }
        WeatherPhase::Playing {
            kind,
            elapsed,
            overlay,
        } => {
            *elapsed += time.delta_secs();
            if let Some(entity) = *overlay
                && let Ok(mut sprite) = overlays.get_mut(entity)
            {
                let progress = (*elapsed / WEATHER_ANIMATION_SECONDS).clamp(0.0, 1.0);
                let alpha = 0.4 * (1.0 - (2.0 * progress - 1.0).abs());
                sprite.color = Color::srgba(0.1, 0.4, 1.0, alpha);
            }
            if *elapsed < WEATHER_ANIMATION_SECONDS {
                turn.weather_animation = Some(animation);
                return;
            }
            if let Some(entity) = overlay.take() {
                commands.entity(entity).despawn();
            }
            *kind
        }
        WeatherPhase::Resolving => {
            if animation.pending.is_empty() {
                return;
            }
            animation.phase = WeatherPhase::Waiting;
            turn.weather_animation = Some(animation);
            return;
        }
    };
    let mut occupied: Vec<_> = collectibles.iter().map(|item| item.cell).collect();
    occupied.extend(
        players
            .iter()
            .filter_map(|transform| board_cell(transform.translation.truncate())),
    );
    occupied.extend(
        turn.serpents
            .iter()
            .flat_map(|serpent| serpent.visual().cells),
    );
    if let Some(animation) = &turn.serpent_animation {
        occupied.extend(
            animation
                .visuals
                .iter()
                .chain(animation.frames.iter().flat_map(|frame| &frame.visuals))
                .flat_map(|visual| visual.cells.iter().copied()),
        );
    }
    if let Some(animation) = &turn.aquatic_animation {
        occupied.extend(animation.swept.iter().copied());
    }
    let clean_cells = |board: &Board| {
        (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .filter(|cell| {
                board.ground(*cell) == Some(Ground::Solid)
                    && !board.is_wet(*cell)
                    && !board.is_frozen(*cell)
                    && !board.has_oil(*cell)
                    && !board.is_poisoned(*cell)
                    && !board.is_acidic(*cell)
                    && !board.has_thorns(*cell)
                    && !board.aquatic_cells().any(|part| part == *cell)
                    && !board.terrain_growth_cells().any(|part| part == *cell)
                    && !occupied.contains(cell)
            })
            .collect::<Vec<_>>()
    };
    if kind == WeatherKind::Rain {
        let water_count = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .filter(|cell| board.ground(*cell) == Some(Ground::Water))
            .count();
        let candidates = clean_cells(&board);
        if water_count * 100 < ROWS * COLUMNS * 33 && !candidates.is_empty() {
            let cell = candidates[rng.random_range(0..candidates.len())];
            board.make_water(cell);
            animation
                .notices
                .push(format!("Rain: water at {}", history::cell_label(cell)));
        } else {
            animation.notices.push("Rain: no new water".into());
        }
    }
    if kind == WeatherKind::Earthquake {
        let mut changes = Vec::new();
        for _ in 0..EARTHQUAKE_TILES {
            let candidates: Vec<_> = clean_cells(&board)
                .into_iter()
                .filter(|cell| {
                    board.can_create_unsafe(&[*cell]) && board.can_block_without_disconnect(*cell)
                })
                .collect();
            if candidates.is_empty() {
                break;
            }
            let cell = candidates[rng.random_range(0..candidates.len())];
            let ground = if rng.random_bool(0.5) {
                Ground::Ridge
            } else {
                Ground::Hole
            };
            board.set(cell, ground);
            let kind = if ground == Ground::Ridge {
                "ridge"
            } else {
                "hole"
            };
            changes.push(format!("{kind} {}", history::cell_label(cell)));
        }
        animation.notices.push(if changes.is_empty() {
            "Earthquake: no safe terrain changes".into()
        } else {
            format!("Earthquake: {}", changes.join(", "))
        });
    }
    if let Ok(mut feedback) = feedback.single_mut() {
        feedback.0 = animation.notices.join(" | ");
    }
    animation.phase = WeatherPhase::Resolving;
    turn.weather_animation = Some(animation);
}

type WeatherTileSprite = (
    &'static mut Transform,
    Option<&'static GroundTile>,
    Option<&'static FrozenTile>,
    Option<&'static PoisonTile>,
    Option<&'static AcidTile>,
    Option<&'static VoidGrowthTile>,
    Option<&'static ThornTile>,
    Option<&'static LifeEssenceTile>,
);
type WeatherTileFilter = Or<(
    With<GroundTile>,
    With<FrozenTile>,
    With<PoisonTile>,
    With<AcidTile>,
    With<VoidGrowthTile>,
    With<ThornTile>,
    With<LifeEssenceTile>,
)>;

fn shake_ground(
    turn: Res<TurnState>,
    mut was_shaking: Local<bool>,
    mut tiles: Query<WeatherTileSprite, WeatherTileFilter>,
) {
    let elapsed = turn.weather_animation.as_ref().and_then(|animation| {
        if let WeatherPhase::Playing {
            kind: WeatherKind::Earthquake,
            elapsed,
            ..
        } = animation.phase
        {
            Some(elapsed)
        } else {
            None
        }
    });
    if elapsed.is_none() && !*was_shaking {
        return;
    }
    let offset = elapsed.map_or(0.0, |elapsed| {
        if ((elapsed / EARTHQUAKE_FRAME_SECONDS) as usize).is_multiple_of(2) {
            EARTHQUAKE_OFFSET
        } else {
            -EARTHQUAKE_OFFSET
        }
    });
    for (mut transform, ground, frozen, poison, acid, growth, thorn, life) in &mut tiles {
        let (cell, icon_offset) = if let Some(tile) = ground {
            (tile.0, 0.0)
        } else if let Some(tile) = frozen {
            (tile.0, 0.0)
        } else if let Some(tile) = poison {
            (tile.0, 0.0)
        } else if let Some(tile) = acid {
            (tile.0, 0.0)
        } else if let Some(tile) = growth {
            (tile.0, 0.0)
        } else if let Some(tile) = thorn {
            (tile.0, 0.0)
        } else if let Some(tile) = life {
            (tile.0, -17.0)
        } else {
            unreachable!("weather tile query requires a tile marker");
        };
        transform.translation.x = cell_center(cell).x + icon_offset + offset;
    }
    *was_shaking = elapsed.is_some();
}

fn initialize_wind(mut turn: ResMut<TurnState>, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    turn.wind = WindDirection::random(rng.as_mut());
}

fn advance_wind(mut turn: ResMut<TurnState>, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    if turn.acted_this_frame {
        turn.wind = WindDirection::random(rng.as_mut());
    }
}

fn update_turn_progress(
    turn: Res<TurnState>,
    bot: Option<Res<ai::BotController>>,
    mut status: Query<&mut Text, With<TurnProgress>>,
) {
    if let Ok(mut status) = status.single_mut() {
        let mut text = format!(
            "ACTIONS: {} AVAILABLE | REST: {}",
            2 - turn.actions_used,
            if turn.actions_used > 0 { "+2" } else { "FULL" },
        );
        let event = if turn.aquatic_animation.is_some() {
            Some("AHUIZOTL MOVING...".to_owned())
        } else if turn.serpent_animation.is_some() {
            Some("COATL MOVING...".to_owned())
        } else if let Some(weather) = &turn.weather_animation {
            Some(
                match weather.phase {
                    WeatherPhase::Playing {
                        kind: WeatherKind::Rain,
                        ..
                    } => "RAIN...",
                    WeatherPhase::Playing {
                        kind: WeatherKind::Earthquake,
                        ..
                    } => "EARTHQUAKE...",
                    _ => "WEATHER APPROACHING...",
                }
                .to_owned(),
            )
        } else if turn.is_frozen(turn.active) {
            Some(format!(
                "FROZEN: {} TURN(S)",
                turn.frozen_turns[turn.active.index()]
                    .max(turn.pending_effects.frozen_players[turn.active.index()])
            ))
        } else {
            None
        };
        if let Some(event) = event {
            text.push_str(" | ");
            text.push_str(&event);
        }
        if bot.as_ref().is_some_and(|bot| bot.computer_mode) && turn.active == PlayerId::Two {
            text.insert_str(0, "COMPUTER | ");
        }
        if status.0 != text {
            status.0 = text;
        }
    }
}

fn update_wind_status(turn: Res<TurnState>, mut status: Query<&mut Text, With<WindStatus>>) {
    if let Ok(mut status) = status.single_mut() {
        status.0 = format!("Wind: {}", turn.wind.label());
    }
}

fn update_energy_bars(turn: Res<TurnState>, mut bars: Query<(&EnergyFill, &mut Node)>) {
    for (fill, mut node) in &mut bars {
        node.width = Val::Percent(
            100.0 * f32::from(turn.strength[fill.0.index()]) / f32::from(MAX_STRENGTH),
        );
    }
}

fn update_magic_bars(turn: Res<TurnState>, mut bars: Query<(&MagicFill, &mut Node)>) {
    for (fill, mut node) in &mut bars {
        node.width =
            Val::Percent(100.0 * f32::from(turn.magic[fill.0.index()]) / f32::from(MAX_MAGIC));
    }
}

fn update_pickup_highlight(
    mut commands: Commands,
    turn: Res<TurnState>,
    board: Res<Board>,
    players: Query<(&Player, &Transform), Without<PickupHighlight>>,
    items: Query<&Collectible>,
    mut highlights: Query<(Entity, &mut Transform), With<PickupHighlight>>,
) {
    let position = players.iter().find_map(|(player, transform)| {
        let position = transform.translation.truncate();
        (player.0 == turn.active && available_pickup(&turn, &board, position, &items))
            .then_some(position)
    });
    match (position, highlights.single_mut()) {
        (Some(position), Ok((_, mut transform))) => {
            transform.translation = position.extend(-0.2);
        }
        (Some(position), Err(_)) => {
            commands.spawn((
                PickupHighlight,
                Sprite {
                    color: Color::srgba(0.2, 1.0, 0.35, 0.7),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                    ..default()
                },
                Transform::from_translation(position.extend(-0.2)),
            ));
        }
        (None, Ok((entity, _))) => commands.entity(entity).despawn(),
        (None, Err(_)) => {}
    }
}

fn update_hearts(
    health: Res<PlayerHealth>,
    mut hearts: Query<(&Heart, &mut ImageNode, &mut Visibility)>,
) {
    for (heart, mut image, mut visibility) in &mut hearts {
        let remaining = health.0[heart.owner.index()].saturating_sub((heart.index * 2) as u8);
        *visibility = if remaining == 0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        image.rect = Some(if remaining == 1 {
            Rect::new(32.0, 0.0, 64.0, 32.0)
        } else {
            Rect::new(0.0, 0.0, 32.0, 32.0)
        });
    }
}

fn update_electric_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut ElectricFlash)>,
) {
    for (entity, mut flash) in &mut flashes {
        if flash.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_reaction_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut steam: Query<(Entity, &mut SteamFlash)>,
    mut fire: Query<(Entity, &mut FireFlash)>,
) {
    for (entity, mut flash) in &mut steam {
        if flash.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut flash) in &mut fire {
        if flash.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

type FrostImpactSprite = (
    Entity,
    &'static FrostImpact,
    &'static mut FrozenAnimation,
    &'static mut Sprite,
    &'static mut Transform,
    &'static mut Visibility,
);
type FrostImpactFilter = (Without<Player>, Without<FrozenTile>);

#[derive(SystemParam)]
struct FrozenAssets<'w> {
    time: Res<'w, Time>,
    sheet: Res<'w, FrozenSheet>,
    effects: Option<Res<'w, effects::Effects>>,
}

fn update_frozen_tiles(
    mut commands: Commands,
    board: Res<Board>,
    turn: Res<TurnState>,
    assets: FrozenAssets,
    mut tiles: Query<(Entity, &FrozenTile, &mut Visibility), Without<FrostImpact>>,
    mut impacts: Query<FrostImpactSprite, FrostImpactFilter>,
    players: Query<(&Player, &Transform), Without<FrostImpact>>,
) {
    let FrozenAssets {
        time,
        sheet,
        effects,
    } = assets;
    for (entity, tile, mut visibility) in &mut tiles {
        let visible = effects
            .as_ref()
            .and_then(|effects| effects.held(tile.0))
            .map_or_else(|| board.is_frozen(tile.0), |old| old.frozen);
        if !visible && !board.is_frozen(tile.0) {
            commands.entity(entity).despawn();
        } else {
            *visibility = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    let mut displayed = [false; 2];
    for (entity, impact, mut animation, mut sprite, mut transform, mut visibility) in &mut impacts {
        let index = impact.0.index();
        if !turn.is_frozen(impact.0) {
            commands.entity(entity).despawn();
        } else if let Some((_, player_transform)) =
            players.iter().find(|(player, _)| player.0 == impact.0)
        {
            transform.translation.x = player_transform.translation.x;
            transform.translation.y = player_transform.translation.y;
            let hidden = board_cell(player_transform.translation.truncate()).is_some_and(|cell| {
                effects
                    .as_ref()
                    .is_some_and(|effects| effects.animating(cell, effects::Effect::Freeze))
            });
            if !hidden && *visibility != Visibility::Hidden {
                animation.timer.tick(time.delta());
                animation.frame = (animation.frame
                    + animation.timer.times_finished_this_tick() as usize)
                    .min(FROZEN_LAST_FRAME);
                sprite.rect = Some(frozen_frame_rect(animation.frame));
            }
            *visibility = if hidden {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            displayed[index] = true;
        }
    }
    for (player, transform) in &players {
        let index = player.0.index();
        if turn.is_frozen(player.0) && !displayed[index] {
            commands.spawn((
                FrostImpact(player.0),
                FrozenAnimation::default(),
                Sprite {
                    image: sheet.0.clone(),
                    rect: Some(frozen_frame_rect(0)),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                    ..default()
                },
                if board_cell(transform.translation.truncate()).is_some_and(|cell| {
                    effects
                        .as_ref()
                        .is_some_and(|effects| effects.animating(cell, effects::Effect::Freeze))
                }) {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
                Transform::from_xyz(transform.translation.x, transform.translation.y, 0.9),
            ));
        }
    }
}

fn frozen_frame_rect(frame: usize) -> Rect {
    let x = frame as f32 * FROZEN_FRAME_SIZE;
    Rect::new(x, 0.0, x + FROZEN_FRAME_SIZE, FROZEN_FRAME_SIZE)
}

fn update_ground_tiles(
    board: Res<Board>,
    turn: Res<TurnState>,
    mut tiles: Query<(&GroundTile, &mut Sprite)>,
) {
    if board.is_changed() || turn.is_changed() {
        for (tile, mut sprite) in &mut tiles {
            sprite.color = if turn
                .aquatic_animation
                .as_ref()
                .is_some_and(|animation| animation.unflattened.contains(&tile.0))
            {
                Ground::Ridge.color()
            } else {
                board
                    .ground(tile.0)
                    .expect("terrain tile must be in bounds")
                    .color()
            };
        }
    }
}

fn update_water_effects(
    mut commands: Commands,
    board: Res<Board>,
    icons: Query<(Entity, &WaterIcon)>,
    mut labels: Query<(&WaterLabel, &mut Text2d)>,
) {
    for (entity, icon) in &icons {
        if !board.is_wet(icon.0) {
            commands.entity(entity).despawn();
        }
    }
    for (label, mut text) in &mut labels {
        let percentage = board.water_percentage(label.0);
        if percentage > 0 {
            text.0 = format!("{percentage}%");
        }
    }
}

fn update_poison_visuals(
    mut commands: Commands,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    tiles: Query<(Entity, &PoisonTile)>,
) {
    let visible = |cell| {
        effects
            .as_ref()
            .and_then(|effects| effects.held(cell))
            .map_or_else(|| board.is_poisoned(cell), |old| old.poison)
    };
    let mut displayed = Vec::new();
    for (entity, tile) in &tiles {
        if visible(tile.0) {
            displayed.push(tile.0);
        } else {
            commands.entity(entity).despawn();
        }
    }
    for y in 0..ROWS {
        for x in 0..COLUMNS {
            let cell = IVec2::new(x as i32, y as i32);
            if visible(cell) && !displayed.contains(&cell) {
                let center = cell_center(cell);
                commands.spawn((
                    PoisonTile(cell),
                    Sprite {
                        color: Color::srgba(0.65, 0.1, 0.75, 0.55),
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, -0.25),
                ));
            }
        }
    }
}

fn update_acid_visuals(
    mut commands: Commands,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    mut tiles: Query<(Entity, &AcidTile, &mut Sprite)>,
) {
    let visible = |cell| {
        effects
            .as_ref()
            .and_then(|effects| effects.held(cell))
            .map_or_else(|| board.is_acidic(cell), |old| old.acid)
    };
    let mut displayed = Vec::new();
    for (entity, tile, mut sprite) in &mut tiles {
        if visible(tile.0) {
            displayed.push(tile.0);
            sprite.color = if board.is_electrified_acid(tile.0) {
                Color::srgba(0.9, 0.9, 0.05, 0.55)
            } else {
                Color::srgba(0.25, 0.8, 0.1, 0.55)
            };
        } else {
            commands.entity(entity).despawn();
        }
    }
    for y in 0..ROWS {
        for x in 0..COLUMNS {
            let cell = IVec2::new(x as i32, y as i32);
            if visible(cell) && !displayed.contains(&cell) {
                let center = cell_center(cell);
                commands.spawn((
                    AcidTile(cell),
                    Sprite {
                        color: if board.is_electrified_acid(cell) {
                            Color::srgba(0.9, 0.9, 0.05, 0.55)
                        } else {
                            Color::srgba(0.25, 0.8, 0.1, 0.55)
                        },
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, -0.3),
                ));
            }
        }
    }
}

fn update_void_growth_visuals(
    mut commands: Commands,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    tiles: Query<(Entity, &VoidGrowthTile)>,
) {
    let cells: Vec<_> = (0..ROWS)
        .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
        .filter(|&cell| {
            effects
                .as_ref()
                .and_then(|effects| effects.held(cell))
                .map_or_else(|| board.terrain_growth_kind_at(cell), |old| old.growth)
                .is_some()
        })
        .collect();
    let mut displayed = Vec::new();
    for (entity, tile) in &tiles {
        if cells.contains(&tile.0) {
            displayed.push(tile.0);
        } else {
            commands.entity(entity).despawn();
        }
    }
    for cell in cells {
        if !displayed.contains(&cell) {
            displayed.push(cell);
            let center = cell_center(cell);
            commands.spawn((
                VoidGrowthTile(cell),
                Sprite {
                    color: Color::srgba(0.25, 0.05, 0.4, 0.6),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, -0.15),
            ));
        }
    }
}

fn update_life_essence_visuals(
    mut commands: Commands,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    images: Res<InventoryImages>,
    tiles: Query<(Entity, &LifeEssenceTile)>,
) {
    let visible = |cell| {
        effects
            .as_ref()
            .and_then(|effects| effects.held(cell))
            .map_or_else(|| board.has_life_essence(cell), |old| old.healing)
    };
    let mut displayed = Vec::new();
    for (entity, tile) in &tiles {
        if visible(tile.0) {
            displayed.push(tile.0);
        } else {
            commands.entity(entity).despawn();
        }
    }
    for y in 0..ROWS {
        for x in 0..COLUMNS {
            let cell = IVec2::new(x as i32, y as i32);
            if visible(cell) && !displayed.contains(&cell) {
                let center = cell_center(cell);
                commands.spawn((
                    LifeEssenceTile(cell),
                    images.sprite(LIFE_ESSENCE_ITEM_INDEX, 24.0),
                    Transform::from_xyz(center.x - 17.0, center.y + 16.0, 0.2),
                ));
            }
        }
    }
}

fn update_thorn_visuals(
    mut commands: Commands,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    tiles: Query<(Entity, &ThornTile)>,
) {
    let visible = |cell| {
        effects
            .as_ref()
            .and_then(|effects| effects.held(cell))
            .map_or_else(|| board.has_thorns(cell), |old| old.thorns)
    };
    let mut displayed = Vec::new();
    for (entity, tile) in &tiles {
        if visible(tile.0) {
            displayed.push(tile.0);
        } else {
            commands.entity(entity).despawn();
        }
    }
    for y in 0..ROWS {
        for x in 0..COLUMNS {
            let cell = IVec2::new(x as i32, y as i32);
            if visible(cell) && !displayed.contains(&cell) {
                let center = cell_center(cell);
                commands.spawn((
                    ThornTile(cell),
                    Sprite {
                        color: Color::srgb(0.12, 0.55, 0.16),
                        custom_size: Some(Vec2::new(12.0, GRID_BLOCK_SIZE - 8.0)),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, -0.3),
                ));
            }
        }
    }
}

fn aquatic_sprite(
    visual: &aquatic::AquaticVisual,
    part: usize,
    frame: usize,
    image: Handle<Image>,
) -> (Sprite, Transform) {
    let turns = if part == 0 { 0 } else { part % 4 };
    let rotate = |mut point: IVec2| {
        for _ in 0..turns {
            point = IVec2::new(-point.y, point.x);
        }
        point
    };
    let mut min = IVec2::splat(6);
    let mut max = IVec2::ZERO;
    for y in 0..7 {
        for x in 0..7 {
            let source = IVec2::new(x, y);
            let cell = visual.origin + rotate(source - IVec2::splat(3));
            if (0..COLUMNS as i32).contains(&cell.x) && (0..ROWS as i32).contains(&cell.y) {
                min = min.min(source);
                max = max.max(source);
            }
        }
    }
    let slot = match frame {
        0..=8 => frame,
        9 => 5,
        10..=33 => frame - 1,
        34 => 23,
        _ => frame - 2,
    };
    let sheet_origin = Vec2::new((slot % 7) as f32 * 224.0, (slot / 7) as f32 * 224.0);
    let rect_min = sheet_origin + min.as_vec2() * 32.0;
    let rect_max = sheet_origin + (max + IVec2::ONE).as_vec2() * 32.0;
    let mut offset = (min.as_vec2() + max.as_vec2()) / 2.0 - Vec2::splat(3.0);
    for _ in 0..turns {
        offset = Vec2::new(-offset.y, offset.x);
    }
    let center = cell_center(visual.origin) + Vec2::new(offset.x, -offset.y) * GRID_BLOCK_SIZE;
    (
        Sprite {
            image,
            rect: Some(Rect::from_corners(rect_min, rect_max)),
            custom_size: Some((rect_max - rect_min) * (GRID_BLOCK_SIZE / 32.0)),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, if part == 0 { -0.18 } else { -0.2 })
            .with_rotation(Quat::from_rotation_z(
                -(turns as f32) * std::f32::consts::FRAC_PI_2,
            )),
    )
}

fn aquatic_arm_frame(visual: &aquatic::AquaticVisual, part: usize) -> usize {
    let length = visual.lengths[part - 1];
    visual
        .sweep_frame
        .map_or(14 + length - 1, |step| [0, 34, 24, 14][length] + step)
}

fn update_aquatic_visuals(
    mut commands: Commands,
    board: Res<Board>,
    turn: Res<TurnState>,
    time: Res<Time>,
    sheet: Res<AquaticSheet>,
    mut tiles: Query<(Entity, &mut AquaticTile, &mut Sprite, &mut Transform)>,
) {
    let creatures: Vec<_> = if let Some(animation) = &turn.aquatic_animation {
        animation.creatures.clone()
    } else {
        board.aquatic_visuals().collect()
    };
    let mut displayed = Vec::new();
    for (entity, mut tile, mut sprite, mut transform) in &mut tiles {
        let Some(visual) = creatures.iter().find(|visual| visual.id == tile.id) else {
            commands.entity(entity).despawn();
            continue;
        };
        if tile.part > 0 && visual.lengths[tile.part - 1] == 0 {
            commands.entity(entity).despawn();
            continue;
        }
        if tile.part == 0 {
            tile.timer.tick(time.delta());
            for _ in 0..tile.timer.times_finished_this_tick() {
                tile.frame = if tile.frame == 13 { 3 } else { tile.frame + 1 };
            }
        }
        let frame = if tile.part == 0 {
            tile.frame
        } else {
            aquatic_arm_frame(visual, tile.part)
        };
        (*sprite, *transform) = aquatic_sprite(visual, tile.part, frame, sheet.0.clone());
        displayed.push((tile.id, tile.part));
    }
    for visual in &creatures {
        for part in 0..=4 {
            if (part > 0 && visual.lengths[part - 1] == 0) || displayed.contains(&(visual.id, part))
            {
                continue;
            }
            let frame = if part == 0 {
                0
            } else {
                aquatic_arm_frame(visual, part)
            };
            let (sprite, transform) = aquatic_sprite(visual, part, frame, sheet.0.clone());
            commands.spawn((
                AquaticTile {
                    id: visual.id,
                    part,
                    frame: 0,
                    timer: Timer::from_seconds(AQUATIC_STEP_SECONDS, TimerMode::Repeating),
                },
                sprite,
                transform,
            ));
        }
    }
}

fn rest(
    selection: (Commands, ResMut<Selection>),
    input: (Res<ButtonInput<KeyCode>>, Option<Res<ai::BotController>>),
    mut turn: ResMut<TurnState>,
    mut board: ResMut<Board>,
    mut health: ResMut<PlayerHealth>,
    players: Query<(&Player, &Transform)>,
    output: (
        Query<&mut Text, With<DropFeedback>>,
        Option<ResMut<sound::SoundCues>>,
    ),
) {
    let (mut commands, mut selection) = selection;
    let (keyboard, bot) = input;
    let (mut feedback, mut sounds) = output;
    if !bot.as_ref().is_some_and(|bot| bot.bot_turn)
        && keyboard.just_pressed(KeyCode::KeyR)
        && !turn.acted_this_frame
        && !turn.animation_active()
        && !turn.pending_placement.as_ref().is_some_and(|p| p.ready)
    {
        match finish_rest(&mut turn, &mut board, &mut health, &players) {
            Ok(()) => {
                clear_selection(&mut commands, &mut selection);
                sound::request(&mut sounds, sound::Cue::Chime);
            }
            Err(message) => {
                if let Ok(mut feedback) = feedback.single_mut() {
                    feedback.0 = message.into();
                }
                sound::request(&mut sounds, sound::Cue::Reject);
            }
        }
    }
}

fn finish_rest(
    turn: &mut TurnState,
    board: &mut Board,
    health: &mut PlayerHealth,
    players: &Query<(&Player, &Transform)>,
) -> Result<(), &'static str> {
    assert!(
        players.iter().any(|(player, _)| player.0 == turn.active),
        "active player must exist"
    );
    if players.iter().any(|(player, transform)| {
        player.0 == turn.active && must_land(turn, board, transform.translation.truncate())
    }) {
        return Err("Land on safe ground before magic runs out");
    }
    if let Some(pending) = &mut turn.pending_placement {
        pending.ready = true;
        turn.partial_rest = true;
        return Ok(());
    }
    let positions = players
        .iter()
        .map(|(player, transform)| (player.0, transform.translation.truncate()));
    turn.finish_action(
        board,
        health,
        positions,
        TurnEffects {
            resting: true,
            ..default()
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn inventory_position(owner: PlayerId, item: usize) -> Vec2 {
        let slot = INVENTORY_ORDER
            .iter()
            .position(|&index| index == item)
            .unwrap();
        let left = match owner {
            PlayerId::One => -RESOLUTION_WIDTH / 2.0,
            PlayerId::Two => RESOLUTION_WIDTH / 2.0 - 2.0 * GRID_BLOCK_SIZE,
        };
        Vec2::new(
            left + (slot % 2) as f32 * GRID_BLOCK_SIZE + GRID_BLOCK_SIZE / 2.0,
            BOARD_TOP - (slot / 2) as f32 * GRID_BLOCK_SIZE - GRID_BLOCK_SIZE / 2.0,
        )
    }

    fn p1_item(item: usize) -> Vec2 {
        inventory_position(PlayerId::One, item)
    }

    fn p2_item(item: usize) -> Vec2 {
        inventory_position(PlayerId::Two, item)
    }

    #[test]
    fn inventory_slots_select_the_requested_item_for_both_players() {
        let expected = [
            WATER_ITEM_INDEX,
            OIL_ITEM_INDEX,
            VOID_DUST_ITEM_INDEX,
            STONE_DUST_ITEM_INDEX,
            SUNSTONE_ITEM_INDEX,
            OBSIDIAN_ITEM_INDEX,
            FROST_CRYSTAL_ITEM_INDEX,
            STORM_SHARD_ITEM_INDEX,
            POISON_ITEM_INDEX,
            ACID_ITEM_INDEX,
            THORNHEART_ITEM_INDEX,
            COATL_EGG_ITEM_INDEX,
            AQUATIC_ITEM_INDEX,
            LIFE_ESSENCE_ITEM_INDEX,
        ];
        for owner in [PlayerId::One, PlayerId::Two] {
            for (slot, item) in expected.into_iter().enumerate() {
                let x = if owner == PlayerId::One {
                    -450.0
                } else {
                    390.0
                };
                let position = Vec2::new(
                    x + (slot % 2) as f32 * GRID_BLOCK_SIZE,
                    180.0 - (slot / 2) as f32 * GRID_BLOCK_SIZE,
                );
                assert_eq!(inventory_item(position, owner), Some(item));
            }
        }
    }

    #[test]
    fn aquatic_sprites_rotate_and_clip_to_board_without_shifting_the_origin() {
        for origin in [IVec2::new(4, 3), IVec2::ZERO, IVec2::new(9, 6)] {
            let visual = aquatic::AquaticVisual {
                id: 0,
                origin,
                lengths: [3; 4],
                sweep_frame: Some(7),
            };
            for part in 0..=4 {
                let (sprite, transform) = aquatic_sprite(&visual, part, 21, Handle::default());
                let expected_direction =
                    [Vec2::Y, Vec2::X, Vec2::NEG_Y, Vec2::NEG_X, Vec2::Y][part];
                assert!(
                    (transform.rotation * Vec3::Y)
                        .truncate()
                        .distance(expected_direction)
                        < 0.001
                );
                let rect = sprite.rect.unwrap();
                let frame_origin = Vec2::new(1344.0, 448.0);
                let source_origin = Vec2::splat(112.0) - (rect.min - frame_origin);
                let local = Vec2::new(
                    source_origin.x - rect.width() / 2.0,
                    rect.height() / 2.0 - source_origin.y,
                ) * (GRID_BLOCK_SIZE / 32.0);
                let world = transform.translation + transform.rotation * local.extend(0.0);
                assert!(world.truncate().distance(cell_center(origin)) < 0.001);
                for x in [-0.5, 0.5] {
                    for y in [-0.5, 0.5] {
                        let corner = transform.translation
                            + transform.rotation
                                * (sprite.custom_size.unwrap() * Vec2::new(x, y)).extend(0.0);
                        assert!(corner.x >= BOARD_LEFT - 0.001 && corner.x <= BOARD_RIGHT + 0.001);
                        assert!(corner.y >= BOARD_BOTTOM - 0.001 && corner.y <= BOARD_TOP + 0.001);
                    }
                }
            }
        }
    }

    #[test]
    fn aquatic_animation_uses_growth_and_each_length_sweep_frames() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.set(origin + IVec2::new(2, 0), Ground::Ridge);
            board.set(origin + IVec2::new(0, 3), Ground::Ridge);
            board.set(origin + IVec2::NEG_X, Ground::Ridge);
            board.activate_aquatic(origin);
        }
        for step in 0..3 {
            app.world_mut()
                .resource_mut::<Board>()
                .finish_aquatic_turn();
            app.update();
            let world = app.world_mut();
            let mut query = world.query::<(&AquaticTile, &Sprite)>();
            let sprites: Vec<_> = query.iter(world).collect();
            assert_eq!(sprites.len(), 4);
            for (tile, sprite) in sprites {
                if tile.part == 0 {
                    continue;
                }
                let length = [0, 1, 2, 0, 3][tile.part].min(step + 1);
                let frame = 14 + length - 1;
                let slot = frame - 1;
                assert_eq!(
                    sprite.rect.unwrap().min,
                    Vec2::new((slot % 7) as f32 * 224.0, (slot / 7) as f32 * 224.0)
                );
            }
        }
        press_key(&mut app, KeyCode::KeyR);
        for step in 3..=9 {
            tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
            let world = app.world_mut();
            let mut query = world.query::<(&AquaticTile, &Sprite)>();
            let sprites: Vec<_> = query.iter(world).collect();
            assert_eq!(sprites.len(), 4);
            for (tile, sprite) in sprites {
                if tile.part == 0 {
                    continue;
                }
                let frame = [0, 34, 24, 0, 14][tile.part] + step;
                let slot = if frame == 34 {
                    23
                } else if frame > 34 {
                    frame - 2
                } else {
                    frame - 1
                };
                assert_eq!(
                    sprite.rect.unwrap().min,
                    Vec2::new((slot % 7) as f32 * 224.0, (slot / 7) as f32 * 224.0)
                );
            }
        }
        finish_aquatic_animation(&mut app);
        let world = app.world_mut();
        assert_eq!(world.query::<&AquaticTile>().iter(world).count(), 0);
    }

    #[test]
    fn aquatic_center_emerges_once_then_loops_emerged_frames() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            board.finish_aquatic_turn();
        }
        app.update();
        for expected in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 3] {
            let world = app.world_mut();
            let mut query = world.query::<&AquaticTile>();
            assert_eq!(
                query.iter(world).find(|tile| tile.part == 0).unwrap().frame,
                expected
            );
            tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS);
        }
    }

    #[test]
    fn serpent_sprites_align_head_body_and_tail_for_all_orientations() {
        let middle = IVec2::new(4, 3);
        for heading in [IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y] {
            for tail_direction in [
                -heading,
                IVec2::new(-heading.y, heading.x),
                IVec2::new(heading.y, -heading.x),
            ] {
                for (frame, venomous) in [(0, false), (1, false), (0, true), (1, true)] {
                    let cells = vec![middle + heading, middle, middle + tail_direction];
                    let visual = serpent::SerpentVisual {
                        cells: cells.clone(),
                        venomous,
                        straight_frame: frame,
                    };
                    let (sprite, transform) = serpent_sprite(&visual, Handle::default());
                    let straight = tail_direction == -heading;
                    let rect = sprite.rect.unwrap();
                    assert_eq!(
                        rect.min.x,
                        if straight {
                            (2 + usize::from(venomous) * 2 + frame) as f32 * 96.0
                        } else {
                            usize::from(venomous) as f32 * 96.0
                        }
                    );
                    let pixels = if straight {
                        [
                            Vec2::new(80.0, 16.0),
                            Vec2::new(48.0, 16.0),
                            Vec2::new(16.0, 16.0),
                        ]
                    } else {
                        [
                            Vec2::new(48.0, 16.0),
                            Vec2::new(16.0, 16.0),
                            Vec2::new(16.0, 48.0),
                        ]
                    };
                    let source_size = rect.size();
                    for (pixel, cell) in pixels.into_iter().zip(cells) {
                        let mut local =
                            Vec2::new(pixel.x - source_size.x / 2.0, source_size.y / 2.0 - pixel.y)
                                * (GRID_BLOCK_SIZE / 32.0);
                        if sprite.flip_y {
                            local.y = -local.y;
                        }
                        let world = transform.translation + transform.rotation * local.extend(0.0);
                        assert!(
                            world.truncate().distance(cell_center(cell)) < 0.001,
                            "heading {heading:?}, tail {tail_direction:?}, cell {cell:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn serpent_rendering_tracks_straight_and_corner_animation_steps() {
        let mut app = interaction_app();
        let visual = |cells, straight_frame, venomous| serpent::SerpentVisual {
            cells,
            venomous,
            straight_frame,
        };
        let initial = visual(
            vec![IVec2::new(4, 3), IVec2::new(3, 3), IVec2::new(2, 3)],
            0,
            false,
        );
        let frames = [
            visual(
                vec![IVec2::new(5, 3), IVec2::new(4, 3), IVec2::new(3, 3)],
                1,
                true,
            ),
            visual(
                vec![IVec2::new(5, 2), IVec2::new(5, 3), IVec2::new(4, 3)],
                1,
                true,
            ),
            visual(
                vec![IVec2::new(5, 1), IVec2::new(5, 2), IVec2::new(5, 3)],
                0,
                true,
            ),
            visual(
                vec![IVec2::new(5, 0), IVec2::new(5, 1), IVec2::new(5, 2)],
                1,
                false,
            ),
        ];
        app.world_mut()
            .resource_mut::<TurnState>()
            .serpent_animation = Some(SerpentAnimation {
            visuals: vec![initial],
            impact: None,
            frames: frames
                .into_iter()
                .map(|visual| SerpentAnimationFrame {
                    visuals: vec![visual],
                    hit: None,
                    impact: None,
                })
                .collect(),
            timer: Timer::from_seconds(SERPENT_STEP_SECONDS, TimerMode::Once),
        });
        app.update();
        for expected_x in [192.0, 480.0, 96.0, 384.0, 288.0] {
            let world = app.world_mut();
            let mut sprites = world.query_filtered::<&Sprite, With<SerpentPart>>();
            assert_eq!(
                sprites.single(world).unwrap().rect.unwrap().min.x,
                expected_x
            );
            tick_coatl_animation(&mut app, SERPENT_STEP_SECONDS + 0.01);
        }
    }

    #[test]
    fn players_use_their_own_frames_from_the_shared_sprite_sheet() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        let sheet = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let player_sheet = sheet.clone();
        app.add_systems(Startup, move |mut commands: Commands| {
            spawn_players(&mut commands, player_sheet.clone());
        });
        app.update();

        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Sprite, &Transform, Has<PersistentWorld>)>();
        let players: Vec<_> = players.iter(world).collect();
        assert_eq!(players.len(), 2);
        let mut seen = [false; 2];
        for (player, sprite, transform, persistent) in players {
            assert!(!seen[player.0.index()]);
            seen[player.0.index()] = true;
            let rect = match player.0 {
                PlayerId::One => Rect::new(0.0, 0.0, 48.0, 48.0),
                PlayerId::Two => Rect::new(0.0, 48.0, 48.0, 96.0),
            };
            assert_eq!(sprite.image, sheet);
            assert_eq!(sprite.rect, Some(rect));
            assert_eq!(sprite.custom_size, Some(Vec2::splat(72.0)));
            assert_eq!(
                transform.translation,
                cell_center(PLAYER_STARTS[player.0.index()]).extend(0.0)
            );
            assert!(persistent);
        }
    }

    #[test]
    fn player_standing_and_hover_animations_loop_and_restart_on_transition() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<TurnState>()
            .add_systems(Startup, |mut commands: Commands| {
                spawn_players(&mut commands, Handle::default());
            })
            .add_systems(Update, animate_players);
        app.update();

        fn frames(app: &mut App) -> [Rect; 2] {
            let world = app.world_mut();
            let mut query = world.query::<(&Player, &Sprite)>();
            let mut frames = [Rect::default(); 2];
            for (player, sprite) in query.iter(world) {
                frames[player.0.index()] = sprite.rect.unwrap();
            }
            frames
        }

        fn tick(app: &mut App, millis: u64) {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(millis));
            app.update();
        }

        assert_eq!(
            frames(&mut app),
            [
                Rect::new(0.0, 0.0, 48.0, 48.0),
                Rect::new(0.0, 48.0, 48.0, 96.0)
            ]
        );
        tick(&mut app, 332);
        assert_eq!(frames(&mut app)[0].min.x, 0.0);
        tick(&mut app, 1);
        assert_eq!(frames(&mut app)[0].min.x, 48.0);
        assert_eq!(frames(&mut app)[1].min.x, 48.0);

        app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
        tick(&mut app, 0);
        assert_eq!(frames(&mut app)[0].min.x, 96.0);
        assert_eq!(frames(&mut app)[1].min.x, 48.0);
        tick(&mut app, 333);
        assert_eq!(frames(&mut app)[0].min.x, 144.0);
        assert_eq!(frames(&mut app)[1].min.x, 0.0);

        app.world_mut().resource_mut::<TurnState>().hovering = [false, true];
        tick(&mut app, 0);
        assert_eq!(frames(&mut app)[0].min.x, 0.0);
        assert_eq!(frames(&mut app)[1].min.x, 96.0);
        tick(&mut app, 999);
        assert_eq!(frames(&mut app)[0].min.x, 48.0);
        assert_eq!(frames(&mut app)[1].min.x, 144.0);
    }

    #[test]
    fn frozen_player_idle_animation_pauses_and_resumes_after_thaw() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<TurnState>()
            .add_systems(Startup, |mut commands: Commands| {
                spawn_players(&mut commands, Handle::default());
            })
            .add_systems(Update, animate_players);
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
        app.world_mut()
            .resource_mut::<TurnState>()
            .pending_effects
            .frozen_players[0] = 2;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(PLAYER_FRAME_DURATION);
        app.update();
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Player, &Sprite)>();
            for (player, sprite) in query.iter(world) {
                assert_eq!(
                    sprite.rect,
                    Some(player_frame_rect(player.0, false, player.0.index()))
                );
            }
        }
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.pending_effects.frozen_players[0] = 0;
            turn.frozen_turns[0] = 1;
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(PLAYER_FRAME_DURATION);
        app.update();
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Player, &Sprite)>();
            assert!(
                query
                    .iter(world)
                    .all(|(player, sprite)| sprite.rect
                        == Some(player_frame_rect(player.0, false, 0)))
            );
        }
        app.world_mut().resource_mut::<TurnState>().frozen_turns[0] = 0;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(233));
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<(&Player, &Sprite)>();
        let (_, sprite) = query
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap();
        assert_eq!(
            sprite.rect,
            Some(player_frame_rect(PlayerId::One, false, 1))
        );
    }

    #[test]
    fn frozen_player_overlay_plays_once_follows_player_and_resets() {
        let mut app = App::new();
        let image = Assets::<Image>::default().add(Image::default());
        app.insert_resource(FrozenSheet(image.clone()))
            .init_resource::<Time>()
            .init_resource::<Board>()
            .init_resource::<TurnState>()
            .add_systems(Startup, |mut commands: Commands| {
                spawn_players(&mut commands, Handle::default());
            })
            .add_systems(Update, update_frozen_tiles);
        app.update();
        app.world_mut()
            .resource_mut::<TurnState>()
            .pending_effects
            .frozen_players[0] = 2;
        app.update();

        fn overlay(app: &mut App) -> (Entity, Sprite, Transform) {
            let world = app.world_mut();
            let mut query =
                world.query_filtered::<(Entity, &Sprite, &Transform), With<FrostImpact>>();
            let (entity, sprite, transform) = query.single(world).unwrap();
            (entity, sprite.clone(), *transform)
        }

        let (first, sprite, transform) = overlay(&mut app);
        assert_eq!(sprite.image, image);
        assert_eq!(sprite.color, Color::WHITE);
        assert_eq!(sprite.rect, Some(frozen_frame_rect(0)));
        assert_eq!(sprite.custom_size, Some(Vec2::splat(GRID_BLOCK_SIZE)));
        assert_eq!(
            transform.translation,
            cell_center(PLAYER_STARTS[0]).extend(0.9)
        );

        move_player_to(&mut app, PlayerId::One, IVec2::new(4, 3));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(FROZEN_FRAME_DURATION);
        app.update();
        let (_, sprite, transform) = overlay(&mut app);
        assert_eq!(sprite.rect, Some(frozen_frame_rect(1)));
        assert_eq!(
            transform.translation,
            cell_center(IVec2::new(4, 3)).extend(0.9)
        );

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(5));
        app.update();
        assert_eq!(overlay(&mut app).1.rect, Some(frozen_frame_rect(5)));
        app.world_mut()
            .resource_mut::<TurnState>()
            .pending_effects
            .frozen_players[0] = 0;
        app.update();
        assert!(app.world().get_entity(first).is_err());
        app.world_mut().resource_mut::<TurnState>().frozen_turns[0] = 2;
        app.update();
        let (next, sprite, _) = overlay(&mut app);
        assert_ne!(first, next);
        assert_eq!(sprite.rect, Some(frozen_frame_rect(0)));
    }

    #[test]
    fn frozen_player_overlay_waits_for_freeze_effect_before_animating() {
        let mut app = App::new();
        app.init_resource::<FrozenSheet>()
            .init_resource::<Time>()
            .init_resource::<Board>()
            .init_resource::<TurnState>()
            .init_resource::<effects::Effects>()
            .add_systems(Startup, |mut commands: Commands| {
                spawn_players(&mut commands, Handle::default());
            })
            .add_systems(
                Update,
                (effects::advance, effects::capture, update_frozen_tiles).chain(),
            );
        app.update();
        app.world_mut().resource_mut::<TurnState>().frozen_turns[0] = 2;
        app.world_mut().spawn(effects::EffectSignal(
            PLAYER_STARTS[0],
            effects::Effect::Freeze,
        ));
        app.update();
        {
            let world = app.world_mut();
            let mut query = world.query_filtered::<(&Sprite, &Visibility), With<FrostImpact>>();
            let (sprite, visibility) = query.single(world).unwrap();
            assert_eq!(sprite.rect, Some(frozen_frame_rect(0)));
            assert_eq!(*visibility, Visibility::Hidden);
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Sprite, &Visibility), With<FrostImpact>>();
        let (sprite, visibility) = query.single(world).unwrap();
        assert_eq!(sprite.rect, Some(frozen_frame_rect(0)));
        assert_eq!(*visibility, Visibility::Inherited);
    }

    #[test]
    fn new_match_resets_board_players_and_temporary_entities() {
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed([7; 8]))
            .insert_resource(menu::Frontend::computer_match())
            .init_resource::<TurnState>()
            .init_resource::<Board>()
            .init_resource::<PlayerHealth>()
            .init_resource::<Weather>()
            .init_resource::<ai::BotController>()
            .init_resource::<help::Help>()
            .init_resource::<history::ComputerHistory>()
            .init_resource::<Selection>()
            .init_resource::<InventoryHover>()
            .init_resource::<BoardCursor>()
            .init_resource::<InventoryImages>()
            .add_systems(Update, reset_match);
        let world = app.world_mut();
        world.resource_mut::<PlayerHealth>().0 = [0, 3];
        world.resource_mut::<Board>().add_acid(PLAYER_STARTS[0]);
        world.resource_mut::<TurnState>().strength = [0, 1];
        world
            .resource_mut::<history::ComputerHistory>()
            .add("Old move".into(), None);
        for player in [PlayerId::One, PlayerId::Two] {
            world.spawn((
                Player(player),
                PersistentWorld,
                Sprite::default(),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
        let old_item = world.spawn((BoardItem, Sprite::default())).id();
        let highlight = world.spawn(Sprite::default()).id();
        let icon = world.spawn(ImageNode::default()).id();
        world.resource_mut::<Selection>().0 = Some(SelectedAction::Item {
            index: SUNSTONE_ITEM_INDEX,
            icon,
            highlight,
        });
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(world.resource::<TurnState>().active, PlayerId::One);
        assert_eq!(world.resource::<TurnState>().strength, [MAX_STRENGTH; 2]);
        assert_eq!(
            world.resource::<TurnState>().spawned_ingredients[SUNSTONE_ITEM_INDEX],
            3
        );
        assert_eq!(
            world.resource::<TurnState>().spawned_ingredients[LIFE_ESSENCE_ITEM_INDEX],
            3
        );
        assert!(!world.resource::<Board>().is_acidic(PLAYER_STARTS[0]));
        assert!(
            world
                .resource::<history::ComputerHistory>()
                .entries()
                .is_empty()
        );
        assert!(world.resource::<Selection>().0.is_none());
        assert!(world.get_entity(old_item).is_err());
        assert!(world.get_entity(highlight).is_err());
        assert!(world.get_entity(icon).is_err());
        let mut players = world.query::<(&Player, &Transform)>();
        for (player, transform) in players.iter(world) {
            assert_eq!(
                board_cell(transform.translation.truncate()),
                Some(PLAYER_STARTS[player.0.index()])
            );
        }
        let mut items = world.query::<&Collectible>();
        assert_eq!(items.iter(world).count(), 2);
    }

    #[test]
    fn seeded_bevy_rng_generates_and_renders_the_board_at_startup() {
        let seed = [7; 8];
        let expected = Board::generate(&mut WyRand::from_seed(seed));
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(seed))
            .add_systems(Startup, (generate_board, spawn_board_terrain).chain());
        app.update();
        let world = app.world_mut();
        let board = world.resource::<Board>();
        for y in 0..ROWS {
            for x in 0..COLUMNS {
                let cell = IVec2::new(x as i32, y as i32);
                assert_eq!(board.ground(cell), expected.ground(cell));
            }
        }
        let mut sprites = world.query::<&Sprite>();
        assert_eq!(sprites.iter(world).count(), COLUMNS * ROWS);
    }

    #[test]
    fn default_bevy_rng_generates_a_board_at_startup() {
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::default())
            .add_systems(Startup, (generate_board, spawn_board_terrain).chain());
        app.update();
        assert!(app.world().contains_resource::<Board>());
        let world = app.world_mut();
        let mut sprites = world.query::<&Sprite>();
        assert_eq!(sprites.iter(world).count(), COLUMNS * ROWS);
    }

    #[test]
    fn grid_lines_are_confined_to_board() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_board_grid);
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<(&Sprite, &Transform)>();
        let lines: Vec<_> = query.iter(world).collect();
        assert_eq!(lines.len(), 11 + 8);
        for (sprite, transform) in lines {
            let half_size = sprite.custom_size.unwrap() / 2.0;
            let center = transform.translation.truncate();
            assert!(center.x - half_size.x >= -300.0);
            assert!(center.x + half_size.x <= 300.0);
            assert!(center.y - half_size.y >= -210.0);
            assert!(center.y + half_size.y <= 210.0);
        }
    }

    #[test]
    fn terrain_renders_each_tile_below_grid_lines() {
        let mut board = Board::default();
        board.set(IVec2::new(1, 1), board::Ground::Water);
        board.set(IVec2::new(2, 1), board::Ground::Ridge);
        let mut app = App::new();
        app.insert_resource(board)
            .add_systems(Startup, spawn_board_terrain);
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<(&Sprite, &Transform)>();
        let tiles: Vec<_> = query.iter(world).collect();
        assert_eq!(tiles.len(), COLUMNS * ROWS);
        for (sprite, transform) in tiles {
            let cell = board_cell(transform.translation.truncate()).unwrap();
            assert_eq!(
                sprite.color,
                world.resource::<Board>().ground(cell).unwrap().color()
            );
            assert_eq!(transform.translation.z, -1.5);
            assert_eq!(sprite.custom_size, Some(Vec2::splat(GRID_BLOCK_SIZE)));
        }
    }

    #[test]
    fn health_displays_have_five_hearts_and_fit_the_side_columns() {
        fn setup_health(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_health_displays(&mut commands, hud, Handle::default());
        }

        let mut app = App::new();
        app.add_systems(Startup, setup_health);
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Node, &Children), With<HealthPanel>>();
        let panels: Vec<_> = query.iter(world).collect();
        assert_eq!(panels.len(), 2);
        assert!(panels.iter().any(|(node, _)| node.left == Val::Px(6.0)));
        assert!(panels.iter().any(|(node, _)| node.right == Val::Px(6.0)));
        for (node, children) in panels {
            assert_eq!(node.top, Val::Px(6.0));
            assert_eq!(node.width, Val::Px(168.0));
            assert_eq!(node.height, Val::Px(48.0));
            assert_eq!(children.len(), 3);
            let hearts = world.get::<Children>(children[0]).unwrap();
            assert_eq!(hearts.len(), 5);
            for child in hearts.iter() {
                assert_eq!(
                    world.get::<ImageNode>(child).unwrap().rect,
                    Some(Rect::new(0.0, 0.0, 32.0, 32.0))
                );
            }
            assert_eq!(node.align_items, AlignItems::Center);
            assert_eq!(
                world.get::<Node>(children[1]).unwrap().width,
                Val::Px(150.0)
            );
            let bar = world.get::<Children>(children[1]).unwrap();
            assert_eq!(bar.len(), 1);
            assert!(world.get::<EnergyFill>(bar[0]).is_some());
            assert_eq!(
                world.get::<Node>(children[2]).unwrap().width,
                Val::Px(150.0)
            );
            let bar = world.get::<Children>(children[2]).unwrap();
            assert_eq!(bar.len(), 1);
            assert!(world.get::<MagicFill>(bar[0]).is_some());
        }
    }

    #[test]
    fn heart_sheet_frames_follow_damage_and_healing_for_both_players() {
        let mut app = interaction_app();
        for health in [[10, 10], [9, 1], [0, 0], [1, 9], [10, 10]] {
            app.world_mut().resource_mut::<PlayerHealth>().0 = health;
            app.update();
            for owner in [PlayerId::One, PlayerId::Two] {
                for index in 0..HEART_COUNT {
                    let remaining = health[owner.index()].saturating_sub((index * 2) as u8);
                    assert_heart(&mut app, owner, index, remaining == 1, remaining > 0);
                }
            }
        }
    }

    #[test]
    fn inventories_fill_rows_two_through_eight_on_both_sides() {
        let expected_names = [
            "Tlaloc's Water",
            "Tlexochtli Ember",
            "Void Dust",
            "Stone Dust",
            "Sunstone",
            "Obsidian",
            "Frost Crystal",
            "Storm Shard",
            "Poison Frog Sphere",
            "Toad Wart",
            "Thorn Heart",
            "Coatl's Egg",
            "Ahuizotl Totem",
            "Life Essence",
        ];
        fn setup_inventory(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &InventoryImages::default(),
                Handle::default(),
            );
        }

        let mut app = App::new();
        app.add_systems(Startup, setup_inventory);
        app.update();
        let world = app.world_mut();
        let mut panels = world.query_filtered::<(&Node, &Children), With<InventoryPanel>>();
        let panels: Vec<_> = panels.iter(world).collect();
        assert_eq!(panels.len(), 2);
        assert!(panels.iter().any(|(node, _)| node.left == Val::Px(0.0)));
        assert!(panels.iter().any(|(node, _)| node.right == Val::Px(0.0)));
        for (node, children) in panels {
            assert_eq!(node.top, Val::Px(60.0));
            assert_eq!(node.width, Val::Px(120.0));
            assert_eq!(node.height, Val::Px(420.0));
            let slots: Vec<_> = children
                .iter()
                .filter_map(|child| {
                    world.get::<InventorySlot>(child)?;
                    Some((
                        world.get::<Node>(child).unwrap(),
                        world.get::<Name>(child).unwrap(),
                        world.get::<Children>(child).unwrap(),
                    ))
                })
                .collect();
            assert_eq!(slots.len(), 14);
            for (index, ((slot, name, children), expected)) in
                slots.iter().zip(expected_names.iter()).enumerate()
            {
                let expected_x = if index % 2 == 0 { 6.0 } else { 60.0 };
                let expected_y =
                    (index / 2) as f32 * GRID_BLOCK_SIZE + if index < 2 { 6.0 } else { 0.0 };
                assert_eq!(slot.left, Val::Px(expected_x));
                assert_eq!(slot.top, Val::Px(expected_y));
                assert_eq!(name.as_str(), *expected);
                assert_eq!(world.get::<Text>(children[1]).unwrap().0, *expected);
                assert_eq!(
                    world.get::<ImageNode>(children[0]).unwrap().rect,
                    Some(Rect::new(
                        (index % 2) as f32 * 32.0,
                        (index / 2) as f32 * 32.0,
                        (index % 2 + 1) as f32 * 32.0,
                        (index / 2 + 1) as f32 * 32.0,
                    ))
                );
                let count = world.get::<InventoryCount>(children[2]).unwrap();
                let count_node = world.get::<Node>(children[2]).unwrap();
                assert_eq!(count.index, INVENTORY_ORDER[index]);
                assert_eq!(count_node.position_type, PositionType::Absolute);
                assert_eq!(count_node.top, Val::Px(2.0));
                assert_eq!(count_node.right, Val::Px(3.0));
                if is_unlimited(INVENTORY_ORDER[index]) {
                    assert!(world.get::<ImageNode>(children[2]).is_some());
                    assert!(world.get::<Text>(children[2]).is_none());
                } else {
                    assert_eq!(world.get::<Text>(children[2]).unwrap().0, "0");
                }
            }
        }
    }

    #[test]
    fn action_icons_use_the_sheet_for_both_players() {
        fn setup_actions(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_actions(&mut commands, hud, Handle::default());
        }

        let mut app = App::new();
        app.add_systems(Startup, setup_actions);
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&ActionSlot, &Children)>();
        let slots: Vec<_> = slots.iter(world).collect();
        assert_eq!(slots.len(), 10);
        for (slot, children) in slots {
            let expected = match slot.action {
                Action::Move => Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                Action::Hover => Some(Rect::new(0.0, 32.0, 32.0, 64.0)),
                Action::Pickup => Some(Rect::new(0.0, 64.0, 32.0, 96.0)),
                Action::Rest => Some(Rect::new(0.0, 96.0, 32.0, 128.0)),
                Action::Undo => None,
            };
            let image = children
                .iter()
                .find_map(|child| world.get::<ImageNode>(child));
            assert_eq!(image.and_then(|image| image.rect), expected);
            assert_eq!(image.is_some(), expected.is_some());
        }
    }

    #[test]
    fn only_active_players_inventory_and_actions_are_visible() {
        fn setup_panels(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &InventoryImages::default(),
                Handle::default(),
            );
            spawn_actions(&mut commands, hud, Handle::default());
        }

        let mut app = App::new();
        app.init_resource::<TurnState>()
            .add_systems(Startup, setup_panels)
            .add_systems(Update, update_private_panels);
        for active in [PlayerId::One, PlayerId::Two, PlayerId::One] {
            app.world_mut().resource_mut::<TurnState>().active = active;
            app.update();
            let world = app.world_mut();
            let mut inventories = world.query::<(&InventoryPanel, &Visibility)>();
            let panels: Vec<_> = inventories.iter(world).collect();
            assert_eq!(panels.len(), 2);
            for (panel, visibility) in panels {
                assert_eq!(
                    *visibility,
                    if panel.0 == active {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                );
            }
            let mut actions = world.query::<(&ActionSlot, &Visibility)>();
            let actions: Vec<_> = actions.iter(world).collect();
            assert_eq!(actions.len(), 10);
            for (action, visibility) in actions {
                assert_eq!(
                    *visibility,
                    if action.owner == active {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                );
            }
        }
    }

    #[test]
    fn hover_highlights_only_active_inventory_item_and_shows_hint() {
        fn setup_inventory(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &InventoryImages::default(),
                Handle::default(),
            );
            commands.spawn((
                DropFeedback,
                Text::new("Click Move or an item to select it"),
            ));
        }

        let mut app = App::new();
        app.init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<InventoryHover>()
            .init_resource::<TurnState>()
            .add_systems(Startup, setup_inventory)
            .add_systems(Update, highlight_inventory_hover);
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(p1_item(WATER_ITEM_INDEX));
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &BackgroundColor, &Node)>();
        for (slot, background, node) in slots.iter(world) {
            let highlighted = slot.owner == PlayerId::One && slot.index == 0;
            assert_eq!(background.0 == Color::NONE, !highlighted);
            assert_eq!(
                node.border,
                UiRect::all(Val::Px(if highlighted { 2.0 } else { 0.5 }))
            );
        }
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Tlaloc's Water - click to select"
        );

        app.world_mut().resource_mut::<BoardCursor>().0 = Some(p2_item(SUNSTONE_ITEM_INDEX));
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &BackgroundColor)>();
        assert!(slots.iter(world).all(|(_, color)| color.0 == Color::NONE));
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Click Move or an item to select it"
        );
    }

    #[test]
    fn hovering_over_board_shows_terrain_and_effects_without_erasing_feedback() {
        let mut app = App::new();
        app.init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<TurnState>()
            .init_resource::<Board>()
            .add_systems(Update, update_cell_feedback);
        let message = app
            .world_mut()
            .spawn((
                DropFeedback,
                Text::new("Previous message"),
                Visibility::Inherited,
            ))
            .id();
        let cell_info = app
            .world_mut()
            .spawn((CellFeedback, Text::new(""), Visibility::Hidden))
            .id();
        let cell = IVec2::new(1, 1);
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(cell));
        app.update();
        assert_eq!(app.world().get::<Text>(cell_info).unwrap().0, "B2: Ground");
        assert_eq!(
            *app.world().get::<Visibility>(message).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(cell_info).unwrap(),
            Visibility::Inherited
        );

        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_poison(cell);
            board.add_acid(cell);
            board.electrify_acid(cell);
            board.add_oil(cell);
            assert!(board.freeze_water(cell));
        }
        app.update();
        assert_eq!(
            app.world().get::<Text>(cell_info).unwrap().0,
            "B2: Water | Frozen, Poison (paused), Electrified acid, Ember"
        );
        app.world_mut().resource_mut::<Selection>().0 =
            Some(SelectedAction::Movement { highlights: vec![] });
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(cell_info).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(message).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            app.world().get::<Text>(message).unwrap().0,
            "Previous message"
        );
        app.world_mut().resource_mut::<Selection>().0 = None;
        app.world_mut().resource_mut::<BoardCursor>().0 = None;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(message).unwrap(),
            Visibility::Inherited
        );
    }

    #[test]
    fn cell_feedback_identifies_all_ground_types() {
        let mut board = Board::default();
        let cell = IVec2::ZERO;
        for (ground, label) in [
            (Ground::Solid, "A1: Ground"),
            (Ground::Water, "A1: Water"),
            (Ground::Ridge, "A1: Ridge"),
            (Ground::Hole, "A1: Hole"),
        ] {
            board.set(cell, ground);
            assert_eq!(cell_feedback(&board, cell).as_deref(), Some(label));
        }
        assert_eq!(cell_feedback(&board, IVec2::new(-1, 0)), None);
    }

    #[test]
    fn status_text_shrinks_to_fit_and_restores_size_for_new_messages() {
        let mut app = App::new();
        app.add_systems(Update, fit_status_text);
        let entity = app
            .world_mut()
            .spawn((
                Text::new("A long message"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextLayoutInfo {
                    scale_factor: 2.0,
                    size: Vec2::new(1200.0, 144.0),
                    ..default()
                },
                FitStatusText {
                    initial_size: 16.0,
                    bounds: Vec2::new(600.0, 36.0),
                    last_text: String::new(),
                },
            ))
            .id();
        app.update();
        app.update();
        let FontSize::Px(size) = app.world().get::<TextFont>(entity).unwrap().font_size else {
            panic!("status text should use pixel font size");
        };
        assert!(size < 8.0);
        app.world_mut().get_mut::<Text>(entity).unwrap().0 = "Short".into();
        app.update();
        assert_eq!(
            app.world().get::<TextFont>(entity).unwrap().font_size,
            FontSize::Px(16.0)
        );
    }

    #[test]
    fn diagonal_is_placeable_and_further_cells_are_throwable() {
        let player = Vec2::new(30.0, 0.0);
        let diagonal = drop_target(player, Vec2::new(90.0, 60.0)).unwrap();
        assert_eq!(diagonal.kind, DropKind::Place);
        assert_eq!(diagonal.distance, 1);
        let far = drop_target(player, Vec2::new(-270.0, 180.0)).unwrap();
        assert_eq!(far.kind, DropKind::Distant);
        assert_eq!(far.distance, 5);
        assert_eq!(drop_target(player, player), None);
        assert_eq!(drop_target(player, Vec2::new(300.0, 0.0)), None);
        assert_eq!(board_cell(Vec2::new(0.0, f32::NAN)), None);
    }

    #[test]
    fn ingredient_throw_costs_follow_distance_and_available_energy() {
        for ingredient in [0, 1, 2, 4, 8] {
            for distance in 1..=9 {
                let mut app = interaction_app();
                {
                    let world = app.world_mut();
                    let mut players = world.query::<(&Player, &mut Transform)>();
                    for (player, mut transform) in players.iter_mut(world) {
                        if player.0 == PlayerId::One {
                            transform.translation = cell_center(IVec2::new(0, 3)).extend(0.0);
                        }
                    }
                }
                let inventory = p1_item(ingredient);
                let target = cell_center(IVec2::new(distance, 3));
                let cost = distance - 1;
                click(&mut app, inventory);
                app.world_mut().resource_mut::<BoardCursor>().0 = Some(target);
                app.update();
                {
                    let world = app.world_mut();
                    let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
                    let text = &feedback.single(world).unwrap().0;
                    if cost <= 5 {
                        assert!(text.contains(&format!("{cost} energy")), "{text}");
                        assert!(text.contains(": PLACE -"), "{text}");
                    } else {
                        assert!(text.contains(&format!("needs {cost}")), "{text}");
                    }
                }
                click(&mut app, target);
                let turn = app.world().resource::<TurnState>();
                if cost <= 5 {
                    assert_eq!(turn.strength[0], ((5 - cost + 2).min(5)) as u8);
                    assert_eq!(turn.active, PlayerId::Two);
                    assert!(app.world().resource::<Selection>().0.is_none());
                    assert_eq!(
                        item_count(&mut app),
                        usize::from(ingredient != WATER_ITEM_INDEX && ingredient != OIL_ITEM_INDEX)
                    );
                } else {
                    assert_eq!(turn.strength[0], 5);
                    assert_eq!(turn.active, PlayerId::One);
                    assert!(app.world().resource::<Selection>().0.is_some());
                    assert_eq!(item_count(&mut app), 0);
                }
            }
        }
        let player = cell_center(IVec2::new(0, 1));
        let diagonal = drop_target(player, cell_center(IVec2::new(6, 5))).unwrap();
        assert_eq!(diagonal.distance, 6);
        assert_eq!(item_energy_cost(&diagonal), 5);
    }

    fn interaction_app() -> App {
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        let mut heart_images = Assets::<Image>::default();
        let heart_sheet = heart_images.add(Image::default());
        app.insert_resource(heart_images);
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<InventoryHover>()
            .init_resource::<TurnState>()
            .init_resource::<Board>()
            .init_resource::<PlayerHealth>()
            .init_resource::<Time>()
            .init_resource::<FrozenSheet>()
            .init_resource::<SerpentSheet>()
            .init_resource::<AquaticSheet>()
            .insert_resource(Weather {
                rain: u16::MAX,
                earthquake: u16::MAX,
            })
            .init_resource::<InventoryImages>()
            .add_systems(
                Update,
                (
                    (
                        effects::advance.run_if(resource_exists::<effects::Effects>),
                        advance_item_flight,
                        begin_turn_frame,
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                        advance_serpent_animation,
                        advance_aquatic_animation,
                        rest,
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                    )
                        .chain(),
                    (
                        undo_action,
                        handle_actions,
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                    )
                        .chain(),
                    (
                        handle_selection,
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                        handle_selection.run_if(pending_placement_ready),
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                        clear_hole_items,
                        handle_selection.run_if(pending_placement_ready),
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                        apply_terrain_changes,
                        trigger_placed_frost,
                        effects::capture.run_if(resource_exists::<effects::Effects>),
                    )
                        .chain(),
                    (
                        finalize_undo,
                        clear_hole_items,
                        update_serpent_visuals,
                        update_collectible_positions,
                    )
                        .chain(),
                    advance_wind,
                    respawn_ingredients,
                    highlight_inventory_hover,
                    queue_weather,
                    advance_weather,
                    effects::capture.run_if(resource_exists::<effects::Effects>),
                    (
                        update_inventory_counts,
                        update_action_slots,
                        update_turn_progress,
                        update_wind_status,
                        update_energy_bars,
                        update_magic_bars,
                        animate_players,
                        update_pickup_highlight,
                        update_hearts,
                    )
                        .chain(),
                    update_electric_flashes,
                    update_reaction_flashes,
                    update_frozen_tiles,
                    update_ground_tiles,
                    (
                        update_water_effects,
                        update_poison_visuals,
                        update_acid_visuals,
                        update_void_growth_visuals,
                        update_life_essence_visuals,
                        update_thorn_visuals,
                        update_aquatic_visuals,
                    )
                        .chain(),
                    shake_ground,
                )
                    .chain()
                    .in_set(menu::GameplaySystems)
                    .run_if(|help: Option<Res<help::Help>>| {
                        help.is_none_or(|help| help.item().is_none())
                    }),
            );
        app.add_systems(Startup, initialize_wind);
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud));
        app.add_systems(Startup, move |mut commands: Commands, hud: Res<Hud>| {
            spawn_inventories(
                &mut commands,
                hud.0,
                &InventoryImages::default(),
                Handle::default(),
            );
            spawn_actions(&mut commands, hud.0, Handle::default());
            spawn_health_displays(&mut commands, hud.0, heart_sheet.clone());
        });
        app.world_mut().spawn((
            Player(PlayerId::One),
            PlayerAnimation::default(),
            Sprite::default(),
            Transform::from_xyz(30.0, 0.0, 0.0),
        ));
        app.world_mut().spawn((
            Player(PlayerId::Two),
            PlayerAnimation::default(),
            Sprite::default(),
            Transform::from_xyz(150.0, -180.0, 0.0),
        ));
        app.world_mut().spawn((DropFeedback, Text::new("")));
        app.world_mut().spawn((TurnProgress, Text::new("")));
        app.world_mut().spawn((WindStatus, Text::new("")));
        app.world_mut().resource_mut::<TurnState>().inventory = [[u8::MAX as usize; 14]; 2];
        // Keep unrelated interaction tests independent of new random items.
        app.world_mut()
            .resource_mut::<TurnState>()
            .spawned_ingredients[SUNSTONE_ITEM_INDEX] = INGREDIENT_POOL_SIZE as u8;
        app
    }

    #[test]
    fn interaction_sounds_follow_success_and_rejection() {
        let mut app = interaction_app();
        app.init_resource::<sound::SoundCues>();
        app.update();
        let invalid = IVec2::new(5, 3);
        app.world_mut().resource_mut::<Board>().make_water(invalid);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(
            app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Select)
        );
        click_part(&mut app, cell_center(invalid));
        assert!(
            app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Reject)
        );
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        assert!(
            app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Pop)
        );
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
    }

    fn computer_app() -> App {
        let mut app = interaction_app();
        app.insert_resource(menu::Frontend::computer_match())
            .init_resource::<ai::BotController>()
            .init_resource::<history::ComputerHistory>()
            .add_systems(Update, ai::plan.after(begin_turn_frame).before(rest));
        app.update();
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.active = PlayerId::Two;
            turn.inventory = [[0; 14]; 2];
            turn.inventory[1][SUNSTONE_ITEM_INDEX] = 1;
            turn.inventory[1][LIFE_ESSENCE_ITEM_INDEX] = 1;
            turn.inventory[1][OIL_ITEM_INDEX] = 1;
            turn.wind = WindDirection::West;
        }
        app
    }

    fn ingredient_help_app() -> App {
        let mut app = interaction_app();
        app.insert_resource(menu::Frontend::computer_match())
            .init_resource::<ai::BotController>()
            .add_message::<bevy::input::mouse::MouseWheel>()
            .add_plugins(help::plugin);
        app.update();
        app
    }

    #[test]
    fn queued_reactions_wait_for_the_prior_effect_animation() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.update();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert!(app.world().resource::<Board>().is_water(target));
        assert!(!app.world().resource::<Board>().is_frozen(target));
        assert!(app.world().resource::<TurnState>().effects_active);
        assert_eq!(
            app.world()
                .resource::<effects::Effects>()
                .held(target)
                .unwrap()
                .ground,
            Ground::Solid
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        tick_coatl_animation(&mut app, 1.5);
        assert!(!app.world().resource::<Board>().is_frozen(target));
        assert!(app.world().resource::<TurnState>().item_flight.is_some());
        finish_item_flights(&mut app);
        assert!(app.world().resource::<Board>().is_frozen(target));
        assert!(app.world().resource::<TurnState>().effects_active);
        assert_eq!(
            app.world()
                .resource::<effects::Effects>()
                .held(target)
                .unwrap()
                .ground,
            Ground::Water
        );
        tick_coatl_animation(&mut app, 1.5);
        assert!(!app.world().resource::<TurnState>().effects_active);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn expiring_poison_remains_visible_until_new_water_and_poison_arrive() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 4);
        let target = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(source);
            board.add_poison(source);
            board.finish_turn([], []);
            board.finish_turn([], []);
            assert!(board.is_poisoned(source));
        }
        app.init_resource::<effects::Effects>();
        app.update();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert!(!app.world().resource::<Board>().is_poisoned(source));
        assert!(app.world().resource::<Board>().is_poisoned(target));
        let effects = app.world().resource::<effects::Effects>();
        assert!(effects.animating(target, effects::Effect::Water));
        assert!(effects.animating(target, effects::Effect::Poison));
        assert!(effects.held(source).unwrap().poison);
        assert!(!effects.held(target).unwrap().poison);
        {
            let world = app.world_mut();
            let tiles: Vec<_> = world
                .query::<&PoisonTile>()
                .iter(world)
                .map(|p| p.0)
                .collect();
            assert!(tiles.contains(&source));
            assert!(!tiles.contains(&target));
        }
        tick_coatl_animation(&mut app, 0.5);
        assert!(
            app.world()
                .resource::<effects::Effects>()
                .held(source)
                .unwrap()
                .poison
        );
        tick_coatl_animation(&mut app, 0.5);
        let world = app.world_mut();
        let tiles: Vec<_> = world
            .query::<&PoisonTile>()
            .iter(world)
            .map(|p| p.0)
            .collect();
        assert!(!tiles.contains(&source));
        assert!(tiles.contains(&target));
        assert!(!world.resource::<TurnState>().effects_active);
    }

    #[test]
    fn completed_growth_stays_visible_until_its_terrain_animations_finish() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            let mut app = interaction_app();
            app.init_resource::<effects::Effects>();
            app.update();
            app.world_mut()
                .resource_mut::<Board>()
                .start_terrain_growth(IVec2::new(4, 3), 7, kind);
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert_eq!(
                app.world_mut()
                    .query::<&VoidGrowthTile>()
                    .iter(app.world())
                    .count(),
                5
            );
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<TurnState>().effects_active);
            {
                let world = app.world_mut();
                let tiles: Vec<_> = world
                    .query::<&VoidGrowthTile>()
                    .iter(world)
                    .map(|tile| tile.0)
                    .collect();
                assert_eq!(tiles.len(), 5);
                for cell in tiles {
                    assert!(
                        world
                            .resource::<effects::Effects>()
                            .held(cell)
                            .unwrap()
                            .growth
                            == Some(kind)
                    );
                }
            }
            tick_coatl_animation(&mut app, 1.0);
            assert_eq!(
                app.world_mut()
                    .query::<&VoidGrowthTile>()
                    .iter(app.world())
                    .count(),
                0
            );
        }
    }

    #[test]
    fn effect_starts_on_landing_after_a_parabolic_throw() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.update();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        let selection = &app.world().resource::<Selection>().0;
        let Some(SelectedAction::Item { icon, .. }) = selection else {
            panic!("water must be selected");
        };
        assert_eq!(
            app.world().get::<ImageNode>(*icon).unwrap().rect,
            Some(Rect::new(0.0, 0.0, 32.0, 32.0))
        );
        click_part(&mut app, cell_center(target));
        let marker = app
            .world()
            .resource::<TurnState>()
            .pending_placement
            .as_ref()
            .unwrap()
            .marker;
        assert_eq!(
            app.world().get::<Sprite>(marker).unwrap().rect,
            Some(Rect::new(0.0, 0.0, 32.0, 32.0))
        );
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyR);
            keys.clear();
        }
        let flight = app
            .world()
            .resource::<TurnState>()
            .item_flight
            .as_ref()
            .unwrap();
        let entity = flight.entity;
        let origin = flight.origin;
        assert_eq!(flight.destination, cell_center(target));
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().custom_size,
            Some(Vec2::splat(GRID_BLOCK_SIZE / 2.0))
        );
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            Some(Rect::new(0.0, 0.0, 32.0, 32.0))
        );
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        assert!(!app.world().resource::<TurnState>().effects_active);

        tick_coatl_animation(&mut app, ITEM_FLIGHT_SECONDS / 2.0);
        let position = app
            .world()
            .get::<Transform>(entity)
            .unwrap()
            .translation
            .truncate();
        let expected = origin.lerp(cell_center(target), 0.5) + Vec2::Y * ITEM_FLIGHT_HEIGHT;
        assert!(position.distance(expected) < 0.01);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyR);
            keys.clear();
        }
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(app.world().resource::<TurnState>().item_flight.is_some());

        tick_coatl_animation(&mut app, ITEM_FLIGHT_SECONDS / 2.0 + 0.01);
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert!(app.world().resource::<TurnState>().effects_active);
        assert!(
            app.world()
                .resource::<effects::Effects>()
                .animating(target, effects::Effect::Water)
        );
    }

    #[test]
    fn inert_placed_item_lands_and_remains_in_the_corner() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyR);
            keys.clear();
        }
        let flight = app
            .world()
            .resource::<TurnState>()
            .item_flight
            .as_ref()
            .unwrap();
        assert_eq!(
            flight.destination,
            cell_center(target) + Vec2::splat(ITEM_CORNER_OFFSET)
        );
        assert_eq!(item_count(&mut app), 0);
        tick_coatl_animation(&mut app, ITEM_FLIGHT_SECONDS + 0.01);
        let world = app.world_mut();
        let (item, transform, sprite) = world
            .query_filtered::<(&Collectible, &Transform, &Sprite), With<PlacedItem>>()
            .single(world)
            .unwrap();
        assert_eq!(item.cell, target);
        assert_eq!(
            transform.translation.truncate(),
            cell_center(target) + Vec2::splat(ITEM_CORNER_OFFSET)
        );
        assert_eq!(sprite.custom_size, Some(Vec2::splat(24.0)));
        assert_eq!(sprite.rect, Some(Rect::new(0.0, 64.0, 32.0, 96.0)));
    }

    #[test]
    fn queued_direct_hit_animates_before_an_unrelated_second_throw() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.update();
        let target = IVec2::new(6, 3);
        let other = IVec2::new(4, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(OIL_ITEM_INDEX));
        click_part(&mut app, cell_center(other));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.pending_effects.frozen_players[1], 2);
        assert!(turn.effects_active);
        assert!(turn.item_flight.is_none());
        assert!(!app.world().resource::<Board>().has_oil(other));
        assert!(
            app.world()
                .resource::<effects::Effects>()
                .animating(target, effects::Effect::Freeze)
        );
    }

    #[test]
    fn ingredient_help_preserves_pending_action_and_pauses_controls() {
        let mut app = ingredient_help_app();
        let target = cell_center(IVec2::new(6, 3));
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, target);
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        let before = {
            let turn = app.world().resource::<TurnState>();
            (
                turn.inventory,
                turn.strength,
                turn.actions_used,
                turn.active,
            )
        };
        click_part(&mut app, Vec2::new(-330.0, -240.0));
        assert_eq!(
            app.world().resource::<help::Help>().item(),
            Some(SUNSTONE_ITEM_INDEX)
        );
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item {
                index: SUNSTONE_ITEM_INDEX,
                ..
            })
        ));
        press_key(&mut app, KeyCode::KeyR);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(
            (
                turn.inventory,
                turn.strength,
                turn.actions_used,
                turn.active
            ),
            before
        );
        assert_eq!(
            turn.pending_placement.as_ref().unwrap().index,
            WATER_ITEM_INDEX
        );
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<help::Help>().item(), None);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item {
                index: SUNSTONE_ITEM_INDEX,
                ..
            })
        ));
        click_part(&mut app, target);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert!(app.world().resource::<Board>().is_water(IVec2::new(6, 3)));
    }

    #[test]
    fn every_selected_ingredient_opens_its_own_guide() {
        let mut app = ingredient_help_app();
        for (index, (name, _)) in INVENTORY_ITEMS.iter().enumerate() {
            let position = p1_item(index);
            click_part(&mut app, position);
            click_part(&mut app, Vec2::new(-330.0, -240.0));
            assert_eq!(app.world().resource::<help::Help>().item(), Some(index));
            let world = app.world_mut();
            let mut labels = world.query::<&Text>();
            assert!(labels.iter(world).any(|text| text.0 == *name), "{name}");
            press_key(&mut app, KeyCode::Escape);
            click_part(&mut app, position);
        }
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.actions_used, 0);
    }

    fn computer_step(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.7));
        app.update();
        finish_item_flights(app);
    }

    #[test]
    fn computer_queues_water_and_ember_through_normal_turn_resolution() {
        let mut app = computer_app();
        {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &mut Transform)>();
            for (player, mut transform) in players.iter_mut(world) {
                let cell = if player.0 == PlayerId::One {
                    IVec2::new(1, 1)
                } else {
                    IVec2::new(4, 1)
                };
                transform.translation = cell_center(cell).extend(0.0);
            }
        }
        computer_step(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.actions_used, 1);
        assert_eq!(
            turn.pending_placement.as_ref().unwrap().index,
            WATER_ITEM_INDEX
        );
        computer_step(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert!(turn.pending_placement.is_none());
        assert_eq!(turn.inventory[1][OIL_ITEM_INDEX], 0);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 10]);
        assert_eq!(
            app.world().resource::<Board>().ground(IVec2::new(1, 1)),
            Some(Ground::Water)
        );
        let entries = app.world().resource::<history::ComputerHistory>().entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Computer used Tlaloc's Water at B2");
        assert_eq!(entries[0].item, Some(WATER_ITEM_INDEX));
        assert_eq!(entries[1].text, "Computer used Tlexochtli Ember at B2");
    }

    #[test]
    fn computer_thaws_itself_and_finishes_its_turn() {
        for on_water in [false, true] {
            let mut app = computer_app();
            let cell = IVec2::new(7, 3);
            {
                let world = app.world_mut();
                let mut players = world.query::<(&Player, &mut Transform)>();
                for (player, mut transform) in players.iter_mut(world) {
                    if player.0 == PlayerId::Two {
                        transform.translation = cell_center(cell).extend(0.0);
                    }
                }
                if on_water {
                    let mut board = world.resource_mut::<Board>();
                    board.make_water(cell);
                    assert!(board.freeze_water(cell));
                }
                world.resource_mut::<TurnState>().frozen_turns[1] = 2;
            }
            computer_step(&mut app);
            assert_eq!(
                app.world()
                    .resource::<TurnState>()
                    .pending_placement
                    .as_ref()
                    .unwrap()
                    .index,
                SUNSTONE_ITEM_INDEX
            );
            computer_step(&mut app);
            let turn = app.world().resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::One);
            assert_eq!(turn.frozen_turns[1], 0);
            assert!(!app.world().resource::<Board>().is_frozen(cell));
            assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
        }
    }

    #[test]
    fn human_input_is_ignored_during_computer_turn_and_bot_inventory_stays_hidden() {
        let mut app = computer_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.add_systems(Update, update_private_panels.after(ai::plan));
        app.update();
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
        let world = app.world_mut();
        let mut panels = world.query::<(&InventoryPanel, &Visibility)>();
        for (panel, visibility) in panels.iter(world) {
            assert_eq!(
                *visibility,
                if panel.0 == PlayerId::One {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            );
        }
        world.resource_mut::<ButtonInput<KeyCode>>().clear();
        world.resource_mut::<BoardCursor>().0 = Some(Vec2::new(150.0, -180.0));
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn computer_picks_up_and_uses_an_item_through_the_normal_handlers() {
        let mut app = computer_app();
        let cell = board_cell(Vec2::new(150.0, -180.0)).unwrap();
        app.world_mut().spawn((
            Collectible {
                index: STORM_SHARD_ITEM_INDEX,
                cell,
            },
            BoardItem,
        ));
        computer_step(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.actions_used, 1);
        assert_eq!(turn.inventory[1][STORM_SHARD_ITEM_INDEX], 1);
        computer_step(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.inventory[1][STORM_SHARD_ITEM_INDEX], 0);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
        let entries = app.world().resource::<history::ComputerHistory>().entries();
        assert_eq!(entries.len(), 2);
        assert!(
            entries[0]
                .text
                .starts_with("Computer picked up Storm Shard at ")
        );
        assert_eq!(entries[0].item, Some(STORM_SHARD_ITEM_INDEX));
        assert!(entries[1].text.starts_with("Computer used Storm Shard at "));
    }

    #[test]
    fn computer_moves_out_of_damaging_terrain() {
        let mut app = computer_app();
        let start = board_cell(Vec2::new(150.0, -180.0)).unwrap();
        app.world_mut().resource_mut::<Board>().add_acid(start);
        computer_step(&mut app);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let destination = players
            .iter(world)
            .find_map(|(player, transform)| {
                (player.0 == PlayerId::Two)
                    .then(|| board_cell(transform.translation.truncate()).unwrap())
            })
            .unwrap();
        assert_ne!(destination, start);
        assert!(!world.resource::<Board>().is_acidic(destination));
        assert_eq!(world.resource::<TurnState>().actions_used, 1);
        let entries = world.resource::<history::ComputerHistory>().entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].text,
            format!("Computer moved to {}", history::cell_label(destination))
        );
    }

    #[test]
    fn computer_lands_before_resolving_a_pending_placement() {
        let mut app = computer_app();
        let computer = IVec2::new(4, 1);
        {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &mut Transform)>();
            for (player, mut transform) in players.iter_mut(world) {
                let cell = if player.0 == PlayerId::One {
                    IVec2::new(1, 1)
                } else {
                    computer
                };
                transform.translation = cell_center(cell).extend(0.0);
            }
        }
        computer_step(&mut app);
        assert!(
            app.world()
                .resource::<TurnState>()
                .pending_placement
                .is_some()
        );
        {
            let world = app.world_mut();
            world.resource_mut::<Board>().set(computer, Ground::Hole);
            let mut turn = world.resource_mut::<TurnState>();
            turn.hovering[1] = true;
            turn.magic[1] = 1;
        }
        computer_step(&mut app);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let destination = players
            .iter(world)
            .find_map(|(player, transform)| {
                (player.0 == PlayerId::Two)
                    .then(|| board_cell(transform.translation.truncate()).unwrap())
            })
            .unwrap();
        assert_ne!(destination, computer);
        assert!(
            world
                .resource::<Board>()
                .ground(destination)
                .is_some_and(Ground::walkable)
        );
        assert_eq!(world.resource::<TurnState>().active, PlayerId::One);
    }

    #[test]
    fn computer_avoids_electrocuting_itself_on_connected_water() {
        let mut app = computer_app();
        {
            let world = app.world_mut();
            let enemy = IVec2::new(4, 3);
            let computer = IVec2::new(5, 3);
            let mut players = world.query::<(&Player, &mut Transform)>();
            for (player, mut transform) in players.iter_mut(world) {
                let cell = if player.0 == PlayerId::One {
                    enemy
                } else {
                    computer
                };
                transform.translation = cell_center(cell).extend(0.0);
            }
            let mut board = world.resource_mut::<Board>();
            board.make_water(enemy);
            board.make_water(computer);
            let mut turn = world.resource_mut::<TurnState>();
            turn.inventory[1][STORM_SHARD_ITEM_INDEX] = 1;
            turn.inventory[1][SUNSTONE_ITEM_INDEX] = 0;
        }
        computer_step(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.inventory[1][STORM_SHARD_ITEM_INDEX], 1);
        assert!(turn.pending_placement.is_none());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
    }

    #[test]
    fn computer_action_cannot_use_an_item_it_does_not_have() {
        let mut app = interaction_app();
        app.update();
        app.world_mut().resource_mut::<TurnState>().active = PlayerId::Two;
        app.world_mut().resource_mut::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX] = 0;
        let mut bot = ai::BotController::default();
        bot.bot_turn = true;
        bot.action = Some(ai::BotAction::Place {
            item: SUNSTONE_ITEM_INDEX,
            cell: board_cell(Vec2::new(30.0, 0.0)).unwrap(),
        });
        app.insert_resource(bot);
        app.update();
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.actions_used, 0);
        assert_eq!(turn.inventory[1][SUNSTONE_ITEM_INDEX], 0);
        assert!(turn.pending_placement.is_none());
    }

    #[test]
    fn local_two_player_turn_remains_human_controlled() {
        let mut app = interaction_app();
        app.insert_resource(menu::Frontend::default())
            .init_resource::<ai::BotController>()
            .add_systems(Update, ai::plan.after(begin_turn_frame).before(rest));
        app.update();
        app.world_mut().resource_mut::<TurnState>().active = PlayerId::Two;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(!app.world().resource::<ai::BotController>().computer_mode);
    }

    fn random_spawn_app() -> App {
        let mut app = interaction_app();
        app.world_mut()
            .resource_mut::<TurnState>()
            .spawned_ingredients = [0; 14];
        app
    }

    fn select_move(app: &mut App) {
        let x = match app.world().resource::<TurnState>().active {
            PlayerId::One => BOARD_LEFT - GRID_BLOCK_SIZE / 2.0,
            PlayerId::Two => BOARD_RIGHT + GRID_BLOCK_SIZE / 2.0,
        };
        click_part(app, Vec2::new(x, BOARD_TOP - GRID_BLOCK_SIZE / 2.0));
    }

    fn click(app: &mut App, position: Vec2) {
        // Reaction tests complete the unused part with rest; use click_part to test turn sequencing.
        let started_partial = {
            let turn = app.world().resource::<TurnState>();
            turn.actions_used > 0
        };
        click_part(app, position);
        let completed_part = {
            let turn = app.world().resource::<TurnState>();
            !started_partial && turn.actions_used > 0
        };
        if completed_part {
            press_key(app, KeyCode::KeyR);
        }
    }

    fn click_part(app: &mut App, position: Vec2) {
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(position);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        finish_item_flights(app);
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.release(key);
        keys.clear();
        finish_item_flights(app);
    }

    fn finish_item_flights(app: &mut App) {
        for _ in 0..2 {
            if app.world().resource::<TurnState>().item_flight.is_none() {
                break;
            }
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(
                    ITEM_FLIGHT_SECONDS + 0.01,
                ));
            app.update();
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::ZERO);
        }
    }

    fn tick_coatl_animation(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::ZERO);
    }

    fn finish_coatl_animation(app: &mut App) {
        for _ in 0..20 {
            if app
                .world()
                .resource::<TurnState>()
                .serpent_animation
                .is_none()
            {
                return;
            }
            tick_coatl_animation(app, 0.7);
        }
        panic!("Coatl animation did not finish");
    }

    fn finish_aquatic_animation(app: &mut App) {
        for _ in 0..30 {
            if app
                .world()
                .resource::<TurnState>()
                .aquatic_animation
                .is_none()
            {
                return;
            }
            tick_coatl_animation(app, 0.3);
        }
        panic!("Aquatic animation did not finish");
    }

    fn finish_weather(app: &mut App) {
        for _ in 0..30 {
            if app
                .world()
                .resource::<TurnState>()
                .weather_animation
                .is_none()
            {
                return;
            }
            tick_coatl_animation(app, WEATHER_ANIMATION_SECONDS);
        }
        panic!("Weather animation did not finish");
    }

    fn item_count(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut items = world.query_filtered::<Entity, With<BoardItem>>();
        items.iter(world).count()
    }

    fn terrain_count(app: &App, ground: Ground) -> usize {
        let board = app.world().resource::<Board>();
        (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .filter(|cell| board.ground(*cell) == Some(ground))
            .count()
    }

    #[test]
    fn weather_timers_count_only_completed_turns_and_reset_in_range() {
        let mut app = interaction_app();
        *app.world_mut().resource_mut::<Weather>() = Weather {
            rain: 2,
            earthquake: 3,
        };
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        assert_eq!(app.world().resource::<Weather>().rain, 2);
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(app.world().resource::<Weather>().earthquake, 3);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<Weather>().rain, 1);
        assert_eq!(app.world().resource::<Weather>().earthquake, 2);
        press_key(&mut app, KeyCode::KeyR);
        finish_weather(&mut app);
        assert_eq!(terrain_count(&app, Ground::Water), 1);
        assert!(RAIN_INTERVAL.contains(&app.world().resource::<Weather>().rain));
        assert_eq!(app.world().resource::<Weather>().earthquake, 1);
        press_key(&mut app, KeyCode::KeyR);
        finish_weather(&mut app);
        assert!(EARTHQUAKE_INTERVAL.contains(&app.world().resource::<Weather>().earthquake));
        assert_eq!(
            terrain_count(&app, Ground::Hole) + terrain_count(&app, Ground::Ridge),
            2
        );
    }

    #[test]
    fn earthquake_waits_for_player_effects_then_shakes_before_terrain_changes() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        let tile = IVec2::new(0, 0);
        let tile_entity = app
            .world_mut()
            .spawn((
                GroundTile(tile),
                Sprite::default(),
                Transform::from_translation(cell_center(tile).extend(-1.5)),
            ))
            .id();
        app.world_mut().resource_mut::<Board>().add_poison(tile);
        let poison_entity = app
            .world_mut()
            .spawn((
                PoisonTile(tile),
                Sprite::default(),
                Transform::from_translation(cell_center(tile).extend(-0.25)),
            ))
            .id();
        app.world_mut().resource_mut::<Weather>().earthquake = 1;
        app.update();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<TurnState>().effects_active);
        assert!(matches!(
            app.world()
                .resource::<TurnState>()
                .weather_animation
                .as_ref()
                .unwrap()
                .phase,
            WeatherPhase::Waiting
        ));
        assert_eq!(
            app.world()
                .get::<Transform>(tile_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x
        );
        tick_coatl_animation(&mut app, 0.5);
        assert!(app.world().resource::<TurnState>().effects_active);
        assert_eq!(
            terrain_count(&app, Ground::Ridge) + terrain_count(&app, Ground::Hole),
            0
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        tick_coatl_animation(&mut app, 0.5);
        assert!(matches!(
            app.world()
                .resource::<TurnState>()
                .weather_animation
                .as_ref()
                .unwrap()
                .phase,
            WeatherPhase::Playing {
                kind: WeatherKind::Earthquake,
                ..
            }
        ));
        assert_eq!(
            app.world()
                .get::<Transform>(tile_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x + EARTHQUAKE_OFFSET
        );
        tick_coatl_animation(&mut app, EARTHQUAKE_FRAME_SECONDS);
        assert_eq!(
            app.world()
                .get::<Transform>(tile_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x - EARTHQUAKE_OFFSET
        );
        assert_eq!(
            app.world()
                .get::<Transform>(poison_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x - EARTHQUAKE_OFFSET
        );
        tick_coatl_animation(&mut app, EARTHQUAKE_FRAME_SECONDS);
        assert_eq!(
            app.world()
                .get::<Transform>(tile_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x + EARTHQUAKE_OFFSET
        );
        tick_coatl_animation(&mut app, 2.19);
        assert_eq!(
            terrain_count(&app, Ground::Ridge) + terrain_count(&app, Ground::Hole),
            0
        );
        tick_coatl_animation(&mut app, 0.02);
        assert_eq!(
            terrain_count(&app, Ground::Ridge) + terrain_count(&app, Ground::Hole),
            2
        );
        assert_eq!(
            app.world()
                .get::<Transform>(tile_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x
        );
        assert_eq!(
            app.world()
                .get::<Transform>(poison_entity)
                .unwrap()
                .translation
                .x,
            cell_center(tile).x
        );
        assert!(app.world().resource::<TurnState>().effects_active);
        let board = app.world().resource::<Board>();
        let effects = app.world().resource::<effects::Effects>();
        for y in 0..ROWS {
            for x in 0..COLUMNS {
                let cell = IVec2::new(x as i32, y as i32);
                if matches!(board.ground(cell), Some(Ground::Ridge | Ground::Hole)) {
                    assert_eq!(effects.held(cell).unwrap().ground, Ground::Solid);
                }
            }
        }
        finish_weather(&mut app);
    }

    #[test]
    fn rain_fades_over_the_board_before_adding_water() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.world_mut().resource_mut::<Weather>().rain = 1;
        app.update();
        press_key(&mut app, KeyCode::KeyR);
        let overlay = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<RainOverlay>>()
                .single(world)
                .unwrap()
        };
        let sprite = app.world().get::<Sprite>(overlay).unwrap();
        assert_eq!(
            sprite.custom_size,
            Some(Vec2::new(
                BOARD_RIGHT - BOARD_LEFT,
                BOARD_TOP - BOARD_BOTTOM
            ))
        );
        assert_eq!(sprite.color.to_srgba().alpha, 0.0);
        tick_coatl_animation(&mut app, 0.625);
        let first = app
            .world()
            .get::<Sprite>(overlay)
            .unwrap()
            .color
            .to_srgba()
            .alpha;
        tick_coatl_animation(&mut app, 0.625);
        let peak = app
            .world()
            .get::<Sprite>(overlay)
            .unwrap()
            .color
            .to_srgba()
            .alpha;
        tick_coatl_animation(&mut app, 0.625);
        let last = app
            .world()
            .get::<Sprite>(overlay)
            .unwrap()
            .color
            .to_srgba()
            .alpha;
        assert!(first > 0.0 && peak > first && (first - last).abs() < 0.001);
        assert_eq!(terrain_count(&app, Ground::Water), 0);
        tick_coatl_animation(&mut app, 0.625);
        assert!(app.world().get_entity(overlay).is_err());
        assert_eq!(terrain_count(&app, Ground::Water), 1);
        assert!(app.world().resource::<TurnState>().effects_active);
        finish_weather(&mut app);
    }

    #[test]
    fn simultaneous_rain_and_earthquake_play_in_order() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        {
            let mut weather = app.world_mut().resource_mut::<Weather>();
            weather.rain = 1;
            weather.earthquake = 1;
        }
        app.update();
        press_key(&mut app, KeyCode::KeyR);
        assert!(matches!(
            app.world()
                .resource::<TurnState>()
                .weather_animation
                .as_ref()
                .unwrap()
                .phase,
            WeatherPhase::Playing {
                kind: WeatherKind::Rain,
                ..
            }
        ));
        tick_coatl_animation(&mut app, WEATHER_ANIMATION_SECONDS);
        assert_eq!(terrain_count(&app, Ground::Water), 1);
        assert!(app.world().resource::<TurnState>().effects_active);
        assert_eq!(
            terrain_count(&app, Ground::Ridge) + terrain_count(&app, Ground::Hole),
            0
        );
        tick_coatl_animation(&mut app, 1.5);
        app.update();
        assert!(matches!(
            app.world()
                .resource::<TurnState>()
                .weather_animation
                .as_ref()
                .unwrap()
                .phase,
            WeatherPhase::Playing {
                kind: WeatherKind::Earthquake,
                ..
            }
        ));
        tick_coatl_animation(&mut app, WEATHER_ANIMATION_SECONDS);
        assert_eq!(
            terrain_count(&app, Ground::Ridge) + terrain_count(&app, Ground::Hole),
            2
        );
        finish_weather(&mut app);
    }

    #[test]
    fn weather_rain_only_adds_water_below_the_33_percent_threshold() {
        for (initial, expected) in [(23, 24), (24, 24)] {
            let mut app = interaction_app();
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                for index in 0..initial {
                    board.make_water(IVec2::new(
                        (index % COLUMNS) as i32,
                        (index / COLUMNS) as i32,
                    ));
                }
            }
            app.world_mut().resource_mut::<Weather>().rain = 1;
            press_key(&mut app, KeyCode::KeyR);
            finish_weather(&mut app);
            assert_eq!(terrain_count(&app, Ground::Water), expected);
            assert!(RAIN_INTERVAL.contains(&app.world().resource::<Weather>().rain));
        }
    }

    #[test]
    fn weather_earthquake_respects_unsafe_cap_and_players() {
        let mut app = interaction_app();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for index in 0..board::MAX_UNSAFE_TILES - 1 {
                board.set(
                    IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32),
                    Ground::Hole,
                );
            }
        }
        let players = [IVec2::new(0, 6), IVec2::new(9, 6)];
        move_player_to(&mut app, PlayerId::One, players[0]);
        move_player_to(&mut app, PlayerId::Two, players[1]);
        app.world_mut().resource_mut::<Weather>().earthquake = 1;
        press_key(&mut app, KeyCode::KeyR);
        finish_weather(&mut app);
        assert_eq!(
            terrain_count(&app, Ground::Hole) + terrain_count(&app, Ground::Ridge),
            board::MAX_UNSAFE_TILES
        );
        for cell in players {
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Solid)
            );
        }
        let board = app.world().resource::<Board>();
        assert!(board.can_create_unsafe(&[]) && !board.can_create_unsafe(&[players[0]]));
        app.world_mut().resource_mut::<Weather>().earthquake = 1;
        press_key(&mut app, KeyCode::KeyR);
        finish_weather(&mut app);
        assert_eq!(
            terrain_count(&app, Ground::Hole) + terrain_count(&app, Ground::Ridge),
            board::MAX_UNSAFE_TILES
        );
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert!(
            feedback
                .single(world)
                .unwrap()
                .0
                .contains("no safe terrain changes")
        );
    }

    #[test]
    fn weather_earthquakes_can_create_both_ridges_and_holes() {
        let mut seen_ridge = false;
        let mut seen_hole = false;
        for seed in 0..8 {
            let mut app = interaction_app();
            app.update();
            {
                let world = app.world_mut();
                let mut rng = world.query_filtered::<&mut WyRand, With<GlobalRng>>();
                *rng.single_mut(world).unwrap() = WyRand::seed_from_u64(seed);
            }
            app.world_mut().resource_mut::<Weather>().earthquake = 1;
            press_key(&mut app, KeyCode::KeyR);
            finish_weather(&mut app);
            seen_ridge |= terrain_count(&app, Ground::Ridge) > 0;
            seen_hole |= terrain_count(&app, Ground::Hole) > 0;
        }
        assert!(seen_ridge && seen_hole);
    }

    #[test]
    fn weather_avoids_items_and_existing_effects() {
        for rain in [false, true] {
            let mut app = interaction_app();
            let safe = IVec2::ZERO;
            let item_cell = IVec2::new(1, 0);
            {
                let world = app.world_mut();
                let mut board = world.resource_mut::<Board>();
                for y in 0..ROWS {
                    for x in 0..COLUMNS {
                        let cell = IVec2::new(x as i32, y as i32);
                        if cell != safe && cell != item_cell {
                            board.add_oil(cell);
                        }
                    }
                }
                world.spawn((
                    BoardItem,
                    Collectible {
                        index: SUNSTONE_ITEM_INDEX,
                        cell: item_cell,
                    },
                ));
            }
            if rain {
                app.world_mut().resource_mut::<Weather>().rain = 1;
            } else {
                app.world_mut().resource_mut::<Weather>().earthquake = 1;
            }
            press_key(&mut app, KeyCode::KeyR);
            finish_weather(&mut app);
            let board = app.world().resource::<Board>();
            assert_eq!(board.ground(item_cell), Some(Ground::Solid));
            assert_eq!(board.ground(IVec2::new(2, 0)), Some(Ground::Solid));
            assert!(board.has_oil(IVec2::new(2, 0)));
            if rain {
                assert_eq!(board.ground(safe), Some(Ground::Water));
            } else {
                assert!(matches!(
                    board.ground(safe),
                    Some(Ground::Ridge | Ground::Hole)
                ));
            }
            assert_eq!(item_count(&mut app), 1);
        }
    }

    #[test]
    fn placement_waits_for_movement_and_explosion_uses_final_positions() {
        let mut app = interaction_app();
        let oil = IVec2::new(6, 3);
        let destination = IVec2::new(4, 3);
        app.world_mut().resource_mut::<Board>().add_oil(oil);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 2;
        app.update();

        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click_part(&mut app, cell_center(oil));
        assert!(app.world().resource::<Board>().has_oil(oil));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        let world = app.world_mut();
        let text = world
            .query_filtered::<&Text, With<TurnProgress>>()
            .single(world)
            .unwrap();
        assert_eq!(text.0, "ACTIONS: 1 AVAILABLE | REST: +2");

        select_move(&mut app);
        click_part(&mut app, cell_center(destination));
        assert!(!app.world().resource::<Board>().has_oil(oil));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.strength[0], 3);
        assert_eq!(turn.actions_used, 0);
        let world = app.world_mut();
        let text = world
            .query_filtered::<&Text, With<TurnProgress>>()
            .single(world)
            .unwrap();
        assert_eq!(text.0, "ACTIONS: 2 AVAILABLE | REST: FULL");
    }

    #[test]
    fn frozen_turn_status_keeps_actions_and_rest_visible() {
        let mut app = interaction_app();
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.frozen_turns[0] = 2;
            turn.actions_used = 1;
        }
        app.update();
        let world = app.world_mut();
        let text = world
            .query_filtered::<&Text, With<TurnProgress>>()
            .single(world)
            .unwrap();
        assert_eq!(
            text.0,
            "ACTIONS: 1 AVAILABLE | REST: +2 | FROZEN: 2 TURN(S)"
        );
    }

    #[test]
    fn weather_status_keeps_actions_and_rest_visible() {
        let mut app = App::new();
        let turn = TurnState {
            weather_animation: Some(WeatherAnimation {
                pending: Default::default(),
                phase: WeatherPhase::Playing {
                    kind: WeatherKind::Rain,
                    elapsed: 0.0,
                    overlay: None,
                },
                notices: vec![],
            }),
            ..default()
        };
        app.insert_resource(turn)
            .add_systems(Update, update_turn_progress);
        let entity = app.world_mut().spawn((TurnProgress, Text::default())).id();
        app.update();
        assert_eq!(
            app.world().get::<Text>(entity).unwrap().0,
            "ACTIONS: 2 AVAILABLE | REST: FULL | RAIN..."
        );
    }

    #[test]
    fn aquatic_animation_status_uses_ahuizotl_name() {
        let mut app = App::new();
        let turn = TurnState {
            aquatic_animation: Some(AquaticAnimation {
                visuals: vec![],
                creatures: vec![],
                frames: Default::default(),
                swept: Default::default(),
                unflattened: Default::default(),
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            }),
            ..default()
        };
        app.insert_resource(turn)
            .add_systems(Update, update_turn_progress);
        let entity = app.world_mut().spawn((TurnProgress, Text::default())).id();
        app.update();
        assert_eq!(
            app.world().get::<Text>(entity).unwrap().0,
            "ACTIONS: 2 AVAILABLE | REST: FULL | AHUIZOTL MOVING..."
        );
    }

    #[test]
    fn queued_direct_hit_uses_the_players_final_tile() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert!(!app.world().resource::<Board>().is_frozen(target));
        select_move(&mut app);
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 2);
        let world = app.world_mut();
        assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 1);
        assert_eq!(world.query::<&PendingFrost>().iter(world).count(), 0);
        assert!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY))
        );
    }

    #[test]
    fn queued_frost_hit_leaves_a_crystal_under_a_hovering_player() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(THORNHEART_ITEM_INDEX));
        click_part(&mut app, cell_center(IVec2::new(4, 3)));
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
        assert!(app.world().resource::<TurnState>().hovering[1]);
        assert!(
            !app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .any(|(item, _)| *item == FROST_CRYSTAL_ITEM_INDEX)
        );
        assert_eq!(
            app.world_mut()
                .query::<&PendingFrost>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn frost_missed_by_hovering_player_lands_in_corner_and_triggers_on_landing() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.update();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(cell));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyR);
            keys.clear();
        }
        assert_eq!(
            app.world()
                .resource::<TurnState>()
                .item_flight
                .as_ref()
                .unwrap()
                .destination,
            cell_center(cell) + Vec2::splat(ITEM_CORNER_OFFSET)
        );
        tick_coatl_animation(&mut app, ITEM_FLIGHT_SECONDS + 0.01);
        {
            let world = app.world_mut();
            assert!(!world.resource::<TurnState>().effects_active);
            assert!(!world.resource::<TurnState>().is_frozen(PlayerId::Two));
            let mut crystals =
                world.query_filtered::<(&Collectible, &Transform, &Sprite), With<PendingFrost>>();
            let (item, transform, sprite) = crystals.single(world).unwrap();
            assert_eq!(item.cell, cell);
            assert_eq!(
                transform.translation.truncate(),
                cell_center(cell) + Vec2::splat(ITEM_CORNER_OFFSET)
            );
            assert_eq!(sprite.custom_size, Some(Vec2::splat(24.0)));
            assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
        }
        click_part(&mut app, Vec2::new(330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[1]);
        assert!(app.world().resource::<TurnState>().is_frozen(PlayerId::Two));
        assert!(app.world().resource::<TurnState>().effects_active);
        assert_eq!(item_count(&mut app), 0);
        tick_coatl_animation(&mut app, 1.0);
        select_move(&mut app);
        assert!(app.world().resource::<Selection>().0.is_none());
        press_key(&mut app, KeyCode::KeyR);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.frozen_turns[1], 2);
        assert_eq!(
            turn.ingredient_respawns,
            [(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn stepping_on_placed_frost_can_be_undone_before_the_turn_finishes() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        select_move(&mut app);
        click_part(&mut app, cell_center(cell));
        assert!(app.world().resource::<TurnState>().is_frozen(PlayerId::Two));
        {
            let world = app.world_mut();
            let mut hidden = world.query_filtered::<&Visibility, With<PickedUpForUndo>>();
            assert_eq!(hidden.single(world).unwrap(), &Visibility::Hidden);
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
        }
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
        click_part(&mut app, Vec2::new(330.0, -60.0));
        assert!(!app.world().resource::<TurnState>().is_frozen(PlayerId::Two));
        assert_eq!(item_count(&mut app), 1);
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );

        select_move(&mut app);
        click_part(&mut app, cell_center(cell));
        assert!(app.world().resource::<TurnState>().is_frozen(PlayerId::Two));
        press_key(&mut app, KeyCode::KeyR);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.frozen_turns[1], 2);
        assert_eq!(
            turn.ingredient_respawns,
            [(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn automatic_landing_on_placed_frost_freezes_the_player() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        app.world_mut().resource_mut::<TurnState>().magic[1] = 1;
        press_key(&mut app, KeyCode::KeyR);
        let turn = app.world().resource::<TurnState>();
        assert!(!turn.hovering[1]);
        assert_eq!(turn.frozen_turns[1], 2);
        assert_eq!(
            turn.ingredient_respawns,
            [(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn hovering_over_placed_frost_and_walking_over_spawned_frost_are_safe() {
        for placed in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX] = 0;
            let mut crystal = app.world_mut().spawn((
                BoardItem,
                PendingFrost(cell),
                Collectible {
                    index: FROST_CRYSTAL_ITEM_INDEX,
                    cell,
                },
            ));
            if placed {
                crystal.insert(PlacedItem);
                app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
            }
            select_move(&mut app);
            click_part(&mut app, cell_center(cell));
            assert!(!app.world().resource::<TurnState>().is_frozen(PlayerId::One));
            assert_eq!(item_count(&mut app), 1);
            if !placed {
                click_part(&mut app, Vec2::new(-330.0, 60.0));
                assert_eq!(
                    app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
                    1
                );
            }
        }
    }

    #[test]
    fn frost_triggered_by_second_move_lasts_two_future_turns() {
        let mut app = interaction_app();
        let cell = IVec2::new(7, 3);
        app.world_mut().spawn((
            BoardItem,
            PlacedItem,
            PendingFrost(cell),
            Collectible {
                index: FROST_CRYSTAL_ITEM_INDEX,
                cell,
            },
        ));
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        select_move(&mut app);
        click_part(&mut app, cell_center(cell));
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 2);
        assert_eq!(item_count(&mut app), 0);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 1);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 0);
    }

    #[test]
    fn multiple_placed_frost_crystals_are_restored_or_consumed_together() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        for _ in 0..2 {
            app.world_mut().spawn((
                BoardItem,
                PlacedItem,
                PendingFrost(cell),
                Collectible {
                    index: FROST_CRYSTAL_ITEM_INDEX,
                    cell,
                },
            ));
        }
        select_move(&mut app);
        click_part(&mut app, cell_center(cell));
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(item_count(&mut app), 2);
        assert!(!app.world().resource::<TurnState>().is_frozen(PlayerId::One));
        select_move(&mut app);
        click_part(&mut app, cell_center(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [
                (FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY),
                (FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)
            ]
        );
    }

    #[test]
    fn moving_then_placing_or_picking_up_completes_one_turn() {
        for pickup in [false, true] {
            let mut app = interaction_app();
            let destination = IVec2::new(6, 3);
            let target = IVec2::new(6, 2);
            app.world_mut().resource_mut::<TurnState>().strength[0] = 1;
            if pickup {
                app.world_mut().resource_mut::<TurnState>().inventory[0]
                    [FROST_CRYSTAL_ITEM_INDEX] = 0;
                app.world_mut().spawn((
                    BoardItem,
                    Collectible {
                        index: FROST_CRYSTAL_ITEM_INDEX,
                        cell: destination,
                    },
                ));
            }
            select_move(&mut app);
            click_part(&mut app, cell_center(destination));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
            assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
            assert_eq!(app.world().resource::<TurnState>().strength[0], 1);
            if pickup {
                click_part(&mut app, Vec2::new(-330.0, 60.0));
                assert_eq!(
                    app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
                    1
                );
            } else {
                click_part(&mut app, p1_item(WATER_ITEM_INDEX));
                click_part(&mut app, cell_center(target));
                assert!(app.world().resource::<Board>().is_water(target));
            }
            let turn = app.world().resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::Two);
            assert_eq!(turn.strength[0], 2);
        }
    }

    #[test]
    fn pickup_then_move_and_partial_or_full_rest() {
        for first in ["move", "placement", "pickup", "none"] {
            let mut app = interaction_app();
            app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
            let start = IVec2::new(5, 3);
            let next = IVec2::new(6, 3);
            match first {
                "move" => {
                    select_move(&mut app);
                    click_part(&mut app, cell_center(next));
                }
                "placement" => {
                    click_part(&mut app, p1_item(WATER_ITEM_INDEX));
                    click_part(&mut app, cell_center(next));
                    assert!(!app.world().resource::<Board>().is_water(next));
                }
                "pickup" => {
                    app.world_mut().resource_mut::<TurnState>().inventory[0]
                        [FROST_CRYSTAL_ITEM_INDEX] = 0;
                    app.world_mut().spawn((
                        BoardItem,
                        Collectible {
                            index: FROST_CRYSTAL_ITEM_INDEX,
                            cell: start,
                        },
                    ));
                    click_part(&mut app, Vec2::new(-330.0, 60.0));
                    assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
                    select_move(&mut app);
                    click_part(&mut app, cell_center(next));
                    assert_eq!(app.world().resource::<TurnState>().strength[0], 1);
                    assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
                    continue;
                }
                _ => {}
            }
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert_eq!(
                app.world().resource::<TurnState>().strength[0],
                if first == "none" { 5 } else { 2 }
            );
            if first == "placement" {
                assert!(app.world().resource::<Board>().is_water(next));
            }
        }
    }

    #[test]
    fn queued_placement_reserves_energy_before_a_jump() {
        let mut app = interaction_app();
        let target = IVec2::new(8, 3);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 3;
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<TurnState>().strength[0], 1);
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(3, 3)));
        assert_eq!(app.world().resource::<TurnState>().strength[0], 0);
        assert!(app.world().resource::<Board>().is_water(target));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn undo_restores_movement_energy_and_cancels_the_next_selection() {
        let mut app = interaction_app();
        let start = IVec2::new(5, 3);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 3;
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(3, 3)));
        assert_eq!(app.world().resource::<TurnState>().strength[0], 2);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_some());
        click_part(&mut app, Vec2::new(330.0, -60.0));
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        let world = app.world_mut();
        let turn = world.resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.actions_used, 0);
        assert_eq!(turn.strength[0], 3);
        assert!(!turn.free_action);
        assert!(world.resource::<Selection>().0.is_none());
        let mut players = world.query::<(&Player, &Transform)>();
        let (_, transform) = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap();
        assert_eq!(board_cell(transform.translation.truncate()), Some(start));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().strength[0], 5);
    }

    #[test]
    fn undo_cancels_a_queued_placement_and_refunds_its_energy_and_item() {
        let mut app = interaction_app();
        let target = IVec2::new(8, 3);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 3;
        app.world_mut().resource_mut::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX] = 1;
        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            0
        );
        {
            let world = app.world_mut();
            let mut counts = world.query::<(&InventoryCount, &Text)>();
            let (_, text) = counts
                .iter(world)
                .find(|(count, _)| {
                    count.owner == PlayerId::One && count.index == SUNSTONE_ITEM_INDEX
                })
                .unwrap();
            assert_eq!(text.0, "0");
        }
        let marker = app
            .world()
            .resource::<TurnState>()
            .pending_placement
            .as_ref()
            .unwrap()
            .marker;
        assert_eq!(app.world().resource::<TurnState>().strength[0], 1);
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.actions_used, 0);
        assert_eq!(turn.strength[0], 3);
        assert_eq!(turn.inventory[0][SUNSTONE_ITEM_INDEX], 1);
        assert!(turn.pending_placement.is_none());
        assert!(app.world().get_entity(marker).is_err());
        assert!(!app.world().resource::<Board>().is_water(target));
        {
            let world = app.world_mut();
            let mut counts = world.query::<(&InventoryCount, &Text)>();
            let (_, text) = counts
                .iter(world)
                .find(|(count, _)| {
                    count.owner == PlayerId::One && count.index == SUNSTONE_ITEM_INDEX
                })
                .unwrap();
            assert_eq!(text.0, "1");
        }
        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_some());
    }

    #[test]
    fn undo_movement_restores_hover_when_the_origin_is_unsafe() {
        let mut app = interaction_app();
        let start = IVec2::new(5, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(start, Ground::Hole);
        app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        click_part(&mut app, Vec2::new(-330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert!(app.world().resource::<TurnState>().hovering[0]);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let (_, transform) = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap();
        assert_eq!(board_cell(transform.translation.truncate()), Some(start));
    }

    #[test]
    fn undo_pickup_restores_original_entities_and_inventory() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0] = [0; 14];
        let frost = app
            .world_mut()
            .spawn((
                BoardItem,
                PendingFrost(cell),
                Collectible {
                    index: FROST_CRYSTAL_ITEM_INDEX,
                    cell,
                },
                Visibility::Visible,
            ))
            .id();
        let storm = app
            .world_mut()
            .spawn((
                BoardItem,
                PendingStormShard(cell),
                Collectible {
                    index: STORM_SHARD_ITEM_INDEX,
                    cell,
                },
            ))
            .id();
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        assert!(app.world().get::<Collectible>(frost).is_none());
        assert!(app.world().get::<Collectible>(storm).is_none());
        assert_eq!(
            app.world().get::<Visibility>(frost),
            Some(&Visibility::Hidden)
        );
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(item_count(&mut app), 2);
        let world = app.world();
        assert_eq!(world.resource::<TurnState>().actions_used, 0);
        assert_eq!(world.resource::<TurnState>().inventory[0], [0; 14]);
        assert!(world.get::<PendingFrost>(frost).is_some());
        assert!(world.get::<PendingStormShard>(storm).is_some());
        assert!(world.get::<PickedUpForUndo>(frost).is_none());
        assert_eq!(world.get::<Visibility>(frost), Some(&Visibility::Visible));
        assert!(world.get::<Visibility>(storm).is_none());
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().get_entity(frost).is_err());
        assert!(app.world().get_entity(storm).is_err());
    }

    #[test]
    fn coatl_egg_and_life_essence_hatch_in_either_order() {
        for water in [false, true] {
            for egg_first in [false, true] {
                let mut app = interaction_app();
                let target = IVec2::new(6, 3);
                app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
                app.world_mut().resource_mut::<PlayerHealth>().0 = [9, 10];
                if water {
                    app.world_mut().resource_mut::<Board>().make_water(target);
                }
                let egg = p1_item(COATL_EGG_ITEM_INDEX);
                let life = p1_item(LIFE_ESSENCE_ITEM_INDEX);
                for position in if egg_first { [egg, life] } else { [life, egg] } {
                    click_part(&mut app, position);
                    click_part(&mut app, cell_center(target));
                }
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.active, PlayerId::Two);
                assert_eq!(turn.inventory[0][COATL_EGG_ITEM_INDEX], 0);
                assert_eq!(turn.inventory[0][LIFE_ESSENCE_ITEM_INDEX], 0);
                assert_eq!(turn.serpents.len(), 1);
                let body: Vec<_> = turn.serpents[0].cells().collect();
                assert_eq!(body.len(), 3);
                assert_eq!((body[0] - target).abs().element_sum(), 2);
                assert!(
                    turn.ingredient_respawns
                        .contains(&(COATL_EGG_ITEM_INDEX, 5))
                );
                assert!(
                    turn.ingredient_respawns
                        .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
                );
                assert!(!app.world().resource::<Board>().has_life_essence(target));
                let world = app.world_mut();
                assert_eq!(world.query::<&SerpentPart>().iter(world).count(), 1);
                assert_eq!(world.query::<&PendingCoatlEgg>().iter(world).count(), 0);
                assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 0);

                finish_coatl_animation(&mut app);
                press_key(&mut app, KeyCode::KeyR);
                finish_coatl_animation(&mut app);
                assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 7]);
                assert!(app.world().resource::<TurnState>().serpents.is_empty());
                let world = app.world_mut();
                assert_eq!(world.query::<&SerpentPart>().iter(world).count(), 0);
            }
        }
    }

    #[test]
    fn aquatic_creature_and_life_essence_activate_in_water_in_either_order() {
        for aquatic_first in [false, true] {
            let mut app = interaction_app();
            let origin = IVec2::new(4, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            app.world_mut().resource_mut::<Board>().make_water(origin);
            let aquatic = p1_item(AQUATIC_ITEM_INDEX);
            let life = p1_item(LIFE_ESSENCE_ITEM_INDEX);
            for icon in if aquatic_first {
                [aquatic, life]
            } else {
                [life, aquatic]
            } {
                click_part(&mut app, icon);
                click_part(&mut app, cell_center(origin));
            }
            let world = app.world_mut();
            let board = world.resource::<Board>();
            assert!(!board.has_life_essence(origin));
            assert_eq!(board.aquatic_cells().count(), 5);
            let turn = world.resource::<TurnState>();
            assert_eq!(turn.inventory[0][AQUATIC_ITEM_INDEX], 0);
            assert_eq!(turn.inventory[0][LIFE_ESSENCE_ITEM_INDEX], 0);
            assert!(turn.ingredient_respawns.contains(&(AQUATIC_ITEM_INDEX, 5)));
            assert!(
                turn.ingredient_respawns
                    .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
            );
            assert_eq!(
                world.query::<&PendingAquaticCreature>().iter(world).count(),
                0
            );
            assert_eq!(world.query::<&AquaticTile>().iter(world).count(), 5);
            for _ in 0..2 {
                press_key(&mut app, KeyCode::KeyR);
                finish_aquatic_animation(&mut app);
            }
            assert!(app.world().resource::<Board>().aquatic_cells().count() > 0);
            press_key(&mut app, KeyCode::KeyR);
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .aquatic_animation
                    .is_some()
            );
            finish_aquatic_animation(&mut app);
            assert_eq!(app.world().resource::<Board>().aquatic_cells().count(), 0);
            let world = app.world_mut();
            assert_eq!(world.query::<&AquaticTile>().iter(world).count(), 0);
        }
    }

    #[test]
    fn aquatic_creature_uses_existing_healing_water_and_does_not_activate_on_dry_ground() {
        let origin = IVec2::new(4, 3);
        let mut app = interaction_app();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.add_life_essence(origin);
        }
        click(&mut app, p1_item(AQUATIC_ITEM_INDEX));
        click(&mut app, cell_center(origin));
        let board = app.world().resource::<Board>();
        assert_eq!(board.aquatic_cells().count(), 5);
        assert!(!board.has_life_essence(origin));
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
        );

        let mut dry = interaction_app();
        for icon in [
            p1_item(AQUATIC_ITEM_INDEX),
            p1_item(LIFE_ESSENCE_ITEM_INDEX),
        ] {
            click_part(&mut dry, icon);
            click_part(&mut dry, cell_center(origin));
        }
        let world = dry.world_mut();
        assert_eq!(world.resource::<Board>().aquatic_cells().count(), 0);
        assert_eq!(
            world.query::<&PendingAquaticCreature>().iter(world).count(),
            1
        );
        assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 1);
        click(&mut dry, p2_item(WATER_ITEM_INDEX));
        click(&mut dry, cell_center(origin));
        assert_eq!(dry.world().resource::<Board>().aquatic_cells().count(), 5);
    }

    #[test]
    fn aquatic_contact_damages_once_per_creature_in_each_growth_and_rotation_turn() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
        }
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        for expected in [8, 6, 4] {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<PlayerHealth>().0, [expected, 10]);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [1, 7]);
        finish_aquatic_animation(&mut app);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [1, 7]);
    }

    #[test]
    fn aquatic_rotation_hits_intermediate_swept_cells_not_just_final_positions() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            for _ in 0..3 {
                board.finish_aquatic_turn();
            }
        }
        let swept = origin + IVec2::new(2, 1);
        move_player_to(&mut app, PlayerId::One, swept);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 7]);
        assert!(
            !app.world()
                .resource::<Board>()
                .aquatic_cells()
                .any(|cell| cell == swept)
        );
    }

    #[test]
    fn aquatic_sweep_removes_items_and_ridges_at_their_animation_frames() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        let base = origin + IVec2::X;
        let intermediate = origin + IVec2::new(2, 1);
        let untouched = IVec2::new(0, 0);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            board.activate_aquatic(origin);
            for _ in 0..3 {
                board.finish_aquatic_turn();
            }
            board.set(intermediate, Ground::Ridge);
        }
        let base_item = app
            .world_mut()
            .spawn((
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell: base,
                },
                BoardItem,
            ))
            .id();
        let intermediate_item = app
            .world_mut()
            .spawn((
                Collectible {
                    index: FROST_CRYSTAL_ITEM_INDEX,
                    cell: intermediate,
                },
                PendingFrost(intermediate),
                BoardItem,
            ))
            .id();
        let unlimited_items: Vec<_> = [
            WATER_ITEM_INDEX,
            VOID_DUST_ITEM_INDEX,
            STONE_DUST_ITEM_INDEX,
        ]
        .into_iter()
        .map(|index| {
            app.world_mut()
                .spawn((Collectible { index, cell: base }, BoardItem))
                .id()
        })
        .collect();
        let untouched_item = app
            .world_mut()
            .spawn((
                Collectible {
                    index: STORM_SHARD_ITEM_INDEX,
                    cell: untouched,
                },
                BoardItem,
            ))
            .id();
        let ridge = app
            .world_mut()
            .spawn((GroundTile(intermediate), Sprite::default()))
            .id();
        press_key(&mut app, KeyCode::KeyR);
        let animation = app
            .world()
            .resource::<TurnState>()
            .aquatic_animation
            .as_ref()
            .unwrap();
        assert!(animation.visuals.contains(&base));
        assert!(!animation.visuals.contains(&(origin + IVec2::new(1, 1))));
        assert_eq!(
            app.world().get::<Sprite>(ridge).unwrap().color,
            Ground::Ridge.color()
        );
        assert!(app.world().get_entity(base_item).is_ok());
        press_key(&mut app, KeyCode::KeyR);
        click_part(&mut app, p2_item(WATER_ITEM_INDEX));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert!(app.world().resource::<Selection>().0.is_none());

        tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
        let animation = app
            .world()
            .resource::<TurnState>()
            .aquatic_animation
            .as_ref()
            .unwrap();
        assert!(animation.visuals.contains(&(origin + IVec2::new(1, 1))));
        assert!(animation.visuals.contains(&(origin + IVec2::new(2, 0))));
        assert!(app.world().get_entity(base_item).is_err());
        assert!(
            unlimited_items
                .iter()
                .all(|&entity| app.world().get_entity(entity).is_err())
        );
        assert!(app.world().get_entity(intermediate_item).is_ok());
        assert_eq!(
            app.world().get::<Sprite>(ridge).unwrap().color,
            Ground::Ridge.color()
        );
        tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
        assert!(app.world().get_entity(intermediate_item).is_err());
        assert_eq!(
            app.world().get::<Sprite>(ridge).unwrap().color,
            Ground::Solid.color()
        );
        finish_aquatic_animation(&mut app);
        assert!(app.world().get_entity(untouched_item).is_ok());
        let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
        assert_eq!(
            respawns
                .iter()
                .filter(|&&(index, _)| index == SUNSTONE_ITEM_INDEX)
                .count(),
            1
        );
        assert_eq!(
            respawns
                .iter()
                .filter(|&&(index, _)| index == FROST_CRYSTAL_ITEM_INDEX)
                .count(),
            1
        );
        assert!(respawns.contains(&(SUNSTONE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)));
        assert!(respawns.contains(&(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)));
        assert!(respawns.iter().all(|&(index, _)| !is_unlimited(index)));
    }

    #[test]
    fn aquatic_sweep_collects_outer_segment_before_middle_segment_finishes() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            for _ in 0..3 {
                board.finish_aquatic_turn();
            }
        }
        let outer = origin + IVec2::new(3, 1);
        let middle = origin + IVec2::new(2, 2);
        let next_outer = origin + IVec2::new(3, 2);
        app.world_mut()
            .resource_mut::<Board>()
            .set(middle, Ground::Ridge);
        let outer_item = app
            .world_mut()
            .spawn((
                BoardItem,
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell: outer,
                },
            ))
            .id();
        let middle_item = app
            .world_mut()
            .spawn((
                BoardItem,
                Collectible {
                    index: FROST_CRYSTAL_ITEM_INDEX,
                    cell: middle,
                },
            ))
            .id();
        let next_outer_item = app
            .world_mut()
            .spawn((
                BoardItem,
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell: next_outer,
                },
            ))
            .id();
        press_key(&mut app, KeyCode::KeyR);
        for _ in 0..2 {
            tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
        }
        assert!(app.world().get_entity(outer_item).is_ok());
        assert!(app.world().get_entity(middle_item).is_ok());
        tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
        assert!(app.world().get_entity(outer_item).is_err());
        assert!(app.world().get_entity(middle_item).is_ok());
        assert!(app.world().get_entity(next_outer_item).is_ok());
        tick_coatl_animation(&mut app, AQUATIC_STEP_SECONDS + 0.01);
        assert!(app.world().get_entity(middle_item).is_err());
        assert!(app.world().get_entity(next_outer_item).is_err());
        assert!(
            app.world()
                .resource::<TurnState>()
                .aquatic_animation
                .as_ref()
                .unwrap()
                .unflattened
                .is_empty()
        );
        finish_aquatic_animation(&mut app);
        let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
        assert_eq!(
            respawns
                .iter()
                .filter(|&&(item, _)| item == SUNSTONE_ITEM_INDEX)
                .count(),
            2
        );
        assert_eq!(
            respawns
                .iter()
                .filter(|&&(item, _)| item == FROST_CRYSTAL_ITEM_INDEX)
                .count(),
            1
        );
    }

    #[test]
    fn aquatic_sweeps_finish_before_coatl_moves_and_keep_input_blocked() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        let players = [Some(IVec2::new(8, 0)), Some(IVec2::new(9, 0))];
        move_player_to(&mut app, PlayerId::One, players[0].unwrap());
        move_player_to(&mut app, PlayerId::Two, players[1].unwrap());
        let serpent = {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            for _ in 0..3 {
                board.finish_aquatic_turn();
            }
            serpent::Serpent::new(IVec2::ZERO, &board, players, [10, 9])
        };
        let initial = serpent.visual();
        app.world_mut().resource_mut::<PlayerHealth>().0 = [10, 9];
        app.world_mut()
            .resource_mut::<TurnState>()
            .serpents
            .push(serpent);
        press_key(&mut app, KeyCode::KeyR);
        finish_aquatic_animation(&mut app);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(
            turn.serpent_animation.as_ref().unwrap().visuals[0].cells,
            initial.cells
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        tick_coatl_animation(&mut app, 0.4);
        assert_ne!(
            app.world()
                .resource::<TurnState>()
                .serpent_animation
                .as_ref()
                .unwrap()
                .visuals[0]
                .cells,
            initial.cells
        );
        finish_coatl_animation(&mut app);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
    }

    #[test]
    fn aquatic_hits_from_different_creatures_stack_and_net_with_healing() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.activate_aquatic(origin);
            board.activate_aquatic(origin);
            let player_cell = origin + IVec2::X;
            board.make_water(player_cell);
            board.add_life_essence(player_cell);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 10]);
    }

    #[test]
    fn aquatic_contact_during_movement_is_recorded_and_undo_cancels_it() {
        for undo in [false, true] {
            let mut app = interaction_app();
            let origin = IVec2::new(4, 3);
            let start = IVec2::new(6, 2);
            let contact = origin + IVec2::X;
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(origin);
                board.activate_aquatic(origin);
                board.finish_aquatic_turn();
            }
            move_player_to(&mut app, PlayerId::One, start);
            select_move(&mut app);
            click_part(&mut app, cell_center(contact));
            if undo {
                click_part(&mut app, Vec2::new(-330.0, -60.0));
                press_key(&mut app, KeyCode::KeyR);
            } else {
                select_move(&mut app);
                click_part(&mut app, cell_center(start));
            }
            assert_eq!(
                app.world().resource::<PlayerHealth>().0[0],
                if undo { 10 } else { 8 }
            );
        }
    }

    #[test]
    fn aquatic_creature_does_not_hit_previous_positions_before_it_awakens() {
        let mut app = interaction_app();
        let origin = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.add_life_essence(origin);
        }
        let destination = IVec2::new(6, 2);
        select_move(&mut app);
        click_part(&mut app, cell_center(destination));
        click_part(&mut app, p1_item(AQUATIC_ITEM_INDEX));
        click_part(&mut app, cell_center(origin));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<Board>().aquatic_cells().count(), 5);
    }

    #[test]
    fn coatl_egg_can_hatch_with_active_life_essence() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        app.world_mut().resource_mut::<PlayerHealth>().0 = [9, 10];
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(target);
            board.add_life_essence(target);
        }
        click(&mut app, p1_item(COATL_EGG_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<TurnState>().serpents.len(), 1);
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        let turn = app.world().resource::<TurnState>();
        assert!(
            turn.ingredient_respawns
                .contains(&(COATL_EGG_ITEM_INDEX, 5))
        );
        assert!(
            turn.ingredient_respawns
                .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn coatl_animates_each_step_and_blocks_actions_between_turns() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<PlayerHealth>().0 = [9, 10];
        for item in [
            p1_item(COATL_EGG_ITEM_INDEX),
            p1_item(LIFE_ESSENCE_ITEM_INDEX),
        ] {
            click_part(&mut app, item);
            click_part(&mut app, cell_center(target));
        }
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert_eq!(animation.visuals[0].cells[0], target);
        assert_eq!(animation.frames.len(), 2);
        press_key(&mut app, KeyCode::KeyR);
        click_part(&mut app, p2_item(WATER_ITEM_INDEX));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.actions_used, 0);
        assert!(app.world().resource::<Selection>().0.is_none());

        for distance in [1, 2] {
            tick_coatl_animation(&mut app, 0.4);
            let animation = app
                .world()
                .resource::<TurnState>()
                .serpent_animation
                .as_ref()
                .unwrap();
            assert_eq!(
                (animation.visuals[0].cells[0] - target).abs().element_sum(),
                distance
            );
        }
        tick_coatl_animation(&mut app, 0.4);
        assert!(
            app.world()
                .resource::<TurnState>()
                .serpent_animation
                .is_none()
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
    }

    #[test]
    fn coatl_hit_is_visible_and_applies_damage_during_the_animation() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<PlayerHealth>().0 = [10, 9];
        for item in [
            p1_item(COATL_EGG_ITEM_INDEX),
            p1_item(LIFE_ESSENCE_ITEM_INDEX),
        ] {
            click_part(&mut app, item);
            click_part(&mut app, cell_center(target));
        }
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert!(app.world().resource::<TurnState>().serpents.is_empty());
        assert_eq!(
            app.world()
                .resource::<TurnState>()
                .serpent_animation
                .as_ref()
                .unwrap()
                .visuals[0]
                .cells[0],
            target
        );
        tick_coatl_animation(&mut app, 0.4);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 9]);
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert_eq!(animation.visuals[0].cells[0], IVec2::new(5, 3));
        assert_eq!(animation.impact, Some(IVec2::new(5, 3)));
        let world = app.world_mut();
        assert_eq!(world.query::<&SerpentPart>().iter(world).count(), 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        tick_coatl_animation(&mut app, 0.4);
        assert!(
            app.world()
                .resource::<TurnState>()
                .serpent_animation
                .is_some()
        );
        tick_coatl_animation(&mut app, 0.3);
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert!(animation.visuals.is_empty());
        assert!(animation.impact.is_none());
        tick_coatl_animation(&mut app, 0.4);
        assert!(
            app.world()
                .resource::<TurnState>()
                .serpent_animation
                .is_none()
        );
    }

    #[test]
    fn coatl_interception_hits_the_blocker_on_the_visible_impact_frame() {
        let mut app = interaction_app();
        let blocker = IVec2::new(4, 3);
        let target = IVec2::new(6, 3);
        let players = [Some(blocker), Some(target)];
        let health = [6, 10];
        move_player_to(&mut app, PlayerId::One, blocker);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<PlayerHealth>().0 = health;
        app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
        let serpent = serpent::Serpent::new(
            IVec2::new(3, 3),
            app.world().resource::<Board>(),
            players,
            health,
        );
        app.world_mut()
            .resource_mut::<TurnState>()
            .serpents
            .push(serpent);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, health);
        tick_coatl_animation(&mut app, 0.4);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [3, 10]);
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert_eq!(animation.impact, Some(blocker));
        assert_eq!(animation.visuals[0].cells[0], blocker);
        finish_coatl_animation(&mut app);
        assert!(app.world().resource::<TurnState>().serpents.is_empty());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [3, 10]);
    }

    #[test]
    fn multiple_coatls_animate_in_order_and_retarget_after_the_first_hit() {
        let mut app = interaction_app();
        let players = [Some(IVec2::new(5, 3)), Some(IVec2::new(7, 6))];
        let health = [10, 9];
        let serpents = {
            let board = app.world().resource::<Board>();
            [IVec2::new(6, 3), IVec2::new(6, 6)]
                .into_iter()
                .map(|cell| serpent::Serpent::new(cell, board, players, health))
                .collect()
        };
        app.world_mut().resource_mut::<PlayerHealth>().0 = health;
        app.world_mut().resource_mut::<TurnState>().serpents = serpents;
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        tick_coatl_animation(&mut app, 0.4);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 9]);
        tick_coatl_animation(&mut app, 0.7);
        tick_coatl_animation(&mut app, 0.4);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 6]);
        finish_coatl_animation(&mut app);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 6]);
    }

    #[test]
    fn poisonous_coatl_kills_on_the_visible_hit_frame() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let victim = IVec2::new(5, 3);
        app.world_mut().resource_mut::<PlayerHealth>().0 = [10, 9];
        app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
        app.world_mut().resource_mut::<Board>().add_poison(victim);
        let serpent = {
            let board = app.world().resource::<Board>();
            serpent::Serpent::new(
                source,
                board,
                [Some(victim), Some(IVec2::new(7, 6))],
                [10, 9],
            )
        };
        app.world_mut()
            .resource_mut::<TurnState>()
            .serpents
            .push(serpent);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        tick_coatl_animation(&mut app, 0.4);
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert!(animation.visuals[0].venomous);
        assert_eq!(animation.impact, Some(victim));
        let expected = serpent_sprite(&animation.visuals[0], Handle::default())
            .0
            .rect;
        assert_eq!(app.world().resource::<PlayerHealth>().0, [0, 9]);
        {
            let world = app.world_mut();
            let mut serpents = world.query_filtered::<(Entity, &Sprite), With<SerpentPart>>();
            let (entity, sprite) = serpents
                .iter(world)
                .find(|(_, sprite)| sprite.rect.is_some())
                .unwrap();
            assert_eq!(sprite.rect, expected);
            assert!(world.get::<Children>(entity).is_none());
        }
        finish_coatl_animation(&mut app);
    }

    #[test]
    fn coatl_hazard_death_is_shown_before_the_serpent_disappears() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let acid = IVec2::new(7, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(9, 3));
        app.world_mut().resource_mut::<PlayerHealth>().0 = [1, 10];
        app.world_mut().resource_mut::<Board>().add_acid(acid);
        let serpent = {
            let board = app.world().resource::<Board>();
            serpent::Serpent::new(
                source,
                board,
                [Some(IVec2::new(5, 3)), Some(IVec2::new(9, 3))],
                [1, 10],
            )
        };
        app.world_mut()
            .resource_mut::<TurnState>()
            .serpents
            .push(serpent);
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<TurnState>().serpents.is_empty());
        tick_coatl_animation(&mut app, 0.4);
        let animation = app
            .world()
            .resource::<TurnState>()
            .serpent_animation
            .as_ref()
            .unwrap();
        assert_eq!(animation.visuals[0].cells[0], acid);
        assert_eq!(animation.impact, Some(acid));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [1, 10]);
        finish_coatl_animation(&mut app);
    }

    #[test]
    fn respawned_coatl_egg_is_available_for_the_reaction() {
        let mut app = interaction_app();
        app.world_mut()
            .resource_mut::<TurnState>()
            .ingredient_respawns
            .push((COATL_EGG_ITEM_INDEX, 1));
        for _ in 0..3 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        let (egg, pending) = world
            .query::<(&Collectible, &PendingCoatlEgg)>()
            .single(world)
            .unwrap();
        assert_eq!(egg.index, COATL_EGG_ITEM_INDEX);
        assert_eq!(egg.cell, pending.0);
    }

    #[test]
    fn two_items_combine_in_one_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        let initial_spawned = app.world().resource::<TurnState>().spawned_ingredients;
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_some());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(
            app.world().resource::<TurnState>().spawned_ingredients,
            initial_spawned
        );
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(cell_center(target));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        finish_item_flights(&mut app);
        assert!(app.world().resource::<Board>().is_water(target));
        assert!(app.world().resource::<Board>().is_frozen(target));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(
            app.world().resource::<TurnState>().spawned_ingredients,
            initial_spawned
        );
        let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
        assert!(!respawns.iter().any(|(index, _)| *index == WATER_ITEM_INDEX));
        assert!(respawns.contains(&(FROST_CRYSTAL_ITEM_INDEX, 5)));
    }

    #[test]
    fn second_item_can_use_a_hole_filled_by_the_first_item() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        assert!(app.world_mut().resource_mut::<Board>().make_hole(target));
        app.world_mut().resource_mut::<Board>().take_new_holes();
        click_part(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert!(app.world().resource::<Board>().is_water(target));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn two_moves_complete_one_turn_and_recover_at_most_one_energy() {
        let mut app = interaction_app();
        let middle = IVec2::new(6, 3);
        let destination = IVec2::new(7, 3);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        select_move(&mut app);
        click_part(&mut app, cell_center(middle));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        select_move(&mut app);
        click_part(&mut app, cell_center(destination));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.strength[0], 1);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        assert_eq!(
            players
                .iter(world)
                .find(|(player, _)| player.0 == PlayerId::One)
                .unwrap()
                .1
                .translation
                .truncate(),
            cell_center(destination)
        );
    }

    #[test]
    fn two_items_ignite_oil_in_either_order_without_an_opponent_turn() {
        for oil_first in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            let oil = p1_item(OIL_ITEM_INDEX);
            let sun = p1_item(SUNSTONE_ITEM_INDEX);
            for item in if oil_first { [oil, sun] } else { [sun, oil] } {
                click_part(&mut app, item);
                click_part(&mut app, cell_center(target));
            }
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert!(!app.world().resource::<Board>().has_oil(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 8]);
            assert_eq!(
                app.world_mut()
                    .query::<&PendingSunstone>()
                    .iter(app.world())
                    .count(),
                0
            );
        }
    }

    #[test]
    fn two_direct_hits_are_combined_before_health_is_applied() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        click_part(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        let world = app.world_mut();
        assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
        let respawns = &world.resource::<TurnState>().ingredient_respawns;
        assert_eq!(respawns.len(), 2);
        assert!(respawns.contains(&(FROST_CRYSTAL_ITEM_INDEX, 5)));
        assert!(respawns.contains(&(STORM_SHARD_ITEM_INDEX, 5)));
    }

    #[test]
    fn queued_direct_hits_consume_each_item_once_but_misses_remain_collectible() {
        for (index, icon) in [
            (FROST_CRYSTAL_ITEM_INDEX, p1_item(FROST_CRYSTAL_ITEM_INDEX)),
            (STORM_SHARD_ITEM_INDEX, p1_item(STORM_SHARD_ITEM_INDEX)),
        ] {
            for hit in [false, true] {
                let mut app = interaction_app();
                let target = IVec2::new(6, 3);
                if hit {
                    move_player_to(&mut app, PlayerId::Two, target);
                }
                for _ in 0..2 {
                    click_part(&mut app, icon);
                    click_part(&mut app, cell_center(target));
                }
                let world = app.world_mut();
                assert_eq!(
                    world.resource::<PlayerHealth>().0,
                    [
                        10,
                        if hit && index == STORM_SHARD_ITEM_INDEX {
                            8
                        } else {
                            10
                        }
                    ]
                );
                assert_eq!(
                    world.query::<&Collectible>().iter(world).count(),
                    if hit { 0 } else { 2 }
                );
                let respawns = &world.resource::<TurnState>().ingredient_respawns;
                assert_eq!(
                    respawns
                        .iter()
                        .filter(|&&(item, delay)| item == index && delay == INGREDIENT_RESPAWN_DELAY)
                        .count(),
                    if hit { 2 } else { 0 }
                );
                if hit {
                    press_key(&mut app, KeyCode::KeyR);
                    assert_eq!(
                        app.world()
                            .resource::<TurnState>()
                            .ingredient_respawns
                            .iter()
                            .filter(|&&(item, delay)| item == index && delay == 4)
                            .count(),
                        2
                    );
                }
            }
        }
    }

    #[test]
    fn same_turn_reactions_do_not_add_an_unreactive_direct_hit() {
        for frost in [false, true] {
            for hit_first in [false, true] {
                let mut app = interaction_app();
                let target = IVec2::new(6, 3);
                move_player_to(&mut app, PlayerId::Two, target);
                let hit = if frost {
                    p1_item(FROST_CRYSTAL_ITEM_INDEX)
                } else {
                    p1_item(STORM_SHARD_ITEM_INDEX)
                };
                let other = if frost {
                    p1_item(WATER_ITEM_INDEX)
                } else {
                    p1_item(OBSIDIAN_ITEM_INDEX)
                };
                for item in if hit_first {
                    [hit, other]
                } else {
                    [other, hit]
                } {
                    click_part(&mut app, item);
                    click_part(&mut app, cell_center(target));
                }
                assert_eq!(
                    app.world().resource::<PlayerHealth>().0[1],
                    if frost { 10 } else { 7 },
                    "frost={frost} hit_first={hit_first}"
                );
                let turn = app.world().resource::<TurnState>();
                let consumed = if frost {
                    FROST_CRYSTAL_ITEM_INDEX
                } else {
                    STORM_SHARD_ITEM_INDEX
                };
                assert_eq!(
                    turn.ingredient_respawns
                        .iter()
                        .filter(|&&(index, delay)| index == consumed && delay == 5)
                        .count(),
                    1,
                    "frost={frost} hit_first={hit_first}"
                );
            }
        }
    }

    #[test]
    fn second_item_invalidated_by_first_can_be_replaced_or_rested() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0][WATER_ITEM_INDEX] = 2;
        for _ in 0..2 {
            click_part(&mut app, p1_item(WATER_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
        }
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.actions_used, 1);
        assert_eq!(turn.inventory[0][WATER_ITEM_INDEX], 2);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        let world = app.world_mut();
        let feedback = world
            .query_filtered::<&Text, With<DropFeedback>>()
            .single(world)
            .unwrap();
        assert!(feedback.0.contains("Cannot place water"));
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert!(app.world().resource::<Board>().is_water(target));
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
    }

    #[test]
    fn queued_item_is_reserved_until_it_resolves() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX] = 1;
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            0
        );
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        assert!(!app.world().resource::<Board>().is_water(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            0
        );
    }

    #[test]
    fn two_copies_can_be_queued_and_are_only_spent_once_each() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX] = 2;
        for target in [IVec2::new(6, 3), IVec2::new(7, 3)] {
            click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
        }
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.inventory[0][FROST_CRYSTAL_ITEM_INDEX], 0);
        assert_eq!(
            app.world_mut()
                .query::<&PendingFrost>()
                .iter(app.world())
                .count(),
            2
        );
    }

    #[test]
    fn queued_second_item_rejected_at_terrain_limit_is_returned() {
        let mut app = interaction_app();
        let targets = [IVec2::new(6, 3), IVec2::new(7, 3)];
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            let mut holes = 0;
            for y in 0..ROWS {
                for x in 0..COLUMNS {
                    let cell = IVec2::new(x as i32, y as i32);
                    if holes < board::MAX_UNSAFE_TILES - 5
                        && !targets
                            .iter()
                            .any(|target| (cell - *target).abs().element_sum() <= 1)
                        && ![IVec2::new(5, 3), IVec2::new(7, 6)].contains(&cell)
                    {
                        board.set(cell, Ground::Hole);
                        holes += 1;
                    }
                }
            }
        }
        for cell in targets {
            app.world_mut().spawn((
                BoardItem,
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
                PendingObsidian(cell),
            ));
        }
        app.world_mut().resource_mut::<TurnState>().inventory[0][STONE_DUST_ITEM_INDEX] = 2;
        for cell in targets {
            click_part(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
            click_part(&mut app, cell_center(cell));
        }
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.actions_used, 1);
        assert_eq!(turn.inventory[0][STONE_DUST_ITEM_INDEX], 2);
        assert_eq!(turn.strength[0], 5);
        assert_eq!(
            app.world().resource::<Board>().ground(targets[0]),
            Some(Ground::Solid)
        );
        assert_eq!(
            app.world().resource::<Board>().ground(targets[1]),
            Some(Ground::Solid)
        );
    }

    #[test]
    fn rejected_second_reaction_keeps_the_first_hit_queued() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            let mut holes = 0;
            for y in 0..ROWS {
                for x in 0..COLUMNS {
                    let cell = IVec2::new(x as i32, y as i32);
                    if holes < board::MAX_UNSAFE_TILES
                        && ![target, IVec2::new(5, 3)].contains(&cell)
                    {
                        board.set(cell, Ground::Hole);
                        holes += 1;
                    }
                }
            }
        }
        app.world_mut().resource_mut::<TurnState>().inventory[0][VOID_DUST_ITEM_INDEX] = 1;
        for icon in [
            p1_item(FROST_CRYSTAL_ITEM_INDEX),
            p1_item(VOID_DUST_ITEM_INDEX),
        ] {
            click_part(&mut app, icon);
            click_part(&mut app, cell_center(target));
        }
        let world = app.world_mut();
        assert_eq!(world.resource::<TurnState>().actions_used, 1);
        assert_eq!(
            world.resource::<TurnState>().inventory[0][VOID_DUST_ITEM_INDEX],
            1
        );
        assert_eq!(world.query::<&PendingFrost>().iter(world).count(), 0);
        assert_eq!(world.resource::<TurnState>().ingredient_respawns, []);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn queued_item_and_pickup_preserve_inventory_counts() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX] = 3;
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: FROST_CRYSTAL_ITEM_INDEX,
                cell: IVec2::new(5, 3),
            },
        ));
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            2
        );
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            3
        );
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn queued_item_is_refunded_if_its_target_becomes_invalid() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX] = 1;
        click_part(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            0
        );
        assert!(app.world_mut().resource_mut::<Board>().make_hole(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            1
        );
        assert!(
            app.world()
                .resource::<TurnState>()
                .pending_placement
                .is_none()
        );
    }

    #[test]
    fn unlimited_ingredients_can_be_used_repeatedly_and_undone_with_empty_inventory() {
        for index in [
            WATER_ITEM_INDEX,
            VOID_DUST_ITEM_INDEX,
            STONE_DUST_ITEM_INDEX,
        ] {
            let mut app = interaction_app();
            app.world_mut().resource_mut::<TurnState>().inventory = [[0; 14]; 2];
            app.update();
            {
                let world = app.world_mut();
                let mut counts = world.query::<(&InventoryCount, &ImageNode)>();
                for player in [PlayerId::One, PlayerId::Two] {
                    assert!(
                        counts
                            .iter(world)
                            .any(|(count, _)| count.owner == player && count.index == index)
                    );
                }
            }
            let inventory = p1_item(index);
            let first = IVec2::new(6, 3);
            let second = IVec2::new(6, 4);
            if index == STONE_DUST_ITEM_INDEX {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(first);
                board.make_water(second);
            }
            click_part(&mut app, inventory);
            click_part(&mut app, cell_center(first));
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .pending_placement
                    .is_some()
            );
            assert_eq!(app.world().resource::<TurnState>().inventory[0][index], 0);
            click_part(&mut app, Vec2::new(-330.0, -60.0));
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .pending_placement
                    .is_none()
            );
            assert_eq!(app.world().resource::<TurnState>().inventory[0][index], 0);
            for cell in [first, second] {
                click_part(&mut app, inventory);
                click_part(&mut app, cell_center(cell));
            }
            let turn = app.world().resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::Two);
            assert_eq!(turn.inventory[0][index], 0);
            assert!(turn.ingredient_respawns.is_empty());
            let board = app.world().resource::<Board>();
            match index {
                WATER_ITEM_INDEX => {
                    assert!([first, second].iter().all(|&cell| board.is_water(cell)))
                }
                VOID_DUST_ITEM_INDEX => assert!(
                    [first, second]
                        .iter()
                        .all(|&cell| board.ground(cell) == Some(Ground::Hole))
                ),
                STONE_DUST_ITEM_INDEX => assert!(
                    [first, second]
                        .iter()
                        .all(|&cell| board.ground(cell) == Some(Ground::Solid))
                ),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn invalidated_second_item_can_be_replaced_with_movement() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0][WATER_ITEM_INDEX] = 2;
        for _ in 0..2 {
            click_part(&mut app, p1_item(WATER_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
        }
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(4, 3)));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert!(app.world().resource::<Board>().is_water(target));
    }

    #[test]
    fn invalidated_second_item_allows_hovering_player_to_escape_new_hole() {
        let mut app = interaction_app();
        let player_cell = IVec2::new(5, 3);
        let target = player_cell;
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[0] = true;
            turn.magic[0] = 1;
        }
        click_part(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(OIL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));

        assert_eq!(
            app.world().resource::<Board>().ground(player_cell),
            Some(Ground::Solid)
        );
        assert_eq!(app.world().resource::<TurnState>().actions_used, 1);
        let destination = [IVec2::X, -IVec2::X, IVec2::Y, -IVec2::Y]
            .into_iter()
            .map(|offset| player_cell + offset)
            .find(|&cell| {
                app.world()
                    .resource::<Board>()
                    .ground(cell)
                    .is_some_and(Ground::walkable)
            })
            .unwrap();
        select_move(&mut app);
        click_part(&mut app, cell_center(destination));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
    }

    #[test]
    fn second_item_on_a_new_hole_is_rejected_and_its_energy_is_refunded() {
        let mut app = interaction_app();
        let target = IVec2::new(8, 3);
        click_part(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(OIL_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.actions_used, 1);
        assert_eq!(turn.strength[0], 3);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        assert_eq!(
            app.world_mut()
                .query::<&PendingVoidDust>()
                .iter(app.world())
                .count(),
            0
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().strength[0], 5);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn partial_rest_adds_two_after_energy_cost() {
        for button in [false, true] {
            let mut app = interaction_app();
            app.world_mut().resource_mut::<TurnState>().strength[0] = 3;
            click_part(&mut app, p1_item(WATER_ITEM_INDEX));
            click_part(&mut app, cell_center(IVec2::new(8, 3)));
            assert_eq!(app.world().resource::<TurnState>().strength[0], 1);
            if button {
                click_part(&mut app, Vec2::new(-330.0, 0.0));
            } else {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert_eq!(app.world().resource::<TurnState>().strength[0], 3);
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert!(app.world().resource::<Board>().is_water(IVec2::new(8, 3)));
        }
    }

    #[test]
    fn players_start_with_sunstone_and_life_essence() {
        let mut app = interaction_app();
        *app.world_mut().resource_mut::<TurnState>() = TurnState::default();
        app.update();
        let unavailable = [
            (FROST_CRYSTAL_ITEM_INDEX, p1_item(FROST_CRYSTAL_ITEM_INDEX)),
            (STORM_SHARD_ITEM_INDEX, p1_item(STORM_SHARD_ITEM_INDEX)),
            (ACID_ITEM_INDEX, p1_item(ACID_ITEM_INDEX)),
            (POISON_ITEM_INDEX, p1_item(POISON_ITEM_INDEX)),
        ];
        for player in 0..2 {
            for index in 0..INVENTORY_ITEMS.len() {
                let expected = usize::from(
                    [OIL_ITEM_INDEX, SUNSTONE_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX].contains(&index),
                );
                assert_eq!(
                    app.world().resource::<TurnState>().inventory[player][index],
                    expected
                );
            }
            {
                let world = app.world_mut();
                let mut counts = world.query::<(&InventoryCount, &Text)>();
                for index in [SUNSTONE_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX] {
                    let (_, text) = counts
                        .iter(world)
                        .find(|(count, _)| count.owner.index() == player && count.index == index)
                        .unwrap();
                    assert_eq!(text.0, "1");
                }
            }
            for position in [
                p1_item(SUNSTONE_ITEM_INDEX),
                p1_item(LIFE_ESSENCE_ITEM_INDEX),
            ] {
                click(&mut app, position + Vec2::new(840.0 * player as f32, 0.0));
                assert!(app.world().resource::<Selection>().0.is_some());
                press_key(&mut app, KeyCode::Escape);
            }
            for (_, position) in unavailable {
                click(&mut app, position + Vec2::new(840.0 * player as f32, 0.0));
                assert!(app.world().resource::<Selection>().0.is_none());
            }
            if player == 0 {
                press_key(&mut app, KeyCode::KeyR);
            }
        }
    }

    #[test]
    fn weighted_random_ingredients_balance_life_and_make_eggs_rarest() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut counts = [0_u32; INVENTORY_ITEMS.len()];
        for _ in 0..50_000 {
            counts[random_ingredient(&mut rng, &[MIN_INGREDIENT_COPIES; 14])] += 1;
        }
        for (index, &count) in counts.iter().enumerate() {
            if is_unlimited(index) {
                assert_eq!(count, 0);
            }
        }
        assert!(counts[COATL_EGG_ITEM_INDEX] < counts[THORNHEART_ITEM_INDEX]);
        assert!(counts[COATL_EGG_ITEM_INDEX] < counts[AQUATIC_ITEM_INDEX]);
        for (index, &count) in counts.iter().enumerate() {
            if !is_unlimited(index)
                && ![
                    COATL_EGG_ITEM_INDEX,
                    THORNHEART_ITEM_INDEX,
                    AQUATIC_ITEM_INDEX,
                ]
                .contains(&index)
            {
                assert!(counts[THORNHEART_ITEM_INDEX] < count);
                assert!(counts[AQUATIC_ITEM_INDEX] < count);
                let difference = count.abs_diff(counts[LIFE_ESSENCE_ITEM_INDEX]);
                assert!(difference < counts[LIFE_ESSENCE_ITEM_INDEX] / 10);
            }
        }
    }

    #[test]
    fn finite_pool_spawns_batches_every_three_turns_and_recycles() {
        let mut app = random_spawn_app();
        *app.world_mut().resource_mut::<TurnState>() = TurnState::default();
        app.add_systems(Startup, spawn_initial_collectibles);
        app.update();
        {
            let world = app.world_mut();
            let mut query = world.query::<&Collectible>();
            let initial: Vec<_> = query.iter(world).collect();
            assert_eq!(initial.len(), 2);
            assert!(initial.iter().any(|item| item.index == SUNSTONE_ITEM_INDEX));
            assert!(
                initial
                    .iter()
                    .any(|item| item.index == LIFE_ESSENCE_ITEM_INDEX)
            );
            assert_ne!(initial[0].cell, initial[1].cell);
            assert!(
                initial
                    .iter()
                    .all(|item| ![IVec2::new(5, 3), IVec2::new(7, 6)].contains(&item.cell))
            );
        }
        let inventory_items = 6;
        let field_capacity = INGREDIENT_POOL_SIZE - inventory_items;
        for completed_turns in 1..=60 {
            let acting_player = app.world().resource::<TurnState>().active;
            press_key(&mut app, KeyCode::KeyR);
            if completed_turns % 3 == 0 {
                assert_eq!(
                    acting_player,
                    if (completed_turns / 3) % 2 == 1 {
                        PlayerId::One
                    } else {
                        PlayerId::Two
                    }
                );
            }
            let expected = (2 + 3 * (completed_turns / 3)).min(field_capacity);
            let world = app.world_mut();
            let mut query = world.query::<&Collectible>();
            let items: Vec<_> = query.iter(world).collect();
            assert_eq!(items.len(), expected);
            let cells: std::collections::HashSet<_> = items.iter().map(|item| item.cell).collect();
            assert_eq!(cells.len(), expected);
            assert!(!cells.contains(&IVec2::new(5, 3)));
            assert!(!cells.contains(&IVec2::new(7, 6)));
        }
        let spawned = app.world().resource::<TurnState>().spawned_ingredients;
        for (index, &count) in spawned.iter().enumerate() {
            if is_unlimited(index) {
                assert_eq!(count, 0);
            } else {
                assert!(count >= 2, "missing copies of {}", INVENTORY_ITEMS[index].0);
            }
        }
        for _ in 0..3 {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(item_count(&mut app), field_capacity);
            assert_eq!(
                app.world().resource::<TurnState>().spawned_ingredients,
                spawned
            );
        }

        let world = app.world_mut();
        let removed: Vec<_> = world
            .query::<(Entity, &Collectible)>()
            .iter(world)
            .take(2)
            .map(|(entity, item)| (entity, item.index))
            .collect();
        for &(entity, index) in &removed {
            world.despawn(entity);
            world
                .resource_mut::<TurnState>()
                .ingredient_respawns
                .push((index, 0));
        }
        for expected in [field_capacity - 2, field_capacity - 2, field_capacity] {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(item_count(&mut app), expected);
            assert_eq!(
                app.world().resource::<TurnState>().spawned_ingredients,
                spawned
            );
        }
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
    }

    #[test]
    fn spawn_waits_for_an_unoccupied_tile_and_prioritizes_recycling() {
        let mut app = random_spawn_app();
        for y in 0..ROWS {
            for x in 0..COLUMNS {
                app.world_mut()
                    .resource_mut::<Board>()
                    .add_oil(IVec2::new(x as i32, y as i32));
            }
        }
        app.world_mut()
            .resource_mut::<TurnState>()
            .ingredient_respawns
            .extend([(SUNSTONE_ITEM_INDEX, 1), (FROST_CRYSTAL_ITEM_INDEX, 1)]);
        for _ in 0..3 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .len(),
            2
        );
        assert_eq!(
            app.world().resource::<TurnState>().spawned_ingredients,
            [0; 14]
        );
        app.world_mut()
            .resource_mut::<Board>()
            .remove_oil(IVec2::ZERO);
        for _ in 0..3 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        let item = world.query::<&Collectible>().single(world).unwrap();
        assert_eq!((item.index, item.cell), (SUNSTONE_ITEM_INDEX, IVec2::ZERO));
        assert_eq!(world.resource::<TurnState>().ingredient_respawns.len(), 2);
        assert_eq!(world.resource::<TurnState>().spawned_ingredients, [0; 14]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(item_count(&mut app), 1);
    }

    #[test]
    fn drop_batch_uses_recycled_items_before_new_items() {
        let mut app = random_spawn_app();
        app.world_mut()
            .resource_mut::<TurnState>()
            .ingredient_respawns
            .extend([(SUNSTONE_ITEM_INDEX, 1), (FROST_CRYSTAL_ITEM_INDEX, 1)]);
        for expected in [0, 0, 3] {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(item_count(&mut app), expected);
        }
        let world = app.world_mut();
        let mut query = world.query::<&Collectible>();
        let items: Vec<_> = query.iter(world).collect();
        assert!(items.iter().any(|item| item.index == SUNSTONE_ITEM_INDEX));
        assert!(
            items
                .iter()
                .any(|item| item.index == FROST_CRYSTAL_ITEM_INDEX)
        );
        assert_eq!(
            items
                .iter()
                .map(|item| item.cell)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );
        let turn = world.resource::<TurnState>();
        assert!(turn.ingredient_respawns.is_empty());
        assert_eq!(
            turn.spawned_ingredients
                .iter()
                .map(|&n| usize::from(n))
                .sum::<usize>(),
            1
        );
    }

    #[test]
    fn pickup_has_no_three_item_limit_and_can_be_undone() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        app.world_mut().resource_mut::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX] = 255;
        for _ in 0..4 {
            app.world_mut().spawn((
                BoardItem,
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell,
                },
            ));
        }
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: WATER_ITEM_INDEX,
                cell,
            },
        ));
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            259
        );
        {
            let world = app.world_mut();
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 1);
        }
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            255
        );
        assert_eq!(item_count(&mut app), 5);
        click(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX],
            259
        );
        assert_eq!(item_count(&mut app), 1);
    }

    #[test]
    fn pickup_undo_does_not_spawn_until_turn_is_completed() {
        let mut app = random_spawn_app();
        *app.world_mut().resource_mut::<TurnState>() = TurnState::default();
        app.add_systems(Startup, spawn_initial_collectibles);
        app.update();
        let cell = {
            let world = app.world_mut();
            world
                .query::<&Collectible>()
                .iter(world)
                .next()
                .unwrap()
                .cell
        };
        move_player_to(&mut app, PlayerId::One, cell);
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(
            app.world()
                .resource::<TurnState>()
                .spawned_ingredients
                .iter()
                .map(|&n| usize::from(n))
                .sum::<usize>(),
            8
        );
        click_part(&mut app, Vec2::new(-330.0, -60.0));
        assert_eq!(item_count(&mut app), 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(item_count(&mut app), 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(item_count(&mut app), 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(item_count(&mut app), 5);
    }

    #[test]
    fn collectible_icons_move_as_players_walk_over_and_away_from_them() {
        let mut app = interaction_app();
        let start = IVec2::new(5, 3);
        let next = IVec2::new(6, 3);
        for cell in [start, next] {
            app.world_mut().spawn((
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell,
                },
                Sprite::default(),
                Transform::default(),
            ));
        }
        select_move(&mut app);
        click_part(&mut app, cell_center(next));
        let world = app.world_mut();
        let mut items = world.query::<(&Collectible, &Sprite, &Transform)>();
        let mut size_at = |cell| {
            let (_, sprite, transform) = items
                .iter(world)
                .find(|(item, _, _)| item.cell == cell)
                .unwrap();
            (sprite.custom_size, transform.translation.truncate())
        };
        assert_eq!(
            size_at(start),
            (Some(Vec2::splat(44.0)), cell_center(start))
        );
        assert_eq!(
            size_at(next),
            (
                Some(Vec2::splat(24.0)),
                cell_center(next) + Vec2::splat(15.0)
            )
        );
    }

    #[test]
    fn unlimited_placed_items_cannot_be_picked_up_or_respawn_after_falling() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::One, cell);
        let unlimited: Vec<_> = [
            WATER_ITEM_INDEX,
            VOID_DUST_ITEM_INDEX,
            STONE_DUST_ITEM_INDEX,
        ]
        .into_iter()
        .map(|index| {
            app.world_mut()
                .spawn((Collectible { index, cell }, BoardItem))
                .id()
        })
        .collect();
        click_part(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
        assert!(
            unlimited
                .iter()
                .all(|&entity| app.world().get_entity(entity).is_ok())
        );
        let limited = app
            .world_mut()
            .spawn((
                Collectible {
                    index: SUNSTONE_ITEM_INDEX,
                    cell,
                },
                BoardItem,
            ))
            .id();
        assert!(app.world_mut().resource_mut::<Board>().make_hole(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(
            unlimited
                .iter()
                .all(|&entity| app.world().get_entity(entity).is_err())
        );
        assert!(app.world().get_entity(limited).is_err());
        let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
        assert_eq!(
            respawns.as_slice(),
            &[(SUNSTONE_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn magic_changes_only_on_the_owners_completed_turn() {
        let mut app = interaction_app();
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert_eq!(app.world().resource::<TurnState>().magic, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic, [4, 5]);
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert_eq!(app.world().resource::<TurnState>().magic, [4, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic, [5, 5]);

        press_key(&mut app, KeyCode::KeyR);
        app.world_mut().resource_mut::<TurnState>().magic[0] = 1;
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert_eq!(app.world().resource::<TurnState>().magic[0], 1);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic[0], 0);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic[0], 1);
    }

    #[test]
    fn hover_move_costs_magic_after_landing_with_each_turn_completion() {
        for player in [PlayerId::One, PlayerId::Two] {
            for finish in ["rest", "move", "placement"] {
                let mut app = interaction_app();
                let start = if player == PlayerId::One {
                    IVec2::new(5, 3)
                } else {
                    IVec2::new(7, 6)
                };
                let x = if player == PlayerId::One {
                    -330.0
                } else {
                    330.0
                };
                let inventory_x = if player == PlayerId::One {
                    -450.0
                } else {
                    390.0
                };
                move_player_to(&mut app, player, start);
                {
                    let mut turn = app.world_mut().resource_mut::<TurnState>();
                    turn.active = player;
                    turn.magic = [3, 3];
                    turn.hovering[player.index()] = true;
                }
                select_move(&mut app);
                click_part(&mut app, cell_center(start + IVec2::X));
                click_part(&mut app, Vec2::new(x, 120.0));
                assert!(!app.world().resource::<TurnState>().hovering[player.index()]);
                match finish {
                    "rest" => press_key(&mut app, KeyCode::KeyR),
                    "move" => {
                        select_move(&mut app);
                        click_part(&mut app, cell_center(start + 2 * IVec2::X));
                    }
                    "placement" => {
                        click_part(&mut app, Vec2::new(inventory_x, 180.0));
                        click_part(&mut app, cell_center(start + IVec2::X - IVec2::Y));
                    }
                    _ => unreachable!(),
                }
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.active, player.other(), "{finish}");
                assert_eq!(turn.magic[player.index()], 2, "{finish}");
                assert_eq!(turn.magic[player.other().index()], 3, "{finish}");
                assert!(!turn.moved_while_hovering);
            }
        }
    }

    #[test]
    fn undo_hover_move_cancels_the_cost_and_failed_moves_do_not_incur_it() {
        for undo in [false, true] {
            let mut app = interaction_app();
            {
                let mut turn = app.world_mut().resource_mut::<TurnState>();
                turn.magic[0] = 3;
                turn.hovering[0] = true;
            }
            select_move(&mut app);
            click_part(
                &mut app,
                cell_center(if undo {
                    IVec2::new(6, 3)
                } else {
                    IVec2::new(9, 3)
                }),
            );
            click_part(&mut app, Vec2::new(-330.0, 120.0));
            if undo {
                click_part(&mut app, Vec2::new(-330.0, -60.0));
            }
            assert!(
                !app.world().resource::<TurnState>().hovering[0],
                "undo={undo}"
            );
            assert!(
                !app.world().resource::<TurnState>().moved_while_hovering,
                "undo={undo}"
            );
            click_part(&mut app, Vec2::new(-330.0, 0.0));
            let turn = app.world().resource::<TurnState>();
            assert_eq!(
                turn.magic[0],
                4,
                "undo={undo}, active={:?}",
                turn.active.index()
            );
            assert!(!turn.moved_while_hovering);
        }
    }

    #[test]
    fn hovering_through_two_moves_costs_only_one_magic() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().hovering[0] = true;
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(7, 3)));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.magic[0], 4);
        assert!(turn.hovering[0]);
    }

    #[test]
    fn landing_after_a_hover_move_spends_the_last_magic_and_allows_later_recovery() {
        let mut app = interaction_app();
        let origin = IVec2::new(5, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(origin, Ground::Ridge);
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[0] = true;
            turn.magic[0] = 1;
        }
        select_move(&mut app);
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        click_part(&mut app, Vec2::new(-330.0, 120.0));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic[0], 0);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().magic[0], 1);
    }

    #[test]
    fn hover_toggle_preserves_item_selection_and_updates_magic_bar_and_sprite() {
        let mut app = interaction_app();
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_some());
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert!(app.world().resource::<Selection>().0.is_some());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        {
            let world = app.world_mut();
            let mut bars = world.query::<(&MagicFill, &Node)>();
            let (fill, node) = bars
                .iter(world)
                .find(|(fill, _)| fill.0 == PlayerId::One)
                .unwrap();
            assert_eq!(fill.0, PlayerId::One);
            assert_eq!(node.width, Val::Percent(100.0));
            let mut players = world.query::<(&Player, &Transform, &Sprite)>();
            let (_, transform, sprite) = players
                .iter(world)
                .find(|(player, _, _)| player.0 == PlayerId::One)
                .unwrap();
            assert_eq!(transform.translation.truncate(), Vec2::new(30.0, 0.0));
            assert!(sprite.rect.unwrap().min.x >= 96.0);
        }
        click(&mut app, cell_center(IVec2::new(6, 2)));
        assert_eq!(app.world().resource::<TurnState>().magic[0], 4);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        press_key(&mut app, KeyCode::KeyR);
        select_move(&mut app);
        let destination = cell_center(IVec2::new(6, 3));
        click(&mut app, destination);
        {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &Transform, &Sprite)>();
            let (_, transform, sprite) = players
                .iter(world)
                .find(|(player, _, _)| player.0 == PlayerId::One)
                .unwrap();
            assert!(sprite.rect.unwrap().min.x >= 96.0);
            assert_eq!(transform.translation.truncate(), destination);
        }
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().magic[0], 3);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Sprite)>();
        let (_, sprite) = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap();
        assert!(sprite.rect.unwrap().min.x < 96.0);
    }

    #[test]
    fn pickup_is_highlighted_only_when_active_player_can_collect() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: SUNSTONE_ITEM_INDEX,
                cell: IVec2::new(5, 3),
            },
        ));
        app.update();
        {
            let world = app.world_mut();
            assert_eq!(world.query::<&PickupHighlight>().iter(world).count(), 1);
            let mut slots = world.query::<(&ActionSlot, &BackgroundColor)>();
            let (_, color) = slots
                .iter(world)
                .find(|(slot, _)| slot.owner == PlayerId::One && slot.action == Action::Pickup)
                .unwrap();
            assert_eq!(color.0, Color::srgba(0.15, 0.55, 0.25, 0.9));
        }
        app.world_mut().resource_mut::<TurnState>().inventory[0][SUNSTONE_ITEM_INDEX] = 64;
        app.update();
        {
            let world = app.world_mut();
            assert_eq!(world.query::<&PickupHighlight>().iter(world).count(), 1);
            let mut slots = world.query::<(&ActionSlot, &BackgroundColor)>();
            let (_, color) = slots
                .iter(world)
                .find(|(slot, _)| slot.owner == PlayerId::One && slot.action == Action::Pickup)
                .unwrap();
            assert_eq!(color.0, Color::srgba(0.15, 0.55, 0.25, 0.9));
        }
    }

    #[test]
    fn hover_requires_a_safe_landing_before_exhaustion() {
        let mut app = interaction_app();
        let ridge = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(ridge, Ground::Ridge);
        app.world_mut().resource_mut::<TurnState>().magic[0] = 2;
        click(&mut app, Vec2::new(-330.0, 120.0));
        select_move(&mut app);
        click(&mut app, cell_center(ridge));
        assert_eq!(app.world().resource::<TurnState>().magic[0], 1);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert!(app.world().resource::<TurnState>().hovering[0]);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_none());
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(7, 3)));
        assert_eq!(app.world().resource::<TurnState>().magic[0], 0);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn hover_cannot_enter_an_unsafe_tile_with_only_one_escape() {
        let target = IVec2::new(6, 3);
        let mut board = Board::default();
        for x in -1..=1 {
            for y in -1..=1 {
                board.set(target + IVec2::new(x, y), Ground::Ridge);
            }
        }
        board.set(IVec2::new(5, 3), Ground::Solid);
        let drop = drop_target(cell_center(IVec2::new(5, 3)), cell_center(target)).unwrap();
        assert_eq!(
            movement_target(&board, &drop, cell_center(IVec2::new(9, 6)), 5, true, 5),
            Err(MovementError::UnsafeHover)
        );
        board.set(IVec2::new(7, 3), Ground::Solid);
        assert_eq!(
            movement_target(&board, &drop, cell_center(IVec2::new(7, 3)), 5, true, 5),
            Err(MovementError::UnsafeHover)
        );
    }

    #[test]
    fn hover_toggle_updates_a_selected_movement_preview_without_ending_the_turn() {
        let mut app = interaction_app();
        let ridge = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(ridge, Ground::Ridge);
        select_move(&mut app);
        for (step, (hovering, expected)) in [(false, false), (true, true), (false, false)]
            .into_iter()
            .enumerate()
        {
            let world = app.world();
            let Some(SelectedAction::Movement { highlights }) = &world.resource::<Selection>().0
            else {
                panic!("movement selection should remain active");
            };
            assert_eq!(world.resource::<TurnState>().hovering[0], hovering);
            assert_eq!(
                highlights.iter().any(|entity| {
                    world.get::<Transform>(*entity).is_some_and(|transform| {
                        board_cell(transform.translation.truncate()) == Some(ridge)
                    })
                }),
                expected
            );
            assert_eq!(world.resource::<TurnState>().active, PlayerId::One);
            assert_eq!(world.resource::<TurnState>().magic[0], MAX_MAGIC);
            if step < 2 {
                click(&mut app, Vec2::new(-330.0, 120.0));
            }
        }
    }

    #[test]
    fn hovering_prevents_electricity_for_each_player_but_not_others() {
        for hovering_player in [PlayerId::One, PlayerId::Two] {
            let mut app = interaction_app();
            move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                for x in 5..=7 {
                    board.set(IVec2::new(x, 3), Ground::Water);
                }
            }
            app.world_mut().resource_mut::<TurnState>().hovering[hovering_player.index()] = true;
            click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(IVec2::new(6, 3)));
            let expected = if hovering_player == PlayerId::One {
                [10, 8]
            } else {
                [8, 10]
            };
            assert_eq!(app.world().resource::<PlayerHealth>().0, expected);
        }
    }

    #[test]
    fn hovering_prevents_water_freeze_and_frozen_tile_damage() {
        for hovering in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            app.world_mut()
                .resource_mut::<Board>()
                .set(target, Ground::Water);
            app.world_mut().resource_mut::<TurnState>().hovering[1] = hovering;
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
            assert_eq!(
                app.world().resource::<TurnState>().frozen_turns[1],
                if hovering { 0 } else { 2 }
            );
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(
                app.world().resource::<PlayerHealth>().0[1],
                if hovering { 10 } else { 9 }
            );
        }
    }

    #[test]
    fn hovering_during_water_freeze_remains_safe_after_magic_exhaustion() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[1] = true;
            turn.magic[1] = 1;
        }
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
        assert!(!app.world().resource::<TurnState>().hovering[1]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
    }

    #[test]
    fn hover_prevents_poison_and_protects_through_the_exhaustion_turn() {
        let mut app = interaction_app();
        let first = IVec2::new(5, 3);
        let second = IVec2::new(7, 6);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_poison(first);
            board.add_poison(second);
        }
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[0] = true;
            turn.magic[0] = 1;
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 8]);
    }

    #[test]
    fn hover_does_not_prevent_steam_or_explosion_damage() {
        for fire in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if fire {
                    board.add_oil(target);
                } else {
                    board.set(target, Ground::Water);
                }
            }
            app.world_mut().resource_mut::<TurnState>().hovering = [true, true];
            click(
                &mut app,
                p1_item(if fire {
                    SUNSTONE_ITEM_INDEX
                } else {
                    OIL_ITEM_INDEX
                }),
            );
            click(&mut app, cell_center(target));
            if fire {
                assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 8]);
            } else {
                assert_eq!(app.world().resource::<PlayerHealth>().0[1], 8);
            }
        }
    }

    #[test]
    fn hovering_prevents_healing_but_not_steam() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(target, Ground::Water);
            board.add_life_essence(target);
            board.add_poison(target);
        }
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 1;
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 0);
    }

    #[test]
    fn pickup_consumes_a_turn_and_removes_the_pending_ingredient() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            0
        );
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        select_move(&mut app);
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][FROST_CRYSTAL_ITEM_INDEX],
            1
        );
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingFrost>().iter(world).count(), 0);
        assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
    }

    #[test]
    fn pickup_collects_multiple_ingredients_in_one_turn() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        app.world_mut().spawn((
            BoardItem,
            PendingFrost(cell),
            Collectible {
                index: FROST_CRYSTAL_ITEM_INDEX,
                cell,
            },
        ));
        app.world_mut().spawn((
            BoardItem,
            PendingSunstone(cell),
            Collectible {
                index: SUNSTONE_ITEM_INDEX,
                cell,
            },
        ));
        click(&mut app, Vec2::new(-330.0, 60.0));
        let world = app.world_mut();
        let turn = world.resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.inventory[0][FROST_CRYSTAL_ITEM_INDEX], 2);
        assert_eq!(turn.inventory[0][SUNSTONE_ITEM_INDEX], 2);
        assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingFrost>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingSunstone>().iter(world).count(), 0);
    }

    #[test]
    fn empty_inventory_allows_only_unlimited_ingredients() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory = [[0; 14]; 2];
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        for index in [
            WATER_ITEM_INDEX,
            VOID_DUST_ITEM_INDEX,
            STONE_DUST_ITEM_INDEX,
        ] {
            let position = p1_item(index);
            click(&mut app, position);
            assert!(app.world().resource::<Selection>().0.is_some());
            press_key(&mut app, KeyCode::Escape);
        }
        for item in [
            p1_item(ACID_ITEM_INDEX),
            p1_item(POISON_ITEM_INDEX),
            p1_item(OIL_ITEM_INDEX),
        ] {
            click(&mut app, item);
            assert!(app.world().resource::<Selection>().0.is_none());
        }
    }

    #[test]
    fn toad_wart_and_poison_frog_sphere_are_consumed_and_respawn() {
        for (index, inventory_position) in [
            (ACID_ITEM_INDEX, p1_item(ACID_ITEM_INDEX)),
            (POISON_ITEM_INDEX, p1_item(POISON_ITEM_INDEX)),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            click(&mut app, inventory_position);
            click(&mut app, cell_center(cell));
            let turn = app.world().resource::<TurnState>();
            assert_eq!(turn.inventory[0][index], 0);
            assert_eq!(
                turn.ingredient_respawns,
                [(index, INGREDIENT_RESPAWN_DELAY)]
            );
            assert_eq!(item_count(&mut app), 0);
            {
                let world = app.world_mut();
                let mut counts = world.query::<(&InventoryCount, &Text)>();
                let (_, count) = counts
                    .iter(world)
                    .find(|(count, _)| count.owner == PlayerId::One && count.index == index)
                    .unwrap();
                assert_eq!(count.0, "0");
            }
            for _ in 0..4 {
                press_key(&mut app, KeyCode::KeyR);
                assert_eq!(item_count(&mut app), 0);
            }
            press_key(&mut app, KeyCode::KeyR);
            let (entity, respawn) = {
                let world = app.world_mut();
                let mut items = world.query::<(Entity, &Collectible, &Name)>();
                let (entity, item, name) = items.single(world).unwrap();
                assert_eq!(item.index, index);
                assert_eq!(name.as_str(), INVENTORY_ITEMS[index].0);
                (entity, item.cell)
            };
            assert!(!app.world().resource::<Board>().is_acidic(respawn));
            assert!(!app.world().resource::<Board>().is_poisoned(respawn));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
            move_player_to(&mut app, PlayerId::One, respawn);
            app.world_mut().resource_mut::<TurnState>().inventory[0][index] = 3;
            click(&mut app, Vec2::new(-330.0, 60.0));
            assert_eq!(app.world().resource::<TurnState>().inventory[0][index], 4);
            assert!(app.world().get_entity(entity).is_err());
        }
    }

    #[test]
    fn reacting_toad_wart_and_poison_frog_sphere_each_respawn_once() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.inventory[0][ACID_ITEM_INDEX], 0);
        assert_eq!(turn.inventory[1][POISON_ITEM_INDEX], 0);
        assert_eq!(
            turn.ingredient_respawns,
            [(ACID_ITEM_INDEX, 4), (POISON_ITEM_INDEX, 5)]
        );
        for _ in 0..7 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        let items: Vec<_> = world
            .query::<&Collectible>()
            .iter(world)
            .map(|item| item.index)
            .collect();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&ACID_ITEM_INDEX));
        assert!(items.contains(&POISON_ITEM_INDEX));
    }

    #[test]
    fn acid_is_temporary_on_ground_and_ridges_but_permanent_on_water() {
        for ground in [Ground::Solid, Ground::Ridge, Ground::Water] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<Board>().set(cell, ground);
            click(&mut app, p1_item(ACID_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().is_acidic(cell));
            {
                let world = app.world_mut();
                assert_eq!(world.query::<&AcidTile>().iter(world).count(), 1);
            }
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            let board = app.world().resource::<Board>();
            assert_eq!(board.ground(cell), Some(ground));
            assert_eq!(board.is_acidic(cell), ground == Ground::Water);
        }
    }

    #[test]
    fn stone_dust_absorbs_acid_and_water_becomes_solid() {
        {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            click(&mut app, p1_item(ACID_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            click(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert!(world.resource::<Board>().is_acidic(cell));
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
            assert!(
                !world
                    .resource::<TurnState>()
                    .ingredient_respawns
                    .contains(&(STONE_DUST_ITEM_INDEX, 5))
            );
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert!(!app.world().resource::<Board>().is_acidic(cell));
        }
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Solid)
        );
        assert!(app.world().resource::<Board>().is_acidic(cell));
        for _ in 0..5 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert!(!app.world().resource::<Board>().is_acidic(cell));
    }

    #[test]
    fn acid_and_poison_create_an_x_in_both_orders_and_keep_water_acidic() {
        for acid_first in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            let water = cell + IVec2::new(1, 1);
            let connected = water + IVec2::X;
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(water, Ground::Water);
                board.set(connected, Ground::Water);
            }
            move_player_to(&mut app, PlayerId::Two, cell);
            let first = if acid_first {
                p1_item(ACID_ITEM_INDEX)
            } else {
                p1_item(POISON_ITEM_INDEX)
            };
            let second = if acid_first {
                p2_item(POISON_ITEM_INDEX)
            } else {
                p2_item(ACID_ITEM_INDEX)
            };
            click(&mut app, first);
            click(&mut app, cell_center(cell));
            move_player_to(&mut app, PlayerId::Two, IVec2::new(8, 3));
            move_player_to(&mut app, PlayerId::One, cell);
            click(&mut app, second);
            click(&mut app, cell_center(cell));
            assert_eq!(app.world().resource::<PlayerHealth>().0[0], 8);
            for offset in [
                IVec2::ZERO,
                IVec2::new(1, 1),
                IVec2::new(1, -1),
                IVec2::new(-1, 1),
                IVec2::new(-1, -1),
            ] {
                let board = app.world().resource::<Board>();
                assert!(board.is_acidic(cell + offset));
                assert!(board.is_poisoned(cell + offset));
            }
            assert!(!app.world().resource::<Board>().is_acidic(cell + IVec2::X));
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<Board>().is_poisoned(connected));
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert!(app.world().resource::<Board>().is_acidic(water));
            assert!(!app.world().resource::<Board>().is_acidic(cell));
        }
    }

    #[test]
    fn storm_shard_and_acid_electrify_in_either_order_without_consuming_obsidian() {
        for shard_first in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            let obsidian = app
                .world_mut()
                .spawn((BoardItem, PendingObsidian(cell)))
                .id();
            let first = if shard_first {
                p1_item(STORM_SHARD_ITEM_INDEX)
            } else {
                p1_item(ACID_ITEM_INDEX)
            };
            let second = if shard_first {
                p2_item(ACID_ITEM_INDEX)
            } else {
                p2_item(STORM_SHARD_ITEM_INDEX)
            };
            // Obsidian reacts to a shard, so introduce it after the first placement.
            if shard_first {
                app.world_mut()
                    .entity_mut(obsidian)
                    .remove::<PendingObsidian>();
            }
            click(&mut app, first);
            click(&mut app, cell_center(cell));
            if shard_first {
                app.world_mut()
                    .entity_mut(obsidian)
                    .insert(PendingObsidian(cell));
            }
            if !shard_first {
                app.world_mut()
                    .entity_mut(obsidian)
                    .remove::<PendingObsidian>();
            }
            click(&mut app, second);
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().is_electrified_acid(cell));
            assert_eq!(
                app.world_mut()
                    .query::<&PendingStormShard>()
                    .iter(app.world())
                    .count(),
                0
            );
            assert!(app.world().get_entity(obsidian).is_ok());
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .ingredient_respawns
                    .contains(&(STORM_SHARD_ITEM_INDEX, 5))
            );
        }
    }

    #[test]
    fn acid_dissolves_dormant_and_active_thorns() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .activate_thornheart(cell, 1, true);
        app.world_mut().spawn((
            BoardItem,
            PendingThornheart(cell),
            Collectible {
                index: THORNHEART_ITEM_INDEX,
                cell,
            },
        ));
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(!app.world().resource::<Board>().has_thorns(cell));
        assert_eq!(
            app.world_mut()
                .query::<&PendingThornheart>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .filter(|entry| entry.0 == THORNHEART_ITEM_INDEX)
                .count(),
            2
        );
    }

    #[test]
    fn electrified_acid_damages_grounded_players_and_expires_on_permanent_acid() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(cell, Ground::Water);
            board.add_acid(cell);
            board.electrify_acid(cell);
        }
        move_player_to(&mut app, PlayerId::One, cell);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 10]);
        for _ in 0..5 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert!(app.world().resource::<Board>().is_acidic(cell));
        assert!(!app.world().resource::<Board>().is_electrified_acid(cell));
    }

    #[test]
    fn water_stone_makes_existing_acid_permanent() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        for _ in 0..6 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert!(app.world().resource::<Board>().is_acidic(cell));
    }

    #[test]
    fn thornheart_cannot_be_placed_on_acid_without_spending_a_turn() {
        for ground in [Ground::Solid, Ground::Water, Ground::Ridge] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(cell, ground);
                board.add_acid(cell);
            }
            click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert!(world.resource::<Board>().is_acidic(cell));
            assert!(!world.resource::<Board>().has_thorns(cell));
            assert_eq!(world.query::<&PendingThornheart>().iter(world).count(), 0);
            let turn = world.resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::One);
            assert_eq!(turn.strength[0], 5);
            assert_eq!(turn.inventory[0][THORNHEART_ITEM_INDEX], 1);
            assert!(turn.ingredient_respawns.is_empty());
            assert!(world.resource::<Selection>().0.is_some());
            assert!(
                world
                    .query_filtered::<&Text, With<DropFeedback>>()
                    .single(world)
                    .unwrap()
                    .0
                    .contains("Cannot place Thornheart on acid")
            );
        }
    }

    #[test]
    fn life_essence_removes_acid_and_electricity_but_preserves_terrain_and_poison() {
        for ground in [Ground::Solid, Ground::Ridge, Ground::Water] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            let neighbor = IVec2::new(7, 4);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            move_player_to(&mut app, PlayerId::Two, cell);
            app.world_mut().resource_mut::<PlayerHealth>().0[1] = 4;
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(cell, ground);
                board.add_acid(cell);
                board.electrify_acid(cell);
                board.add_poison(cell);
                board.add_acid(neighbor);
            }
            click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            let board = world.resource::<Board>();
            assert_eq!(board.ground(cell), Some(ground));
            assert!(!board.is_acidic(cell));
            assert!(!board.is_electrified_acid(cell));
            assert!(board.is_poisoned(cell));
            assert!(board.is_acidic(neighbor));
            assert_eq!(board.has_life_essence(cell), ground == Ground::Water);
            assert_eq!(
                world.resource::<PlayerHealth>().0[1],
                if ground == Ground::Water { 4 } else { 3 }
            );
            assert_eq!(
                world.resource::<TurnState>().inventory[0][LIFE_ESSENCE_ITEM_INDEX],
                0
            );
            assert_eq!(world.resource::<TurnState>().active, PlayerId::Two);
            assert_eq!(
                world.query::<&PendingLifeEssence>().iter(world).count(),
                usize::from(ground != Ground::Water)
            );
            assert_eq!(world.query::<&AcidTile>().iter(world).count(), 1);
        }
    }

    #[test]
    fn failed_life_essence_placement_does_not_remove_acid() {
        let mut app = interaction_app();
        let cell = IVec2::new(8, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        app.world_mut().resource_mut::<Board>().add_acid(cell);
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().is_acidic(cell));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.inventory[0][LIFE_ESSENCE_ITEM_INDEX], 1);
    }

    #[test]
    fn life_essence_expires_after_three_turns_and_returns_for_pickup() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 0;
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 1);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 2);
        assert!(app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 3);
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(LIFE_ESSENCE_ITEM_INDEX, 5)]
        );
        for _ in 0..4 {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(item_count(&mut app), 0);
        }
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        let cell = {
            let world = app.world_mut();
            let mut query = world.query::<&Collectible>();
            let item = query.single(world).unwrap();
            assert_eq!(item.index, LIFE_ESSENCE_ITEM_INDEX);
            item.cell
        };
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 3);
        let active = app.world().resource::<TurnState>().active;
        move_player_to(&mut app, active, cell);
        app.world_mut().resource_mut::<TurnState>().inventory[active.index()]
            [LIFE_ESSENCE_ITEM_INDEX] = 0;
        let action_x = if active == PlayerId::One {
            -330.0
        } else {
            330.0
        };
        click(&mut app, Vec2::new(action_x, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[active.index()][LIFE_ESSENCE_ITEM_INDEX],
            1
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn life_essence_heals_a_player_entering_on_the_second_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 0;
        select_move(&mut app);
        click_part(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 1);
        assert!(app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 2);
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(LIFE_ESSENCE_ITEM_INDEX, 5)]
        );
    }

    #[test]
    fn life_essence_heals_a_player_entering_on_the_last_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<Board>().make_water(target);
        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().has_life_essence(target));
        select_move(&mut app);
        click_part(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 1);
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 1);
    }

    #[test]
    fn hovering_blocks_healing_until_after_landing() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(cell, Ground::Water);
            board.add_life_essence(cell);
        }
        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        app.world_mut().resource_mut::<TurnState>().magic[0] = 1;
        click(&mut app, Vec2::new(-330.0, 120.0));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 0);
        assert!(!app.world().resource::<TurnState>().hovering[0]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 1);
    }

    #[test]
    fn activated_life_essence_cannot_be_picked_up() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        move_player_to(&mut app, PlayerId::Two, target);
        assert!(app.world().resource::<Board>().has_life_essence(target));
        click(&mut app, Vec2::new(330.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(
            app.world().resource::<TurnState>().inventory[1][LIFE_ESSENCE_ITEM_INDEX],
            1
        );
        assert!(app.world().resource::<Board>().has_life_essence(target));
        assert!(life_essence_cells(&mut app).contains(&target));
    }

    #[test]
    fn dormant_life_essence_on_a_ridge_can_be_picked_up() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Ridge);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        select_move(&mut app);
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][LIFE_ESSENCE_ITEM_INDEX],
            1
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn thornheart_and_life_activate_in_either_order_on_allowed_ground_types() {
        for ground in [Ground::Solid, Ground::Ridge, Ground::Water] {
            for life_first in [false, true] {
                let mut app = interaction_app();
                let target = IVec2::new(6, 3);
                app.world_mut().resource_mut::<Board>().set(target, ground);
                let first = if life_first {
                    p1_item(LIFE_ESSENCE_ITEM_INDEX)
                } else {
                    p1_item(THORNHEART_ITEM_INDEX)
                };
                let second = if life_first {
                    p2_item(THORNHEART_ITEM_INDEX)
                } else {
                    p2_item(LIFE_ESSENCE_ITEM_INDEX)
                };
                click(&mut app, first);
                click(&mut app, cell_center(target));
                click(&mut app, second);
                click(&mut app, cell_center(target));
                let world = app.world_mut();
                assert!(
                    world.resource::<Board>().has_thorns(target),
                    "{ground:?} {life_first}"
                );
                assert_eq!(world.query::<&PendingThornheart>().iter(world).count(), 0);
                assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 0);
                assert_eq!(world.query::<&ThornTile>().iter(world).count(), 1);
                assert_eq!(
                    world.resource::<Board>().has_life_essence(target),
                    ground == Ground::Water
                );
            }
        }
    }

    #[test]
    fn active_thornheart_has_a_distinct_green_vine_visual() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let dormant = {
            let world = app.world_mut();
            world
                .query_filtered::<&Sprite, With<PendingThornheart>>()
                .single(world)
                .unwrap()
                .clone()
        };
        click(&mut app, p2_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let world = app.world_mut();
        let vine = world
            .query_filtered::<&Sprite, With<ThornTile>>()
            .single(world)
            .unwrap();
        let color = vine.color.to_srgba();
        let size = vine.custom_size.unwrap();
        assert!(color.green > color.red && color.green > color.blue);
        assert!(size.y > size.x);
        assert_ne!(vine.color, dormant.color);
        assert_ne!(vine.custom_size, dormant.custom_size);
    }

    #[test]
    fn thorn_damage_is_netted_with_healing_but_hover_does_not_protect() {
        for hovering in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(target, Ground::Water);
                board.add_life_essence(target);
            }
            app.world_mut().resource_mut::<TurnState>().hovering[1] = hovering;
            app.world_mut().resource_mut::<PlayerHealth>().0[1] = 9;
            click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(
                app.world().resource::<PlayerHealth>().0[1],
                if hovering { 7 } else { 8 }
            );
            let world = app.world_mut();
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
            assert!(world.resource::<Board>().has_thorns(target));
            click(&mut app, Vec2::new(330.0, 60.0));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        }
    }

    #[test]
    fn dormant_thornheart_can_be_picked_up() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::KeyR);
        move_player_to(&mut app, PlayerId::One, target);
        click(&mut app, Vec2::new(-330.0, 60.0));
        let world = app.world_mut();
        assert_eq!(
            world.resource::<TurnState>().inventory[0][THORNHEART_ITEM_INDEX],
            1
        );
        assert_eq!(world.query::<&PendingThornheart>().iter(world).count(), 0);
    }

    #[test]
    fn thornheart_keeps_its_growth_duration_after_healing_expires() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(target, Ground::Water);
            board.add_life_essence(target);
            board.finish_life_turn();
        }
        click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert!(app.world().resource::<Board>().has_thorns(target));
        assert!(app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        for _ in 0..4 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        assert!(world.query::<&ThornTile>().iter(world).count() > 1);
    }

    #[test]
    fn used_thornheart_and_dry_life_essence_respawn_as_collectibles() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(THORNHEART_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        for _ in 0..19 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingThornheart>().iter(world).count(), 1);
        assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 1);
        assert!(!world.resource::<Board>().has_thorns(target));
        assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
    }

    #[test]
    fn adjacent_explosion_removes_thorns_and_schedules_their_return() {
        let mut app = interaction_app();
        let oil = IVec2::new(6, 3);
        let thorn = IVec2::new(7, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.activate_thornheart(thorn, 1, true);
            board.finish_thorn_turn();
            board.add_oil(oil);
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(oil));
        assert!(!app.world().resource::<Board>().has_thorns(thorn));
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(THORNHEART_ITEM_INDEX, 5))
        );
        for _ in 0..5 {
            press_key(&mut app, KeyCode::KeyR);
        }
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingThornheart>().iter(world).count(), 1);
        assert!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn water_activates_all_dormant_life_essences_on_the_tile() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 2);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert!(app.world().resource::<Board>().has_life_essence(target));
        {
            let world = app.world_mut();
            assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 0);
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(LIFE_ESSENCE_ITEM_INDEX, 5), (LIFE_ESSENCE_ITEM_INDEX, 5)]
        );
    }

    #[test]
    fn respawn_waits_until_a_suitable_tile_is_available() {
        let mut app = interaction_app();
        app.world_mut()
            .resource_mut::<TurnState>()
            .ingredient_respawns
            .push((LIFE_ESSENCE_ITEM_INDEX, 1));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for y in 0..ROWS {
                for x in 0..COLUMNS {
                    board.set(IVec2::new(x as i32, y as i32), Ground::Ridge);
                }
            }
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(LIFE_ESSENCE_ITEM_INDEX, 0)]
        );
        assert_eq!(item_count(&mut app), 0);
        app.world_mut()
            .resource_mut::<Board>()
            .set(IVec2::new(0, 0), Ground::Solid);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
        assert_eq!(item_count(&mut app), 1);
    }

    #[test]
    fn ember_recycles_only_after_its_layer_is_removed() {
        for remover in [SUNSTONE_ITEM_INDEX, VOID_DUST_ITEM_INDEX] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            click(&mut app, p1_item(OIL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().has_oil(cell));
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .ingredient_respawns
                    .is_empty()
            );
            click(&mut app, p2_item(remover));
            click(&mut app, cell_center(cell));
            let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
            assert_eq!(
                respawns
                    .iter()
                    .filter(|&&(item, delay)| item == OIL_ITEM_INDEX && delay == 5)
                    .count(),
                1
            );
            assert!(!app.world().resource::<Board>().has_oil(cell));
        }
    }

    #[test]
    fn connected_ember_layers_recycle_once_after_five_turns() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_oil(source);
            board.add_oil(source + IVec2::X);
        }
        app.world_mut().resource_mut::<TurnState>().turns_until_drop = 200;
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(source));
        for remaining in (0..=5).rev() {
            let respawns = &app.world().resource::<TurnState>().ingredient_respawns;
            assert_eq!(
                respawns
                    .iter()
                    .filter(|&&(item, delay)| item == OIL_ITEM_INDEX && delay == remaining)
                    .count(),
                2
            );
            if remaining > 0 {
                press_key(&mut app, KeyCode::KeyR);
            }
        }
    }

    #[test]
    fn life_essence_reapplication_refreshes_duration_without_losing_sources() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().has_life_essence(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().has_life_essence(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(!app.world().resource::<Board>().has_life_essence(cell));
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(LIFE_ESSENCE_ITEM_INDEX, 5), (LIFE_ESSENCE_ITEM_INDEX, 5)]
        );
    }

    #[test]
    fn item_requires_two_clicks_and_previews_after_release() {
        let mut app = interaction_app();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item { index: 0, .. })
        ));
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);

        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(90.0, 60.0));
        app.update();
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert!(
            feedback
                .single(world)
                .unwrap()
                .0
                .contains("PLACE - 1 grid step")
        );
        assert_eq!(item_count(&mut app), 0);
        click(&mut app, Vec2::new(90.0, 60.0));
        assert_eq!(item_count(&mut app), 0);
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
    }

    #[test]
    fn unlimited_water_creates_permanent_water_without_respawning() {
        let mut app = interaction_app();
        app.add_systems(Startup, spawn_board_terrain);
        let cell = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][WATER_ITEM_INDEX],
            1
        );
        {
            let world = app.world_mut();
            let sprite = world
                .query::<(&GroundTile, &Sprite)>()
                .iter(world)
                .find(|(tile, _)| tile.0 == cell)
                .unwrap()
                .1;
            assert_eq!(sprite.color, Ground::Water.color());
            assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
        }
        for _ in 0..4 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert_eq!(item_count(&mut app), 0);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn water_on_ridge_remains_usable_but_cannot_be_picked_up() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Ridge);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Ridge)
        );
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][WATER_ITEM_INDEX],
            1
        );
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
        for _ in 0..6 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Ridge)
        );
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .is_empty()
        );
        assert_eq!(
            app.world_mut()
                .query::<&PendingWaterStone>()
                .iter(app.world())
                .count(),
            1
        );
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        app.world_mut().resource_mut::<TurnState>().inventory[1][WATER_ITEM_INDEX] = 0;
        click(&mut app, Vec2::new(330.0, 60.0));
        assert_eq!(
            app.world().resource::<TurnState>().inventory[1][WATER_ITEM_INDEX],
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&PendingWaterStone>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn water_stone_and_toad_wart_corrode_ridges_in_either_order() {
        for water_first in [false, true] {
            let mut app = interaction_app();
            app.add_systems(Startup, spawn_board_terrain);
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            app.world_mut()
                .resource_mut::<Board>()
                .set(cell, Ground::Ridge);
            let first = if water_first {
                p1_item(WATER_ITEM_INDEX)
            } else {
                p1_item(ACID_ITEM_INDEX)
            };
            let second = if water_first {
                p2_item(ACID_ITEM_INDEX)
            } else {
                p2_item(WATER_ITEM_INDEX)
            };
            click(&mut app, first);
            click(&mut app, cell_center(cell));
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Ridge)
            );
            click(&mut app, second);
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert_eq!(
                world.resource::<Board>().ground(cell),
                Some(Ground::Solid),
                "water first: {water_first}"
            );
            assert!(world.resource::<Board>().is_acidic(cell));
            assert_eq!(world.query::<&PendingWaterStone>().iter(world).count(), 0);
            let turn = world.resource::<TurnState>();
            assert_eq!(
                turn.inventory[usize::from(!water_first)][WATER_ITEM_INDEX],
                1
            );
            assert_eq!(turn.inventory[usize::from(water_first)][ACID_ITEM_INDEX], 0);
            assert!(
                !turn
                    .ingredient_respawns
                    .iter()
                    .any(|(index, _)| *index == WATER_ITEM_INDEX)
            );
            assert!(
                turn.ingredient_respawns
                    .contains(&(ACID_ITEM_INDEX, if water_first { 5 } else { 4 }))
            );
            let sprite = world
                .query::<(&GroundTile, &Sprite)>()
                .iter(world)
                .find(|(tile, _)| tile.0 == cell)
                .unwrap()
                .1;
            assert_eq!(sprite.color, Ground::Solid.color());
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Solid)
            );
            assert!(!app.world().resource::<Board>().is_acidic(cell));
        }
    }

    #[test]
    fn acid_from_poison_reaction_corrodes_ridge_with_placed_water_stone() {
        let mut app = interaction_app();
        let ridge = IVec2::new(6, 3);
        let source = IVec2::new(5, 4);
        app.world_mut()
            .resource_mut::<Board>()
            .set(ridge, Ground::Ridge);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(ridge));
        click(&mut app, p2_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(source));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(ridge), Some(Ground::Solid));
        assert!(world.resource::<Board>().is_acidic(ridge));
        assert_eq!(world.query::<&PendingWaterStone>().iter(world).count(), 0);
        assert!(
            !world
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .any(|(index, _)| *index == WATER_ITEM_INDEX)
        );
    }

    #[test]
    fn water_on_a_ridge_does_not_react_after_acid_expires() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Ridge);
        click(&mut app, p1_item(ACID_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        for _ in 0..5 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert!(!app.world().resource::<Board>().is_acidic(cell));
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Ridge));
        assert_eq!(world.query::<&PendingWaterStone>().iter(world).count(), 1);
        assert!(
            !world
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .any(|entry| entry.0 == WATER_ITEM_INDEX)
        );
    }

    fn water_label(app: &mut App, cell: IVec2) -> Option<String> {
        let world = app.world_mut();
        let mut labels = world.query::<(&WaterLabel, &Text2d)>();
        labels
            .iter(world)
            .find_map(|(label, text)| (label.0 == cell).then(|| text.0.clone()))
    }

    fn poison_cells(app: &mut App) -> Vec<IVec2> {
        let world = app.world_mut();
        let mut tiles = world.query::<&PoisonTile>();
        let mut cells: Vec<_> = tiles.iter(world).map(|tile| tile.0).collect();
        cells.sort_unstable_by_key(|cell| (cell.x, cell.y));
        cells
    }

    fn life_essence_cells(app: &mut App) -> Vec<IVec2> {
        let world = app.world_mut();
        let mut tiles = world.query::<&LifeEssenceTile>();
        let mut cells: Vec<_> = tiles.iter(world).map(|tile| tile.0).collect();
        cells.sort_unstable_by_key(|cell| (cell.x, cell.y));
        cells
    }

    #[test]
    fn life_essence_heals_only_its_water_tile_for_three_turns() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(target, Ground::Water);
            board.set(IVec2::new(5, 3), Ground::Water);
        }
        app.world_mut().resource_mut::<PlayerHealth>().0 = [6, 4];
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(life_essence_cells(&mut app), [target]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [6, 5]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [6, 6]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [6, 7]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [6, 7]);
    }

    #[test]
    fn life_essence_healing_does_not_exceed_full_health() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<Board>().make_water(target);
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 9;
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 10);
    }

    #[test]
    fn life_essence_does_not_stack_or_heal_on_cancelled_actions() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 0;
        for _ in 0..2 {
            click_part(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
        }
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 1);
        click(&mut app, cell_center(target));
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 1);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 2);
        assert_eq!(life_essence_cells(&mut app), [target]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 3);
        assert!(life_essence_cells(&mut app).is_empty());
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 4);
        assert_eq!(life_essence_cells(&mut app), [target]);
    }

    #[test]
    fn life_essence_on_created_water_lasts_three_turns() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        move_player_to(&mut app, PlayerId::One, target);
        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        click(&mut app, p2_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 1);
        assert_eq!(life_essence_cells(&mut app), [target]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 2);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(life_essence_cells(&mut app), [target]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 3);
        assert!(life_essence_cells(&mut app).is_empty());
        assert!(!app.world().resource::<Board>().has_life_essence(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 3);
    }

    #[test]
    fn water_activates_life_essence_placed_on_dry_ground() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 1);
        assert!(life_essence_cells(&mut app).is_empty());
        move_player_to(&mut app, PlayerId::One, target);
        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 1);
        assert_eq!(life_essence_cells(&mut app), [target]);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn healing_and_explosion_damage_are_netted_before_clamping() {
        for (starting_health, expected_health) in [(1, 1), (9, 9), (10, 10)] {
            let mut app = interaction_app();
            let healing_cell = IVec2::new(7, 3);
            let explosion = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, healing_cell);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(healing_cell, Ground::Water);
                board.add_life_essence(healing_cell);
                board.add_oil(explosion);
            }
            app.world_mut().resource_mut::<PlayerHealth>().0[1] = starting_health;
            click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
            click(&mut app, cell_center(explosion));
            assert_eq!(
                app.world().resource::<PlayerHealth>().0,
                [9, expected_health]
            );
        }
    }

    #[test]
    fn healing_nets_with_electricity_and_poison_before_zero_is_applied() {
        for (starting_health, poisoned, expected) in [(1, false, 0), (10, false, 9), (1, true, 0)] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(target, Ground::Water);
                board.add_life_essence(target);
                if poisoned {
                    board.add_poison(target);
                }
            }
            app.world_mut().resource_mut::<PlayerHealth>().0[1] = starting_health;
            click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [9, expected]);
            assert_eq!(life_essence_cells(&mut app), [target]);
            assert_eq!(
                poison_cells(&mut app),
                if poisoned { vec![target] } else { vec![] }
            );
        }
    }

    #[test]
    fn frozen_healing_water_nets_healing_with_tile_damage() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(target, Ground::Water);
            board.add_life_essence(target);
        }
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 1;
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 2);
        assert!(app.world().resource::<Board>().is_frozen(target));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 2);
    }

    #[test]
    fn poison_damages_both_players_as_water_spreads_and_clears() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let adjacent = IVec2::new(5, 3);
        move_player_to(&mut app, PlayerId::Two, source);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(source, Ground::Water);
            board.set(adjacent, Ground::Water);
        }
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(poison_cells(&mut app), [source]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_heart(&mut app, PlayerId::Two, 4, true, true);
        click(&mut app, cell_center(source));
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(poison_cells(&mut app), [source]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(poison_cells(&mut app), [adjacent, source]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 8]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(poison_cells(&mut app), [adjacent]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 8]);
        press_key(&mut app, KeyCode::KeyR);
        assert!(poison_cells(&mut app).is_empty());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 8]);
    }

    #[test]
    fn leaving_a_poisoned_tile_avoids_the_end_of_turn_damage() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, source);
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(7, 3)));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(poison_cells(&mut app), [source]);
        press_key(&mut app, KeyCode::KeyR);
        assert!(poison_cells(&mut app).is_empty());
    }

    #[test]
    fn water_on_nearly_expired_dry_poison_allows_it_to_spread_next_turn() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let neighbor = IVec2::new(7, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(neighbor, Ground::Water);
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(poison_cells(&mut app), [source]);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(poison_cells(&mut app), [source]);
        assert_eq!(
            app.world().resource::<Board>().ground(source),
            Some(Ground::Water)
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(poison_cells(&mut app), [neighbor]);
        assert_eq!(
            app.world().resource::<Board>().ground(source),
            Some(Ground::Water)
        );
    }

    #[test]
    fn frost_and_electricity_work_normally_on_poisoned_water() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for cell in [IVec2::new(5, 3), source, IVec2::new(7, 3)] {
                board.set(cell, Ground::Water);
            }
        }
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert!(app.world().resource::<Board>().is_frozen(source));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 7]);
        assert_eq!(poison_cells(&mut app), [source]);
    }

    #[test]
    fn frozen_poison_is_suspended_until_the_turn_after_thaw() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        app.world_mut()
            .resource_mut::<Board>()
            .set(source, Ground::Water);
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        move_player_to(&mut app, PlayerId::One, source);
        click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(poison_cells(&mut app), [source]);
        for expected_health in [9, 9, 8] {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<PlayerHealth>().0[0], expected_health);
            assert_eq!(poison_cells(&mut app), [source]);
        }
        assert!(app.world().resource::<Board>().is_frozen(source));
        app.world_mut().resource_mut::<Board>().thaw(source);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 10]);
        assert_eq!(poison_cells(&mut app), [source]);
    }

    #[test]
    fn poison_cannot_be_placed_directly_on_frozen_water() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(source, Ground::Water);
            assert!(board.freeze_water(source));
        }
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert!(poison_cells(&mut app).is_empty());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
    }

    #[test]
    fn ember_on_water_creates_steam_and_recycles_once() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(!app.world().resource::<Board>().has_oil(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);

        assert!(!steam_positions(&mut app).is_empty());
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(OIL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn sunstone_on_water_waits_and_ember_produces_steam() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut().resource_mut::<Board>().make_water(cell);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(steam_positions(&mut app).is_empty());
        assert_eq!(item_count(&mut app), 1);
        click(&mut app, p2_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(!steam_positions(&mut app).is_empty());
        assert!(app.world().resource::<Board>().is_water(cell));
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(
            app.world().resource::<TurnState>().ingredient_respawns,
            [(OIL_ITEM_INDEX, 5)]
        );
    }

    #[test]
    fn queued_frost_and_ember_reject_conflicts_before_either_resolves() {
        for ember_first in [false, true] {
            for offset in [IVec2::ZERO, IVec2::ONE, IVec2::new(2, 0)] {
                let mut app = interaction_app();
                let frost = IVec2::new(6, 3);
                let ember = frost + offset;
                move_player_to(&mut app, PlayerId::Two, IVec2::new(10, 5));
                app.world_mut().resource_mut::<TurnState>().inventory[0]
                    [FROST_CRYSTAL_ITEM_INDEX] = 1;
                app.world_mut().resource_mut::<TurnState>().inventory[0][OIL_ITEM_INDEX] = 1;
                {
                    let mut board = app.world_mut().resource_mut::<Board>();
                    board.set(frost, Ground::Water);
                    board.set(ember, Ground::Water);
                }
                let (first, second) = if ember_first {
                    ((OIL_ITEM_INDEX, ember), (FROST_CRYSTAL_ITEM_INDEX, frost))
                } else {
                    ((FROST_CRYSTAL_ITEM_INDEX, frost), (OIL_ITEM_INDEX, ember))
                };
                click_part(&mut app, p1_item(first.0));
                click_part(&mut app, cell_center(first.1));
                let energy = app.world().resource::<TurnState>().strength[0];
                click_part(&mut app, p1_item(second.0));
                click_part(&mut app, cell_center(second.1));
                if offset == IVec2::new(2, 0) {
                    assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
                    assert!(app.world().resource::<Board>().is_frozen(frost));
                    assert!(!steam_positions(&mut app).is_empty());
                } else {
                    let turn = app.world().resource::<TurnState>();
                    assert_eq!(turn.active, PlayerId::One);
                    assert_eq!(turn.actions_used, 1);
                    assert_eq!(turn.strength[0], energy);
                    assert_eq!(turn.inventory[0][second.0], 1);
                    assert!(turn.pending_placement.is_some());
                    assert!(!app.world().resource::<Board>().is_frozen(frost));
                    assert!(!app.world().resource::<Board>().has_oil(ember));
                    assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
                    assert!(steam_positions(&mut app).is_empty());
                }
            }
        }
    }

    #[test]
    fn frost_and_ember_cannot_be_placed_on_existing_opposites() {
        for item in [FROST_CRYSTAL_ITEM_INDEX, OIL_ITEM_INDEX] {
            for placed in [false, true] {
                let mut app = interaction_app();
                let cell = IVec2::new(8, 3);
                app.world_mut().resource_mut::<TurnState>().inventory[0][item] = 1;
                if item == FROST_CRYSTAL_ITEM_INDEX {
                    app.world_mut().resource_mut::<Board>().add_oil(cell);
                } else {
                    let mut crystal = app.world_mut().spawn((
                        BoardItem,
                        Collectible {
                            index: FROST_CRYSTAL_ITEM_INDEX,
                            cell,
                        },
                        PendingFrost(cell),
                    ));
                    if placed {
                        crystal.insert(PlacedItem);
                    }
                }
                click_part(&mut app, p1_item(item));
                click_part(&mut app, cell_center(cell));
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.active, PlayerId::One);
                assert_eq!(turn.actions_used, 0);
                assert_eq!(turn.strength[0], 5);
                assert_eq!(turn.inventory[0][item], 1);
                assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
                let world = app.world_mut();
                assert!(
                    world
                        .query_filtered::<&Text, With<DropFeedback>>()
                        .single(world)
                        .unwrap()
                        .0
                        .contains("cannot share a tile")
                );
            }
        }
    }

    #[test]
    fn queued_frost_and_ember_reject_same_dry_tile_even_on_a_player() {
        for first in [FROST_CRYSTAL_ITEM_INDEX, OIL_ITEM_INDEX] {
            for occupied in [false, true] {
                let mut app = interaction_app();
                let cell = IVec2::new(6, 3);
                let second = if first == FROST_CRYSTAL_ITEM_INDEX {
                    OIL_ITEM_INDEX
                } else {
                    FROST_CRYSTAL_ITEM_INDEX
                };
                {
                    let mut turn = app.world_mut().resource_mut::<TurnState>();
                    turn.inventory[0][first] = 1;
                    turn.inventory[0][second] = 1;
                }
                if occupied {
                    move_player_to(&mut app, PlayerId::Two, cell);
                }
                click_part(&mut app, p1_item(first));
                click_part(&mut app, cell_center(cell));
                let energy = app.world().resource::<TurnState>().strength[0];
                click_part(&mut app, p1_item(second));
                click_part(&mut app, cell_center(cell));
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.active, PlayerId::One);
                assert_eq!(turn.actions_used, 1);
                assert_eq!(turn.strength[0], energy);
                assert_eq!(turn.inventory[0][second], 1);
                assert!(turn.pending_placement.is_some());
                assert_eq!(turn.frozen_turns, [0, 0]);
                assert!(!app.world().resource::<Board>().has_oil(cell));
                assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
                click_part(&mut app, Vec2::new(-330.0, -60.0));
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.actions_used, 0);
                assert_eq!(turn.inventory[0][first], 1);
            }
        }
    }

    #[test]
    fn ember_can_be_placed_on_a_frozen_player_without_a_crystal() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<TurnState>().frozen_turns[1] = 2;
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().has_oil(cell));
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
    }

    #[test]
    fn frost_and_ember_can_be_queued_on_separate_dry_tiles() {
        for first in [FROST_CRYSTAL_ITEM_INDEX, OIL_ITEM_INDEX] {
            let mut app = interaction_app();
            let frost = IVec2::new(6, 3);
            let ember = frost + IVec2::X;
            let first_cell = if first == FROST_CRYSTAL_ITEM_INDEX {
                frost
            } else {
                ember
            };
            let (second, second_cell) = if first == FROST_CRYSTAL_ITEM_INDEX {
                (OIL_ITEM_INDEX, ember)
            } else {
                (FROST_CRYSTAL_ITEM_INDEX, frost)
            };
            click_part(&mut app, p1_item(first));
            click_part(&mut app, cell_center(first_cell));
            click_part(&mut app, p1_item(second));
            click_part(&mut app, cell_center(second_cell));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert!(app.world().resource::<Board>().has_oil(ember));
            let world = app.world_mut();
            assert!(
                world
                    .query::<&PendingFrost>()
                    .iter(world)
                    .any(|item| item.0 == frost)
            );
        }
    }

    #[test]
    fn queued_water_and_void_reject_both_orders_without_effects() {
        for (first, second) in [
            (WATER_ITEM_INDEX, VOID_DUST_ITEM_INDEX),
            (VOID_DUST_ITEM_INDEX, WATER_ITEM_INDEX),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            click_part(&mut app, p1_item(first));
            click_part(&mut app, cell_center(cell));
            let energy = app.world().resource::<TurnState>().strength[0];
            click_part(&mut app, p1_item(second));
            click_part(&mut app, cell_center(cell));
            let turn = app.world().resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::One);
            assert_eq!(turn.actions_used, 1);
            assert_eq!(turn.strength[0], energy);
            assert!(turn.pending_placement.is_some());
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Solid)
            );
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
            click_part(&mut app, Vec2::new(-330.0, -60.0));
            assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
        }
    }

    #[test]
    fn ember_coats_solid_and_ridge_and_rejects_duplicate_placement() {
        for ground in [Ground::Solid, Ground::Ridge] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
            app.world_mut().resource_mut::<Board>().set(cell, ground);
            click(&mut app, p1_item(OIL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().has_oil(cell));
            assert_eq!(item_count(&mut app), 0);
            press_key(&mut app, KeyCode::KeyR);
            for _ in 0..20 {
                press_key(&mut app, KeyCode::KeyR);
                assert!(app.world().resource::<Board>().has_oil(cell));
            }
            app.world_mut().resource_mut::<TurnState>().inventory[0][OIL_ITEM_INDEX] = 1;
            let before = app.world().resource::<TurnState>().inventory[0][OIL_ITEM_INDEX];
            let energy = app.world().resource::<TurnState>().strength[0];
            click(&mut app, p1_item(OIL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().has_oil(cell));
            assert_eq!(
                app.world().resource::<TurnState>().inventory[0][OIL_ITEM_INDEX],
                before
            );
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
            assert_eq!(app.world().resource::<TurnState>().strength[0], energy);
            assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
            assert_eq!(item_count(&mut app), 0);
            assert_eq!(app.world().resource::<Board>().ground(cell), Some(ground));
        }
    }

    #[test]
    fn water_on_ember_creates_steam_and_permanent_water() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(!app.world().resource::<Board>().has_oil(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(water_label(&mut app, cell), None);
        assert_eq!(item_count(&mut app), 0);
        assert!(!steam_positions(&mut app).is_empty());
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(OIL_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn oil_survives_water_transitions_and_fire_still_consumes_it() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        for _ in 0..3 {
            press_key(&mut app, KeyCode::KeyR);
        }
        assert!(app.world().resource::<Board>().has_oil(cell));
        app.world_mut().resource_mut::<Board>().make_water(cell);
        for _ in 0..20 {
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<Board>().has_oil(cell));
        }
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Solid);
        for _ in 0..20 {
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<Board>().has_oil(cell));
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(!app.world().resource::<Board>().has_oil(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Solid)
        );
    }

    #[test]
    fn oil_persists_when_wet_ground_dries() {
        let mut board = Board::default();
        let cell = IVec2::new(6, 3);
        board.add_oil(cell);
        board.make_wet(cell);
        for _ in 0..4 {
            board.finish_turn([], None);
            assert!(board.has_oil(cell));
        }
        board.finish_turn([], None);
        assert!(!board.is_water(cell));
        assert!(board.has_oil(cell));
    }

    #[test]
    fn steam_covers_neighbors_and_three_leading_tiles_for_every_wind() {
        let board = Board::default();
        let source = IVec2::new(4, 3);
        for (wind, extension) in [
            (WindDirection::North, [(-1, -2), (0, -2), (1, -2)]),
            (WindDirection::NorthEast, [(1, -2), (2, -2), (2, -1)]),
            (WindDirection::East, [(2, -1), (2, 0), (2, 1)]),
            (WindDirection::SouthEast, [(2, 1), (2, 2), (1, 2)]),
            (WindDirection::South, [(-1, 2), (0, 2), (1, 2)]),
            (WindDirection::SouthWest, [(-1, 2), (-2, 2), (-2, 1)]),
            (WindDirection::West, [(-2, -1), (-2, 0), (-2, 1)]),
            (WindDirection::NorthWest, [(-2, -1), (-2, -2), (-1, -2)]),
        ] {
            let mut expected = std::collections::HashSet::new();
            for y in -1..=1 {
                for x in -1..=1 {
                    expected.insert(source + IVec2::new(x, y));
                }
            }
            expected.extend(extension.map(|(x, y)| source + IVec2::new(x, y)));
            let cells = steam_cells(&board, source, wind);
            assert_eq!(cells.len(), 12, "{wind:?}");
            assert_eq!(
                cells.into_iter().collect::<std::collections::HashSet<_>>(),
                expected,
                "{wind:?}"
            );
        }
    }

    #[test]
    fn steam_clips_to_the_board_and_deflects_wind_at_ridges() {
        let mut board = Board::default();
        let corner = IVec2::ZERO;
        let cells = steam_cells(&board, corner, WindDirection::West);
        assert_eq!(
            cells.into_iter().collect::<std::collections::HashSet<_>>(),
            [corner, IVec2::X, IVec2::Y, IVec2::ONE]
                .into_iter()
                .collect()
        );
        let source = IVec2::new(4, 3);
        board.set(source + IVec2::new(-1, 0), Ground::Ridge);
        board.set(source + IVec2::new(-2, -1), Ground::Ridge);
        let cells = steam_cells(&board, source, WindDirection::West);
        assert!(!cells.contains(&(source + IVec2::new(-1, 0))));
        assert!(!cells.contains(&(source + IVec2::new(-2, 0))));
        assert!(!cells.contains(&(source + IVec2::new(-2, -1))));
        assert!(cells.contains(&(source + IVec2::new(-1, -2))));
        board.set(source + IVec2::new(-1, -2), Ground::Ridge);
        assert!(
            !steam_cells(&board, source, WindDirection::West)
                .contains(&(source + IVec2::new(-1, -2)))
        );
    }

    #[test]
    fn adjacent_ridges_block_steam_and_their_downwind_extension() {
        let source = IVec2::new(4, 3);
        for wind in [
            WindDirection::North,
            WindDirection::NorthEast,
            WindDirection::East,
            WindDirection::SouthEast,
            WindDirection::South,
            WindDirection::SouthWest,
            WindDirection::West,
            WindDirection::NorthWest,
        ] {
            for y in -1..=1 {
                for x in -1..=1 {
                    if x == 0 && y == 0 {
                        continue;
                    }
                    let mut board = Board::default();
                    let ridge = source + IVec2::new(x, y);
                    let beyond = ridge + wind.offset();
                    let original = steam_cells(&board, source, wind);
                    board.set(ridge, Ground::Ridge);
                    let cells = steam_cells(&board, source, wind);
                    assert!(!cells.contains(&ridge), "{wind:?} {ridge:?}");
                    if (beyond - source).abs().max_element() > 1 && original.contains(&beyond) {
                        assert!(!cells.contains(&beyond), "{wind:?} {ridge:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn overlapping_poison_does_not_stack_turn_damage() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, source);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_poison(source);
            board.add_poison(source);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(poison_cells(&mut app), [source]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 8]);
    }

    #[test]
    fn poison_obeys_throw_energy_and_rejects_unaffordable_targets() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(IVec2::new(8, 3)));
        assert!(poison_cells(&mut app).is_empty());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(app.world().resource::<Selection>().0.is_some());
        click(&mut app, cell_center(IVec2::new(6, 3)));
        assert_eq!(poison_cells(&mut app), [IVec2::new(6, 3)]);
        assert_eq!(app.world().resource::<TurnState>().strength[0], 2);
    }

    fn steam_positions(app: &mut App) -> Vec<IVec2> {
        let world = app.world_mut();
        let mut steam = world.query_filtered::<&Transform, With<SteamFlash>>();
        let mut cells: Vec<_> = steam
            .iter(world)
            .map(|transform| board_cell(transform.translation.truncate()).unwrap())
            .collect();
        cells.sort_unstable_by_key(|cell| (cell.x, cell.y));
        cells
    }

    fn west_steam_positions() -> Vec<IVec2> {
        (4..=7)
            .flat_map(|x| (2..=4).map(move |y| IVec2::new(x, y)))
            .collect()
    }

    #[test]
    fn wind_is_visible_and_rolls_only_after_completed_actions() {
        let mut app = interaction_app();
        app.update();
        let mut expected_rng = WyRand::seed_from_u64(42);
        let expected = WindDirection::random(&mut expected_rng);
        {
            let world = app.world_mut();
            let mut rng = world.query_filtered::<&mut WyRand, With<GlobalRng>>();
            *rng.single_mut(world).unwrap() = WyRand::seed_from_u64(42);
            world.resource_mut::<TurnState>().wind = WindDirection::West;
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(
            app.world().resource::<TurnState>().wind,
            WindDirection::West
        );
        {
            let world = app.world_mut();
            let mut text = world.query_filtered::<&Text, With<WindStatus>>();
            assert_eq!(text.single(world).unwrap().0, "Wind: W");
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().wind, expected);
        let world = app.world_mut();
        let mut text = world.query_filtered::<&Text, With<WindStatus>>();
        assert_eq!(
            text.single(world).unwrap().0,
            format!("Wind: {}", expected.label())
        );
    }

    #[test]
    fn ember_uses_current_wind_and_leaves_water() {
        let mut app = interaction_app();
        app.update();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<TurnState>().wind = WindDirection::West;
        app.world_mut().resource_mut::<Board>().make_water(target);
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(steam_positions(&mut app), west_steam_positions());
        assert!(app.world().resource::<Board>().is_water(target));
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
        app.world_mut().resource_mut::<TurnState>().wind = WindDirection::East;
        app.update();
        assert_eq!(steam_positions(&mut app), west_steam_positions());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert!(steam_positions(&mut app).is_empty());
    }

    #[test]
    fn water_on_a_placed_sunstone_does_not_create_steam() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 1);
        app.world_mut().resource_mut::<TurnState>().wind = WindDirection::West;
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert!(steam_positions(&mut app).is_empty());
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<Board>().water_percentage(target), 0);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(water_label(&mut app, target), None);
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
    }

    #[test]
    fn steam_deals_one_heart_at_center_and_half_to_every_other_affected_tile() {
        let target = IVec2::new(6, 3);
        for cell in west_steam_positions()
            .into_iter()
            .chain(std::iter::once(IVec2::new(3, 3)))
        {
            for hovering in [false, true] {
                let mut app = interaction_app();
                app.update();
                let player_one = if cell == IVec2::new(6, 4) {
                    IVec2::new(5, 4)
                } else {
                    IVec2::new(6, 4)
                };
                move_player_to(&mut app, PlayerId::One, player_one);
                move_player_to(&mut app, PlayerId::Two, cell);
                app.world_mut().resource_mut::<TurnState>().hovering[1] = hovering;
                app.world_mut().resource_mut::<TurnState>().wind = WindDirection::West;
                app.world_mut().resource_mut::<Board>().make_water(target);
                click(&mut app, p1_item(OIL_ITEM_INDEX));
                click(&mut app, cell_center(target));
                let expected = if cell == target {
                    8
                } else if cell == IVec2::new(3, 3) {
                    10
                } else {
                    9
                };
                assert_eq!(
                    app.world().resource::<PlayerHealth>().0[1],
                    expected,
                    "cell={cell:?} hovering={hovering}"
                );
                assert_eq!(steam_positions(&mut app), west_steam_positions());
            }
        }
    }

    #[test]
    fn oil_on_a_placed_sunstone_ignites_and_is_consumed() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        assert!(!app.world().resource::<Board>().has_oil(target));
        assert_eq!(item_count(&mut app), 0);
        assert!(steam_positions(&mut app).is_empty());
        let world = app.world_mut();
        let mut fire = world.query_filtered::<Entity, With<FireFlash>>();
        assert_eq!(fire.iter(world).count(), 9);
    }

    #[test]
    fn placed_sunstone_reacts_only_to_a_valid_matching_tile() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        app.world_mut().resource_mut::<TurnState>().strength[1] = 0;
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 1);
        assert!(steam_positions(&mut app).is_empty());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        click(&mut app, cell_center(IVec2::new(7, 5)));
        assert_eq!(item_count(&mut app), 1);
        assert!(steam_positions(&mut app).is_empty());
        let world = app.world_mut();
        let mut sunstones = world.query::<&PendingSunstone>();
        assert_eq!(sunstones.iter(world).count(), 1);
    }

    #[test]
    fn water_on_sunstone_still_resolves_placed_frost_and_storm_shard() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(IVec2::new(5, 3), Ground::Water);
            board.set(IVec2::new(7, 3), Ground::Water);
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 3);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 7]);
        assert!(app.world().resource::<Board>().is_frozen(target));
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 1);
        assert!(steam_positions(&mut app).is_empty());
        assert_eq!(frozen_sprite_count(&mut app), 3);
    }

    #[test]
    fn water_stone_on_sunstone_preserves_poison_on_the_new_water() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(POISON_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(poison_cells(&mut app), [target]);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(poison_cells(&mut app), [target]);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
    }

    #[test]
    fn sunstone_melts_ice_without_creating_steam_or_damage() {
        for natural_water in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if natural_water {
                    board.set(target, Ground::Water);
                } else {
                    board.make_wet(target);
                }
                assert!(board.freeze_water(target));
            }
            click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert!(!app.world().resource::<Board>().is_frozen(target));
            assert!(app.world().resource::<Board>().is_water(target));
            assert_eq!(
                app.world().resource::<Board>().water_percentage(target),
                if natural_water { 0 } else { 80 }
            );
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
            assert!(steam_positions(&mut app).is_empty());
            assert_eq!(item_count(&mut app), 0);
            press_key(&mut app, KeyCode::KeyR);
            if !natural_water {
                assert_eq!(app.world().resource::<Board>().water_percentage(target), 60);
            }
        }
    }

    #[test]
    fn sunstone_updates_placed_water_and_frozen_visuals() {
        for freeze_first in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
            click(&mut app, p1_item(WATER_ITEM_INDEX));
            click(&mut app, cell_center(target));
            if freeze_first {
                click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
                click(&mut app, cell_center(target));
                assert_eq!(frozen_sprite_count(&mut app), 1);
                click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
            } else {
                click(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
            }
            click(&mut app, cell_center(target));
            assert_eq!(frozen_sprite_count(&mut app), 0);
            assert_eq!(water_label(&mut app, target), None);
            assert_eq!(
                app.world().resource::<Board>().ground(target),
                Some(Ground::Water)
            );
            assert_eq!(item_count(&mut app), usize::from(!freeze_first));
            if freeze_first {
                press_key(&mut app, KeyCode::KeyR);
                assert_eq!(
                    app.world().resource::<Board>().ground(target),
                    Some(Ground::Water)
                );
            }
        }
    }

    #[test]
    fn sunstone_consumes_oil_and_damages_the_target_and_neighbors() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 4));
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        move_player_to(&mut app, PlayerId::One, target);
        click(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 9]);
        assert!(!app.world().resource::<Board>().has_oil(target));
        assert_eq!(item_count(&mut app), 0);
        let world = app.world_mut();
        let mut fire = world.query_filtered::<&Transform, With<FireFlash>>();
        let mut cells: Vec<_> = fire
            .iter(world)
            .map(|transform| board_cell(transform.translation.truncate()).unwrap())
            .collect();
        cells.sort_unstable_by_key(|cell| (cell.x, cell.y));
        let mut expected = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                expected.push(target + IVec2::new(x, y));
            }
        }
        assert_eq!(cells, expected);
    }

    #[test]
    fn obsidian_reactions_blast_in_the_requested_pattern_in_both_orders() {
        for storm in [false, true] {
            for obsidian_first in [false, true] {
                let mut app = interaction_app();
                let target = IVec2::new(6, 3);
                move_player_to(
                    &mut app,
                    PlayerId::Two,
                    if storm {
                        IVec2::new(8, 5)
                    } else {
                        IVec2::new(8, 3)
                    },
                );
                let first = if obsidian_first {
                    p1_item(OBSIDIAN_ITEM_INDEX)
                } else if storm {
                    p1_item(STORM_SHARD_ITEM_INDEX)
                } else {
                    p1_item(SUNSTONE_ITEM_INDEX)
                };
                let second = if obsidian_first && storm {
                    p2_item(STORM_SHARD_ITEM_INDEX)
                } else if obsidian_first {
                    p2_item(SUNSTONE_ITEM_INDEX)
                } else {
                    p2_item(OBSIDIAN_ITEM_INDEX)
                };
                click(&mut app, first);
                click(&mut app, cell_center(target));
                app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
                click(&mut app, second);
                click(&mut app, cell_center(target));
                let directions = if storm {
                    [
                        IVec2::new(1, 1),
                        IVec2::new(1, -1),
                        IVec2::new(-1, 1),
                        IVec2::new(-1, -1),
                    ]
                } else {
                    [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y]
                };
                let mut expected = vec![target];
                for direction in directions {
                    expected.extend([target + direction, target + direction * 2]);
                }
                let world = app.world_mut();
                let mut flashes = world.query_filtered::<&Transform, With<FireFlash>>();
                let mut actual: Vec<_> = flashes
                    .iter(world)
                    .map(|transform| board_cell(transform.translation.truncate()).unwrap())
                    .collect();
                actual.sort_unstable_by_key(|cell| (cell.x, cell.y));
                expected.sort_unstable_by_key(|cell| (cell.x, cell.y));
                assert_eq!(actual, expected);
                assert_eq!(world.resource::<PlayerHealth>().0[1], 7);
                assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
                let respawns = &world.resource::<TurnState>().ingredient_respawns;
                assert!(respawns.contains(&(OBSIDIAN_ITEM_INDEX, 5)));
                assert!(respawns.contains(&(
                    if storm {
                        STORM_SHARD_ITEM_INDEX
                    } else {
                        SUNSTONE_ITEM_INDEX
                    },
                    5
                )));
            }
        }
    }

    #[test]
    fn obsidian_combines_storm_shard_and_sunstone_reactions() {
        for sunstone_first in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
            let first = if sunstone_first {
                p1_item(SUNSTONE_ITEM_INDEX)
            } else {
                p1_item(STORM_SHARD_ITEM_INDEX)
            };
            let second = if sunstone_first {
                p2_item(STORM_SHARD_ITEM_INDEX)
            } else {
                p2_item(SUNSTONE_ITEM_INDEX)
            };
            click(&mut app, first);
            click(&mut app, cell_center(target));
            click(&mut app, second);
            click(&mut app, cell_center(target));
            move_player_to(&mut app, PlayerId::Two, target);
            click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
            click(&mut app, cell_center(target));
            let world = app.world_mut();
            assert_eq!(world.resource::<PlayerHealth>().0, [7, 4]);
            assert_eq!(world.query::<&PendingStormShard>().iter(world).count(), 0);
            assert_eq!(world.query::<&PendingSunstone>().iter(world).count(), 0);
            assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
            let mut expected = vec![target, target];
            for direction in [
                IVec2::X,
                IVec2::NEG_X,
                IVec2::Y,
                IVec2::NEG_Y,
                IVec2::new(1, 1),
                IVec2::new(1, -1),
                IVec2::new(-1, 1),
                IVec2::new(-1, -1),
            ] {
                expected.extend([target + direction, target + direction * 2]);
            }
            let mut cells: Vec<_> = world
                .query_filtered::<&Transform, With<FireFlash>>()
                .iter(world)
                .map(|transform| board_cell(transform.translation.truncate()).unwrap())
                .collect();
            cells.sort_unstable_by_key(|cell| (cell.x, cell.y));
            expected.sort_unstable_by_key(|cell| (cell.x, cell.y));
            assert_eq!(cells, expected);
            let respawns = &world.resource::<TurnState>().ingredient_respawns;
            for index in [
                OBSIDIAN_ITEM_INDEX,
                SUNSTONE_ITEM_INDEX,
                STORM_SHARD_ITEM_INDEX,
            ] {
                assert_eq!(
                    respawns
                        .iter()
                        .filter(|entry| **entry == (index, 5))
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn combined_obsidian_reactions_in_water_do_not_duplicate_the_electric_blast() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        app.world_mut()
            .spawn((BoardItem, PendingStormShard(target)));
        app.world_mut().spawn((BoardItem, PendingSunstone(target)));
        click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 4);
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingStormShard>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingSunstone>().iter(world).count(), 0);
        assert_eq!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .filter(|entry| entry.0 == OBSIDIAN_ITEM_INDEX)
                .count(),
            1
        );
    }

    #[test]
    fn obsidian_cannot_be_placed_in_holes_and_keeps_the_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Hole);
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let world = app.world_mut();
        assert_eq!(world.resource::<TurnState>().active, PlayerId::One);
        assert_eq!(
            world.resource::<TurnState>().inventory[0][OBSIDIAN_ITEM_INDEX],
            1
        );
        assert!(world.resource::<Selection>().0.is_some());
        assert!(
            world
                .query_filtered::<&Text, With<DropFeedback>>()
                .single(world)
                .unwrap()
                .0
                .contains("hole")
        );
    }

    #[test]
    fn obsidian_blast_patterns_clip_at_board_edges() {
        let board = Board::default();
        let origin = IVec2::ZERO;
        let diagonal = Blast {
            source: origin,
            kind: BlastKind::Diagonal,
        }
        .cells(&board);
        let cross = Blast {
            source: origin,
            kind: BlastKind::Cross,
        }
        .cells(&board);
        assert_eq!(diagonal.len(), 3);
        assert_eq!(cross.len(), 5);
        assert!(diagonal.contains(&IVec2::new(2, 2)));
        assert!(cross.contains(&IVec2::new(2, 0)));
        assert!(cross.contains(&IVec2::new(0, 2)));
    }

    #[test]
    fn ridges_block_obsidian_rays_unless_the_source_is_on_a_ridge() {
        let source = IVec2::new(4, 3);
        for (kind, direction, other) in [
            (BlastKind::Cross, IVec2::X, IVec2::NEG_X),
            (BlastKind::Diagonal, IVec2::new(1, 1), IVec2::new(-1, -1)),
        ] {
            for distance in [1, 2] {
                let mut board = Board::default();
                let ridge = source + direction * distance;
                board.set(ridge, Ground::Ridge);
                let blast = Blast { source, kind };
                let cells = blast.cells(&board);
                assert!(!cells.contains(&ridge));
                assert!(cells.contains(&(source + other * 2)));
                if distance == 1 {
                    assert!(!cells.contains(&(source + direction * 2)));
                } else {
                    assert!(cells.contains(&(source + direction)));
                }
                board.set(source, Ground::Ridge);
                let cells = blast.cells(&board);
                assert!(cells.contains(&ridge));
                assert!(cells.contains(&(source + direction * 2)));
            }
        }
    }

    #[test]
    fn ridge_shields_players_oil_obsidian_and_thorns_from_obsidian_blasts() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let ridge = IVec2::new(7, 3);
        let behind = IVec2::new(8, 3);
        move_player_to(&mut app, PlayerId::Two, ridge);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        for cell in [source, behind] {
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(cell),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
            ));
        }
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(ridge, Ground::Ridge);
            board.add_oil(behind);
            board.activate_thornheart(behind, 1, true);
            board.finish_thorn_turn();
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(source));
        let world = app.world_mut();
        assert_eq!(world.resource::<PlayerHealth>().0[1], 10);
        assert!(world.resource::<Board>().has_oil(behind));
        assert!(world.resource::<Board>().has_thorns(behind));
        let pending: Vec<_> = world
            .query::<&PendingObsidian>()
            .iter(world)
            .map(|p| p.0)
            .collect();
        assert_eq!(pending, [behind]);
        let flashes: Vec<_> = world
            .query_filtered::<&Transform, With<FireFlash>>()
            .iter(world)
            .map(|transform| board_cell(transform.translation.truncate()).unwrap())
            .collect();
        assert!(!flashes.contains(&ridge));
        assert!(!flashes.contains(&behind));
    }

    #[test]
    fn chain_triggered_obsidian_uses_the_two_tile_cross() {
        let mut board = Board::default();
        let oil = IVec2::new(3, 3);
        let source = IVec2::new(4, 3);
        board.add_oil(oil);
        let result = resolve_blasts(&mut board, &[(None, source)], &[], Some(oil), None);
        assert_eq!(result.consumed_obsidian.len(), 1);
        let blast = result
            .blasts
            .iter()
            .find(|blast| blast.source == source)
            .unwrap();
        let cells = blast.cells(&board);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(source + IVec2::X * 2)));
        assert!(cells.contains(&(source + IVec2::NEG_Y * 2)));
        assert!(!cells.contains(&(source + IVec2::new(1, 1))));
    }

    #[test]
    fn stone_dust_returns_water_to_solid_ground() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let world = app.world_mut();
        assert_eq!(
            world.resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        assert!(!world.resource::<Board>().is_water(target));
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert_eq!(world.query::<&WaterIcon>().iter(world).count(), 0);
        let turn = world.resource::<TurnState>();
        assert_eq!(turn.inventory[0][WATER_ITEM_INDEX], 1);
        assert_eq!(turn.inventory[1][STONE_DUST_ITEM_INDEX], 1);
        assert!(turn.ingredient_respawns.is_empty());
    }

    #[test]
    fn void_on_solid_consumes_nonreacting_collectibles_in_the_new_hole() {
        for index in [
            SUNSTONE_ITEM_INDEX,
            FROST_CRYSTAL_ITEM_INDEX,
            STORM_SHARD_ITEM_INDEX,
            COATL_EGG_ITEM_INDEX,
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            let first = p1_item(index);
            click(&mut app, first);
            click(&mut app, cell_center(cell));
            click(&mut app, p2_item(VOID_DUST_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            let board = world.resource::<Board>();
            let holes: Vec<_> = (0..ROWS)
                .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
                .filter(|cell| board.ground(*cell) == Some(Ground::Hole))
                .collect();
            assert_eq!(holes, [cell], "{index}");
            assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
            let respawns = &world.resource::<TurnState>().ingredient_respawns;
            assert!(
                !respawns
                    .iter()
                    .any(|(item, _)| *item == VOID_DUST_ITEM_INDEX)
            );
            assert_eq!(
                respawns.contains(&(index, 5)),
                !is_unlimited(index),
                "{index}"
            );
        }
    }

    #[test]
    fn void_on_ridge_levels_it_without_creating_a_hole() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Ridge);
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Solid));
        assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
        assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
    }

    #[test]
    fn void_hole_clears_tile_effects_and_respects_the_terrain_limit() {
        let target = IVec2::new(6, 3);
        let mut app = interaction_app();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_oil(target);
            board.add_acid(target);
            board.add_poison(target);
        }
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let board = app.world().resource::<Board>();
        assert_eq!(board.ground(target), Some(Ground::Hole));
        assert!(!board.has_oil(target));
        assert!(!board.is_acidic(target));
        assert!(!board.is_poisoned(target));
        assert!(steam_positions(&mut app).is_empty());

        let mut app = interaction_app();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            let mut count = 0;
            for y in 0..ROWS {
                for x in 0..COLUMNS {
                    let cell = IVec2::new(x as i32, y as i32);
                    if cell != target && count < board::MAX_UNSAFE_TILES {
                        board.set(cell, Ground::Hole);
                        count += 1;
                    }
                }
            }
        }
        click_part(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        assert_eq!(
            app.world().resource::<TurnState>().inventory[0][VOID_DUST_ITEM_INDEX],
            u8::MAX as usize
        );
        assert_eq!(app.world().resource::<TurnState>().actions_used, 0);
    }

    #[test]
    fn void_dust_turns_wet_solid_ground_into_a_hole() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<Board>().make_wet(target);
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let board = app.world().resource::<Board>();
        assert_eq!(board.ground(target), Some(Ground::Hole));
        assert!(!board.is_water(target));
    }

    #[test]
    fn void_does_not_react_with_sunstone_or_frost_on_water() {
        for index in [SUNSTONE_ITEM_INDEX, FROST_CRYSTAL_ITEM_INDEX] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<Board>().make_water(cell);
            let mut item = app
                .world_mut()
                .spawn((BoardItem, Collectible { index, cell }));
            if index == SUNSTONE_ITEM_INDEX {
                item.insert(PendingSunstone(cell));
            } else {
                item.insert(PendingFrost(cell));
            }
            click_part(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
            click_part(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Water));
            assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 1);
            assert_eq!(world.resource::<TurnState>().actions_used, 0);
        }
    }

    #[test]
    fn void_on_obsidian_explodes_and_makes_a_cross_of_holes() {
        for ground in [Ground::Solid, Ground::Water] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, IVec2::new(8, 3));
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(cell, ground);
                for x in -1..=1 {
                    for y in -1..=1 {
                        if x != 0 || y != 0 {
                            board.set(cell + IVec2::new(x, y), Ground::Ridge);
                        }
                    }
                }
            }
            click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            click(&mut app, p2_item(VOID_DUST_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            for x in -1..=1 {
                for y in -1..=1 {
                    assert_eq!(
                        world.resource::<Board>().ground(cell + IVec2::new(x, y)),
                        Some(if x == 0 || y == 0 {
                            Ground::Hole
                        } else {
                            Ground::Solid
                        })
                    );
                }
            }
            assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
            assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
            assert_eq!(world.query::<&FireFlash>().iter(world).count(), 9);
            let respawns = &world.resource::<TurnState>().ingredient_respawns;
            assert!(
                !respawns
                    .iter()
                    .any(|(index, _)| *index == VOID_DUST_ITEM_INDEX)
            );
            assert!(respawns.contains(&(OBSIDIAN_ITEM_INDEX, 5)));
        }
    }

    #[test]
    fn void_obsidian_explodes_before_the_cross_becomes_holes() {
        let mut app = interaction_app();
        let center = IVec2::new(6, 3);
        let diagonal = center + IVec2::ONE;
        let adjacent = center + IVec2::X;
        move_player_to(&mut app, PlayerId::One, center);
        move_player_to(&mut app, PlayerId::Two, diagonal);
        app.world_mut().spawn((
            BoardItem,
            PendingObsidian(center),
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell: center,
            },
        ));
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: COATL_EGG_ITEM_INDEX,
                cell: adjacent,
            },
        ));
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(center));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 9]);
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(center), Some(Ground::Hole));
        assert_eq!(
            world.resource::<Board>().ground(adjacent),
            Some(Ground::Hole)
        );
        assert_eq!(
            world.resource::<Board>().ground(diagonal),
            Some(Ground::Solid)
        );
        assert!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(COATL_EGG_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY))
        );
    }

    #[test]
    fn stone_obsidian_explodes_before_creating_the_ridge_cross() {
        let mut app = interaction_app();
        let center = IVec2::new(6, 3);
        let diagonal = center + IVec2::ONE;
        move_player_to(&mut app, PlayerId::One, center);
        move_player_to(&mut app, PlayerId::Two, diagonal);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for x in -1..=1 {
                for y in -1..=1 {
                    if x != 0 || y != 0 {
                        board.set(center + IVec2::new(x, y), Ground::Ridge);
                    }
                }
            }
        }
        app.world_mut().spawn((
            BoardItem,
            PendingObsidian(center),
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell: center,
            },
        ));
        click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(center));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 9]);
        let world = app.world_mut();
        for x in -1..=1 {
            for y in -1..=1 {
                assert_eq!(
                    world.resource::<Board>().ground(center + IVec2::new(x, y)),
                    Some(if x == 0 || y == 0 {
                        Ground::Ridge
                    } else {
                        Ground::Solid
                    })
                );
            }
        }
        assert_eq!(world.query::<&FireFlash>().iter(world).count(), 9);
        assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert_eq!(
            world.resource::<TurnState>().ingredient_respawns,
            [(OBSIDIAN_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn stone_obsidian_blast_chains_before_the_ridges_form() {
        let mut app = interaction_app();
        let center = IVec2::new(6, 3);
        let ember = center + IVec2::new(-1, -1);
        let other_obsidian = center + IVec2::ONE;
        move_player_to(&mut app, PlayerId::Two, center + IVec2::new(1, -1));
        app.world_mut().resource_mut::<Board>().add_oil(ember);
        for cell in [center, other_obsidian] {
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(cell),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
            ));
        }
        click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(center));
        let world = app.world_mut();
        assert_eq!(world.resource::<PlayerHealth>().0, [8, 6]);
        assert!(!world.resource::<Board>().has_oil(ember));
        assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
        assert_eq!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .filter(|&&(index, _)| index == OBSIDIAN_ITEM_INDEX)
                .count(),
            2
        );
        assert_eq!(world.resource::<Board>().ground(ember), Some(Ground::Solid));
        assert_eq!(
            world.resource::<Board>().ground(other_obsidian),
            Some(Ground::Solid)
        );
        assert_eq!(
            world.resource::<Board>().ground(center + IVec2::X),
            Some(Ground::Ridge)
        );
        assert!(world.query::<&FireFlash>().iter(world).count() > 9);
    }

    #[test]
    fn obsidian_on_existing_stone_dust_also_explodes() {
        let mut app = interaction_app();
        let center = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::One, center);
        move_player_to(&mut app, PlayerId::Two, center + IVec2::ONE);
        app.world_mut().spawn((
            BoardItem,
            PendingStoneDust(center),
            Collectible {
                index: STONE_DUST_ITEM_INDEX,
                cell: center,
            },
        ));
        click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
        click(&mut app, cell_center(center));
        let world = app.world_mut();
        assert_eq!(world.resource::<PlayerHealth>().0, [8, 9]);
        assert_eq!(
            world.resource::<Board>().ground(center),
            Some(Ground::Ridge)
        );
        assert_eq!(world.query::<&FireFlash>().iter(world).count(), 9);
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert_eq!(
            world.resource::<TurnState>().ingredient_respawns,
            [(OBSIDIAN_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
    }

    #[test]
    fn obsidian_dust_crosses_are_clipped_at_board_edges() {
        for (index, expected) in [
            (VOID_DUST_ITEM_INDEX, Ground::Hole),
            (STONE_DUST_ITEM_INDEX, Ground::Ridge),
        ] {
            let mut app = interaction_app();
            let center = IVec2::ZERO;
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(center),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell: center,
                },
            ));
            move_player_to(&mut app, PlayerId::One, IVec2::new(1, 0));
            let inventory = p1_item(index);
            click(&mut app, inventory);
            click(&mut app, cell_center(center));
            let board = app.world().resource::<Board>();
            for cell in [center, IVec2::X, IVec2::Y] {
                assert_eq!(board.ground(cell), Some(expected), "{index}");
            }
            assert_eq!(board.ground(IVec2::ONE), Some(Ground::Solid));
        }
    }

    #[test]
    fn void_and_life_grow_then_create_only_holes() {
        for ground in [Ground::Solid, Ground::Water] {
            let mut app = interaction_app();
            let center = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            app.world_mut().resource_mut::<Board>().set(center, ground);
            click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
            click(&mut app, cell_center(center));
            click(&mut app, p2_item(VOID_DUST_ITEM_INDEX));
            click(&mut app, cell_center(center));
            for expected in [1, 2, 3, 4, 5] {
                let world = app.world_mut();
                assert_eq!(
                    world.query::<&VoidGrowthTile>().iter(world).count(),
                    expected
                );
                assert_eq!(world.resource::<Board>().ground(center), Some(ground));
                press_key(&mut app, KeyCode::KeyR);
            }
            let world = app.world_mut();
            assert_eq!(world.query::<&VoidGrowthTile>().iter(world).count(), 0);
            let board = world.resource::<Board>();
            let grounds: Vec<_> = (0..ROWS)
                .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
                .map(|cell| board.ground(cell).unwrap())
                .collect();
            assert_eq!(
                grounds
                    .iter()
                    .filter(|ground| **ground == Ground::Hole)
                    .count(),
                5
            );
            assert_eq!(
                grounds
                    .iter()
                    .filter(|ground| **ground == Ground::Water)
                    .count(),
                0
            );
            assert_eq!(board.ground(center), Some(Ground::Hole));
        }
    }

    #[test]
    fn void_reacts_with_active_life_on_water() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_life_essence(cell);
        }
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Water));
        assert!(!world.resource::<Board>().has_life_essence(cell));
        assert_eq!(world.query::<&VoidGrowthTile>().iter(world).count(), 1);
        let respawns = &world.resource::<TurnState>().ingredient_respawns;
        assert_eq!(
            respawns
                .iter()
                .filter(|(index, _)| *index == LIFE_ESSENCE_ITEM_INDEX)
                .count(),
            1
        );
        assert_eq!(
            respawns
                .iter()
                .filter(|(index, _)| *index == VOID_DUST_ITEM_INDEX)
                .count(),
            0
        );
    }

    #[test]
    fn sunstone_still_ignites_obsidian_when_void_dust_is_present() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: VOID_DUST_ITEM_INDEX,
                cell,
            },
            PendingVoidDust(cell),
        ));
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell,
            },
            PendingObsidian(cell),
        ));
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.query::<&FireFlash>().iter(world).count(), 9);
        assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 1);
        let respawns = &world.resource::<TurnState>().ingredient_respawns;
        for index in [SUNSTONE_ITEM_INDEX, OBSIDIAN_ITEM_INDEX] {
            assert_eq!(
                respawns.iter().filter(|(item, _)| *item == index).count(),
                1
            );
        }
    }

    #[test]
    fn void_obsidian_blast_chains_to_oil_and_other_obsidian() {
        let mut board = Board::default();
        let center = IVec2::new(4, 3);
        let oil = center + IVec2::X;
        let other = center + IVec2::new(1, 1);
        board.set(oil, Ground::Ridge);
        board.add_oil(oil);
        let result = resolve_blasts(&mut board, &[(None, other)], &[], None, Some(center));
        assert_eq!(board.ground(oil), Some(Ground::Solid));
        assert!(!board.has_oil(oil));
        assert_eq!(result.consumed_obsidian.len(), 1);
        assert_eq!(result.blasts.len(), 3);
    }

    #[test]
    fn void_has_no_special_reaction_to_other_ingredients() {
        for index in [
            WATER_ITEM_INDEX,
            THORNHEART_ITEM_INDEX,
            STORM_SHARD_ITEM_INDEX,
            OIL_ITEM_INDEX,
            ACID_ITEM_INDEX,
            POISON_ITEM_INDEX,
            11,
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().spawn((
                BoardItem,
                Collectible {
                    index: VOID_DUST_ITEM_INDEX,
                    cell,
                },
                PendingVoidDust(cell),
            ));
            let inventory = p1_item(index);
            click(&mut app, inventory);
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert_eq!(
                world.query::<&PendingVoidDust>().iter(world).count(),
                1,
                "{index}"
            );
            assert_ne!(world.resource::<Board>().ground(cell), Some(Ground::Hole));
            assert_eq!(world.query::<&VoidGrowthTile>().iter(world).count(), 0);
            assert_eq!(world.query::<&FireFlash>().iter(world).count(), 0);
        }
    }

    #[test]
    fn void_placement_on_water_without_a_reaction_is_rejected() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut().resource_mut::<Board>().make_water(cell);
        click(&mut app, p1_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Water));
        assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
        assert_eq!(world.resource::<TurnState>().actions_used, 0);
        assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
    }

    #[test]
    fn ineffective_dust_placements_are_rejected_without_cost() {
        for (index, ground) in [
            (VOID_DUST_ITEM_INDEX, Ground::Water),
            (STONE_DUST_ITEM_INDEX, Ground::Solid),
            (STONE_DUST_ITEM_INDEX, Ground::Ridge),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(8, 3);
            app.world_mut().resource_mut::<Board>().set(cell, ground);
            let inventory = p1_item(index);
            click_part(&mut app, inventory);
            click_part(&mut app, cell_center(cell));
            let world = app.world_mut();
            let turn = world.resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::One);
            assert_eq!(turn.actions_used, 0);
            assert_eq!(turn.strength[0], MAX_STRENGTH);
            assert!(turn.pending_placement.is_none());
            assert_eq!(world.query::<&Collectible>().iter(world).count(), 0);
            assert!(
                world
                    .query_filtered::<&Text, With<DropFeedback>>()
                    .single(world)
                    .unwrap()
                    .0
                    .contains("needs")
            );
        }
    }

    #[test]
    fn stone_dust_clears_poison_on_solid_ridge_and_water() {
        for (ground, frozen) in [
            (Ground::Solid, false),
            (Ground::Ridge, false),
            (Ground::Water, false),
            (Ground::Water, true),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(cell, ground);
                board.add_poison(cell);
                board.add_poison(cell);
                if frozen {
                    assert!(board.freeze_water(cell));
                }
            }
            click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            let board = world.resource::<Board>();
            assert!(!board.is_poisoned(cell));
            assert!(!board.is_frozen(cell));
            assert_eq!(
                board.ground(cell),
                Some(if ground == Ground::Water {
                    Ground::Solid
                } else {
                    ground
                })
            );
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
            assert_eq!(world.resource::<TurnState>().active, PlayerId::Two);
        }
    }

    #[test]
    fn dust_can_react_with_an_earlier_queued_item() {
        for (first, dust, ground) in [
            (OBSIDIAN_ITEM_INDEX, VOID_DUST_ITEM_INDEX, Ground::Water),
            (LIFE_ESSENCE_ITEM_INDEX, VOID_DUST_ITEM_INDEX, Ground::Water),
            (OBSIDIAN_ITEM_INDEX, STONE_DUST_ITEM_INDEX, Ground::Solid),
            (
                LIFE_ESSENCE_ITEM_INDEX,
                STONE_DUST_ITEM_INDEX,
                Ground::Solid,
            ),
            (ACID_ITEM_INDEX, STONE_DUST_ITEM_INDEX, Ground::Ridge),
            (POISON_ITEM_INDEX, STONE_DUST_ITEM_INDEX, Ground::Solid),
            (WATER_ITEM_INDEX, STONE_DUST_ITEM_INDEX, Ground::Solid),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<Board>().set(cell, ground);
            for index in [first, dust] {
                let inventory = p1_item(index);
                click_part(&mut app, inventory);
                click_part(&mut app, cell_center(cell));
            }
            let world = app.world_mut();
            assert_eq!(
                world.resource::<TurnState>().active,
                PlayerId::Two,
                "{first}, {dust}"
            );
            assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 0);
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
            let board = world.resource::<Board>();
            match (first, dust) {
                (OBSIDIAN_ITEM_INDEX, VOID_DUST_ITEM_INDEX) => {
                    assert_eq!(board.ground(cell), Some(Ground::Hole))
                }
                (OBSIDIAN_ITEM_INDEX, STONE_DUST_ITEM_INDEX) => {
                    assert_eq!(board.ground(cell), Some(Ground::Ridge))
                }
                (LIFE_ESSENCE_ITEM_INDEX, _) => assert_eq!(board.terrain_growth_cells().count(), 1),
                (ACID_ITEM_INDEX, _) => assert!(board.is_acidic(cell)),
                (POISON_ITEM_INDEX, _) => assert!(!board.is_poisoned(cell)),
                (WATER_ITEM_INDEX, _) => assert!(!board.is_water(cell)),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn second_dust_placement_is_checked_before_first_item_resolves() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_oil(cell);
        }
        for position in [p1_item(SUNSTONE_ITEM_INDEX), p1_item(STONE_DUST_ITEM_INDEX)] {
            click_part(&mut app, position);
            click_part(&mut app, cell_center(cell));
        }
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Water));
        assert!(world.resource::<Board>().has_oil(cell));
        assert_eq!(world.resource::<TurnState>().active, PlayerId::One);
        assert_eq!(world.resource::<TurnState>().actions_used, 1);
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert!(
            world
                .query_filtered::<&Text, With<DropFeedback>>()
                .single(world)
                .unwrap()
                .0
                .contains("Stone Dust needs")
        );
    }

    #[test]
    fn only_stone_dust_can_be_placed_in_a_hole() {
        for index in 0..INVENTORY_ITEMS.len() {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            app.world_mut()
                .resource_mut::<Board>()
                .set(cell, Ground::Hole);
            let inventory = p1_item(index);
            click(&mut app, inventory);
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            let turn = world.resource::<TurnState>();
            if index == STONE_DUST_ITEM_INDEX {
                assert_eq!(turn.active, PlayerId::Two);
                assert_eq!(turn.inventory[0][index], 1);
                assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Solid));
            } else {
                assert_eq!(turn.active, PlayerId::One, "{index}");
                assert_eq!(turn.strength[0], 5, "{index}");
                assert_eq!(turn.inventory[0][index], 1, "{index}");
                assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Hole));
                assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
                assert!(
                    world
                        .query_filtered::<&Text, With<DropFeedback>>()
                        .single(world)
                        .unwrap()
                        .0
                        .contains("Only Stone Dust")
                );
            }
        }
    }

    #[test]
    fn stone_dust_and_life_grow_only_ridges() {
        for ground in [Ground::Solid, Ground::Water] {
            let mut app = interaction_app();
            let center = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            app.world_mut().resource_mut::<Board>().set(center, ground);
            click(&mut app, p1_item(LIFE_ESSENCE_ITEM_INDEX));
            click(&mut app, cell_center(center));
            click(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
            click(&mut app, cell_center(center));
            {
                let world = app.world_mut();
                assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
                assert_eq!(world.query::<&PendingLifeEssence>().iter(world).count(), 0);
                let respawns = &world.resource::<TurnState>().ingredient_respawns;
                for index in [STONE_DUST_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX] {
                    assert_eq!(
                        respawns.iter().filter(|(item, _)| *item == index).count(),
                        usize::from(index == LIFE_ESSENCE_ITEM_INDEX)
                    );
                }
            }
            for expected in 1..=4 {
                assert_eq!(
                    app.world_mut()
                        .query::<&VoidGrowthTile>()
                        .iter(app.world())
                        .count(),
                    expected
                );
                press_key(&mut app, KeyCode::KeyR);
            }
            let cells: Vec<_> = app
                .world()
                .resource::<Board>()
                .terrain_growth_cells()
                .collect();
            assert_eq!(cells.len(), 5);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(cells[1]);
                board.add_acid(cells[1]);
                assert!(board.freeze_water(cells[1]));
                board.set(cells[3], Ground::Hole);
                board.set(cells[4], Ground::Hole);
            }
            press_key(&mut app, KeyCode::KeyR);
            let board = app.world().resource::<Board>();
            assert_eq!(board.ground(center), Some(Ground::Ridge));
            for cell in &cells[1..3] {
                assert_eq!(board.ground(*cell), Some(Ground::Ridge));
            }
            for cell in &cells[3..5] {
                assert_eq!(board.ground(*cell), Some(Ground::Ridge));
            }
            assert!(!board.is_frozen(cells[1]));
            for _ in 0..6 {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert!(!app.world().resource::<Board>().is_acidic(cells[1]));
        }
    }

    #[test]
    fn stone_dust_reacts_with_active_life_on_water() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_life_essence(cell);
        }
        click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Water));
        assert!(!world.resource::<Board>().has_life_essence(cell));
        assert_eq!(world.query::<&VoidGrowthTile>().iter(world).count(), 1);
        for index in [STONE_DUST_ITEM_INDEX, LIFE_ESSENCE_ITEM_INDEX] {
            assert_eq!(
                world
                    .resource::<TurnState>()
                    .ingredient_respawns
                    .iter()
                    .filter(|(item, _)| *item == index)
                    .count(),
                usize::from(index == LIFE_ESSENCE_ITEM_INDEX)
            );
        }
    }

    #[test]
    fn terrain_growth_displaces_players_without_magic_to_distinct_safe_tiles() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            let mut app = interaction_app();
            let center = IVec2::new(4, 3);
            app.world_mut()
                .resource_mut::<Board>()
                .start_terrain_growth(center, 7, kind);
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            let cells: Vec<_> = app
                .world()
                .resource::<Board>()
                .terrain_growth_cells()
                .collect();
            assert_eq!(cells.len(), 5);
            move_player_to(&mut app, PlayerId::One, cells[0]);
            move_player_to(&mut app, PlayerId::Two, cells[2]);
            app.world_mut().resource_mut::<TurnState>().magic = [0, 0];
            press_key(&mut app, KeyCode::KeyR);
            let world = app.world_mut();
            let positions: Vec<_> = world
                .query::<(&Player, &Transform)>()
                .iter(world)
                .map(|(player, transform)| {
                    (
                        player.0,
                        board_cell(transform.translation.truncate()).unwrap(),
                    )
                })
                .collect();
            for (player, cell) in &positions {
                let origin = if *player == PlayerId::One {
                    cells[0]
                } else {
                    cells[2]
                };
                assert_eq!((*cell - origin).abs().max_element(), 1);
                assert!(world.resource::<Board>().ground(*cell).unwrap().walkable());
            }
            assert_ne!(positions[0].1, positions[1].1);
            assert_eq!(world.resource::<PlayerHealth>().0, [9, 9]);
        }
    }

    #[test]
    fn new_unsafe_terrain_frees_directly_frozen_players_and_starts_hover() {
        for player in [PlayerId::One, PlayerId::Two] {
            for pending_freeze in [false, true] {
                let mut board = Board::default();
                let mut turn = TurnState::default();
                let mut health = PlayerHealth::default();
                let origin = PLAYER_STARTS[player.index()];
                assert!(board.make_hole(origin));
                if pending_freeze {
                    turn.pending_effects.frozen_players[player.index()] = 2;
                } else {
                    turn.frozen_turns[player.index()] = 2;
                }
                turn.finish_action(
                    &mut board,
                    &mut health,
                    PLAYER_STARTS.into_iter().enumerate().map(|(index, cell)| {
                        (
                            if index == 0 {
                                PlayerId::One
                            } else {
                                PlayerId::Two
                            },
                            cell_center(cell),
                        )
                    }),
                    TurnEffects::default(),
                );
                assert!(turn.hovering[player.index()]);
                assert_eq!(turn.frozen_turns[player.index()], 0);
                assert!(turn.terrain_displacements.is_empty());
                assert_eq!(health.0[player.index()], 10);
            }
        }
    }

    #[test]
    fn terrain_changes_free_a_player_frozen_by_a_direct_hit_on_dry_ground() {
        for (item, expected_ground) in [
            (VOID_DUST_ITEM_INDEX, Ground::Hole),
            (STONE_DUST_ITEM_INDEX, Ground::Ridge),
        ] {
            for magic in [0, 2] {
                let mut app = interaction_app();
                let cell = IVec2::new(6, 3);
                move_player_to(&mut app, PlayerId::Two, cell);
                click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
                click(&mut app, cell_center(cell));
                assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
                assert!(!app.world().resource::<Board>().is_frozen(cell));
                assert_eq!(
                    app.world_mut()
                        .query::<&FrostImpact>()
                        .iter(app.world())
                        .count(),
                    1
                );
                app.world_mut().resource_mut::<TurnState>().magic[1] = magic;
                if item == STONE_DUST_ITEM_INDEX {
                    app.world_mut().spawn((
                        BoardItem,
                        PendingObsidian(cell),
                        Collectible {
                            index: OBSIDIAN_ITEM_INDEX,
                            cell,
                        },
                    ));
                }
                let inventory = p2_item(item);
                click(&mut app, inventory);
                click(&mut app, cell_center(cell));

                let world = app.world_mut();
                assert_eq!(
                    world.resource::<Board>().ground(cell),
                    Some(expected_ground)
                );
                assert_eq!(world.resource::<TurnState>().frozen_turns[1], 0);
                assert_eq!(world.resource::<TurnState>().hovering[1], magic > 0);
                let explosion_damage = if item == STONE_DUST_ITEM_INDEX { 2 } else { 0 };
                assert_eq!(
                    world.resource::<PlayerHealth>().0[1],
                    10 - explosion_damage - u8::from(magic == 0)
                );
                let mut players = world.query::<(&Player, &Transform)>();
                let (_, transform) = players
                    .iter(world)
                    .find(|(player, _)| player.0 == PlayerId::Two)
                    .unwrap();
                let destination = board_cell(transform.translation.truncate()).unwrap();
                assert_eq!(destination == cell, magic > 0);
                if magic == 0 {
                    assert!(
                        world
                            .resource::<Board>()
                            .ground(destination)
                            .unwrap()
                            .walkable()
                    );
                }
                assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
            }
        }
    }

    #[test]
    fn animated_hole_frees_directly_frozen_player_without_moving_ice() {
        let mut app = interaction_app();
        app.init_resource::<effects::Effects>();
        app.update();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<TurnState>().effects_active);
        tick_coatl_animation(&mut app, 1.0);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
        assert_eq!(
            app.world_mut()
                .query::<&FrostImpact>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<TurnState>().magic[1] = 0;
        click(&mut app, p2_item(VOID_DUST_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<TurnState>().effects_active);
        tick_coatl_animation(&mut app, 1.0);
        let world = app.world_mut();
        assert_eq!(world.resource::<Board>().ground(cell), Some(Ground::Hole));
        assert_eq!(world.resource::<TurnState>().frozen_turns[1], 0);
        assert_eq!(world.resource::<PlayerHealth>().0[1], 9);
        assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
        let mut players = world.query::<(&Player, &Transform)>();
        let (_, transform) = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::Two)
            .unwrap();
        assert_ne!(board_cell(transform.translation.truncate()), Some(cell));
    }

    #[test]
    fn animated_terrain_explosion_clears_ice_and_frees_its_occupant() {
        for (item, expected_ground) in [
            (VOID_DUST_ITEM_INDEX, Ground::Hole),
            (STONE_DUST_ITEM_INDEX, Ground::Ridge),
        ] {
            let mut app = interaction_app();
            app.init_resource::<effects::Effects>();
            app.update();
            let cell = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, cell);
            app.world_mut().resource_mut::<Board>().make_water(cell);
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(cell),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
            ));
            app.update();
            tick_coatl_animation(&mut app, 1.0);
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            tick_coatl_animation(&mut app, 1.0);
            assert!(app.world().resource::<Board>().is_frozen(cell));
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
            assert_eq!(frozen_sprite_count(&mut app), 1);
            app.world_mut().resource_mut::<TurnState>().magic[1] = 0;
            let inventory = p2_item(item);
            click(&mut app, inventory);
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<TurnState>().effects_active);
            tick_coatl_animation(&mut app, 1.0);
            tick_coatl_animation(&mut app, 1.0);
            let world = app.world_mut();
            assert_eq!(
                world.resource::<Board>().ground(cell),
                Some(expected_ground)
            );
            assert!(!world.resource::<Board>().is_frozen(cell));
            assert_eq!(world.resource::<TurnState>().frozen_turns[1], 0);
            assert_eq!(world.resource::<PlayerHealth>().0[1], 7);
            assert_eq!(world.query::<&FrozenTile>().iter(world).count(), 0);
            assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
            let mut players = world.query::<(&Player, &Transform)>();
            let (_, transform) = players
                .iter(world)
                .find(|(player, _)| player.0 == PlayerId::Two)
                .unwrap();
            assert_ne!(board_cell(transform.translation.truncate()), Some(cell));
        }
    }

    #[test]
    fn unsafe_terrain_thaws_frozen_occupants_before_hover_or_displacement() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            for player in [PlayerId::One, PlayerId::Two] {
                for acting in [false, true] {
                    for magic in [0, 1, 2] {
                        for blast_thaws_first in [false, true] {
                            let mut board = Board::default();
                            let mut turn = TurnState::default();
                            let mut health = PlayerHealth::default();
                            let origin = PLAYER_STARTS[player.index()];
                            turn.active = if acting { player } else { player.other() };
                            turn.magic[player.index()] = magic;
                            turn.frozen_turns[player.index()] = 2;
                            turn.pending_effects.frozen_players[player.index()] = 2;
                            board.make_water(origin);
                            assert!(board.freeze_water(origin));
                            if blast_thaws_first {
                                board.thaw(origin);
                            }
                            match kind {
                                TerrainGrowthKind::Void => assert!(board.make_hole(origin)),
                                TerrainGrowthKind::Stone => assert!(board.make_ridge(origin)),
                            }
                            turn.finish_action(
                                &mut board,
                                &mut health,
                                PLAYER_STARTS.into_iter().enumerate().map(|(index, cell)| {
                                    (
                                        if index == 0 {
                                            PlayerId::One
                                        } else {
                                            PlayerId::Two
                                        },
                                        cell_center(cell),
                                    )
                                }),
                                TurnEffects::default(),
                            );

                            let can_hover = magic > u8::from(acting);
                            assert!(!board.is_frozen(origin));
                            assert_eq!(turn.frozen_turns[player.index()], 0);
                            assert_eq!(turn.hovering[player.index()], can_hover);
                            assert_eq!(health.0[player.index()], if can_hover { 10 } else { 9 });
                            let destination = turn
                                .terrain_displacements
                                .iter()
                                .find_map(|&(owner, cell)| (owner == player).then_some(cell));
                            assert_eq!(destination.is_none(), can_hover);
                            if let Some(destination) = destination {
                                assert!(board.ground(destination).unwrap().walkable());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn new_hole_frees_player_even_if_tile_was_thawed_in_an_earlier_turn() {
        let mut board = Board::default();
        let mut turn = TurnState::default();
        let mut health = PlayerHealth::default();
        let origin = PLAYER_STARTS[1];
        let positions = || {
            PLAYER_STARTS.into_iter().enumerate().map(|(index, cell)| {
                (
                    if index == 0 {
                        PlayerId::One
                    } else {
                        PlayerId::Two
                    },
                    cell_center(cell),
                )
            })
        };
        board.make_water(origin);
        assert!(board.freeze_water(origin));
        board.thaw(origin);
        turn.finish_action(&mut board, &mut health, positions(), TurnEffects::default());
        turn.frozen_turns[1] = 2;
        assert!(board.make_hole(origin));
        turn.finish_action(&mut board, &mut health, positions(), TurnEffects::default());
        assert!(turn.hovering[1]);
        assert_eq!(turn.frozen_turns[1], 0);
        assert!(turn.terrain_displacements.is_empty());
        assert_eq!(health.0[1], 10);
    }

    #[test]
    fn unsafe_terrain_removes_frozen_tile_art_after_its_effect() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            let mut app = interaction_app();
            app.init_resource::<effects::Effects>();
            app.update();
            let cell = IVec2::new(4, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(cell);
                assert!(board.freeze_water(cell));
            }
            app.world_mut().spawn((
                FrozenTile(cell),
                Sprite::default(),
                Visibility::Inherited,
                Transform::from_translation(cell_center(cell).extend(-0.5)),
            ));
            app.update();
            tick_coatl_animation(&mut app, 1.0);
            assert_eq!(frozen_sprite_count(&mut app), 1);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                match kind {
                    TerrainGrowthKind::Void => assert!(board.make_hole(cell)),
                    TerrainGrowthKind::Stone => assert!(board.make_ridge(cell)),
                }
            }
            app.update();
            assert!(app.world().resource::<TurnState>().effects_active);
            tick_coatl_animation(&mut app, 1.0);
            assert!(!app.world().resource::<Board>().is_frozen(cell));
            assert_eq!(frozen_sprite_count(&mut app), 0);
        }
    }

    #[test]
    fn obsidian_terrain_explosion_frees_a_player_frozen_on_ice() {
        for (item, expected_ground) in [
            (VOID_DUST_ITEM_INDEX, Ground::Hole),
            (STONE_DUST_ITEM_INDEX, Ground::Ridge),
        ] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, cell);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(cell);
                assert!(board.freeze_water(cell));
            }
            app.world_mut().resource_mut::<TurnState>().frozen_turns[1] = 2;
            app.world_mut().spawn((
                FrozenTile(cell),
                Sprite::default(),
                Visibility::Inherited,
                Transform::from_translation(cell_center(cell).extend(-0.5)),
            ));
            app.update();
            assert_eq!(frozen_sprite_count(&mut app), 1);
            assert_eq!(
                app.world_mut()
                    .query::<&FrostImpact>()
                    .iter(app.world())
                    .count(),
                1
            );
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(cell),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
            ));
            let inventory = p1_item(item);
            click(&mut app, inventory);
            click(&mut app, cell_center(cell));
            let world = app.world_mut();
            assert_eq!(
                world.resource::<Board>().ground(cell),
                Some(expected_ground)
            );
            assert!(!world.resource::<Board>().is_frozen(cell));
            assert_eq!(world.resource::<TurnState>().frozen_turns[1], 0);
            assert!(world.resource::<TurnState>().hovering[1]);
            assert!(
                world
                    .resource::<TurnState>()
                    .terrain_displacements
                    .is_empty()
            );
            let mut players = world.query::<(&Player, &Transform)>();
            let (_, transform) = players
                .iter(world)
                .find(|(player, _)| player.0 == PlayerId::Two)
                .unwrap();
            assert_eq!(board_cell(transform.translation.truncate()), Some(cell));
            assert_eq!(world.query::<&FrozenTile>().iter(world).count(), 0);
            assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
        }
    }

    #[test]
    fn terrain_growth_activates_or_preserves_hover_when_magic_is_sufficient() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            for acting in [false, true] {
                for magic in [0, 1, 2] {
                    for already_hovering in [false, true] {
                        if magic == 0 && already_hovering {
                            continue;
                        }
                        let mut app = interaction_app();
                        app.world_mut()
                            .resource_mut::<Board>()
                            .start_terrain_growth(IVec2::new(4, 3), 7, kind);
                        for _ in 0..5 {
                            press_key(&mut app, KeyCode::KeyR);
                        }
                        let origin = app
                            .world()
                            .resource::<Board>()
                            .terrain_growth_cells()
                            .nth(1)
                            .unwrap();
                        let active = app.world().resource::<TurnState>().active;
                        let player = if acting { active } else { active.other() };
                        move_player_to(&mut app, player, origin);
                        move_player_to(&mut app, player.other(), IVec2::ZERO);
                        {
                            let mut turn = app.world_mut().resource_mut::<TurnState>();
                            turn.magic[player.index()] = magic;
                            turn.hovering[player.index()] = already_hovering;
                        }
                        press_key(&mut app, KeyCode::KeyR);
                        let world = app.world_mut();
                        let position = world
                            .query::<(&Player, &Transform)>()
                            .iter(world)
                            .find(|(candidate, _)| candidate.0 == player)
                            .map(|(_, transform)| {
                                board_cell(transform.translation.truncate()).unwrap()
                            })
                            .unwrap();
                        let turn = world.resource::<TurnState>();
                        let can_hover = magic > u8::from(acting);
                        assert_eq!(
                            position == origin,
                            can_hover,
                            "acting={acting} magic={magic} hovering={already_hovering}"
                        );
                        assert_eq!(
                            turn.hovering[player.index()],
                            can_hover,
                            "acting={acting} magic={magic} hovering={already_hovering}"
                        );
                        if can_hover {
                            assert_eq!(turn.magic[player.index()], magic - u8::from(acting));
                            assert!(
                                !world
                                    .resource::<Board>()
                                    .ground(position)
                                    .unwrap()
                                    .walkable()
                            );
                        } else {
                            assert!(
                                world
                                    .resource::<Board>()
                                    .ground(position)
                                    .unwrap()
                                    .walkable()
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn new_unsafe_terrain_activates_hover_without_adjacent_safe_exits() {
        let mut app = interaction_app();
        let origin = IVec2::new(5, 3);
        move_player_to(&mut app, PlayerId::One, origin);
        app.world_mut().resource_mut::<TurnState>().magic[0] = 2;
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for y in -1..=1 {
                for x in -1..=1 {
                    if x != 0 || y != 0 {
                        board.set(origin + IVec2::new(x, y), Ground::Ridge);
                    }
                }
            }
            assert!(board.make_hole(origin));
        }
        press_key(&mut app, KeyCode::KeyR);
        let world = app.world_mut();
        let position = world
            .query::<(&Player, &Transform)>()
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .map(|(_, transform)| board_cell(transform.translation.truncate()).unwrap())
            .unwrap();
        assert_eq!(position, origin);
        assert!(world.resource::<TurnState>().hovering[0]);
        assert_eq!(world.resource::<TurnState>().magic[0], 1);
    }

    #[test]
    fn terrain_displacement_searches_beyond_blocked_adjacent_tiles() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            let mut app = interaction_app();
            app.world_mut()
                .resource_mut::<Board>()
                .start_terrain_growth(IVec2::new(4, 3), 7, kind);
            for _ in 0..5 {
                press_key(&mut app, KeyCode::KeyR);
            }
            let cells: Vec<_> = app
                .world()
                .resource::<Board>()
                .terrain_growth_cells()
                .collect();
            let origin = cells[1];
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                for y in -1..=1 {
                    for x in -1..=1 {
                        let cell = origin + IVec2::new(x, y);
                        if cell != origin
                            && board.ground(cell).is_some()
                            && !cells[3..].contains(&cell)
                        {
                            board.set(cell, Ground::Ridge);
                        }
                    }
                }
                // The safe arm must not create an adjacent landing tile in this test.
                for cell in &cells[3..] {
                    if (*cell - origin).abs().max_element() == 1 {
                        board.set(*cell, Ground::Solid);
                    }
                }
            }
            // Occupy an adjacent safe-arm tile if growth creates one.
            let adjacent_safe_arm = cells[3..]
                .iter()
                .copied()
                .find(|cell| (*cell - origin).abs().max_element() == 1);
            move_player_to(&mut app, PlayerId::One, origin);
            move_player_to(
                &mut app,
                PlayerId::Two,
                adjacent_safe_arm.unwrap_or(IVec2::ZERO),
            );
            app.world_mut().resource_mut::<TurnState>().magic[0] = 0;
            press_key(&mut app, KeyCode::KeyR);
            let world = app.world_mut();
            let position = world
                .query::<(&Player, &Transform)>()
                .iter(world)
                .find(|(player, _)| player.0 == PlayerId::One)
                .map(|(_, transform)| board_cell(transform.translation.truncate()).unwrap())
                .unwrap();
            assert_eq!((position - origin).abs().max_element(), 2);
            assert!(
                world
                    .resource::<Board>()
                    .ground(position)
                    .unwrap()
                    .walkable()
            );
        }
    }

    #[test]
    fn unsafe_terrain_limit_counts_holes_and_ridges_together() {
        let mut board = Board::default();
        let cells: Vec<_> = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .collect();
        for (index, cell) in cells[..board::MAX_UNSAFE_TILES - 1].iter().enumerate() {
            board.set(
                *cell,
                if index % 2 == 0 {
                    Ground::Hole
                } else {
                    Ground::Ridge
                },
            );
        }
        let last = cells[board::MAX_UNSAFE_TILES - 1];
        let blocked = cells[board::MAX_UNSAFE_TILES];
        assert!(board.make_hole(last));
        board.add_oil(blocked);
        assert!(!board.make_hole(blocked));
        assert_eq!(board.ground(blocked), Some(Ground::Solid));
        assert!(board.has_oil(blocked));
        assert!(!board.make_ridge(blocked));
        assert!(board.make_ridge(last));
        board.make_water(cells[0]);
        assert!(board.make_ridge(blocked));
    }

    #[test]
    fn displaced_player_receives_destination_tile_effects() {
        for already_hovering in [false, true] {
            let mut app = interaction_app();
            let origin = IVec2::new(5, 3);
            let destination = origin + IVec2::X;
            move_player_to(&mut app, PlayerId::One, origin);
            {
                let mut turn = app.world_mut().resource_mut::<TurnState>();
                turn.magic[0] = u8::from(already_hovering);
                turn.hovering[0] = already_hovering;
            }
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                for y in -1..=1 {
                    for x in -1..=1 {
                        let cell = origin + IVec2::new(x, y);
                        if cell != origin && cell != destination {
                            board.set(cell, Ground::Ridge);
                        }
                    }
                }
                board.add_acid(destination);
                assert!(board.make_hole(origin));
            }
            let positions: Vec<_> = {
                let world = app.world_mut();
                let mut players = world.query::<(&Player, &Transform)>();
                players
                    .iter(world)
                    .map(|(player, transform)| (player.0, transform.translation.truncate()))
                    .collect()
            };
            app.world_mut()
                .resource_scope(|world, mut turn: Mut<TurnState>| {
                    world.resource_scope(|world, mut board: Mut<Board>| {
                        let mut health = world.resource_mut::<PlayerHealth>();
                        turn.finish_action(
                            &mut board,
                            &mut health,
                            positions,
                            TurnEffects::default(),
                        );
                    });
                });
            app.update();
            let world = app.world_mut();
            let position = world
                .query::<(&Player, &Transform)>()
                .iter(world)
                .find(|(player, _)| player.0 == PlayerId::One)
                .map(|(_, transform)| board_cell(transform.translation.truncate()).unwrap())
                .unwrap();
            assert_eq!(position, destination);
            assert!(!world.resource::<TurnState>().hovering[0]);
            assert_eq!(world.resource::<PlayerHealth>().0[0], 8);
        }
    }

    #[test]
    fn terrain_displacement_onto_placed_frost_preserves_destination_hazards() {
        let mut app = interaction_app();
        let origin = IVec2::new(5, 3);
        let destination = origin + IVec2::X;
        app.world_mut().resource_mut::<TurnState>().magic[0] = 0;
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for y in -1..=1 {
                for x in -1..=1 {
                    let cell = origin + IVec2::new(x, y);
                    if cell != origin && cell != destination {
                        board.set(cell, Ground::Ridge);
                    }
                }
            }
            board.add_acid(destination);
            assert!(board.make_hole(origin));
        }
        app.world_mut().spawn((
            BoardItem,
            PlacedItem,
            PendingFrost(destination),
            Collectible {
                index: FROST_CRYSTAL_ITEM_INDEX,
                cell: destination,
            },
        ));
        let positions: Vec<_> = {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &Transform)>();
            players
                .iter(world)
                .map(|(player, transform)| (player.0, transform.translation.truncate()))
                .collect()
        };
        app.world_mut()
            .resource_scope(|world, mut turn: Mut<TurnState>| {
                world.resource_scope(|world, mut board: Mut<Board>| {
                    let mut health = world.resource_mut::<PlayerHealth>();
                    turn.finish_action(&mut board, &mut health, positions, TurnEffects::default());
                });
            });
        app.update();
        let world = app.world_mut();
        let player_cell = world
            .query::<(&Player, &Transform)>()
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .and_then(|(_, transform)| board_cell(transform.translation.truncate()));
        assert_eq!(player_cell, Some(destination));
        assert_eq!(world.resource::<PlayerHealth>().0[0], 8);
        assert_eq!(world.resource::<TurnState>().frozen_turns[0], 2);
        assert_eq!(
            world.resource::<TurnState>().ingredient_respawns,
            [(FROST_CRYSTAL_ITEM_INDEX, INGREDIENT_RESPAWN_DELAY)]
        );
        assert_eq!(world.query::<&PendingFrost>().iter(world).count(), 0);
    }

    #[test]
    fn terrain_growth_respects_the_cap_on_both_arms() {
        for kind in [TerrainGrowthKind::Void, TerrainGrowthKind::Stone] {
            for free_slot in [false, true] {
                let mut board = Board::default();
                board.start_terrain_growth(IVec2::new(4, 3), 7, kind);
                for _ in 0..5 {
                    board.advance_terrain_growth();
                }
                let cells: Vec<_> = board.terrain_growth_cells().collect();
                let mut count = 0;
                let limit = board::MAX_UNSAFE_TILES - usize::from(free_slot);
                for y in 0..ROWS {
                    for x in 0..COLUMNS {
                        let cell = IVec2::new(x as i32, y as i32);
                        if !cells.contains(&cell) && count < limit {
                            board.set(cell, Ground::Ridge);
                            count += 1;
                        }
                    }
                }
                assert_eq!(count, limit);
                board.advance_terrain_growth();
                assert_eq!(
                    board.ground(cells[1]),
                    Some(if free_slot {
                        match kind {
                            TerrainGrowthKind::Void => Ground::Hole,
                            TerrainGrowthKind::Stone => Ground::Ridge,
                        }
                    } else {
                        Ground::Solid
                    })
                );
                assert_eq!(board.ground(cells[2]), Some(Ground::Solid));
                for cell in &cells[3..5] {
                    assert_eq!(board.ground(*cell), Some(Ground::Solid));
                }
                assert_eq!(board.ground(cells[0]), Some(Ground::Solid));
                assert!(board.take_terrain_events().1);
            }

            let mut board = Board::default();
            board.start_terrain_growth(IVec2::new(4, 3), 7, kind);
            for _ in 0..5 {
                board.advance_terrain_growth();
            }
            let cells: Vec<_> = board.terrain_growth_cells().collect();
            board.set(cells[3], Ground::Ridge);
            board.set(cells[4], Ground::Hole);
            let mut count = 2;
            for y in 0..ROWS {
                for x in 0..COLUMNS {
                    let cell = IVec2::new(x as i32, y as i32);
                    if !cells.contains(&cell) && count < board::MAX_UNSAFE_TILES {
                        board.set(cell, Ground::Ridge);
                        count += 1;
                    }
                }
            }
            board.advance_terrain_growth();
            for cell in &cells[1..3] {
                assert_eq!(board.ground(*cell), Some(Ground::Solid));
            }
            assert_eq!(board.ground(cells[0]), Some(Ground::Solid));
            for cell in &cells[3..5] {
                assert_eq!(
                    board.ground(*cell),
                    Some(match kind {
                        TerrainGrowthKind::Void => Ground::Hole,
                        TerrainGrowthKind::Stone => Ground::Ridge,
                    })
                );
            }
            assert!(board.take_terrain_events().1);
        }
    }

    #[test]
    fn immediate_rift_and_ridge_are_rejected_at_terrain_limit_without_spending() {
        for ridge in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                let mut count = 0;
                for y in 0..ROWS {
                    for x in 0..COLUMNS {
                        let cell = IVec2::new(x as i32, y as i32);
                        if cell != target
                            && !PLAYER_STARTS.contains(&cell)
                            && count < board::MAX_UNSAFE_TILES
                        {
                            board.set(
                                cell,
                                if count % 2 == 0 {
                                    Ground::Hole
                                } else {
                                    Ground::Ridge
                                },
                            );
                            count += 1;
                        }
                    }
                }
            }
            if ridge {
                app.world_mut().spawn((
                    BoardItem,
                    Collectible {
                        index: STONE_DUST_ITEM_INDEX,
                        cell: target,
                    },
                    PendingStoneDust(target),
                ));
            } else {
                app.world_mut().spawn((
                    BoardItem,
                    Collectible {
                        index: VOID_DUST_ITEM_INDEX,
                        cell: target,
                    },
                    PendingVoidDust(target),
                ));
            }
            let index = OBSIDIAN_ITEM_INDEX;
            let inventory = p1_item(index);
            click(&mut app, inventory);
            click(&mut app, cell_center(target));
            let world = app.world_mut();
            assert_eq!(
                world.resource::<Board>().ground(target),
                Some(Ground::Solid)
            );
            let turn = world.resource::<TurnState>();
            assert_eq!(turn.active, PlayerId::One);
            assert_eq!(turn.inventory[0][index], 1);
            assert_eq!(turn.strength[0], 5);
            assert!(turn.ingredient_respawns.is_empty());
            assert_eq!(world.query::<&FireFlash>().iter(world).count(), 0);
            assert_eq!(world.resource::<PlayerHealth>().0, [10, 10]);
            assert!(
                world
                    .query_filtered::<&Text, With<DropFeedback>>()
                    .single(world)
                    .unwrap()
                    .0
                    .contains("60%")
            );
        }
    }

    #[test]
    fn stone_dust_fills_holes_and_replaces_frozen_natural_water() {
        for ground in [Ground::Hole, Ground::Water] {
            let mut app = interaction_app();
            app.add_systems(Startup, spawn_board_terrain);
            let target = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(target, ground);
                if ground == Ground::Water {
                    assert!(board.freeze_water(target));
                }
            }
            click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
            click(&mut app, cell_center(target));
            let world = app.world_mut();
            let board = world.resource::<Board>();
            assert_eq!(board.ground(target), Some(Ground::Solid));
            assert!(!board.is_water(target));
            assert!(!board.is_frozen(target));
            assert!(!board.has_life_essence(target));
            let sprite = world
                .query::<(&GroundTile, &Sprite)>()
                .iter(world)
                .find(|(tile, _)| tile.0 == target)
                .unwrap()
                .1;
            assert_eq!(sprite.color, Ground::Solid.color());
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
            let respawns = &world.resource::<TurnState>().ingredient_respawns;
            assert!(
                !respawns
                    .iter()
                    .any(|(index, _)| *index == STONE_DUST_ITEM_INDEX)
            );
            assert!(!respawns.contains(&(LIFE_ESSENCE_ITEM_INDEX, 5)));
        }
    }

    #[test]
    fn stone_dust_does_not_cancel_void_dust_on_water() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().resource_mut::<Board>().make_water(target);
        app.world_mut().spawn((
            BoardItem,
            PendingVoidDust(target),
            Collectible {
                index: VOID_DUST_ITEM_INDEX,
                cell: target,
            },
        ));
        click(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let world = app.world_mut();
        assert_eq!(
            world.resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingVoidDust>().iter(world).count(), 1);
        assert!(world.resource::<TurnState>().ingredient_respawns.is_empty());
    }

    #[test]
    fn stone_dust_does_not_react_to_sunstone_or_wind() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click_part(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingSunstone>().iter(world).count(), 1);
        assert_eq!(world.resource::<TurnState>().actions_used, 0);
        assert_eq!(
            world.resource::<Board>().ground(target),
            Some(Ground::Solid)
        );
    }

    #[test]
    fn water_still_electrifies_before_stone_dust_drains_it() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        app.world_mut().spawn((
            BoardItem,
            PendingStormShard(target),
            Collectible {
                index: STORM_SHARD_ITEM_INDEX,
                cell: target,
            },
        ));
        move_player_to(&mut app, PlayerId::One, target);
        click_part(&mut app, p1_item(WATER_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        click_part(&mut app, p1_item(STONE_DUST_ITEM_INDEX));
        click_part(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 8);
        assert!(!app.world().resource::<Board>().is_water(target));
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingStormShard>().iter(world).count(), 0);
        assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
    }

    #[test]
    fn stone_dust_on_obsidian_creates_a_visible_ridge_cross() {
        for has_life in [false, true] {
            let mut app = interaction_app();
            app.add_systems(Startup, spawn_board_terrain);
            let target = IVec2::new(6, 3);
            if has_life {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(target, Ground::Water);
                board.add_life_essence(target);
                assert!(board.freeze_water(target));
            }
            click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
            click(&mut app, cell_center(target));
            click(&mut app, p2_item(STONE_DUST_ITEM_INDEX));
            click(&mut app, cell_center(target));
            let world = app.world_mut();
            assert_eq!(
                world.resource::<Board>().ground(target),
                Some(Ground::Ridge)
            );
            for offset in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
                assert_eq!(
                    world.resource::<Board>().ground(target + offset),
                    Some(Ground::Ridge)
                );
            }
            assert_eq!(
                world.resource::<Board>().ground(target + IVec2::ONE),
                Some(Ground::Solid)
            );
            assert!(!world.resource::<Board>().is_frozen(target));
            assert!(!world.resource::<Board>().has_life_essence(target));
            let sprite = world
                .query::<(&GroundTile, &Sprite)>()
                .iter(world)
                .find(|(tile, _)| tile.0 == target)
                .unwrap()
                .1;
            assert_eq!(sprite.color, Ground::Ridge.color());
            assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
            let respawns = &world.resource::<TurnState>().ingredient_respawns;
            assert!(respawns.contains(&(OBSIDIAN_ITEM_INDEX, 5)));
            assert!(
                !respawns
                    .iter()
                    .any(|(index, _)| *index == STONE_DUST_ITEM_INDEX)
            );
            assert_eq!(respawns.contains(&(LIFE_ESSENCE_ITEM_INDEX, 5)), has_life);
            for _ in 0..7 {
                press_key(&mut app, KeyCode::KeyR);
            }
            let world = app.world_mut();
            assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 1);
            assert_eq!(world.query::<&PendingStoneDust>().iter(world).count(), 0);
        }
    }

    #[test]
    fn electricity_reaches_obsidian_through_connected_water_and_nets_damage() {
        let mut app = interaction_app();
        let obsidian = IVec2::new(6, 3);
        let water = IVec2::new(7, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(obsidian, Ground::Water);
            board.set(water, Ground::Water);
            board.add_life_essence(obsidian);
        }
        move_player_to(&mut app, PlayerId::Two, IVec2::new(8, 5));
        click(&mut app, p1_item(OBSIDIAN_ITEM_INDEX));
        click(&mut app, cell_center(obsidian));
        move_player_to(&mut app, PlayerId::One, obsidian);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        click(&mut app, p2_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(water));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [6, 7]);
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
        assert!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(OBSIDIAN_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn oil_and_obsidian_chain_explosions_once_each() {
        let mut app = interaction_app();
        let oil = IVec2::new(6, 3);
        let first = IVec2::new(7, 3);
        let second = IVec2::new(8, 3);
        let final_oil = IVec2::new(9, 3);
        move_player_to(&mut app, PlayerId::Two, first);
        for cell in [first, second] {
            app.world_mut().spawn((
                BoardItem,
                PendingObsidian(cell),
                Collectible {
                    index: OBSIDIAN_ITEM_INDEX,
                    cell,
                },
            ));
        }
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_oil(oil);
            board.add_oil(final_oil);
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(oil));
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingObsidian>().iter(world).count(), 0);
        assert!(!world.resource::<Board>().has_oil(final_oil));
        assert_eq!(
            world
                .resource::<TurnState>()
                .ingredient_respawns
                .iter()
                .filter(|(index, _)| *index == OBSIDIAN_ITEM_INDEX)
                .count(),
            2
        );
        assert_eq!(world.resource::<PlayerHealth>().0[1], 3);
    }

    #[test]
    fn obsidian_explosion_removes_thorns_in_its_footprint() {
        let mut app = interaction_app();
        let obsidian = IVec2::new(6, 3);
        let thorn = IVec2::new(8, 3);
        app.world_mut().spawn((
            BoardItem,
            PendingObsidian(obsidian),
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell: obsidian,
            },
        ));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.activate_thornheart(thorn, 1, true);
            board.finish_thorn_turn();
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(obsidian));
        assert!(!app.world().resource::<Board>().has_thorns(thorn));
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(THORNHEART_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn sunstone_ignites_connected_oil_once_and_stacks_blast_damage() {
        let mut app = interaction_app();
        let burning = [
            IVec2::new(6, 3),
            IVec2::new(7, 3),
            IVec2::new(7, 4),
            IVec2::new(8, 4),
        ];
        let disconnected = IVec2::new(8, 6);
        move_player_to(&mut app, PlayerId::Two, burning[1]);
        app.world_mut().resource_mut::<TurnState>().hovering[1] = true;
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for cell in burning.into_iter().chain([disconnected]) {
                board.add_oil(cell);
            }
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(burning[0]));
        let board = app.world().resource::<Board>();
        assert!(burning.iter().all(|cell| !board.has_oil(*cell)));
        assert!(board.has_oil(disconnected));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 5]);
        assert!(steam_positions(&mut app).is_empty());
        let world = app.world_mut();
        let mut flashes = world.query_filtered::<(&Transform, &Sprite), With<FireFlash>>();
        let cells: Vec<_> = flashes
            .iter(world)
            .map(|(transform, _)| board_cell(transform.translation.truncate()).unwrap())
            .collect();
        let mut unique = cells.clone();
        unique.sort_unstable_by_key(|cell| (cell.x, cell.y));
        unique.dedup();
        assert_eq!(cells.len(), 4 * 9);
        assert!(cells.len() > unique.len());
        for source in burning {
            let (_, sprite) = flashes
                .iter(world)
                .find(|(transform, _)| board_cell(transform.translation.truncate()) == Some(source))
                .unwrap();
            assert_eq!(sprite.color, Color::srgba(1.0, 0.35, 0.0, 0.65));
        }
    }

    #[test]
    fn connected_oil_drains_wet_ground_and_thaws_all_frozen_tiles() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let frozen = [IVec2::new(7, 3), IVec2::new(7, 4)];
        let wet = IVec2::new(8, 4);
        let natural = IVec2::new(9, 5);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for cell in [source, frozen[0], frozen[1], wet, natural] {
                board.add_oil(cell);
            }
            for cell in frozen {
                board.make_wet(cell);
                assert!(board.freeze_water(cell));
            }
            board.make_wet(wet);
            board.add_life_essence(wet);
            board.set(natural, Ground::Water);
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(source));
        let board = app.world().resource::<Board>();
        for cell in frozen {
            assert!(!board.is_frozen(cell));
            assert_eq!(board.water_percentage(cell), 100);
        }
        assert_eq!(board.water_percentage(wet), 0);
        assert!(!board.has_life_essence(wet));
        assert_eq!(board.ground(natural), Some(Ground::Solid));
        assert!(!board.has_oil(natural));
        assert!(steam_positions(&mut app).is_empty());
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
        );
        press_key(&mut app, KeyCode::KeyR);
        for cell in frozen {
            assert_eq!(app.world().resource::<Board>().water_percentage(cell), 80);
        }
    }

    #[test]
    fn oil_added_to_placed_sunstone_ignites_other_connected_oil() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let adjacent = IVec2::new(7, 3);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(source));
        click(&mut app, p2_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(adjacent));
        assert!(app.world().resource::<Board>().has_oil(adjacent));
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert!(!app.world().resource::<Board>().has_oil(source));
        assert!(!app.world().resource::<Board>().has_oil(adjacent));
        let world = app.world_mut();
        assert_eq!(world.query::<&PendingSunstone>().iter(world).count(), 0);
    }

    #[test]
    fn connected_blast_damage_is_netted_with_healing() {
        let mut app = interaction_app();
        let source = IVec2::new(6, 3);
        let adjacent = IVec2::new(7, 3);
        let healing = IVec2::new(7, 4);
        move_player_to(&mut app, PlayerId::Two, healing);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.add_oil(source);
            board.add_oil(adjacent);
            board.set(healing, Ground::Water);
            board.add_life_essence(healing);
        }
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 1;
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(source));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 0]);
    }

    #[test]
    fn sunstone_on_oily_water_ignites_without_steam() {
        for natural_water in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if natural_water {
                    board.set(target, Ground::Water);
                } else {
                    board.make_wet(target);
                }
                board.add_oil(target);
            }
            click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
            assert!(!app.world().resource::<Board>().has_oil(target));
            assert_eq!(app.world().resource::<Board>().water_percentage(target), 0);
            assert!(!app.world().resource::<Board>().is_water(target));
            assert!(steam_positions(&mut app).is_empty());
        }
    }

    #[test]
    fn fire_on_water_clears_tile_effects_without_draining_adjacent_water() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        let neighbor = IVec2::new(7, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(target);
            board.make_water(neighbor);
            board.add_oil(target);
            board.add_life_essence(target);
            board.add_acid(target);
            board.electrify_acid(target);
            board.add_poison(target);
            board.activate_thornheart(target, 42, false);
            board.start_terrain_growth(target, 42, TerrainGrowthKind::Void);
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        let board = app.world().resource::<Board>();
        assert_eq!(board.ground(target), Some(Ground::Solid));
        assert!(!board.is_water(target));
        assert!(!board.has_oil(target));
        assert!(!board.has_life_essence(target));
        assert!(!board.is_acidic(target));
        assert!(!board.is_electrified_acid(target));
        assert!(!board.is_poisoned(target));
        assert!(!board.has_thorns(target));
        assert!(!board.terrain_growth_cells().any(|cell| cell == target));
        assert_eq!(board.ground(neighbor), Some(Ground::Water));
        assert!(
            app.world()
                .resource::<TurnState>()
                .ingredient_respawns
                .contains(&(LIFE_ESSENCE_ITEM_INDEX, 5))
        );
    }

    #[test]
    fn chained_fire_thaws_frozen_water_before_it_can_be_drained() {
        let mut board = Board::default();
        let source = IVec2::new(4, 3);
        let frozen = IVec2::new(5, 3);
        for cell in [source, frozen] {
            board.make_water(cell);
            board.add_oil(cell);
        }
        assert!(board.freeze_water(frozen));
        resolve_blasts(&mut board, &[], &[], Some(source), None);
        assert_eq!(board.ground(source), Some(Ground::Solid));
        assert_eq!(board.ground(frozen), Some(Ground::Water));
        assert!(!board.is_frozen(frozen));
        assert!(!board.has_oil(frozen));

        board.add_oil(frozen);
        resolve_blasts(&mut board, &[], &[], Some(frozen), None);
        assert_eq!(board.ground(frozen), Some(Ground::Solid));
    }

    #[test]
    fn sunstone_on_water_after_steam_does_not_create_more_steam() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        assert!(steam_positions(&mut app).is_empty());
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(water_label(&mut app, target), None);
        assert!(steam_positions(&mut app).is_empty());
    }

    #[test]
    fn oil_blast_indicators_are_clipped_to_the_board_and_expire() {
        let mut app = interaction_app();
        let target = IVec2::new(0, 0);
        move_player_to(&mut app, PlayerId::One, IVec2::new(1, 0));
        app.world_mut().resource_mut::<Board>().add_oil(target);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        {
            let world = app.world_mut();
            let mut fire = world.query_filtered::<&Transform, With<FireFlash>>();
            let cells: Vec<_> = fire
                .iter(world)
                .map(|transform| board_cell(transform.translation.truncate()).unwrap())
                .collect();
            assert_eq!(cells.len(), 4);
            assert!(
                [target, IVec2::new(0, 1), IVec2::new(1, 0), IVec2::new(1, 1)]
                    .iter()
                    .all(|cell| cells.contains(cell))
            );
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        let world = app.world_mut();
        let mut fire = world.query_filtered::<Entity, With<FireFlash>>();
        assert_eq!(fire.iter(world).count(), 0);
    }

    #[test]
    fn sunstone_thaws_oily_ice_and_ignites_without_steam() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_wet(target);
            board.add_oil(target);
            assert!(board.freeze_water(target));
        }
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 8]);
        assert!(!app.world().resource::<Board>().has_oil(target));
        assert!(!app.world().resource::<Board>().is_frozen(target));
        assert_eq!(
            app.world().resource::<Board>().water_percentage(target),
            100
        );
        assert!(steam_positions(&mut app).is_empty());
    }

    #[test]
    fn steam_damages_players_once_and_stops_at_adjacent_ridges() {
        let mut app = interaction_app();
        app.update();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::One, IVec2::new(6, 4));
        move_player_to(&mut app, PlayerId::Two, IVec2::new(6, 2));
        app.world_mut().resource_mut::<TurnState>().wind = WindDirection::West;
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(target, Ground::Water);
            board.set(IVec2::new(5, 3), Ground::Ridge);
        }
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        assert_eq!(
            steam_positions(&mut app),
            west_steam_positions()
                .into_iter()
                .filter(|cell| ![IVec2::new(5, 3), IVec2::new(4, 3)].contains(cell))
                .collect::<Vec<_>>()
        );
        app.update();
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);

        let mut app = interaction_app();
        app.update();
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<TurnState>().wind = WindDirection::West;
        app.world_mut().resource_mut::<PlayerHealth>().0 = [1, 0];
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [0, 0]);
    }

    #[test]
    fn steam_does_not_hit_players_on_or_behind_ridges() {
        let target = IVec2::new(6, 3);
        let ridge = IVec2::new(5, 3);
        let behind = IVec2::new(4, 3);
        for (player_cell, expected_health) in [(ridge, 10), (behind, 10), (IVec2::new(5, 4), 9)] {
            let mut app = interaction_app();
            move_player_to(&mut app, PlayerId::One, IVec2::new(7, 4));
            move_player_to(&mut app, PlayerId::Two, player_cell);
            {
                let mut turn = app.world_mut().resource_mut::<TurnState>();
                turn.wind = WindDirection::West;
                turn.hovering[1] = player_cell == ridge;
            }
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(target);
                board.set(ridge, Ground::Ridge);
            }
            click(&mut app, p1_item(OIL_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0[1], expected_health);
            let cells = steam_positions(&mut app);
            assert!(!cells.contains(&ridge));
            assert!(!cells.contains(&behind));
        }
    }

    #[test]
    fn ember_on_frozen_water_is_rejected_without_spending() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            assert!(board.freeze_water(cell));
        }
        app.world_mut().resource_mut::<TurnState>().inventory[0][OIL_ITEM_INDEX] = 1;
        click(&mut app, p1_item(OIL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.inventory[0][OIL_ITEM_INDEX], 1);
        assert_eq!(turn.actions_used, 0);
        assert_eq!(turn.active, PlayerId::One);
        assert_eq!(turn.strength[0], 5);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert!(!app.world().resource::<Board>().has_oil(cell));
        assert!(steam_positions(&mut app).is_empty());
    }

    #[test]
    fn water_stone_creates_water_that_does_not_evaporate() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        for _ in 0..10 {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Water)
            );
            assert_eq!(water_label(&mut app, cell), None);
        }
    }

    #[test]
    fn multiple_water_stones_create_independent_permanent_tiles() {
        let mut app = interaction_app();
        let first = IVec2::new(6, 3);
        let second = IVec2::new(6, 5);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(first));
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(second));
        for _ in 0..6 {
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(
                app.world().resource::<Board>().ground(first),
                Some(Ground::Water)
            );
            assert_eq!(
                app.world().resource::<Board>().ground(second),
                Some(Ground::Water)
            );
        }
    }

    #[test]
    fn water_stone_converts_temporary_wet_ground_without_expiring_life_or_ice() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_wet(cell);
            board.add_life_essence(cell);
            assert!(board.freeze_water(cell));
        }
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let board = app.world().resource::<Board>();
        assert_eq!(board.ground(cell), Some(Ground::Water));
        assert_eq!(board.water_percentage(cell), 0);
        assert!(board.is_frozen(cell));
        assert!(board.has_life_essence(cell));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn water_rejects_water_and_holes_without_ending_the_turn() {
        for (ground, message) in [
            (Ground::Water, "Cannot place water on a water tile"),
            (Ground::Hole, "Only Stone Dust can be placed in a hole"),
        ] {
            for cell in [IVec2::new(6, 3), IVec2::new(8, 3)] {
                let mut app = interaction_app();
                app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
                app.world_mut().resource_mut::<Board>().set(cell, ground);
                app.world_mut()
                    .resource_mut::<Board>()
                    .add_oil(IVec2::new(0, 0));
                {
                    let mut turn = app.world_mut().resource_mut::<TurnState>();
                    turn.magic[0] = 4;
                    turn.hovering[0] = true;
                }
                let other = app.world_mut().spawn((BoardItem, Name::new("Other"))).id();
                click(&mut app, p1_item(WATER_ITEM_INDEX));
                click(&mut app, cell_center(cell));
                assert_eq!(app.world().resource::<Board>().ground(cell), Some(ground));
                assert!(!app.world().resource::<Board>().is_wet(cell));
                assert_eq!(item_count(&mut app), 1);
                assert!(app.world().get_entity(other).is_ok());
                assert!(matches!(
                    app.world().resource::<Selection>().0,
                    Some(SelectedAction::Item {
                        index: WATER_ITEM_INDEX,
                        ..
                    })
                ));
                assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
                assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
                assert_eq!(
                    app.world().resource::<TurnState>().inventory[0][WATER_ITEM_INDEX],
                    1
                );
                assert_eq!(app.world().resource::<TurnState>().magic, [4, 5]);
                assert!(app.world().resource::<Board>().has_oil(IVec2::new(0, 0)));
                {
                    let world = app.world_mut();
                    let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
                    assert!(feedback.single(world).unwrap().0.contains(message));
                }
                let valid = IVec2::new(6, 4);
                click(&mut app, cell_center(valid));
                assert_eq!(
                    app.world().resource::<Board>().ground(valid),
                    Some(Ground::Water)
                );
                assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
                assert_eq!(app.world().resource::<TurnState>().magic[0], 3);
            }
        }
    }

    #[test]
    fn adjacent_water_placement_works_without_energy_on_dry_and_wet_ground() {
        for already_wet in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if already_wet {
                    board.make_wet(cell);
                }
            }
            app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
            click(&mut app, p1_item(WATER_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert_eq!(app.world().resource::<TurnState>().strength, [2, 5]);
            assert!(app.world().resource::<Selection>().0.is_none());
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Water)
            );
            assert_eq!(item_count(&mut app), 0);
        }
    }

    #[test]
    fn throwing_water_onto_water_is_rejected_even_without_energy() {
        let cell = IVec2::new(7, 3);
        for energy in [0, 1] {
            let mut app = interaction_app();
            app.world_mut()
                .resource_mut::<Board>()
                .set(cell, Ground::Water);
            app.world_mut().resource_mut::<TurnState>().strength[0] = energy;
            click(&mut app, p1_item(WATER_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert_eq!(item_count(&mut app), 0);
            assert_eq!(
                app.world().resource::<Board>().ground(cell),
                Some(Ground::Water)
            );
            assert_eq!(app.world().resource::<TurnState>().strength[0], energy);
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
            assert!(app.world().resource::<Selection>().0.is_some());
            let world = app.world_mut();
            let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
            assert!(feedback.single(world).unwrap().0.contains("water tile"));
        }
    }

    #[test]
    fn water_on_the_players_own_water_tile_keeps_the_selection() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item {
                index: WATER_ITEM_INDEX,
                ..
            })
        ));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert!(feedback.single(world).unwrap().0.contains("water tile"));
    }

    #[test]
    fn adjacent_water_can_freeze_a_placed_crystal_without_energy() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let world = app.world_mut();
            let mut players = world.query::<(&Player, &mut Transform)>();
            for (player, mut transform) in players.iter_mut(world) {
                if player.0 == PlayerId::Two {
                    transform.translation = cell_center(IVec2::new(7, 3)).extend(0.0);
                }
            }
        }
        app.world_mut().resource_mut::<TurnState>().strength[1] = 0;
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 2]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(item_count(&mut app), 0);
    }

    fn frozen_sprite_count(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut sprites = world.query_filtered::<Entity, With<FrozenTile>>();
        sprites.iter(world).count()
    }

    fn move_player_to(app: &mut App, player_id: PlayerId, cell: IVec2) {
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &mut Transform)>();
        let (_, mut transform) = players
            .iter_mut(world)
            .find(|(player, _)| player.0 == player_id)
            .unwrap();
        transform.translation = cell_center(cell).extend(0.0);
    }

    fn assert_heart(app: &mut App, owner: PlayerId, index: usize, half: bool, visible: bool) {
        let world = app.world_mut();
        let mut hearts = world.query::<(&Heart, &ImageNode, &Visibility)>();
        let (_, image, visibility) = hearts
            .iter(world)
            .find(|(heart, _, _)| heart.owner == owner && heart.index == index)
            .unwrap();
        assert_eq!(
            image.rect,
            if half {
                Some(Rect::new(32.0, 0.0, 64.0, 32.0))
            } else {
                Some(Rect::new(0.0, 0.0, 32.0, 32.0))
            }
        );
        assert_eq!(*visibility != Visibility::Hidden, visible);
    }

    #[test]
    fn water_freeze_blocks_two_turns_and_ice_deals_half_a_heart_per_turn() {
        for wet_ground in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, cell);
            let mut board = app.world_mut().resource_mut::<Board>();
            if wet_ground {
                board.make_wet(cell);
            } else {
                board.set(cell, Ground::Water);
            }
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
            select_move(&mut app);
            assert!(app.world().resource::<Selection>().0.is_none());
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
            assert_eq!(app.world().resource::<PlayerHealth>().0[1], 9);
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
            assert_eq!(app.world().resource::<PlayerHealth>().0[1], 9);
            select_move(&mut app);
            assert!(app.world().resource::<Selection>().0.is_none());
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 8]);
            assert!(app.world().resource::<Board>().is_frozen(cell));
            press_key(&mut app, KeyCode::KeyR);
            select_move(&mut app);
            assert!(matches!(
                app.world().resource::<Selection>().0,
                Some(SelectedAction::Movement { .. })
            ));
            press_key(&mut app, KeyCode::Escape);
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 7]);
        }
    }

    #[test]
    fn ice_damage_depends_on_where_an_unfrozen_player_ends_the_turn() {
        for leave_ice in [false, true] {
            let mut app = interaction_app();
            let ice = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(ice);
                assert!(board.freeze_water(ice));
            }
            select_move(&mut app);
            click_part(&mut app, cell_center(ice));
            assert_eq!(app.world().resource::<PlayerHealth>().0[0], 10);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 0);
            if leave_ice {
                select_move(&mut app);
                click_part(&mut app, cell_center(IVec2::new(7, 3)));
            } else {
                press_key(&mut app, KeyCode::KeyR);
            }
            assert_eq!(
                app.world().resource::<PlayerHealth>().0[0],
                if leave_ice { 10 } else { 9 }
            );
        }
    }

    #[test]
    fn frozen_player_cannot_start_hovering_until_thawed() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<Board>().make_water(cell);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, Vec2::new(330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[1]);
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&Text, With<DropFeedback>>()
                .single(world)
                .unwrap()
                .0,
            "Frozen players cannot start hovering"
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[1]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 8]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(330.0, 120.0));
        assert!(app.world().resource::<TurnState>().hovering[1]);
    }

    #[test]
    fn pending_freeze_also_prevents_starting_hover() {
        let mut app = interaction_app();
        app.world_mut()
            .resource_mut::<TurnState>()
            .pending_effects
            .frozen_players[0] = 2;
        click(&mut app, Vec2::new(-330.0, 120.0));
        assert!(!app.world().resource::<TurnState>().hovering[0]);
    }

    #[test]
    fn frozen_player_takes_no_ice_damage_if_tile_thaws() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().resource_mut::<Board>().make_water(cell);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 9);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.thaw(cell);
            board.set(cell, Ground::Solid);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 9);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
    }

    #[test]
    fn frost_freezes_adjacent_water_without_spreading_further() {
        let mut app = interaction_app();
        let source = IVec2::new(4, 3);
        let water = [source, IVec2::new(5, 3), IVec2::new(3, 4), IVec2::new(4, 2)];
        let outside = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(3, 4));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for cell in water.into_iter().chain([outside]) {
                board.make_water(cell);
            }
        }
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(source));
        let board = app.world().resource::<Board>();
        assert!(water.into_iter().all(|cell| board.is_frozen(cell)));
        assert!(!board.is_frozen(outside));
        assert!(!board.is_frozen(IVec2::new(3, 3)));
        assert_eq!(app.world().resource::<TurnState>().frozen_turns, [2, 2]);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
        assert_eq!(frozen_sprite_count(&mut app), 4);
    }

    #[test]
    fn frozen_player_can_undo_self_placement_then_use_sunstone_to_thaw() {
        for water in [false, true] {
            let mut app = interaction_app();
            let target = IVec2::new(6, 3);
            move_player_to(&mut app, PlayerId::Two, target);
            if water {
                app.world_mut().resource_mut::<Board>().make_water(target);
            }
            app.world_mut().resource_mut::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX] = 1;
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(target));
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
            click_part(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
            assert_eq!(
                app.world().resource::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX],
                0
            );
            click_part(&mut app, Vec2::new(330.0, -60.0));
            assert_eq!(
                app.world().resource::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX],
                1
            );
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
            click_part(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
            click_part(&mut app, cell_center(target));
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
            assert_eq!(
                app.world().resource::<TurnState>().inventory[1][SUNSTONE_ITEM_INDEX],
                0
            );
            assert!(!app.world().resource::<Board>().is_frozen(target));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
            assert!(steam_positions(&mut app).is_empty());
            assert!(
                app.world()
                    .resource::<TurnState>()
                    .ingredient_respawns
                    .contains(&(SUNSTONE_ITEM_INDEX, 5))
            );
            let world = app.world_mut();
            assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
            press_key(&mut app, KeyCode::KeyR);
            select_move(&mut app);
            assert!(matches!(
                app.world().resource::<Selection>().0,
                Some(SelectedAction::Movement { .. })
            ));
        }
    }

    #[test]
    fn self_targeting_frost_preserves_the_full_duration_for_the_next_turn() {
        let mut app = interaction_app();
        let source = IVec2::new(5, 3);
        app.world_mut().resource_mut::<Board>().make_water(source);
        click_part(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click_part(&mut app, cell_center(source));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 2);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 9);
        press_key(&mut app, KeyCode::KeyR);
        click_part(&mut app, cell_center(source));
        assert!(app.world().resource::<Selection>().0.is_none());
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 1);
        assert_eq!(app.world().resource::<PlayerHealth>().0[0], 8);
    }

    #[test]
    fn queued_sunstone_clears_a_direct_frost_hit_in_the_same_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        for item in [
            p1_item(FROST_CRYSTAL_ITEM_INDEX),
            p1_item(SUNSTONE_ITEM_INDEX),
        ] {
            click_part(&mut app, item);
            click_part(&mut app, cell_center(target));
        }
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.frozen_turns[1], 0);
        assert!(
            turn.ingredient_respawns
                .contains(&(FROST_CRYSTAL_ITEM_INDEX, 5))
        );
        assert!(turn.ingredient_respawns.contains(&(SUNSTONE_ITEM_INDEX, 5)));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
    }

    #[test]
    fn queued_sunstone_thaws_water_and_players_frozen_earlier_in_the_turn() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut().resource_mut::<Board>().make_water(target);
        for item in [
            p1_item(FROST_CRYSTAL_ITEM_INDEX),
            p1_item(SUNSTONE_ITEM_INDEX),
        ] {
            click_part(&mut app, item);
            click_part(&mut app, cell_center(target));
        }
        assert!(!app.world().resource::<Board>().is_frozen(target));
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.frozen_turns[1], 0);
        assert!(
            turn.ingredient_respawns
                .contains(&(FROST_CRYSTAL_ITEM_INDEX, 5))
        );
        assert!(turn.ingredient_respawns.contains(&(SUNSTONE_ITEM_INDEX, 5)));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert!(steam_positions(&mut app).is_empty());
    }

    #[test]
    fn frost_does_not_freeze_a_hovering_player_on_a_ridge() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Ridge);
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[1] = true;
            turn.magic[1] = 1;
        }
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
        press_key(&mut app, KeyCode::KeyR);
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.active, PlayerId::Two);
        assert_eq!(turn.magic[1], 1);
        assert!(turn.hovering[1]);
        select_move(&mut app);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Movement { .. })
        ));
    }

    #[test]
    fn moving_while_opponent_is_frozen_does_not_damage_or_count_their_turn() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        press_key(&mut app, KeyCode::KeyR);
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(5, 4)));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn freezing_adjacent_existing_ice_freezes_both_grounded_players() {
        let mut app = interaction_app();
        let existing = IVec2::new(5, 3);
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, target);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(existing, Ground::Water);
            board.set(target, Ground::Water);
            assert!(board.freeze_water(existing));
        }
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 10]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns, [2, 2]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns, [2, 1]);
    }

    #[test]
    fn frozen_player_cannot_move_off_ice() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(7, 3)));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
    }

    #[test]
    fn water_on_placed_frost_freezes_water_without_harming_hovering_occupant() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, Vec2::new(-330.0, 120.0));
        select_move(&mut app);
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[0], 0);
    }

    #[test]
    fn ice_and_storm_damage_combine_and_saturate_at_zero() {
        for starting_health in [10, 1] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            let adjacent = IVec2::new(7, 3);
            move_player_to(&mut app, PlayerId::Two, cell);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(cell, Ground::Water);
                board.set(adjacent, Ground::Water);
            }
            app.world_mut().resource_mut::<PlayerHealth>().0[1] = starting_health;
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            click(&mut app, p2_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(adjacent));
            assert_eq!(
                app.world().resource::<PlayerHealth>().0,
                [9, starting_health.saturating_sub(3)]
            );
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        }
    }

    #[test]
    fn refreezing_refreshes_the_occupants_duration() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        press_key(&mut app, KeyCode::KeyR);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 9]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 8]);
        assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
        assert!(app.world().resource::<Board>().is_frozen(cell));
    }

    #[test]
    fn frost_freezes_water_permanently() {
        for wet_ground in [false, true] {
            let mut app = interaction_app();
            let cell = IVec2::new(6, 3);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if wet_ground {
                    board.make_wet(cell);
                } else {
                    board.set(cell, Ground::Water);
                }
            }
            click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
            click(&mut app, cell_center(cell));
            assert!(app.world().resource::<Board>().is_frozen(cell));
            assert_eq!(frozen_sprite_count(&mut app), 1);
            assert_eq!(item_count(&mut app), 0);
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);

            select_move(&mut app);
            press_key(&mut app, KeyCode::Escape);
            assert!(app.world().resource::<Board>().is_frozen(cell));
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<Board>().is_frozen(cell));
            press_key(&mut app, KeyCode::KeyR);
            assert!(app.world().resource::<Board>().is_frozen(cell));
            select_move(&mut app);
            click(&mut app, cell_center(IVec2::new(7, 5)));
            assert!(app.world().resource::<Board>().is_frozen(cell));
            assert_eq!(frozen_sprite_count(&mut app), 1);
            for _ in 0..10 {
                press_key(&mut app, KeyCode::KeyR);
                assert!(app.world().resource::<Board>().is_frozen(cell));
            }
            let board = app.world().resource::<Board>();
            assert!(board.is_water(cell));
            assert_eq!(
                board.ground(cell),
                Some(if wet_ground {
                    Ground::Solid
                } else {
                    Ground::Water
                })
            );
        }
    }

    #[test]
    fn explosions_thaw_all_ice_in_their_footprint() {
        for kind in [
            BlastKind::Oil,
            BlastKind::Dust,
            BlastKind::Diagonal,
            BlastKind::Cross,
        ] {
            let mut board = Board::default();
            let source = IVec2::new(4, 3);
            let hit = match kind {
                BlastKind::Oil | BlastKind::Dust => source + IVec2::new(1, 1),
                BlastKind::Diagonal => source + IVec2::new(2, 2),
                BlastKind::Cross => source + IVec2::X * 2,
            };
            let outside = IVec2::ZERO;
            for cell in [hit, outside] {
                board.make_water(cell);
                assert!(board.freeze_water(cell));
            }
            if matches!(kind, BlastKind::Oil) {
                board.add_oil(source);
            }
            let ignition = [ObsidianIgnition {
                index: 0,
                diagonal: matches!(kind, BlastKind::Diagonal),
                cross: matches!(kind, BlastKind::Cross),
            }];
            let initial = if matches!(kind, BlastKind::Diagonal | BlastKind::Cross) {
                &ignition[..]
            } else {
                &[]
            };
            resolve_blasts(
                &mut board,
                &[(None, source)],
                initial,
                matches!(kind, BlastKind::Oil).then_some(source),
                matches!(kind, BlastKind::Dust).then_some(source),
            );
            assert!(!board.is_frozen(hit));
            assert_eq!(board.ground(hit), Some(Ground::Water));
            assert!(board.is_frozen(outside));
        }
    }

    #[test]
    fn electricity_does_not_thaw_frozen_water() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            assert!(board.freeze_water(cell));
        }
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
    }

    #[test]
    fn electricity_triggered_obsidian_explosion_thaws_ice() {
        let mut app = interaction_app();
        let water = IVec2::new(6, 3);
        let obsidian = IVec2::new(7, 3);
        let ice = IVec2::new(8, 4);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            for cell in [water, obsidian, ice] {
                board.make_water(cell);
            }
            assert!(board.freeze_water(ice));
        }
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell: obsidian,
            },
            PendingObsidian(obsidian),
        ));
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(water));
        let board = app.world().resource::<Board>();
        assert!(!board.is_frozen(ice));
        assert!(board.is_water(ice));
    }

    #[test]
    fn frost_and_storm_hit_players_on_dry_tiles_without_reactions() {
        for frost in [false, true] {
            for hover in [false, true] {
                let mut app = interaction_app();
                let cell = IVec2::new(6, 3);
                move_player_to(&mut app, PlayerId::Two, cell);
                app.world_mut().resource_mut::<TurnState>().hovering[1] = hover;
                let inventory = if frost {
                    p1_item(FROST_CRYSTAL_ITEM_INDEX)
                } else {
                    p1_item(STORM_SHARD_ITEM_INDEX)
                };
                click(&mut app, inventory);
                click(&mut app, cell_center(cell));
                let expected_health = [10, if frost { 10 } else { 9 }];
                assert_eq!(app.world().resource::<PlayerHealth>().0, expected_health);
                assert_eq!(
                    app.world().resource::<TurnState>().frozen_turns[1],
                    2 * u8::from(frost && !hover)
                );
                assert!(!app.world().resource::<Board>().is_frozen(cell));
                {
                    let world = app.world_mut();
                    let missed = frost && hover;
                    assert_eq!(
                        world.query::<&FrostImpact>().iter(world).count(),
                        usize::from(frost && !hover)
                    );
                    assert_eq!(
                        world.query::<&PendingFrost>().iter(world).count(),
                        usize::from(missed)
                    );
                    assert_eq!(world.query::<&PendingStormShard>().iter(world).count(), 0);
                    assert_eq!(
                        world.query::<&Collectible>().iter(world).count(),
                        usize::from(missed)
                    );
                    let respawns = &world.resource::<TurnState>().ingredient_respawns;
                    if missed {
                        assert!(respawns.is_empty());
                    } else {
                        assert_eq!(
                            respawns,
                            &[(
                                if frost {
                                    FROST_CRYSTAL_ITEM_INDEX
                                } else {
                                    STORM_SHARD_ITEM_INDEX
                                },
                                INGREDIENT_RESPAWN_DELAY
                            )]
                        );
                    }
                }
                app.update();
                assert_eq!(app.world().resource::<PlayerHealth>().0, expected_health);
                assert_eq!(
                    app.world_mut()
                        .query::<&FrostImpact>()
                        .iter(app.world())
                        .count(),
                    usize::from(frost && !hover)
                );
                press_key(&mut app, KeyCode::KeyR);
                assert_eq!(app.world().resource::<PlayerHealth>().0, expected_health);
                assert_eq!(
                    app.world().resource::<TurnState>().active,
                    PlayerId::One,
                    "frost={frost} hover={hover}"
                );
                assert_eq!(
                    app.world_mut()
                        .query::<&FrostImpact>()
                        .iter(app.world())
                        .count(),
                    usize::from(frost && !hover)
                );
                if frost && !hover {
                    assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 1);
                    press_key(&mut app, KeyCode::KeyR);
                    press_key(&mut app, KeyCode::KeyR);
                    assert_eq!(app.world().resource::<TurnState>().frozen_turns[1], 0);
                    assert_eq!(app.world().resource::<PlayerHealth>().0, expected_health);
                    assert_eq!(
                        app.world_mut()
                            .query::<&FrostImpact>()
                            .iter(app.world())
                            .count(),
                        0
                    );
                }
            }
        }
    }

    #[test]
    fn dry_frost_without_a_player_does_not_create_an_impact() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        let world = app.world_mut();
        assert_eq!(world.query::<&FrostImpact>().iter(world).count(), 0);
        assert_eq!(world.resource::<PlayerHealth>().0, [10, 10]);
    }

    #[test]
    fn storm_obsidian_reaction_does_not_add_a_direct_hit() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, cell);
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: OBSIDIAN_ITEM_INDEX,
                cell,
            },
            PendingObsidian(cell),
        ));
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(app.world().resource::<PlayerHealth>().0[1], 7);
    }

    #[test]
    fn either_player_can_refreeze_water_without_changing_its_duration() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Water);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(frozen_sprite_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(frozen_sprite_count(&mut app), 1);
    }

    #[test]
    fn created_water_remains_frozen_after_multiple_turns() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(item_count(&mut app), 0);
        click(&mut app, p2_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(item_count(&mut app), 0);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(frozen_sprite_count(&mut app), 1);
    }

    #[test]
    fn water_on_a_placed_frost_crystal_freezes_permanently() {
        let mut app = interaction_app();
        let cell = IVec2::new(6, 3);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(item_count(&mut app), 1);
        assert!(!app.world().resource::<Board>().is_wet(cell));

        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(cell));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(frozen_sprite_count(&mut app), 1);
        {
            let world = app.world_mut();
            let mut crystals = world.query::<&PendingFrost>();
            assert_eq!(crystals.iter(world).count(), 0);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        press_key(&mut app, KeyCode::KeyR);
        assert!(app.world().resource::<Board>().is_frozen(cell));
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(frozen_sprite_count(&mut app), 1);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<Board>().ground(cell),
            Some(Ground::Water)
        );
    }

    #[test]
    fn water_only_reacts_with_a_crystal_on_the_target_cell_after_a_valid_action() {
        let mut app = interaction_app();
        let frost_cell = IVec2::new(6, 3);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(frost_cell));
        let other = app.world_mut().spawn((BoardItem, Name::new("Other"))).id();
        app.world_mut().resource_mut::<TurnState>().strength[1] = 0;
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(frost_cell));
        assert!(!app.world().resource::<Board>().is_wet(frost_cell));
        assert!(!app.world().resource::<Board>().is_frozen(frost_cell));
        assert_eq!(item_count(&mut app), 2);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        press_key(&mut app, KeyCode::Escape);
        press_key(&mut app, KeyCode::KeyR);
        press_key(&mut app, KeyCode::KeyR);

        let other_cell = IVec2::new(6, 5);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(other_cell));
        assert_eq!(
            app.world().resource::<Board>().ground(other_cell),
            Some(Ground::Water)
        );
        assert!(!app.world().resource::<Board>().is_frozen(other_cell));
        assert!(!app.world().resource::<Board>().is_frozen(frost_cell));
        assert!(app.world().get_entity(other).is_ok());
        let world = app.world_mut();
        let mut crystals = world.query::<&PendingFrost>();
        assert_eq!(crystals.iter(world).count(), 1);
    }

    #[test]
    fn frost_on_dry_ground_or_without_throw_energy_does_not_freeze_water() {
        let mut app = interaction_app();
        let adjacent = IVec2::new(6, 3);
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(adjacent));
        assert!(!app.world().resource::<Board>().is_frozen(adjacent));
        assert_eq!(item_count(&mut app), 1);

        let mut app = interaction_app();
        let distant = IVec2::new(8, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(distant, Ground::Water);
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(distant));
        assert!(!app.world().resource::<Board>().is_frozen(distant));
        assert!(app.world().resource::<Selection>().0.is_some());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
    }

    fn visible_hearts(app: &mut App, player: PlayerId) -> usize {
        let world = app.world_mut();
        let mut hearts = world.query::<(&Heart, &Visibility)>();
        hearts
            .iter(world)
            .filter(|(heart, visibility)| {
                heart.owner == player && **visibility != Visibility::Hidden
            })
            .count()
    }

    #[test]
    fn water_on_a_placed_storm_shard_electrifies_connected_water() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(IVec2::new(5, 3), Ground::Water);
            board.make_wet(IVec2::new(6, 4));
            board.set(IVec2::new(7, 5), Ground::Water);
            board.set(IVec2::new(7, 6), Ground::Water);
        }
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        assert_eq!(item_count(&mut app), 1);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 8]);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(water_label(&mut app, target), None);
        assert_eq!(item_count(&mut app), 0);
        {
            let world = app.world_mut();
            let mut shards = world.query::<&PendingStormShard>();
            assert_eq!(shards.iter(world).count(), 0);
            let mut flashes = world.query_filtered::<Entity, With<ElectricFlash>>();
            assert_eq!(flashes.iter(world).count(), 20);
        }
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 8]);
    }

    #[test]
    fn water_only_activates_a_placed_shard_on_a_valid_matching_tile() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        app.world_mut().resource_mut::<TurnState>().strength[1] = 0;
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert!(!app.world().resource::<Board>().is_wet(target));
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        press_key(&mut app, KeyCode::Escape);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(IVec2::new(7, 5)));
        assert!(!app.world().resource::<Board>().is_wet(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        {
            let world = app.world_mut();
            let mut shards = world.query::<&PendingStormShard>();
            assert_eq!(shards.iter(world).count(), 1);
        }
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        let world = app.world_mut();
        let mut shards = world.query::<&PendingStormShard>();
        assert_eq!(shards.iter(world).count(), 0);
    }

    #[test]
    fn water_reacts_with_a_shard_and_frost_on_the_same_tile() {
        let mut app = interaction_app();
        let target = IVec2::new(6, 3);
        move_player_to(&mut app, PlayerId::Two, IVec2::new(7, 3));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(IVec2::new(5, 3), Ground::Water);
            board.set(IVec2::new(7, 3), Ground::Water);
        }
        click(&mut app, p1_item(FROST_CRYSTAL_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p2_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 8]);
        assert!(app.world().resource::<Board>().is_frozen(target));
        assert_eq!(
            app.world().resource::<Board>().ground(target),
            Some(Ground::Water)
        );
        assert_eq!(item_count(&mut app), 0);
        let world = app.world_mut();
        let mut crystals = world.query::<&PendingFrost>();
        assert_eq!(crystals.iter(world).count(), 0);
        let mut shards = world.query::<&PendingStormShard>();
        assert_eq!(shards.iter(world).count(), 0);
    }

    #[test]
    fn storm_shard_immediately_shocks_connected_water_and_wet_ground_once() {
        for wet_origin in [false, true] {
            let mut app = interaction_app();
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.set(IVec2::new(5, 3), Ground::Water);
                if wet_origin {
                    board.make_wet(IVec2::new(6, 3));
                } else {
                    board.set(IVec2::new(6, 3), Ground::Water);
                }
                board.make_wet(IVec2::new(6, 4));
                board.set(IVec2::new(7, 5), Ground::Water);
                board.set(IVec2::new(7, 6), Ground::Water);
            }
            click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(IVec2::new(6, 3)));
            assert_eq!(app.world().resource::<PlayerHealth>().0, [8, 8]);
            assert_eq!(visible_hearts(&mut app, PlayerId::One), 4);
            assert_eq!(visible_hearts(&mut app, PlayerId::Two), 4);
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
            assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
            assert!(app.world().resource::<Board>().is_wet(IVec2::new(6, 4)));
            assert_eq!(item_count(&mut app), 0);
            let world = app.world_mut();
            let mut flashes = world.query_filtered::<Entity, With<ElectricFlash>>();
            assert_eq!(flashes.iter(world).count(), 20);
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.update();
            let world = app.world_mut();
            let mut flashes = world.query_filtered::<Entity, With<ElectricFlash>>();
            assert_eq!(flashes.iter(world).count(), 0);
            assert_eq!(world.resource::<PlayerHealth>().0, [8, 8]);
        }
    }

    #[test]
    fn electrical_water_shocks_adjacent_players_once_and_respects_hover() {
        let origin = IVec2::new(4, 3);
        for (first, second, hovering, expected) in [
            (IVec2::new(5, 3), IVec2::new(4, 4), false, [8, 9]),
            (IVec2::new(4, 4), IVec2::new(6, 4), false, [9, 9]),
            (IVec2::new(4, 4), IVec2::new(6, 4), true, [10, 9]),
        ] {
            let mut app = interaction_app();
            move_player_to(&mut app, PlayerId::One, first);
            move_player_to(&mut app, PlayerId::Two, second);
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                board.make_water(origin);
                board.make_water(origin + IVec2::X);
            }
            app.world_mut().resource_mut::<TurnState>().hovering[0] = hovering;
            click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(origin));
            assert_eq!(app.world().resource::<PlayerHealth>().0, expected);
            press_key(&mut app, KeyCode::KeyR);
            assert_eq!(app.world().resource::<PlayerHealth>().0, expected);
        }
    }

    #[test]
    fn electrical_flash_marks_water_and_bounded_neighbors_with_a_fainter_halo() {
        let mut app = interaction_app();
        let origin = IVec2::ZERO;
        move_player_to(&mut app, PlayerId::One, IVec2::new(1, 1));
        move_player_to(&mut app, PlayerId::Two, IVec2::new(0, 1));
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(origin);
            board.make_water(IVec2::X);
        }
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(origin));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Transform, &Sprite), With<ElectricFlash>>();
        let flashes: std::collections::HashMap<_, _> = query
            .iter(world)
            .map(|(transform, sprite)| {
                (
                    board_cell(transform.translation.truncate()).unwrap(),
                    sprite.color.to_srgba().alpha,
                )
            })
            .collect();
        assert_eq!(flashes.len(), 6);
        for cell in [origin, IVec2::X] {
            assert_eq!(flashes[&cell], 0.65);
        }
        for cell in [
            IVec2::new(2, 0),
            IVec2::new(0, 1),
            IVec2::new(1, 1),
            IVec2::new(2, 1),
        ] {
            assert_eq!(flashes[&cell], 0.5);
        }
        assert_eq!(query.iter(world).count(), flashes.len());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<ElectricFlash>>()
                .iter(world)
                .count(),
            0
        );
    }

    #[test]
    fn water_on_a_placed_storm_shard_also_shocks_adjacent_players() {
        let mut app = interaction_app();
        let target = IVec2::new(4, 3);
        move_player_to(&mut app, PlayerId::One, IVec2::new(5, 3));
        move_player_to(&mut app, PlayerId::Two, IVec2::new(4, 4));
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 10]);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [9, 9]);
    }

    #[test]
    fn storm_shard_does_not_shock_disconnected_water_or_distant_ground() {
        for conductive_target in [false, true] {
            let mut app = interaction_app();
            {
                let mut board = app.world_mut().resource_mut::<Board>();
                if conductive_target {
                    board.set(IVec2::new(6, 3), Ground::Water);
                }
                board.set(IVec2::new(7, 6), Ground::Water);
            }
            click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
            click(&mut app, cell_center(IVec2::new(6, 3)));
            assert_eq!(
                app.world().resource::<PlayerHealth>().0,
                [if conductive_target { 9 } else { 10 }, 10]
            );
            assert_eq!(item_count(&mut app), usize::from(!conductive_target));
            let world = app.world_mut();
            let mut flashes = world.query_filtered::<Entity, With<ElectricFlash>>();
            assert_eq!(
                flashes.iter(world).count(),
                9 * usize::from(conductive_target)
            );
        }
    }

    #[test]
    fn thrown_storm_shard_costs_energy_and_health_cannot_drop_below_zero() {
        let mut app = interaction_app();
        let target = IVec2::new(7, 6);
        app.world_mut()
            .resource_mut::<Board>()
            .set(target, Ground::Water);
        app.world_mut().resource_mut::<PlayerHealth>().0[1] = 0;
        click(&mut app, p1_item(STORM_SHARD_ITEM_INDEX));
        click(&mut app, cell_center(target));
        assert_eq!(app.world().resource::<PlayerHealth>().0, [10, 0]);
        assert_eq!(visible_hearts(&mut app, PlayerId::Two), 0);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn renamed_ingredients_are_used_for_hover_and_placed_items() {
        for (position, name) in [
            (p1_item(SUNSTONE_ITEM_INDEX), "Sunstone"),
            (p1_item(AQUATIC_ITEM_INDEX), "Ahuizotl Totem"),
        ] {
            let mut app = interaction_app();
            app.world_mut().resource_mut::<BoardCursor>().0 = Some(position);
            app.update();
            let world = app.world_mut();
            let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
            assert_eq!(
                feedback.single(world).unwrap().0,
                format!("{name} - click to select")
            );
            click(&mut app, position);
            click(&mut app, Vec2::new(90.0, 60.0));
            let world = app.world_mut();
            let mut items = world.query_filtered::<&Name, With<BoardItem>>();
            assert_eq!(items.single(world).unwrap().as_str(), name);
        }
    }

    #[test]
    fn selected_item_preview_is_centered_on_cursor_in_hud() {
        let mut app = interaction_app();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        let icon = match &app.world().resource::<Selection>().0 {
            Some(SelectedAction::Item { icon, .. }) => *icon,
            _ => panic!("item should be selected"),
        };
        assert!(app.world().get::<ImageNode>(icon).is_some());
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(90.0, 60.0));
        app.update();
        let node = app.world().get::<Node>(icon).unwrap();
        assert_eq!(node.left, Val::Px(548.0));
        assert_eq!(node.top, Val::Px(188.0));
        assert_eq!(node.width, Val::Px(44.0));
        assert_eq!(node.height, Val::Px(44.0));
        assert_eq!(
            app.world().get::<Visibility>(icon),
            Some(&Visibility::Visible)
        );
        app.world_mut().resource_mut::<BoardCursor>().0 = None;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(icon),
            Some(&Visibility::Hidden)
        );
    }

    fn slot_border(app: &mut App, owner: PlayerId, index: usize) -> UiRect {
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &Node)>();
        slots
            .iter(world)
            .find_map(|(slot, node)| {
                (slot.owner == owner && slot.index == index).then_some(node.border)
            })
            .unwrap()
    }

    #[test]
    fn selected_slot_remains_highlighted_until_cancelled_or_placed() {
        let mut app = interaction_app();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 0),
            UiRect::all(Val::Px(3.0))
        );
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(p1_item(SUNSTONE_ITEM_INDEX));
        app.update();
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 0),
            UiRect::all(Val::Px(3.0))
        );
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 1),
            UiRect::all(Val::Px(2.0))
        );
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 0),
            UiRect::all(Val::Px(0.5))
        );

        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, Vec2::new(90.0, 60.0));
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 0),
            UiRect::all(Val::Px(0.5))
        );
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn item_selection_can_be_cancelled_or_changed_without_a_turn() {
        let mut app = interaction_app();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item { index: 1, .. })
        ));
        press_key(&mut app, KeyCode::Escape);
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(item_count(&mut app), 0);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
    }

    #[test]
    fn switching_between_item_and_move_clears_previews() {
        let mut app = interaction_app();
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        let (icon, highlight) = match &app.world().resource::<Selection>().0 {
            Some(SelectedAction::Item {
                icon, highlight, ..
            }) => (*icon, *highlight),
            _ => panic!("item should be selected"),
        };
        select_move(&mut app);
        assert!(app.world().get_entity(icon).is_err());
        assert!(app.world().get_entity(highlight).is_err());
        let highlights = match &app.world().resource::<Selection>().0 {
            Some(SelectedAction::Movement { highlights }) => highlights.clone(),
            _ => panic!("movement should be selected"),
        };
        assert_eq!(highlights.len(), 24);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(
            highlights
                .iter()
                .all(|entity| app.world().get_entity(*entity).is_err())
        );
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item { .. })
        ));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(item_count(&mut app), 0);
    }

    #[test]
    fn clicking_the_board_without_a_selection_does_not_select_or_move() {
        let mut app = interaction_app();
        click_part(&mut app, cell_center(IVec2::new(5, 3)));
        click_part(&mut app, cell_center(IVec2::new(6, 3)));
        assert!(app.world().resource::<Selection>().0.is_none());
        let turn = app.world().resource::<TurnState>();
        assert_eq!(turn.actions_used, 0);
        assert_eq!(turn.active, PlayerId::One);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let position = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap()
            .1;
        assert_eq!(
            position.translation.truncate(),
            cell_center(IVec2::new(5, 3))
        );
    }

    #[test]
    fn action_column_hitboxes_follow_the_visible_order_for_both_players() {
        for player in [PlayerId::One, PlayerId::Two] {
            let x = if player == PlayerId::One {
                -330.0
            } else {
                330.0
            };
            for (row, action) in [
                Action::Move,
                Action::Hover,
                Action::Pickup,
                Action::Rest,
                Action::Undo,
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(action.row(), row);
                assert_eq!(
                    action_cell(Vec2::new(x, 180.0 - 60.0 * row as f32), player),
                    Some(row)
                );
            }
            assert_eq!(action_cell(Vec2::new(x, -120.0), player), None);
        }
    }

    #[test]
    fn player_selection_walks_diagonally_and_rejects_out_of_range() {
        let mut app = interaction_app();
        select_move(&mut app);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Movement { .. })
        ));
        click(&mut app, Vec2::new(-270.0, 180.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Movement { .. })
        ));
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Too far - jump up to 2 grid steps"
        );

        select_move(&mut app);
        assert!(app.world().resource::<Selection>().0.is_none());
        select_move(&mut app);
        click(&mut app, Vec2::new(90.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let player = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap()
            .1;
        assert_eq!(player.translation.truncate(), Vec2::new(90.0, 60.0));
        assert!(world.resource::<Selection>().0.is_none());
    }

    #[test]
    fn water_is_walkable_but_grounded_jumps_cannot_cross_ridges() {
        let mut app = interaction_app();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(IVec2::new(6, 3), board::Ground::Water);
            board.set(IVec2::new(7, 3), board::Ground::Ridge);
        }
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(6, 3)));
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        press_key(&mut app, KeyCode::KeyR);
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(8, 3)));
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let player = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap()
            .1;
        assert_eq!(player.translation.truncate(), cell_center(IVec2::new(6, 3)));
    }

    #[test]
    fn ridges_block_offset_jumps_but_hovering_can_cross_them() {
        let mut board = Board::default();
        let start = IVec2::new(4, 3);
        let target = drop_target(cell_center(start), cell_center(IVec2::new(6, 4))).unwrap();
        for ridge in [IVec2::new(5, 3), IVec2::new(5, 4)] {
            board.set(ridge, Ground::Ridge);
            assert_eq!(
                movement_target(&board, &target, cell_center(IVec2::new(9, 6)), 5, false, 5),
                Err(MovementError::RidgeBlocksJump)
            );
            assert_eq!(
                movement_target(&board, &target, cell_center(IVec2::new(9, 6)), 5, true, 5),
                Ok(Movement::Jump)
            );
            board.set(ridge, Ground::Solid);
        }
        board.set(IVec2::new(5, 3), Ground::Hole);
        assert_eq!(
            movement_target(&board, &target, cell_center(IVec2::new(9, 6)), 5, false, 5),
            Ok(Movement::Jump)
        );
    }

    #[test]
    fn blocked_and_exhausted_targets_do_not_consume_turn() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.set(IVec2::new(6, 3), board::Ground::Ridge);
            board.set(IVec2::new(5, 2), board::Ground::Hole);
        }
        select_move(&mut app);
        for (cell, message) in [
            (IVec2::new(6, 3), "Cannot land on a ridge or hole"),
            (IVec2::new(5, 2), "Cannot land on a ridge or hole"),
            (IVec2::new(7, 3), "Not enough energy"),
            (IVec2::new(7, 6), "Too far"),
        ] {
            click(&mut app, cell_center(cell));
            assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
            assert!(app.world().resource::<Selection>().0.is_some());
            let world = app.world_mut();
            let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
            assert!(feedback.single(world).unwrap().0.contains(message));
        }
        let world = app.world_mut();
        let mut players = world.query::<(&Player, &Transform)>();
        let player = players
            .iter(world)
            .find(|(player, _)| player.0 == PlayerId::One)
            .unwrap()
            .1;
        assert_eq!(player.translation.truncate(), cell_center(IVec2::new(5, 3)));
        assert_eq!(world.resource::<TurnState>().strength[0], 0);
    }

    #[test]
    fn jump_rejects_occupied_landing_and_allows_offset_two_jump_over_hole() {
        let mut app = interaction_app();
        let opponent = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &Player)>();
            query
                .iter(world)
                .find(|(_, player)| player.0 == PlayerId::Two)
                .unwrap()
                .0
        };
        app.world_mut()
            .get_mut::<Transform>(opponent)
            .unwrap()
            .translation = cell_center(IVec2::new(7, 3)).extend(0.0);
        app.world_mut()
            .resource_mut::<Board>()
            .set(IVec2::new(6, 3), board::Ground::Hole);
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(7, 3)));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        click(&mut app, cell_center(IVec2::new(7, 4)));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
    }

    #[test]
    fn opponent_cell_and_outside_board_do_not_consume_turn() {
        let mut app = interaction_app();
        let opponent = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &Player)>();
            query
                .iter(world)
                .find(|(_, player)| player.0 == PlayerId::Two)
                .unwrap()
                .0
        };
        app.world_mut()
            .get_mut::<Transform>(opponent)
            .unwrap()
            .translation = Vec3::new(90.0, 0.0, 0.0);
        select_move(&mut app);
        click(&mut app, Vec2::new(90.0, 0.0));
        click(&mut app, Vec2::new(301.0, 0.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Movement { .. })
        ));
        press_key(&mut app, KeyCode::Escape);
        assert!(app.world().resource::<Selection>().0.is_none());
    }

    #[test]
    fn turns_strength_and_right_inventory_apply_to_two_click_actions() {
        let mut app = interaction_app();
        click(&mut app, p2_item(SUNSTONE_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, Vec2::new(30.0, -120.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        click(&mut app, p2_item(THORNHEART_ITEM_INDEX));
        click(&mut app, Vec2::new(30.0, -120.0));
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
    }

    fn energy_bar_width(app: &mut App, player: PlayerId) -> Val {
        let world = app.world_mut();
        let mut bars = world.query::<(&EnergyFill, &Node)>();
        bars.iter(world)
            .find_map(|(fill, node)| (fill.0 == player).then_some(node.width))
            .unwrap()
    }

    #[test]
    fn rest_button_restores_energy_and_ends_the_active_players_turn() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength = [2, 3];
        app.world_mut().resource_mut::<TurnState>().magic = [3, 3];
        click(&mut app, Vec2::new(330.0, 0.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        assert!(app.world().resource::<Selection>().0.is_some());
        click(&mut app, Vec2::new(-330.0, 0.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 3]);
        assert_eq!(app.world().resource::<TurnState>().magic, [4, 3]);
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, Vec2::new(330.0, 0.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().magic, [4, 4]);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
    }

    #[test]
    fn resting_fully_restores_energy_with_both_controls() {
        for energy in [0, 1, 2, 4, 5] {
            for button in [false, true] {
                for player in [PlayerId::One, PlayerId::Two] {
                    let mut app = interaction_app();
                    {
                        let mut turn = app.world_mut().resource_mut::<TurnState>();
                        turn.active = player;
                        turn.strength = [2, 2];
                        turn.strength[player.index()] = energy;
                    }
                    if button {
                        let x = if player == PlayerId::One {
                            -330.0
                        } else {
                            330.0
                        };
                        click(&mut app, Vec2::new(x, 0.0));
                    } else {
                        press_key(&mut app, KeyCode::KeyR);
                    }
                    let turn = app.world().resource::<TurnState>();
                    assert_eq!(turn.strength[player.index()], MAX_STRENGTH);
                    assert_eq!(turn.strength[player.other().index()], 2);
                    assert_eq!(turn.active, player.other());
                }
            }
        }
    }

    #[test]
    fn rest_button_obeys_the_same_unsafe_hover_restriction_as_r() {
        let mut app = interaction_app();
        let cell = IVec2::new(5, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Ridge);
        {
            let mut turn = app.world_mut().resource_mut::<TurnState>();
            turn.hovering[0] = true;
            turn.magic[0] = 1;
            turn.strength[0] = 2;
        }
        click(&mut app, Vec2::new(-330.0, 0.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        assert_eq!(app.world().resource::<TurnState>().strength[0], 2);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        let world = app.world_mut();
        let feedback = world
            .query_filtered::<&Text, With<DropFeedback>>()
            .single(world)
            .unwrap();
        assert!(feedback.0.contains("Land on safe ground"));
    }

    #[test]
    fn energy_bars_follow_each_players_throw_and_rest() {
        let mut app = interaction_app();
        app.update();
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(100.0)
        );
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::Two),
            Val::Percent(100.0)
        );

        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, Vec2::new(-90.0, 0.0));
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(100.0)
        );
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::Two),
            Val::Percent(100.0)
        );

        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(100.0)
        );
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(100.0)
        );
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::Two),
            Val::Percent(100.0)
        );

        app.world_mut().resource_mut::<TurnState>().strength[1] = 0;
        app.update();
        assert_eq!(energy_bar_width(&mut app, PlayerId::Two), Val::Percent(0.0));
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::Two),
            Val::Percent(100.0)
        );
    }

    #[test]
    fn insufficient_strength_keeps_selection_until_cancelled() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        click(&mut app, p1_item(WATER_ITEM_INDEX));
        click(&mut app, Vec2::new(-270.0, 180.0));
        assert_eq!(item_count(&mut app), 0);
        assert!(app.world().resource::<Selection>().0.is_some());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert!(
            feedback
                .single(world)
                .unwrap()
                .0
                .contains("Not enough energy")
        );
        press_key(&mut app, KeyCode::Escape);
        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn exhausted_player_can_still_place_and_arrow_keys_do_not_move() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        press_key(&mut app, KeyCode::ArrowRight);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        click(&mut app, p1_item(SUNSTONE_ITEM_INDEX));
        click(&mut app, Vec2::new(90.0, 60.0));
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().strength, [2, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn walking_and_rest_restore_energy_up_to_the_maximum_even_while_hovering() {
        for energy in [0, 4, 5] {
            for hover in [false, true] {
                let mut app = interaction_app();
                app.world_mut().resource_mut::<TurnState>().strength = [energy, 2];
                if hover {
                    click(&mut app, Vec2::new(-330.0, 120.0));
                    assert_eq!(app.world().resource::<TurnState>().strength[0], energy);
                }
                select_move(&mut app);
                click(&mut app, cell_center(IVec2::new(6, 3)));
                let turn = app.world().resource::<TurnState>();
                assert_eq!(turn.strength, [(energy + 2).min(MAX_STRENGTH), 2]);
                assert_eq!(turn.active, PlayerId::Two);
                assert_eq!(turn.magic[0], if hover { 4 } else { 5 });
            }
        }
    }

    #[test]
    fn pickup_restores_energy_only_when_it_succeeds() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength = [0, 2];
        app.world_mut().resource_mut::<TurnState>().inventory = [[1; 14]; 2];
        click(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().strength, [0, 2]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        let cell = IVec2::new(5, 3);
        app.world_mut().spawn((
            BoardItem,
            Collectible {
                index: FROST_CRYSTAL_ITEM_INDEX,
                cell,
            },
            PendingFrost(cell),
        ));
        click(&mut app, Vec2::new(-330.0, 60.0));
        assert_eq!(app.world().resource::<TurnState>().strength, [2, 2]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn rest_recovers_energy_after_jump_and_distant_placement() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength = [3, 3];
        select_move(&mut app);
        click(&mut app, cell_center(IVec2::new(7, 3)));
        assert_eq!(app.world().resource::<TurnState>().strength, [4, 3]);
        click(&mut app, p2_item(WATER_ITEM_INDEX));
        click(&mut app, cell_center(IVec2::new(7, 4)));
        assert_eq!(app.world().resource::<TurnState>().strength, [4, 4]);
    }
}
