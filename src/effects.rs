use std::collections::{HashMap, HashSet};

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

use crate::{
    Board, COLUMNS, ElectricFlash, FireFlash, GRID_BLOCK_SIZE, Ground, ROWS, SteamFlash, TurnState,
    board_cell, cell_center,
};

const FRAME_SECONDS: f32 = 0.15;
const FRAMES: usize = 6;
const DURATION: f32 = FRAME_SECONDS * FRAMES as f32;
const FRAME_SIZE: f32 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Effect {
    Water,
    Acid,
    Poison,
    Fuel,
    Freeze,
    Electric,
    RidgeDust,
    Void,
    Steam,
    Explosion,
}

impl Effect {
    fn row(self) -> usize {
        match self {
            Self::Water => 0,
            Self::Acid => 1,
            Self::Poison => 2,
            Self::Fuel => 3,
            Self::Freeze => 4,
            Self::Electric => 5,
            Self::RidgeDust => 6,
            Self::Void => 7,
            Self::Steam => 8,
            Self::Explosion => 9,
        }
    }

    fn rect(self, frame: usize) -> Rect {
        let x = frame as f32 * FRAME_SIZE;
        let y = self.row() as f32 * FRAME_SIZE;
        Rect::new(x, y, x + FRAME_SIZE, y + FRAME_SIZE)
    }
}

#[derive(Component)]
pub(crate) struct EffectSignal(pub IVec2, pub Effect);

#[derive(Component)]
pub(crate) struct Captured;

#[derive(Clone, Copy)]
pub(crate) struct TileState {
    pub ground: Ground,
    pub frozen: bool,
    pub poison: bool,
    pub acid: bool,
    pub electric_acid: bool,
    pub fuel: bool,
}

impl TileState {
    fn read(board: &Board, turn: &TurnState, cell: IVec2) -> Self {
        Self {
            ground: if turn
                .aquatic_animation
                .as_ref()
                .is_some_and(|animation| animation.unflattened.contains(&cell))
            {
                Ground::Ridge
            } else {
                board.ground(cell).expect("effect cell must be in bounds")
            },
            frozen: board.is_frozen(cell),
            poison: board.is_poisoned(cell),
            acid: board.is_acidic(cell),
            electric_acid: board.is_electrified_acid(cell),
            fuel: board.has_oil(cell),
        }
    }
}

struct Playing {
    cell: IVec2,
    entity: Entity,
    effect: Effect,
    elapsed: f32,
    delay: f32,
}

#[derive(Resource, Default)]
pub(crate) struct Effects {
    image: Handle<Image>,
    previous: Option<Vec<TileState>>,
    held: HashMap<IVec2, TileState>,
    playing: Vec<Playing>,
}

impl Effects {
    pub(crate) fn reset(&mut self) {
        self.previous = None;
        self.held.clear();
        self.playing.clear();
    }

    pub(crate) fn held(&self, cell: IVec2) -> Option<TileState> {
        self.held.get(&cell).copied()
    }

    pub(crate) fn animating(&self, cell: IVec2, effect: Effect) -> bool {
        self.playing
            .iter()
            .any(|playing| playing.cell == cell && playing.effect == effect)
    }
}

pub(crate) fn load(mut effects: ResMut<Effects>, assets: Res<AssetServer>) {
    effects.image = assets
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load("effects-Sheet.png");
}

fn cells() -> impl Iterator<Item = IVec2> {
    (0..ROWS).flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
}

type FlashQuery = (
    Entity,
    &'static Transform,
    Has<FireFlash>,
    Has<SteamFlash>,
    Has<ElectricFlash>,
);
type FlashFilter = (
    Or<(With<FireFlash>, With<SteamFlash>, With<ElectricFlash>)>,
    Without<Captured>,
);

