use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use bevy_rand::prelude::WyRand;
use rand::{RngExt, SeedableRng};

use crate::{
    AcidTile, Board, FrozenTile, GRID_BLOCK_SIZE, Ground, GroundTile, PoisonTile,
    TerrainGrowthKind, ThornTile, TurnState, VoidGrowthTile, effects, help, menu,
    spawn_board_terrain,
};

const TERRAIN_SHEET: &str = "basictilesheet.png";
const EFFECT_SHEET: &str = "ground-effects-Sheet.png";
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
}

impl Effect {
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
                .after(crate::setup)
                .after(load_sheet),
        )
        .add_systems(
            Update,
            (
                update_ground_art,
                update_effect_art,
                spawn_poison_bubbles,
                animate_poison_bubbles,
            )
                .chain()
                .after(crate::AquaticVisualSystems)
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

// Aseprite omitted transparent frames and packed repeated frames into the same atlas cell.
fn atlas_rect(frame: u32) -> Option<Rect> {
    let index = match frame {
        0 | 1 => frame,
        2..=4 => 2,
        5 => 1,
        6 => 3,
        7..=21 => frame - 3,
        22 => 18,
        37..=48 => frame - 18,
        59..=71 => frame - 28,
        73..=96 => frame - 29,
        _ => return None,
    };
    let x = (index % 9 * SOURCE_TILE) as f32;
    let y = (index / 9 * SOURCE_TILE) as f32;
    Some(Rect::new(x, y, x + 32.0, y + 32.0))
}

fn effect_frame(effect: Effect, elapsed: f32) -> Option<u32> {
    Some(match effect {
        Effect::Wet => return None,
        Effect::Ember => {
            let time = elapsed.rem_euclid(0.75);
            if time < 0.4 {
                (time / 0.1) as u32
            } else if time < 0.65 {
                4
            } else {
                5
            }
        }
        Effect::Frozen => {
            let time = elapsed.rem_euclid(1.75);
            if time < 1.25 {
                6
            } else {
                7 + ((time - 1.25) / 0.1) as u32
            }
        }
        Effect::Poison => 12,
        Effect::Acid => 73 + ((elapsed / 0.333) as u32 % 6),
        Effect::ElectricAcid => 79 + ((elapsed / 0.333) as u32 % 6),
        Effect::Healing => 85 + ((elapsed / 0.333) as u32 % 9),
        Effect::Thorns => 94,
        Effect::VoidGrowth => 95,
        Effect::StoneGrowth => 96,
    })
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
                        image: if effect == Effect::Wet {
                            default()
                        } else {
                            sheet.effects.clone()
                        },
                        rect: effect_frame(effect, 0.0).and_then(atlas_rect),
                        color: if effect == Effect::Wet {
                            Color::srgba(0.12, 0.45, 0.75, 0.35)
                        } else {
                            Color::WHITE
                        },
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
    time: Res<Time>,
    effects: Option<Res<effects::Effects>>,
    mut tiles: Query<(&GroundTile, &mut Sprite), Without<GroundArt>>,
    mut art: Query<(&GroundArt, &mut Sprite, &mut Visibility), Without<GroundTile>>,
) {
    let changed = board.is_changed()
        || turn.is_changed()
        || effects.as_ref().is_some_and(|effects| effects.is_changed());
    if changed {
        for (tile, mut sprite) in &mut tiles {
            sprite.rect = Some(tile_rect(ground_index(
                animated_ground(&board, &turn, effects.as_deref(), tile.0)
                    .expect("terrain tile must be in bounds"),
            )));
            sprite.color = Color::WHITE;
        }
    }
    for (piece, mut sprite, mut visibility) in &mut art {
        match piece.part {
            GroundArtPart::Border(side) => {
                if !changed {
                    continue;
                }
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
                let next_visibility = if present {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != next_visibility {
                    *visibility = next_visibility;
                }
                if present {
                    let rect = effect_frame(effect, time.elapsed_secs()).and_then(atlas_rect);
                    if sprite.rect != rect {
                        sprite.rect = rect;
                    }
                }
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
)>;

type EffectSprite = (
    &'static mut Sprite,
    &'static mut Visibility,
    Option<&'static FrozenTile>,
    Option<&'static PoisonTile>,
    Option<&'static AcidTile>,
    Option<&'static VoidGrowthTile>,
    Option<&'static ThornTile>,
);

fn update_effect_art(
    sheet: Res<GroundSheet>,
    board: Res<Board>,
    time: Res<Time>,
    effects: Option<Res<effects::Effects>>,
    mut sprites: Query<EffectSprite, EffectMarkers>,
) {
    for (mut sprite, mut visibility, frozen, poison, acid, growth, thorn) in &mut sprites {
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
        } else {
            unreachable!("effect query requires a marker")
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
        sprite.image = sheet.effects.clone();
        sprite.rect = effect_frame(effect, time.elapsed_secs()).and_then(atlas_rect);
        sprite.color = Color::WHITE;
        sprite.custom_size = Some(Vec2::splat(GRID_BLOCK_SIZE));
    }
}

#[derive(Component)]
struct PoisonBubblesAttached;

#[derive(Component)]
struct PoisonBubble {
    first_frame: u32,
    elapsed: f32,
    rng: WyRand,
}

fn spawn_poison_bubbles(
    mut commands: Commands,
    sheet: Res<GroundSheet>,
    time: Res<Time>,
    tiles: Query<(Entity, &PoisonTile), Without<PoisonBubblesAttached>>,
) {
    for (entity, tile) in &tiles {
        commands
            .entity(entity)
            .insert(PoisonBubblesAttached)
            .with_children(|parent| {
                for (layer, first_frame) in [13, 33, 53].into_iter().enumerate() {
                    let seed = time.elapsed().as_nanos() as u64
                        ^ ((tile.0.x as u64) << 32)
                        ^ ((tile.0.y as u64) << 16)
                        ^ (layer as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15);
                    let mut rng = WyRand::seed_from_u64(seed);
                    let elapsed = -rng.random_range(0.0..2.0);
                    parent.spawn((
                        PoisonBubble {
                            first_frame,
                            elapsed,
                            rng,
                        },
                        Sprite {
                            image: sheet.effects.clone(),
                            rect: atlas_rect(first_frame),
                            custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                            ..default()
                        },
                        Visibility::Hidden,
                        Transform::from_xyz(0.0, 0.0, 0.001 * (layer + 1) as f32),
                    ));
                }
            });
    }
}

fn animate_poison_bubbles(
    time: Res<Time>,
    mut bubbles: Query<(&mut PoisonBubble, &mut Sprite, &mut Visibility)>,
) {
    for (mut bubble, mut sprite, mut visibility) in &mut bubbles {
        bubble.elapsed += time.delta_secs();
        while bubble.elapsed >= 2.0 {
            bubble.elapsed -= 2.0 + bubble.rng.random_range(0.3..2.0);
        }
        let frame = (bubble.elapsed >= 0.0)
            .then(|| bubble.first_frame + (bubble.elapsed / 0.1) as u32)
            .and_then(atlas_rect);
        if let Some(rect) = frame {
            sprite.rect = Some(rect);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{COLUMNS, FireFlash, InventoryImages, ROWS};

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
    fn atlas_uses_packed_and_transparent_frames() {
        assert_eq!(atlas_rect(2), atlas_rect(4));
        assert_eq!(atlas_rect(1), atlas_rect(5));
        assert_eq!(atlas_rect(21), atlas_rect(22));
        assert_eq!(atlas_rect(23), None);
        assert_eq!(atlas_rect(33), None);
        assert_eq!(atlas_rect(37), Some(Rect::new(32.0, 64.0, 64.0, 96.0)));
        assert_eq!(atlas_rect(72), None);
        assert_eq!(atlas_rect(96), Some(Rect::new(128.0, 224.0, 160.0, 256.0)));
    }

    #[test]
    fn animated_overlays_use_exported_frame_durations() {
        assert_eq!(effect_frame(Effect::Ember, 0.41), Some(4));
        assert_eq!(effect_frame(Effect::Ember, 0.66), Some(5));
        assert_eq!(effect_frame(Effect::Ember, 0.76), Some(0));
        assert_eq!(effect_frame(Effect::Frozen, 1.24), Some(6));
        assert_eq!(effect_frame(Effect::Frozen, 1.26), Some(7));
        assert_eq!(effect_frame(Effect::Frozen, 1.76), Some(6));
        assert_eq!(effect_frame(Effect::Acid, 0.34), Some(74));
        assert_eq!(effect_frame(Effect::ElectricAcid, 0.34), Some(80));
        assert_eq!(effect_frame(Effect::Healing, 0.34), Some(86));
    }

    #[test]
    fn poison_bubbles_are_independent_layers_and_disappear_with_their_tile() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GroundSheet>()
            .add_systems(
                Update,
                (spawn_poison_bubbles, animate_poison_bubbles).chain(),
            );
        let parent = app
            .world_mut()
            .spawn((
                PoisonTile(IVec2::new(4, 3)),
                Sprite::default(),
                Visibility::Inherited,
                Transform::default(),
            ))
            .id();
        app.update();
        let entities: Vec<_> = {
            let world = app.world_mut();
            world
                .query::<(Entity, &PoisonBubble)>()
                .iter(world)
                .map(|(entity, bubble)| (entity, bubble.first_frame, bubble.elapsed))
                .collect()
        };
        assert_eq!(entities.len(), 3);
        assert_ne!(entities[0].2, entities[1].2);
        assert_ne!(entities[1].2, entities[2].2);
        for &(entity, first, _) in &entities {
            let elapsed = match first {
                13 => 0.01,
                33 => 0.41,
                53 => 0.61,
                _ => unreachable!(),
            };
            app.world_mut()
                .get_mut::<PoisonBubble>(entity)
                .unwrap()
                .elapsed = elapsed;
        }
        app.update();
        for &(entity, first, _) in &entities {
            let frame = match first {
                13 => 13,
                33 => 37,
                53 => 59,
                _ => unreachable!(),
            };
            assert_eq!(
                app.world().get::<Sprite>(entity).unwrap().rect,
                atlas_rect(frame)
            );
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Inherited
            );
        }
        app.world_mut()
            .get_mut::<PoisonBubble>(entities[0].0)
            .unwrap()
            .elapsed = 1.1;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(entities[0].0).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(entities[1].0).unwrap(),
            Visibility::Inherited
        );
        app.world_mut().despawn(parent);
        for (entity, _, _) in entities {
            assert!(app.world().get_entity(entity).is_err());
        }
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
            .init_resource::<Time>()
            .init_resource::<InventoryImages>()
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
        assert_eq!(overlay.1.rect, atlas_rect(0));

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
    fn ridge_ember_overlay_tracks_fuel_and_effect_animation() {
        let cell = IVec2::new(4, 3);
        let mut board = Board::default();
        board.set(cell, Ground::Ridge);
        let mut app = App::new();
        app.insert_resource(board)
            .init_resource::<TurnState>()
            .init_resource::<Time>()
            .init_resource::<effects::Effects>()
            .init_resource::<GroundSheet>()
            .init_resource::<InventoryImages>()
            .add_systems(Startup, (spawn_board_terrain, attach_ground_art).chain())
            .add_systems(
                Update,
                (effects::advance, effects::capture, update_ground_art).chain(),
            );
        app.update();

        let marker = {
            let world = app.world_mut();
            let mut pieces = world.query::<(Entity, &GroundArt)>();
            pieces
                .iter(world)
                .find(|(_, piece)| {
                    piece.cell == cell
                        && matches!(piece.part, GroundArtPart::Overlay(Effect::Ember))
                })
                .unwrap()
                .0
        };
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            app.world().get::<Sprite>(marker).unwrap().rect,
            atlas_rect(0)
        );

        app.world_mut().resource_mut::<Board>().add_oil(cell);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Hidden
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Inherited
        );

        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Solid);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Ridge);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Inherited
        );

        app.world_mut().resource_mut::<Board>().remove_oil(cell);
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.5));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(marker).unwrap(),
            Visibility::Hidden
        );
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
            .init_resource::<InventoryImages>()
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
            .init_resource::<InventoryImages>()
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
            .init_resource::<Time>()
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
            atlas_rect(73)
        );
        app.world_mut().resource_mut::<Board>().electrify_acid(cell);
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            atlas_rect(79)
        );
    }

    #[test]
    fn transient_art_preserves_flash_intensity_across_updates() {
        let mut app = App::new();
        app.insert_resource(Board::default())
            .insert_resource(GroundSheet::default())
            .init_resource::<Time>()
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
        assert_eq!(sprite.rect, None);
    }
}
