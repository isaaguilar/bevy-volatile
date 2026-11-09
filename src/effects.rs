use std::collections::{HashMap, HashSet};

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

use crate::{
    Board, COLUMNS, ElectricFlash, FireFlash, GRID_BLOCK_SIZE, Ground, ROWS, SteamFlash,
    TerrainGrowthKind, TurnState, board_cell, cell_center, sound,
};

const FRAME_SECONDS: f32 = 0.15;
const FRAMES: usize = 6;
pub(crate) const DURATION: f32 = FRAME_SECONDS * FRAMES as f32;
const FRAME_SIZE: f32 = 48.0;

fn overlap_delay(frames: usize) -> f32 {
    frames.div_ceil(2) as f32 * FRAME_SECONDS
}

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

impl From<Effect> for volatile_online_protocol::VisualEffect {
    fn from(effect: Effect) -> Self {
        use volatile_online_protocol::VisualEffect;
        match effect {
            Effect::Water => VisualEffect::Water,
            Effect::Acid => VisualEffect::Acid,
            Effect::Poison => VisualEffect::Poison,
            Effect::Fuel => VisualEffect::Fuel,
            Effect::Freeze => VisualEffect::Freeze,
            Effect::Electric => VisualEffect::Electric,
            Effect::RidgeDust => VisualEffect::RidgeDust,
            Effect::Void => VisualEffect::Void,
            Effect::Steam => VisualEffect::Steam,
            Effect::Explosion => VisualEffect::Explosion,
        }
    }
}

impl From<volatile_online_protocol::VisualEffect> for Effect {
    fn from(effect: volatile_online_protocol::VisualEffect) -> Self {
        use volatile_online_protocol::VisualEffect;
        match effect {
            VisualEffect::Water => Effect::Water,
            VisualEffect::Acid => Effect::Acid,
            VisualEffect::Poison => Effect::Poison,
            VisualEffect::Fuel => Effect::Fuel,
            VisualEffect::Freeze => Effect::Freeze,
            VisualEffect::Electric => Effect::Electric,
            VisualEffect::RidgeDust => Effect::RidgeDust,
            VisualEffect::Void => Effect::Void,
            VisualEffect::Steam => Effect::Steam,
            VisualEffect::Explosion => Effect::Explosion,
        }
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
    pub wet: bool,
    pub healing: bool,
    pub thorns: bool,
    pub growth: Option<TerrainGrowthKind>,
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
            wet: board.is_wet(cell),
            healing: board.has_life_essence(cell),
            thorns: board.has_thorns(cell),
            growth: board.terrain_growth_kind_at(cell),
        }
    }
}

struct Playing {
    cell: IVec2,
    entity: Entity,
    effect: Effect,
    elapsed: f32,
    delay: f32,
    layer: f32,
}

#[derive(Default)]
struct DepartingEffects {
    frozen: bool,
    poison: bool,
    acid: bool,
    electric_acid: bool,
    fuel: bool,
    wet: bool,
    healing: bool,
    thorns: bool,
    growth: Option<TerrainGrowthKind>,
}

#[derive(Resource, Default)]
pub(crate) struct Effects {
    image: Handle<Image>,
    previous: Option<Vec<TileState>>,
    held: HashMap<IVec2, TileState>,
    departing: HashMap<IVec2, DepartingEffects>,
    playing: Vec<Playing>,
}

impl Effects {
    pub(crate) fn reset(&mut self) {
        self.previous = None;
        self.held.clear();
        self.departing.clear();
        self.playing.clear();
    }