pub(crate) fn capture(
    mut commands: Commands,
    board: Res<Board>,
    mut turn: ResMut<TurnState>,
    mut effects: ResMut<Effects>,
    flashes: Query<FlashQuery, FlashFilter>,
    signals: Query<(Entity, &EffectSignal)>,
) {
    let current: Vec<_> = cells()
        .map(|cell| TileState::read(&board, &turn, cell))
        .collect();
    let Some(previous) = effects.previous.replace(current.clone()) else {
        return;
    };
    let mut requested = HashSet::new();
    for (entity, transform, fire, steam, electric) in &flashes {
        commands
            .entity(entity)
            .insert((Captured, Visibility::Hidden));
        if let Some(cell) = board_cell(transform.translation.truncate()) {
            if fire {
                requested.insert((cell, Effect::Explosion));
            }
            if steam {
                requested.insert((cell, Effect::Steam));
            }
            if electric {
                requested.insert((cell, Effect::Electric));
            }
        }
    }
    for (entity, signal) in &signals {
        commands.entity(entity).despawn();
        requested.insert((signal.0, signal.1));
    }
    for (index, cell) in cells().enumerate() {
        let old = previous[index];
        let new = current[index];
        match (old.ground, new.ground) {
            (_, Ground::Water) if old.ground != Ground::Water => {
                requested.insert((cell, Effect::Water));
            }
            (_, Ground::Ridge) if old.ground != Ground::Ridge => {
                requested.insert((cell, Effect::RidgeDust));
            }
            (_, Ground::Hole) if old.ground != Ground::Hole => {
                requested.insert((cell, Effect::Void));
            }
            (Ground::Ridge | Ground::Hole, Ground::Solid) => {
                requested.insert((cell, Effect::RidgeDust));
            }
            _ => {}
        }
        if !old.frozen && new.frozen {
            requested.insert((cell, Effect::Freeze));
        }
        if !old.poison && new.poison {
            requested.insert((cell, Effect::Poison));
        }
        if !old.acid && new.acid {
            requested.insert((cell, Effect::Acid));
        }
        if !old.electric_acid && new.electric_acid {
            requested.insert((cell, Effect::Electric));
        }
        if !old.fuel && new.fuel {
            requested.insert((cell, Effect::Fuel));
        }
    }
    for &(cell, effect) in &requested {
        if effects
            .playing
            .iter()
            .any(|playing| playing.cell == cell && playing.effect == effect)
        {
            continue;
        }
        let index = cell.y as usize * COLUMNS + cell.x as usize;
        if let Some(old) = previous.get(index) {
            effects.held.entry(cell).or_insert(*old);
        }
        let delay = if matches!(effect, Effect::RidgeDust | Effect::Void)
            && requested.contains(&(cell, Effect::Explosion))
        {
            DURATION
        } else {
            0.0
        };
        let center = cell_center(cell);
        let entity = commands
            .spawn((
                Sprite {
                    image: effects.image.clone(),
                    rect: Some(effect.rect(0)),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
                    ..default()
                },
                if delay > 0.0 {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                },
                Transform::from_xyz(center.x, center.y, 1.7 + effect.row() as f32 * 0.001),
            ))
            .id();
        effects.playing.push(Playing {
            cell,
            entity,
            effect,
            elapsed: 0.0,
            delay,
        });
    }
    turn.effects_active = !effects.playing.is_empty();
}

pub(crate) fn advance(
    time: Res<Time>,
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    mut effects: ResMut<Effects>,
    mut sprites: Query<(&mut Sprite, &mut Visibility)>,
) {
    if effects.playing.is_empty() {
        return;
    }
    effects.playing.retain_mut(|playing| {
        playing.elapsed += time.delta_secs();
        let elapsed = playing.elapsed - playing.delay;
        if elapsed >= DURATION {
            commands.entity(playing.entity).despawn();
            return false;
        }
        if let Ok((mut sprite, mut visibility)) = sprites.get_mut(playing.entity) {
            *visibility = if elapsed >= 0.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if elapsed >= 0.0 {
                let frame = (elapsed / FRAME_SECONDS) as usize;
                sprite.rect = Some(playing.effect.rect(frame.min(FRAMES - 1)));
            }
        }
        true
    });
    let active_cells: HashSet<_> = effects.playing.iter().map(|playing| playing.cell).collect();
    effects.held.retain(|cell, _| active_cells.contains(cell));
    turn.effects_active = !effects.playing.is_empty();
}

