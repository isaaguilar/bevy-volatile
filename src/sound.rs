use bevy::{audio::Volume, prelude::*};

use crate::{Board, PlayerHealth, effects::Effect};
use volatile_online_protocol::SoundEffect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum Cue {
    Select,
    Reject,
    Pop,
    Whoosh,
    Water,
    Chime,
    Ice,
    Electric,
    Sizzle,
    Explosion,
    Stone,
    Organic,
    Impact,
    MenuOption,
}

const CUES: [(Cue, &str); 14] = [
    (Cue::Select, "sfx/select.ogg"),
    (Cue::Reject, "sfx/reject.ogg"),
    (Cue::Pop, "sfx/pop.ogg"),
    (Cue::Whoosh, "sfx/magicwhoosh.ogg"),
    (Cue::Water, "sfx/watersplash.ogg"),
    (Cue::Chime, "sfx/chime-01.ogg"),
    (Cue::Ice, "sfx/icecrackle.ogg"),
    (Cue::Electric, "sfx/electric.ogg"),
    (Cue::Sizzle, "sfx/sizzle-01.ogg"),
    (Cue::Explosion, "sfx/explosion.ogg"),
    (Cue::Stone, "sfx/stonerumble-01.ogg"),
    (Cue::Organic, "sfx/organic-01.ogg"),
    (Cue::Impact, "sfx/impact-01.ogg"),
    (Cue::MenuOption, "sfx/menuoption.ogg"),
];

#[derive(Resource, Default)]
pub(crate) struct SoundCues {
    pending: [bool; CUES.len()],
    previous_health: Option<[u8; 2]>,
    previous_aquatic: Vec<crate::aquatic::AquaticVisual>,
}

impl SoundCues {
    pub(crate) fn request(&mut self, cue: Cue) {
        self.pending[cue as usize] = true;
    }

    pub(crate) fn reset_match(&mut self) {
        self.previous_health = None;
        self.previous_aquatic.clear();
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending.fill(false);
    }

    pub(crate) fn take_gameplay_events(&mut self) -> Vec<SoundEffect> {
        CUES.iter()
            .filter_map(|(cue, _)| {
                if !std::mem::take(&mut self.pending[*cue as usize]) {
                    return None;
                }
                match cue {
                    Cue::Pop => Some(SoundEffect::Pop),
                    Cue::Whoosh => Some(SoundEffect::Whoosh),
                    Cue::Water => Some(SoundEffect::Water),
                    Cue::Chime => Some(SoundEffect::Chime),
                    Cue::Ice => Some(SoundEffect::Ice),
                    Cue::Electric => Some(SoundEffect::Electric),
                    Cue::Sizzle => Some(SoundEffect::Sizzle),
                    Cue::Explosion => Some(SoundEffect::Explosion),
                    Cue::Stone => Some(SoundEffect::Stone),
                    Cue::Organic => Some(SoundEffect::Organic),
                    Cue::Impact => Some(SoundEffect::Impact),
                    Cue::Select | Cue::Reject | Cue::MenuOption => None,
                }
            })
            .collect()
    }

    pub(crate) fn request_gameplay(&mut self, sound: SoundEffect) {
        self.request(match sound {
            SoundEffect::Pop => Cue::Pop,
            SoundEffect::Whoosh => Cue::Whoosh,
            SoundEffect::Water => Cue::Water,
            SoundEffect::Chime => Cue::Chime,
            SoundEffect::Ice => Cue::Ice,
            SoundEffect::Electric => Cue::Electric,
            SoundEffect::Sizzle => Cue::Sizzle,
            SoundEffect::Explosion => Cue::Explosion,
            SoundEffect::Stone => Cue::Stone,
            SoundEffect::Organic => Cue::Organic,
            SoundEffect::Impact => Cue::Impact,
        });
    }

    #[cfg(test)]
    pub(crate) fn take(&mut self, cue: Cue) -> bool {
        std::mem::take(&mut self.pending[cue as usize])
    }
}

pub(crate) fn request(cues: &mut Option<ResMut<SoundCues>>, cue: Cue) {
    if let Some(cues) = cues {
        cues.request(cue);
    }
}

pub(crate) fn effect_cue(effect: Effect) -> Option<Cue> {
    match effect {
        Effect::Water => Some(Cue::Water),
        Effect::Acid | Effect::Poison | Effect::Steam => Some(Cue::Sizzle),
        Effect::Freeze => Some(Cue::Ice),
        Effect::Electric => Some(Cue::Electric),
        Effect::RidgeDust | Effect::Void => Some(Cue::Stone),
        Effect::Explosion => Some(Cue::Explosion),
        Effect::Fuel => None,
    }
}

#[derive(Resource)]
struct SoundAssets([Handle<AudioSource>; CUES.len()]);

#[derive(Component)]
struct SoundPlayback;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<SoundCues>()
        .add_systems(Startup, load)
        .add_systems(Last, play);
}

fn load(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SoundAssets(CUES.map(|(_, path)| assets.load(path))));
}