    pub(crate) fn held(&self, cell: IVec2) -> Option<TileState> {
        let held = self.held.get(&cell).copied();
        let Some(departing) = self.departing.get(&cell) else {
            return held;
        };
        let index = cell.y as usize * COLUMNS + cell.x as usize;
        let mut state = held.unwrap_or_else(|| {
            self.previous
                .as_ref()
                .expect("departing effects require a board snapshot")[index]
        });
        state.frozen |= departing.frozen;
        state.poison |= departing.poison;
        state.acid |= departing.acid;
        state.electric_acid |= departing.electric_acid;
        state.fuel |= departing.fuel;
        state.wet |= departing.wet;
        state.healing |= departing.healing;
        state.thorns |= departing.thorns;
        state.growth = departing.growth.or(state.growth);
        Some(state)
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
    output: (
        Option<ResMut<sound::SoundCues>>,
        Option<ResMut<crate::headless::AnimationLog>>,
        Res<Time>,
    ),
) {
    let (mut sounds, mut animations, time) = output;
    let current: Vec<_> = cells()
        .map(|cell| TileState::read(&board, &turn, cell))
        .collect();
    let Some(previous) = effects.previous.replace(current.clone()) else {
        return;
    };
    let mut requested = HashMap::<(IVec2, Effect), usize>::new();
    for (entity, transform, fire, steam, electric) in &flashes {
        commands
            .entity(entity)
            .insert((Captured, Visibility::Hidden));
        if let Some(cell) = board_cell(transform.translation.truncate()) {
            if fire {
                *requested.entry((cell, Effect::Explosion)).or_default() += 1;
            }
            if steam {
                *requested.entry((cell, Effect::Steam)).or_default() += 1;
            }
            if electric {
                *requested.entry((cell, Effect::Electric)).or_default() += 1;
            }
        }
    }
    let mut signaled = HashMap::<(IVec2, Effect), usize>::new();
    for (entity, signal) in &signals {
        commands.entity(entity).despawn();
        *signaled.entry((signal.0, signal.1)).or_default() += 1;
    }
    for (key, count) in signaled {
        let requested_count = requested.entry(key).or_default();
        *requested_count = (*requested_count).max(count);
    }
    for (index, cell) in cells().enumerate() {
        let old = previous[index];
        let new = current[index];
        if old.frozen && !new.frozen {
            sound::request(&mut sounds, sound::Cue::Ice);
        }
        if !old.healing && new.healing {
            sound::request(&mut sounds, sound::Cue::Chime);
        }
        if old.thorns != new.thorns {
            sound::request(&mut sounds, sound::Cue::Organic);
        }
        match (old.ground, new.ground) {
            (_, Ground::Water) if old.ground != Ground::Water => {
                requested.entry((cell, Effect::Water)).or_insert(1);
            }
            (_, Ground::Ridge) if old.ground != Ground::Ridge => {
                requested.entry((cell, Effect::RidgeDust)).or_insert(1);
            }
            (_, Ground::Hole) if old.ground != Ground::Hole => {
                requested.entry((cell, Effect::Void)).or_insert(1);
            }
            (Ground::Ridge | Ground::Hole, Ground::Solid) => {
                requested.entry((cell, Effect::RidgeDust)).or_insert(1);
            }
            _ => {}
        }
        if !old.frozen && new.frozen {
            requested.entry((cell, Effect::Freeze)).or_insert(1);
        }
        if !old.poison && new.poison {
            requested.entry((cell, Effect::Poison)).or_insert(1);
        }
        if !old.acid && new.acid {
            requested.entry((cell, Effect::Acid)).or_insert(1);
        }
        if !old.electric_acid && new.electric_acid {
            requested.entry((cell, Effect::Electric)).or_insert(1);
        }
        if !old.fuel && new.fuel {
            requested.entry((cell, Effect::Fuel)).or_insert(1);
        }
    }
    if !requested.is_empty() || !effects.playing.is_empty() {
        for (index, cell) in cells().enumerate() {
            let old = previous[index];
            let new = current[index];
            let visible = effects.held.get(&cell).copied().unwrap_or(old);
            if (old.frozen && visible.frozen && !new.frozen)
                || (old.poison && visible.poison && !new.poison)
                || (old.acid && visible.acid && !new.acid)
                || (old.electric_acid && visible.electric_acid && !new.electric_acid)
                || (old.fuel && visible.fuel && !new.fuel)
                || (old.wet && visible.wet && !new.wet)
                || (old.healing && visible.healing && !new.healing)
                || (old.thorns && visible.thorns && !new.thorns)
                || (old.growth.is_some() && visible.growth.is_some() && new.growth.is_none())
            {
                let departing = effects.departing.entry(cell).or_default();
                departing.frozen |= old.frozen && visible.frozen && !new.frozen;
                departing.poison |= old.poison && visible.poison && !new.poison;
                departing.acid |= old.acid && visible.acid && !new.acid;
                departing.electric_acid |=
                    old.electric_acid && visible.electric_acid && !new.electric_acid;
                departing.fuel |= old.fuel && visible.fuel && !new.fuel;
                departing.wet |= old.wet && visible.wet && !new.wet;
                departing.healing |= old.healing && visible.healing && !new.healing;
                departing.thorns |= old.thorns && visible.thorns && !new.thorns;
                if old.growth.is_some() && visible.growth.is_some() && new.growth.is_none() {
                    departing.growth = visible.growth;
                }
            }
        }
    }
    let mut requested: Vec<_> = requested.into_iter().collect();
    requested.sort_by_key(|((cell, effect), _)| {
        (
            cell.y,
            cell.x,
            usize::from(*effect != Effect::Explosion),
            effect.row(),
        )
    });
    for ((cell, effect), count) in requested {
        let index = cell.y as usize * COLUMNS + cell.x as usize;
        if let Some(old) = previous.get(index) {
            effects.held.entry(cell).or_insert(*old);
        }
        for _ in 0..count {
            let predecessors = || {
                effects.playing.iter().filter(|playing| {
                    playing.cell == cell
                        && (playing.effect == effect
                            || (matches!(effect, Effect::RidgeDust | Effect::Void)
                                && playing.effect == Effect::Explosion))
                })
            };
            let delay = predecessors()
                .map(|playing| playing.delay - playing.elapsed + overlap_delay(FRAMES))
                .fold(0.0_f32, f32::max);
            if delay == 0.0
                && let Some(cue) = sound::effect_cue(effect)
            {
                sound::request(&mut sounds, cue);
            }
            let layer = predecessors()
                .map(|playing| playing.layer + 0.0001)
                .fold(1.7 + effect.row() as f32 * 0.001, f32::max);
            if let Some(log) = &mut animations {
                log.record(
                    time.elapsed() + std::time::Duration::from_secs_f32(delay),
                    volatile_online_protocol::Animation::Effect {
                        effect: effect.into(),
                        x: cell.x,
                        y: cell.y,
                    },
                );
            }
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
                    Transform::from_xyz(center.x, center.y, layer),
                ))
                .id();
            effects.playing.push(Playing {
                cell,
                entity,
                effect,
                elapsed: 0.0,
                delay,
                layer,
            });
        }
    }
    turn.effects_active = !effects.playing.is_empty();
}