pub(crate) fn visible_effect(
    effects: Option<&Effects>,
    cell: IVec2,
    frozen: bool,
    poison: bool,
    acid: bool,
    current: bool,
) -> bool {
    effects
        .and_then(|effects| effects.held(cell))
        .map_or(current, |old| {
            if frozen {
                old.frozen
            } else if poison {
                old.poison
            } else if acid {
                old.acid
            } else {
                current
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Board>()
            .init_resource::<TurnState>()
            .init_resource::<Time>()
            .init_resource::<Effects>()
            .add_systems(Update, (advance, capture).chain());
        app.update();
        app
    }

    fn tick(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::ZERO);
    }

    #[test]
    fn water_and_overlapping_hazards_animate_before_the_new_visuals() {
        let mut app = app();
        let cell = IVec2::new(4, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_water(cell);
            board.add_acid(cell);
            board.add_poison(cell);
            board.electrify_acid(cell);
            assert!(board.freeze_water(cell));
        }
        app.update();
        let effects = app.world().resource::<Effects>();
        let old = effects.held(cell).unwrap();
        assert_eq!(old.ground, Ground::Solid);
        assert!(!old.acid && !old.poison && !old.frozen && !old.electric_acid);
        let kinds: HashSet<_> = effects
            .playing
            .iter()
            .map(|playing| playing.effect)
            .collect();
        assert_eq!(
            kinds,
            HashSet::from([
                Effect::Water,
                Effect::Acid,
                Effect::Poison,
                Effect::Freeze,
                Effect::Electric,
            ])
        );
        assert!(effects.playing.iter().all(|playing| playing.delay == 0.0));
        let water = effects
            .playing
            .iter()
            .find(|playing| playing.effect == Effect::Water)
            .unwrap()
            .entity;
        assert_eq!(
            app.world().get::<Sprite>(water).unwrap().rect,
            Some(Effect::Water.rect(0))
        );
        assert!(app.world().resource::<TurnState>().animation_active());
        tick(&mut app, 0.15);
        assert_eq!(
            app.world().get::<Sprite>(water).unwrap().rect,
            Some(Effect::Water.rect(1))
        );
        tick(&mut app, 0.751);
        assert!(app.world().resource::<Effects>().held(cell).is_none());
        assert!(!app.world().resource::<TurnState>().animation_active());
        assert!(app.world().get_entity(water).is_err());
    }

    #[test]
    fn explosion_precedes_the_resulting_hole_or_ridge() {
        for ground in [Ground::Hole, Ground::Ridge] {
            let mut app = app();
            let cell = IVec2::new(4, 3);
            app.world_mut().resource_mut::<Board>().set(cell, ground);
            app.world_mut().spawn((
                FireFlash(Timer::from_seconds(1.0, TimerMode::Once)),
                Transform::from_translation(cell_center(cell).extend(0.0)),
            ));
            app.update();
            let effects = app.world().resource::<Effects>();
            let result = if ground == Ground::Hole {
                Effect::Void
            } else {
                Effect::RidgeDust
            };
            let transition = effects
                .playing
                .iter()
                .find(|playing| playing.effect == result)
                .unwrap();
            let entity = transition.entity;
            assert_eq!(transition.delay, DURATION);
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Hidden
            );
            tick(&mut app, DURATION);
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Visible
            );
            assert_eq!(
                app.world().resource::<Effects>().held(cell).unwrap().ground,
                Ground::Solid
            );
            assert!(app.world().resource::<TurnState>().animation_active());
            tick(&mut app, DURATION);
            assert!(app.world().resource::<Effects>().held(cell).is_none());
            assert!(!app.world().resource::<TurnState>().animation_active());
        }
    }
}
