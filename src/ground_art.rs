use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

use crate::{
    AcidTile, Board, ElectricFlash, FireFlash, FrozenTile, GRID_BLOCK_SIZE, Ground, GroundTile,
    PoisonTile, SteamFlash, TerrainGrowthKind, ThornTile, TurnState, VoidGrowthTile, effects, help,
    menu, spawn_board_terrain, update_aquatic_visuals,
};

const TERRAIN_SHEET: &str = "basictilesheet.png";
const EFFECT_SHEET: &str = "ground_sprites.png";
const EFFECTS_Y: u32 = 2048;
const SOURCE_TILE: u32 = 32;

#[derive(Resource, Default)]
struct GroundSheet {
    terrain: Handle<Image>,
    effects: Handle<Image>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    fn index(self) -> u32 {
        match self {
            Self::Top => 0,
            Self::Bottom => 1,
            Self::Left => 2,
            Self::Right => 3,
        }
    }

    fn offset(self) -> IVec2 {
        match self {
            Self::Top => IVec2::NEG_Y,
            Self::Bottom => IVec2::Y,
            Self::Left => IVec2::NEG_X,
            Self::Right => IVec2::X,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Effect {
    Wet,
    Frozen,
    Poison,
    Acid,
    ElectricAcid,
    Ember,
    Healing,
    Thorns,
    VoidGrowth,
    StoneGrowth,
    ElectricWater,
    AdjacentShock,
    Steam,
    Fire,
}

impl Effect {
    fn row(self) -> u32 {
        match self {
            Self::Wet => 0,
            Self::Frozen => 1,
            Self::Poison => 2,
            Self::Acid => 3,
            Self::ElectricAcid => 4,
            Self::Ember => 5,
            Self::Healing => 6,
            Self::Thorns => 7,
            Self::VoidGrowth => 8,
            Self::StoneGrowth => 9,
            Self::ElectricWater => 10,
            Self::AdjacentShock => 11,
            Self::Steam => 12,
            Self::Fire => 13,
        }
    }

    fn present(self, board: &Board, cell: IVec2) -> bool {
        if board.ground(cell).is_none() {
            return false;
        }
        match self {
            Self::Wet => board.is_wet(cell),
            Self::Frozen => board.is_frozen(cell),
            Self::Poison => board.is_poisoned(cell),
            Self::Acid => board.is_acidic(cell) && !board.is_electrified_acid(cell),
            Self::ElectricAcid => board.is_electrified_acid(cell),
            Self::Ember => board.has_oil(cell),
            Self::Healing => board.has_life_essence(cell),
            Self::Thorns => board.has_thorns(cell),
            Self::VoidGrowth => board.terrain_growth_kind_at(cell) == Some(TerrainGrowthKind::Void),
            Self::StoneGrowth => {
                board.terrain_growth_kind_at(cell) == Some(TerrainGrowthKind::Stone)
            }
            Self::ElectricWater | Self::AdjacentShock | Self::Steam | Self::Fire => false,
        }
    }
}

#[derive(Component)]
struct GroundArt {
    cell: IVec2,
    part: GroundArtPart,
}

enum GroundArtPart {
    Border(Side),
    Overlay(Effect),
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_sheet)
        .add_systems(
            Startup,
            attach_ground_art
                .after(spawn_board_terrain)
                .after(load_sheet),
        )
        .add_systems(
            Update,
            (update_ground_art, update_effect_art)
                .chain()
                .after(update_aquatic_visuals)
                .in_set(menu::GameplaySystems)
                .run_if(menu::in_game)
                .run_if(help::closed),
        );
}

fn load_sheet(mut commands: Commands, assets: Res<AssetServer>) {
    let load = |path| {
        assets
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load(path)
    };
    commands.insert_resource(GroundSheet {
        terrain: load(TERRAIN_SHEET),
        effects: load(EFFECT_SHEET),
    });
}

fn ground_index(ground: Ground) -> u32 {
    match ground {
        Ground::Solid => 0,
        Ground::Water => 1,
        Ground::Ridge => 2,
        Ground::Hole => 3,
    }
}

fn visible_ground(board: &Board, turn: &TurnState, cell: IVec2) -> Option<Ground> {
    let ground = board.ground(cell)?;
    Some(
        if turn
            .aquatic_animation
            .as_ref()
            .is_some_and(|animation| animation.unflattened.contains(&cell))
        {
            Ground::Ridge
        } else {
            ground
        },
    )
}

fn animated_ground(
    board: &Board,
    turn: &TurnState,
    effects: Option<&effects::Effects>,
    cell: IVec2,
) -> Option<Ground> {
    effects
        .and_then(|effects| effects.held(cell))
        .map(|state| state.ground)
        .or_else(|| visible_ground(board, turn, cell))
}

fn tile_rect(column: u32) -> Rect {
    let x = column * SOURCE_TILE;
    Rect::new(x as f32, 0.0, (x + SOURCE_TILE) as f32, SOURCE_TILE as f32)
}

fn border_rect(board: &Board, turn: &TurnState, cell: IVec2, side: Side) -> Option<Rect> {
    animated_border_rect(board, turn, None, cell, side)
}

fn animated_border_rect(
    board: &Board,
    turn: &TurnState,
    effects: Option<&effects::Effects>,
    cell: IVec2,
    side: Side,
) -> Option<Rect> {
    let center = animated_ground(board, turn, effects, cell)?;
    let neighbor = animated_ground(board, turn, effects, cell + side.offset())?;
    let first_column = match (center, neighbor) {
        (Ground::Water, Ground::Solid) => 4,
        (Ground::Water, Ground::Ridge) => 8,
        (Ground::Solid, Ground::Ridge) => 12,
        (Ground::Hole, Ground::Solid) => 16,
        (Ground::Hole, Ground::Ridge) => 20,
        (Ground::Hole, Ground::Water) => 24,
        _ => return None,
    };
    Some(tile_rect(first_column + side.index()))
}

fn normalize_mask(mut mask: u8) -> u8 {
    for (diagonal, vertical, horizontal) in [(16, 1, 2), (32, 4, 2), (64, 4, 8), (128, 1, 8)] {
        if mask & vertical == 0 || mask & horizontal == 0 {
            mask &= !diagonal;
        }
    }
    mask
}

fn effect_rect(effect: Effect, cell: IVec2, board: &Board) -> Rect {
    animated_effect_rect(effect, cell, board, None)
}

fn animated_effect_rect(
    effect: Effect,
    cell: IVec2,
    board: &Board,
    effects: Option<&effects::Effects>,
) -> Rect {
    let directions = [
        (IVec2::NEG_Y, 1),
        (IVec2::X, 2),
        (IVec2::Y, 4),
        (IVec2::NEG_X, 8),
        (IVec2::new(1, -1), 16),
        (IVec2::new(1, 1), 32),
        (IVec2::new(-1, 1), 64),
        (IVec2::new(-1, -1), 128),
    ];
    let mask = normalize_mask(directions.into_iter().fold(0, |mask, (offset, bit)| {
        let neighbor = cell + offset;
        let present = effects
            .and_then(|effects| effects.held(neighbor))
            .map_or_else(
                || effect.present(board, neighbor),
                |old| match effect {
                    Effect::Wet => old.wet,
                    Effect::Frozen => old.frozen,
                    Effect::Poison => old.poison,
                    Effect::Acid => old.acid && !old.electric_acid,
                    Effect::ElectricAcid => old.electric_acid,
                    Effect::Ember => old.fuel,
                    Effect::Healing => old.healing,
                    Effect::Thorns => old.thorns,
                    Effect::VoidGrowth => old.growth == Some(TerrainGrowthKind::Void),
                    Effect::StoneGrowth => old.growth == Some(TerrainGrowthKind::Stone),
                    _ => effect.present(board, neighbor),
                },
            );
        if present { mask | bit } else { mask }
    }));
    let column = (0..mask)
        .filter(|&value| normalize_mask(value) == value)
        .count() as u32;
    let x = column * SOURCE_TILE;
    let y = EFFECTS_Y + effect.row() * 64 + 32;
    Rect::new(x as f32, y as f32, (x + 32) as f32, (y + 32) as f32)
}

fn attach_ground_art(
    mut commands: Commands,
    board: Res<Board>,
    turn: Res<TurnState>,
    sheet: Res<GroundSheet>,
    mut tiles: Query<(Entity, &GroundTile, &mut Sprite)>,
) {
    for (entity, tile, mut sprite) in &mut tiles {
        sprite.image = sheet.terrain.clone();
        sprite.rect = Some(tile_rect(ground_index(
            visible_ground(&board, &turn, tile.0).expect("terrain tile must be in bounds"),
        )));
        sprite.color = Color::WHITE;
        commands.entity(entity).with_children(|parent| {
            for side in Side::ALL {
                let rect = border_rect(&board, &turn, tile.0, side);
                parent.spawn((
                    GroundArt {
                        cell: tile.0,
                        part: GroundArtPart::Border(side),
                    },
                    Sprite {
                        image: sheet.terrain.clone(),
                        rect: Some(rect.unwrap_or_else(|| tile_rect(0))),
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                        ..default()
                    },
                    if rect.is_some() {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    Transform::from_xyz(0.0, 0.0, 0.01 * (side.index() + 1) as f32),
                ));
            }
            for (effect, z) in [
                (Effect::Wet, 0.9),
                (Effect::Ember, 1.1),
                (Effect::Healing, 1.15),
            ] {
                parent.spawn((
                    GroundArt {
                        cell: tile.0,
                        part: GroundArtPart::Overlay(effect),
                    },
                    Sprite {
                        image: sheet.effects.clone(),
                        rect: Some(effect_rect(effect, tile.0, &board)),
                        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                        ..default()
                    },
                    if effect.present(&board, tile.0) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    Transform::from_xyz(0.0, 0.0, z),
                ));
            }
        });
    }
}

fn update_ground_art(
    board: Res<Board>,
    turn: Res<TurnState>,
    effects: Option<Res<effects::Effects>>,
    mut tiles: Query<(&GroundTile, &mut Sprite), Without<GroundArt>>,
    mut art: Query<(&GroundArt, &mut Sprite, &mut Visibility), Without<GroundTile>>,
) {
    if !board.is_changed()
        && !turn.is_changed()
        && !effects.as_ref().is_some_and(|e| e.is_changed())
    {
        return;
    }
    for (tile, mut sprite) in &mut tiles {
        sprite.rect = Some(tile_rect(ground_index(
            animated_ground(&board, &turn, effects.as_deref(), tile.0)
                .expect("terrain tile must be in bounds"),
        )));
        sprite.color = Color::WHITE;
    }
    for (piece, mut sprite, mut visibility) in &mut art {
        match piece.part {
            GroundArtPart::Border(side) => {
                *visibility = if let Some(rect) =
                    animated_border_rect(&board, &turn, effects.as_deref(), piece.cell, side)
                {
                    sprite.rect = Some(rect);
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            GroundArtPart::Overlay(effect) => {
                let present = effects
                    .as_ref()
                    .and_then(|effects| effects.held(piece.cell))
                    .map_or_else(
                        || effect.present(&board, piece.cell),
                        |old| match effect {
                            Effect::Wet => old.wet,
                            Effect::Ember => old.fuel,
                            Effect::Healing => old.healing,
                            _ => effect.present(&board, piece.cell),
                        },
                    );
                *visibility = if present {
                    sprite.rect = Some(animated_effect_rect(
                        effect,
                        piece.cell,
                        &board,
                        effects.as_deref(),
                    ));
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

type EffectMarkers = Or<(
    With<FrozenTile>,
    With<PoisonTile>,
    With<AcidTile>,
    With<VoidGrowthTile>,
    With<ThornTile>,
    With<ElectricFlash>,
    With<SteamFlash>,
    With<FireFlash>,
)>;

type EffectSprite = (
    &'static mut Sprite,
    &'static mut Visibility,
    &'static Transform,
    Option<&'static FrozenTile>,
    Option<&'static PoisonTile>,
    Option<&'static AcidTile>,
    Option<&'static VoidGrowthTile>,
    Option<&'static ThornTile>,
    Has<ElectricFlash>,
    Has<SteamFlash>,
    Has<FireFlash>,
);

fn update_effect_art(
    sheet: Res<GroundSheet>,
    board: Res<Board>,
    effects: Option<Res<effects::Effects>>,
    mut sprites: Query<EffectSprite, EffectMarkers>,
) {
    for (
        mut sprite,
        mut visibility,
        transform,
        frozen,
        poison,
        acid,
        growth,
        thorn,
        electric,
        steam,
        fire,
    ) in &mut sprites
    {
        let (cell, effect) = if let Some(tile) = frozen {
            (tile.0, Effect::Frozen)
        } else if let Some(tile) = poison {
            (tile.0, Effect::Poison)
        } else if let Some(tile) = acid {
            (
                tile.0,
                if effects
                    .as_ref()
                    .and_then(|effects| effects.held(tile.0))
                    .map_or_else(
                        || board.is_electrified_acid(tile.0),
                        |old| old.electric_acid,
                    )
                {
                    Effect::ElectricAcid
                } else {
                    Effect::Acid
                },
            )
        } else if let Some(tile) = growth {
            (
                tile.0,
                if effects
                    .as_ref()
                    .and_then(|effects| effects.held(tile.0))
                    .map_or_else(|| board.terrain_growth_kind_at(tile.0), |old| old.growth)
                    == Some(TerrainGrowthKind::Stone)
                {
                    Effect::StoneGrowth
                } else {
                    Effect::VoidGrowth
                },
            )
        } else if let Some(tile) = thorn {
            (tile.0, Effect::Thorns)
        } else if let Some(cell) = crate::board_cell(transform.translation.truncate()) {
            let effect = if electric {
                if board.is_water(cell) {
                    Effect::ElectricWater
                } else {
                    Effect::AdjacentShock
                }
            } else if steam {
                Effect::Steam
            } else if fire {
                Effect::Fire
            } else {
                unreachable!("effect query requires a marker")
            };
            (cell, effect)
        } else {
            continue;
        };
        if frozen.is_some() || poison.is_some() || acid.is_some() {
            *visibility = if effects::visible_effect(
                effects.as_deref(),
                cell,
                frozen.is_some(),
                poison.is_some(),
                acid.is_some(),
                true,
            ) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        let alpha = if electric || steam || fire {
            sprite.color.to_srgba().alpha
        } else {
            1.0
        };
        sprite.image = sheet.effects.clone();
        sprite.rect = Some(animated_effect_rect(
            effect,
            cell,
            &board,
            effects.as_deref(),
        ));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        sprite.custom_size = Some(Vec2::splat(GRID_BLOCK_SIZE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{COLUMNS, ROWS};

    #[test]
    fn terrain_edges_match_each_sheet_group_and_direction() {
        let mut board = Board::default();
        let turn = TurnState::default();
        let cell = IVec2::new(4, 3);
        for (center, neighbor, first_column) in [
            (Ground::Water, Ground::Solid, 4),
            (Ground::Water, Ground::Ridge, 8),
            (Ground::Solid, Ground::Ridge, 12),
            (Ground::Hole, Ground::Solid, 16),
            (Ground::Hole, Ground::Ridge, 20),
            (Ground::Hole, Ground::Water, 24),
        ] {
            board.set(cell, center);
            for (index, side) in Side::ALL.into_iter().enumerate() {
                board.set(cell + side.offset(), neighbor);
                assert_eq!(
                    border_rect(&board, &turn, cell, side),
                    Some(tile_rect(first_column + index as u32))
                );
                board.set(cell + side.offset(), center);
                assert_eq!(border_rect(&board, &turn, cell, side), None);
            }
        }
        assert_eq!(border_rect(&board, &turn, IVec2::ZERO, Side::Top), None);
        assert_eq!(border_rect(&board, &turn, IVec2::ZERO, Side::Left), None);
    }

    #[test]
    fn connected_effects_use_the_47_mask_layout_and_stay_in_bounds() {
        let cell = IVec2::new(4, 3);
        let mut board = Board::default();
        board.add_poison(cell);
        assert_eq!(
            effect_rect(Effect::Poison, cell, &board),
            Rect::new(0.0, 2208.0, 32.0, 2240.0)
        );
        for y in -1..=1 {
            for x in -1..=1 {
                board.add_poison(cell + IVec2::new(x, y));
            }
        }
        assert_eq!(
            effect_rect(Effect::Poison, cell, &board),
            Rect::new(1472.0, 2208.0, 1504.0, 2240.0)
        );
        assert_eq!(
            (0..=u8::MAX)
                .filter(|&mask| normalize_mask(mask) == mask)
                .count(),
            47
        );
        assert_eq!(
            effect_rect(Effect::Wet, IVec2::ZERO, &board),
            Rect::new(0.0, 2080.0, 32.0, 2112.0)
        );
    }

    #[test]
    fn aquatic_animation_keeps_unflattened_ridges_visible_in_neighbor_joins() {
        let board = Board::default();
        let cell = IVec2::new(4, 3);
        let ridge = cell + IVec2::NEG_Y;
        let mut turn = TurnState::default();
        let before = border_rect(&board, &turn, cell, Side::Top);
        turn.aquatic_animation = Some(crate::AquaticAnimation {
            visuals: Vec::new(),
            creatures: Vec::new(),
            frames: Default::default(),
            swept: Default::default(),
            unflattened: [ridge].into_iter().collect(),
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        });
        assert_eq!(visible_ground(&board, &turn, ridge), Some(Ground::Ridge));
        assert_eq!(
            border_rect(&board, &turn, cell, Side::Top),
            Some(tile_rect(12))
        );
        turn.aquatic_animation = None;
        assert_eq!(border_rect(&board, &turn, cell, Side::Top), before);
    }

    #[test]
    fn board_changes_refresh_neighbor_art_and_effect_overlays() {
        let mut app = App::new();
        app.insert_resource(Board::default())
            .insert_resource(TurnState::default())
            .insert_resource(GroundSheet::default())
            .add_systems(Startup, (spawn_board_terrain, attach_ground_art).chain())
            .add_systems(Update, update_ground_art);
        app.update();

        let cell = IVec2::new(4, 3);
        let north = cell + IVec2::NEG_Y;
        let world = app.world_mut();
        let mut pieces = world.query::<(&GroundArt, &Sprite, &Visibility, &Transform)>();
        assert_eq!(pieces.iter(world).count(), COLUMNS * ROWS * 7);
        for side in Side::ALL {
            let piece = pieces
                .iter(world)
                .find(|(piece, _, _, _)| {
                    piece.cell == cell
                        && matches!(piece.part, GroundArtPart::Border(s) if s == side)
                })
                .unwrap();
            assert_eq!(piece.3.translation.z, 0.01 * (side.index() + 1) as f32);
        }
        world.resource_mut::<Board>().set(cell, Ground::Water);
        world.resource_mut::<Board>().set(north, Ground::Ridge);
        world
            .resource_mut::<Board>()
            .set(cell + IVec2::X, Ground::Ridge);
        world.resource_mut::<Board>().add_oil(cell);
        app.update();

        let world = app.world_mut();
        let mut tiles = world.query::<(&GroundTile, &Sprite)>();
        let base = tiles
            .iter(world)
            .find(|(tile, _)| tile.0 == cell)
            .unwrap()
            .1;
        assert_eq!(base.rect, Some(tile_rect(1)));
        assert_eq!(base.color, Color::WHITE);
        let mut pieces = world.query::<(&GroundArt, &Sprite, &Visibility, &Transform)>();
        for (side, column) in [
            (Side::Top, 8),
            (Side::Bottom, 5),
            (Side::Left, 6),
            (Side::Right, 11),
        ] {
            let edge = pieces
                .iter(world)
                .find(|(piece, _, _, _)| {
                    piece.cell == cell
                        && matches!(piece.part, GroundArtPart::Border(s) if s == side)
                })
                .unwrap();
            assert_eq!(edge.1.rect, Some(tile_rect(column)));
            assert_eq!(*edge.2, Visibility::Inherited);
        }
        let overlay = pieces
            .iter(world)
            .find(|(piece, _, _, _)| {
                piece.cell == cell && matches!(piece.part, GroundArtPart::Overlay(Effect::Ember))
            })
            .unwrap();
        assert_eq!(*overlay.2, Visibility::Inherited);
        assert_eq!(
            overlay.1.rect,
            Some(effect_rect(Effect::Ember, cell, world.resource()))
        );

        world.resource_mut::<Board>().remove_oil(cell);
        world.resource_mut::<Board>().set(north, Ground::Water);
        app.update();
        let world = app.world_mut();
        let mut pieces = world.query::<(&GroundArt, &Visibility)>();
        assert!(pieces.iter(world).any(|(piece, visibility)| {
            piece.cell == cell
                && matches!(piece.part, GroundArtPart::Overlay(Effect::Ember))
                && *visibility == Visibility::Hidden
        }));
        assert!(pieces.iter(world).any(|(piece, visibility)| {
            piece.cell == cell
                && matches!(piece.part, GroundArtPart::Border(Side::Top))
                && *visibility == Visibility::Hidden
        }));
    }

    #[test]
    fn terrain_and_hazards_reveal_after_their_animations() {
        let mut app = App::new();
        app.init_resource::<Board>()
            .init_resource::<TurnState>()
            .init_resource::<Time>()
            .init_resource::<effects::Effects>()
            .init_resource::<crate::FrozenSheet>()
            .init_resource::<GroundSheet>()
            .add_systems(Startup, (spawn_board_terrain, attach_ground_art).chain())
            .add_systems(
                Update,
                (
                    effects::advance,
                    effects::capture,
                    crate::update_frozen_tiles,
                    crate::update_poison_visuals,
                    crate::update_acid_visuals,
                    update_ground_art,
                    update_effect_art,
                )
                    .chain(),
            );
        app.update();
        let cell = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_poison(cell);
            board.add_acid(cell);
            assert!(board.freeze_water(cell));
        }

        let frozen = app
            .world_mut()
            .spawn((FrozenTile(cell), Sprite::default(), Transform::default()))
            .id();
        app.update();
        {
            let world = app.world_mut();
            let mut tiles = world.query::<(&GroundTile, &Sprite)>();
            assert_eq!(
                tiles
                    .iter(world)
                    .find(|(tile, _)| tile.0 == cell)
                    .unwrap()
                    .1
                    .rect,
                Some(tile_rect(0))
            );
            assert_eq!(world.query::<&PoisonTile>().iter(world).count(), 0);
            assert_eq!(world.query::<&AcidTile>().iter(world).count(), 0);
            assert_eq!(
                *world.get::<Visibility>(frozen).unwrap(),
                Visibility::Hidden
            );
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        app.update();
        let world = app.world_mut();
        let mut tiles = world.query::<(&GroundTile, &Sprite)>();
        assert_eq!(
            tiles
                .iter(world)
                .find(|(tile, _)| tile.0 == cell)
                .unwrap()
                .1
                .rect,
            Some(tile_rect(1))
        );
        assert_eq!(world.query::<&PoisonTile>().iter(world).count(), 1);
        assert_eq!(world.query::<&AcidTile>().iter(world).count(), 1);
        assert_eq!(
            *world.get::<Visibility>(frozen).unwrap(),
            Visibility::Inherited
        );
    }

    #[test]
    fn outgoing_poison_art_remains_until_incoming_water_and_poison_are_visible() {
        let source = IVec2::new(4, 3);
        let target = source + IVec2::X;
        let mut board = Board::default();
        board.make_water(source);
        board.add_poison(source);
        board.finish_turn([], []);
        board.finish_turn([], []);
        let mut app = App::new();
        app.insert_resource(board)
            .init_resource::<TurnState>()
            .init_resource::<Time>()
            .init_resource::<effects::Effects>()
            .init_resource::<GroundSheet>()
            .add_systems(Startup, (spawn_board_terrain, attach_ground_art).chain())
            .add_systems(
                Update,
                (
                    effects::advance,
                    effects::capture,
                    crate::update_poison_visuals,
                    update_ground_art,
                    update_effect_art,
                )
                    .chain(),
            );
        app.update();
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(target);
            board.finish_turn([], []);
            assert!(!board.is_poisoned(source));
            assert!(board.is_poisoned(target));
        }
        app.update();
        {
            let world = app.world_mut();
            let mut poison = world.query::<(&PoisonTile, &Visibility)>();
            let (tile, visibility) = poison.single(world).unwrap();
            assert_eq!(tile.0, source);
            assert_eq!(*visibility, Visibility::Inherited);
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0));
        app.update();
        let world = app.world_mut();
        let mut poison = world.query::<(&PoisonTile, &Visibility)>();
        let (tile, visibility) = poison.single(world).unwrap();
        assert_eq!(tile.0, target);
        assert_eq!(*visibility, Visibility::Inherited);
    }

    #[test]
    fn hazard_art_switches_when_acid_is_electrified() {
        let mut app = App::new();
        app.insert_resource(Board::default())
            .insert_resource(GroundSheet::default())
            .add_systems(Update, update_effect_art);
        let cell = IVec2::new(4, 3);
        app.world_mut().resource_mut::<Board>().add_acid(cell);
        let entity = app
            .world_mut()
            .spawn((AcidTile(cell), Sprite::default(), Transform::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            Some(effect_rect(Effect::Acid, cell, app.world().resource()))
        );
        app.world_mut().resource_mut::<Board>().electrify_acid(cell);
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            Some(effect_rect(
                Effect::ElectricAcid,
                cell,
                app.world().resource()
            ))
        );
    }

    #[test]
    fn transient_art_preserves_flash_intensity_across_updates() {
        let mut app = App::new();
        app.insert_resource(Board::default())
            .insert_resource(GroundSheet::default())
            .add_systems(Update, update_effect_art);
        let entity = app
            .world_mut()
            .spawn((
                FireFlash(Timer::from_seconds(1.0, TimerMode::Once)),
                Sprite {
                    color: Color::srgba(1.0, 0.35, 0.0, 0.35),
                    ..default()
                },
                Transform::from_translation(crate::cell_center(IVec2::new(4, 3)).extend(1.6)),
            ))
            .id();
        app.update();
        app.update();
        let sprite = app.world().get::<Sprite>(entity).unwrap();
        assert_eq!(sprite.color.to_srgba().alpha, 0.35);
        assert_eq!(
            sprite.rect,
            Some(effect_rect(
                Effect::Fire,
                IVec2::new(4, 3),
                app.world().resource()
            ))
        );
    }
}