pub(crate) fn visual_sprite(effects: &Effects, effect: Effect) -> Sprite {
    Sprite {
        image: effects.image.clone(),
        rect: Some(effect.rect(0)),
        custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE)),
        ..default()
    }
}

pub(crate) fn visual_layer(effect: Effect) -> f32 {
    1.7 + effect.row() as f32 * 0.001
}

pub(crate) fn visual_frame(sprite: &mut Sprite, effect: Effect, elapsed: f32) {
    let frame = (elapsed / FRAME_SECONDS) as usize;
    sprite.rect = Some(effect.rect(frame.min(FRAMES - 1)));
}

pub(crate) fn advance(
    time: Res<Time>,
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    mut effects: ResMut<Effects>,
    mut sprites: Query<(&mut Sprite, &mut Visibility)>,
    mut sounds: Option<ResMut<sound::SoundCues>>,
) {
    if effects.playing.is_empty() {
        return;
    }
    effects.playing.retain_mut(|playing| {
        let previous_elapsed = playing.elapsed;
        playing.elapsed += time.delta_secs();
        if previous_elapsed < playing.delay
            && playing.elapsed >= playing.delay
            && let Some(cue) = sound::effect_cue(playing.effect)
        {
            sound::request(&mut sounds, cue);
        }
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
    if effects.playing.is_empty() {
        effects.departing.clear();
    }
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
    fn sound_follows_effect_start_and_coalesces_multiple_tiles() {
        let mut app = app();
        app.init_resource::<sound::SoundCues>();
        let first = IVec2::new(4, 3);
        let second = IVec2::new(5, 3);
        app.world_mut().resource_mut::<Board>().make_water(first);
        app.world_mut().resource_mut::<Board>().make_water(second);
        app.update();
        let mut sounds = app.world_mut().resource_mut::<sound::SoundCues>();
        assert!(sounds.take(sound::Cue::Water));
        assert!(!sounds.take(sound::Cue::Water));
    }

    #[test]
    fn delayed_terrain_sound_waits_for_explosion_animation() {
        let mut app = app();
        app.init_resource::<sound::SoundCues>();
        let cell = IVec2::new(4, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Hole);
        app.world_mut().spawn((
            FireFlash(Timer::from_seconds(1.0, TimerMode::Once)),
            Transform::from_translation(cell_center(cell).extend(0.0)),
        ));
        app.update();
        {
            let mut sounds = app.world_mut().resource_mut::<sound::SoundCues>();
            assert!(sounds.take(sound::Cue::Explosion));
            assert!(!sounds.take(sound::Cue::Stone));
        }
        tick(&mut app, overlap_delay(FRAMES));
        assert!(
            app.world_mut()
                .resource_mut::<sound::SoundCues>()
                .take(sound::Cue::Stone)
        );
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
    fn terrain_animation_starts_halfway_through_explosion() {
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
            assert_eq!(transition.delay, overlap_delay(FRAMES));
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Hidden
            );
            tick(&mut app, overlap_delay(FRAMES) - 0.01);
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Hidden
            );
            tick(&mut app, 0.02);
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                Visibility::Visible
            );
            assert!(
                app.world()
                    .resource::<Effects>()
                    .animating(cell, Effect::Explosion)
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

    #[test]
    fn repeated_effects_overlap_and_terrain_waits_for_the_last_explosion() {
        let mut app = app();
        let cell = IVec2::new(4, 3);
        app.world_mut()
            .resource_mut::<Board>()
            .set(cell, Ground::Hole);
        for _ in 0..3 {
            app.world_mut().spawn((
                FireFlash(Timer::from_seconds(1.0, TimerMode::Once)),
                Transform::from_translation(cell_center(cell).extend(0.0)),
            ));
        }
        app.update();
        let effects = app.world().resource::<Effects>();
        let explosions: Vec<_> = effects
            .playing
            .iter()
            .filter(|playing| playing.effect == Effect::Explosion)
            .collect();
        assert_eq!(explosions.len(), 3);
        for (index, explosion) in explosions.iter().enumerate() {
            assert!((explosion.delay - index as f32 * overlap_delay(FRAMES)).abs() < 0.001);
        }
        assert!(explosions[0].layer < explosions[1].layer);
        assert!(explosions[1].layer < explosions[2].layer);
        let transition = effects
            .playing
            .iter()
            .find(|playing| playing.effect == Effect::Void)
            .unwrap();
        assert!((transition.delay - 3.0 * overlap_delay(FRAMES)).abs() < 0.001);
        assert!(transition.layer > explosions[2].layer);
        let second = explosions[1].entity;
        tick(&mut app, overlap_delay(FRAMES) + 0.01);
        assert_eq!(
            app.world().get::<Visibility>(second),
            Some(&Visibility::Visible)
        );
        assert!(app.world().resource::<TurnState>().animation_active());
        tick(&mut app, 3.0 * overlap_delay(FRAMES) + DURATION);
        assert!(app.world().resource::<Effects>().held(cell).is_none());
        assert!(!app.world().resource::<TurnState>().animation_active());
    }

    #[test]
    fn repeated_signal_during_animation_waits_only_until_halfway() {
        let mut app = app();
        let cell = IVec2::new(4, 3);
        app.world_mut().spawn(EffectSignal(cell, Effect::Freeze));
        app.update();
        tick(&mut app, FRAME_SECONDS);
        app.world_mut().spawn(EffectSignal(cell, Effect::Freeze));
        app.update();
        let effects = app.world().resource::<Effects>();
        let freezes: Vec<_> = effects
            .playing
            .iter()
            .filter(|playing| playing.effect == Effect::Freeze)
            .collect();
        assert_eq!(freezes.len(), 2);
        let second = freezes[1];
        assert!((second.delay - (overlap_delay(FRAMES) - FRAME_SECONDS)).abs() < 0.001);
        let entity = second.entity;
        let delay = second.delay;
        tick(&mut app, delay + 0.01);
        assert_eq!(
            app.world().get::<Visibility>(entity),
            Some(&Visibility::Visible)
        );
    }

    #[test]
    fn flash_and_signal_for_the_same_effect_do_not_duplicate_it() {
        let mut app = app();
        let cell = IVec2::new(4, 3);
        app.world_mut().spawn(EffectSignal(cell, Effect::Electric));
        app.world_mut().spawn((
            ElectricFlash(Timer::from_seconds(1.0, TimerMode::Once)),
            Transform::from_translation(cell_center(cell).extend(0.0)),
        ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Effects>()
                .playing
                .iter()
                .filter(|playing| playing.effect == Effect::Electric)
                .count(),
            1
        );
    }

    #[test]
    fn outgoing_tile_effects_wait_for_incoming_animations_elsewhere() {
        let mut app = app();
        let source = IVec2::new(4, 3);
        let target = IVec2::new(5, 3);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.make_wet(source);
            board.add_poison(source);
            board.add_acid(source);
            board.electrify_acid(source);
            board.add_oil(source);
            board.add_life_essence(source);
            assert!(board.freeze_water(source));
        }
        app.update();
        tick(&mut app, DURATION + 0.01);
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.thaw(source);
            board.clear_poison(source);
            board.clear_acid(source);
            board.remove_oil(source);
            board.dry(source);
            board.make_water(target);
        }
        app.update();
        let effects = app.world().resource::<Effects>();
        let old = effects.held(source).unwrap();
        assert!(old.frozen && old.poison && old.acid && old.electric_acid && old.fuel);
        assert!(old.wet && old.healing);
        assert_eq!(old.ground, Ground::Solid);
        assert!(effects.animating(target, Effect::Water));
        tick(&mut app, DURATION + 0.01);
        assert!(app.world().resource::<Effects>().held(source).is_none());
    }

    #[test]
    fn outgoing_effects_disappear_immediately_without_incoming_animations() {
        let mut app = app();
        let source = IVec2::new(4, 3);
        app.world_mut().resource_mut::<Board>().add_poison(source);
        app.update();
        tick(&mut app, DURATION + 0.01);
        app.world_mut().resource_mut::<Board>().clear_poison(source);
        app.update();
        assert!(app.world().resource::<Effects>().held(source).is_none());
        assert!(!app.world().resource::<TurnState>().effects_active);
    }

    #[test]
    fn an_effect_removed_before_reveal_is_not_made_visible() {
        let mut app = app();
        let source = IVec2::new(4, 3);
        let target = IVec2::new(5, 3);
        app.world_mut().resource_mut::<Board>().add_poison(source);
        app.update();
        assert!(
            !app.world()
                .resource::<Effects>()
                .held(source)
                .unwrap()
                .poison
        );
        {
            let mut board = app.world_mut().resource_mut::<Board>();
            board.clear_poison(source);
            board.make_water(target);
        }
        app.update();
        assert!(
            !app.world()
                .resource::<Effects>()
                .held(source)
                .unwrap()
                .poison
        );
        tick(&mut app, DURATION + 0.01);
        assert!(!app.world().resource::<Board>().is_poisoned(source));
        assert!(app.world().resource::<Effects>().held(source).is_none());
    }

    #[test]
    fn odd_frame_counts_round_halfway_up() {
        assert_eq!(overlap_delay(5), 3.0 * FRAME_SECONDS);
        assert_eq!(overlap_delay(6), 3.0 * FRAME_SECONDS);
        assert_eq!(overlap_delay(7), 4.0 * FRAME_SECONDS);
    }
}