fn play(
    mut commands: Commands,
    assets: Res<SoundAssets>,
    mut cues: ResMut<SoundCues>,
    settings: Res<crate::audio_settings::AudioSettings>,
    playing: Query<Entity, With<SoundPlayback>>,
    time: Res<Time>,
    mut last_played: Local<[Option<f64>; CUES.len()]>,
) {
    if !settings.sfx {
        cues.pending.fill(false);
        for entity in &playing {
            commands.entity(entity).despawn();
        }
        return;
    }
    let now = time.elapsed_secs_f64();
    for (index, pending) in cues.pending.iter_mut().enumerate() {
        if !std::mem::take(pending) {
            continue;
        }
        if CUES[index].0 != Cue::MenuOption
            && last_played[index].is_some_and(|last| now - last < 0.2)
        {
            continue;
        }
        last_played[index] = Some(now);
        let volume = match CUES[index].0 {
            Cue::Water => 2.0,
            Cue::Select => 1.4,
            Cue::Chime => 1.0,
            Cue::Explosion => 0.35,
            _ => 0.65,
        };
        commands.spawn((
            AudioPlayer::new(assets.0[index].clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
            SoundPlayback,
        ));
    }
}

pub(crate) fn health_changes(health: Res<PlayerHealth>, mut cues: ResMut<SoundCues>) {
    if let Some(old) = cues.previous_health {
        if health.0.iter().zip(old).any(|(&new, old)| new < old) {
            cues.request(Cue::Impact);
        }
        if health.0.iter().zip(old).any(|(&new, old)| new > old) {
            cues.request(Cue::Chime);
        }
    }
    cues.previous_health = Some(health.0);
}

pub(crate) fn aquatic_growth(board: Res<Board>, mut cues: ResMut<SoundCues>) {
    let current: Vec<_> = board.aquatic_visuals().collect();
    let mut emerged = false;
    let mut grew = false;
    for creature in &current {
        match cues
            .previous_aquatic
            .iter()
            .find(|old| old.id == creature.id)
        {
            None => emerged = true,
            Some(old) if old.lengths != creature.lengths => grew = true,
            _ => {}
        }
    }
    if emerged {
        cues.request(Cue::Water);
    }
    if grew {
        cues.request(Cue::Organic);
    }
    cues.previous_aquatic = current;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_option_sounds_play_on_rapid_changes_and_respect_mute() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<SoundCues>()
            .init_resource::<crate::audio_settings::AudioSettings>()
            .insert_resource(SoundAssets(std::array::from_fn(|_| Handle::default())))
            .add_systems(Update, play);
        for _ in 0..2 {
            app.world_mut()
                .resource_mut::<SoundCues>()
                .request(Cue::MenuOption);
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&SoundPlayback>()
                .iter(app.world())
                .count(),
            2
        );
        app.world_mut()
            .resource_mut::<crate::audio_settings::AudioSettings>()
            .sfx = false;
        app.world_mut()
            .resource_mut::<SoundCues>()
            .request(Cue::MenuOption);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundPlayback>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(
            !app.world_mut()
                .resource_mut::<SoundCues>()
                .take(Cue::MenuOption)
        );
    }

    #[test]
    fn disabling_sfx_stops_active_sounds_and_discards_muted_cues() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<SoundCues>()
            .init_resource::<crate::audio_settings::AudioSettings>()
            .insert_resource(SoundAssets(std::array::from_fn(|_| Handle::default())))
            .add_systems(Update, play);
        app.world_mut()
            .resource_mut::<SoundCues>()
            .request(Cue::Water);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundPlayback>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut()
            .resource_mut::<crate::audio_settings::AudioSettings>()
            .sfx = false;
        app.world_mut()
            .resource_mut::<SoundCues>()
            .request(Cue::Ice);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundPlayback>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(!app.world_mut().resource_mut::<SoundCues>().take(Cue::Ice));
        app.world_mut()
            .resource_mut::<crate::audio_settings::AudioSettings>()
            .sfx = true;
        app.world_mut()
            .resource_mut::<SoundCues>()
            .request(Cue::Ice);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundPlayback>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn reactions_use_shared_sounds_without_sizzling_ember() {
        assert_eq!(effect_cue(Effect::Explosion), Some(Cue::Explosion));
        assert_eq!(effect_cue(Effect::Fuel), None);
        assert_eq!(effect_cue(Effect::Poison), Some(Cue::Sizzle));
        assert_eq!(effect_cue(Effect::Acid), Some(Cue::Sizzle));
    }

    #[test]
    fn repeated_requests_are_coalesced_by_cue() {
        let mut cues = SoundCues::default();
        cues.request(Cue::Explosion);
        cues.request(Cue::Explosion);
        cues.request(Cue::Water);
        assert_eq!(cues.pending.iter().filter(|&&pending| pending).count(), 2);
    }

    #[test]
    fn health_sounds_reflect_net_changes_and_skip_match_reset() {
        let mut app = App::new();
        app.init_resource::<PlayerHealth>()
            .init_resource::<SoundCues>()
            .add_systems(Update, health_changes);
        app.update();
        app.world_mut().resource_mut::<PlayerHealth>().0 = [9, 10];
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<SoundCues>()
                .take(Cue::Impact)
        );
        app.world_mut().resource_mut::<PlayerHealth>().0 = [10, 10];
        app.update();
        assert!(app.world_mut().resource_mut::<SoundCues>().take(Cue::Chime));
        app.world_mut().resource_mut::<SoundCues>().reset_match();
        app.world_mut().resource_mut::<PlayerHealth>().0 = [7, 10];
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<SoundCues>()
                .take(Cue::Impact)
        );
    }
}
