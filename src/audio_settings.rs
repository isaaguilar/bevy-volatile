use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    ui::Pressed,
    ui_widgets::Button,
};
use bevy_aspect_ratio_mask::Hud;

#[derive(Resource)]
pub(crate) struct AudioSettings {
    pub music: bool,
    pub sfx: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            music: true,
            sfx: true,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(crate) enum AudioChannel {
    Music,
    Sfx,
}

impl AudioSettings {
    pub(crate) fn enabled(&self, channel: AudioChannel) -> bool {
        match channel {
            AudioChannel::Music => self.music,
            AudioChannel::Sfx => self.sfx,
        }
    }

    pub(crate) fn toggle(&mut self, channel: AudioChannel) {
        match channel {
            AudioChannel::Music => self.music = !self.music,
            AudioChannel::Sfx => self.sfx = !self.sfx,
        }
    }
}

#[derive(Component)]
struct AudioToggle;

#[derive(Component)]
pub(crate) struct AudioToggleRow;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AudioHudInput;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<AudioSettings>()
        .add_systems(Startup, spawn_toggles.after(crate::setup))
        .add_systems(
            Update,
            toggle_from_hud
                .in_set(AudioHudInput)
                .run_if(audio_controls_visible),
        )
        .add_systems(
            Update,
            (update_icons, update_visibility).after(crate::menu::FrontendSystems),
        );
}

fn audio_controls_visible(frontend: Res<crate::menu::Frontend>) -> bool {
    frontend.audio_controls_visible()
}

fn update_visibility(
    frontend: Res<crate::menu::Frontend>,
    mut rows: Query<&mut Visibility, With<AudioToggleRow>>,
) {
    for mut visibility in &mut rows {
        *visibility = if frontend.audio_controls_visible() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn icon_rect(enabled: bool) -> Rect {
    let x = if enabled { 0.0 } else { 18.0 };
    Rect::new(x, 0.0, x + 18.0, 18.0)
}

fn spawn_toggles(mut commands: Commands, hud: Res<Hud>, assets: Res<AssetServer>) {
    let load = |path| {
        assets
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load(path)
    };
    commands.entity(hud.0).with_children(|parent| {
        parent
            .spawn((
                AudioToggleRow,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0 * crate::GRID_BLOCK_SIZE + 6.0),
                    top: Val::Px(6.0),
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                GlobalZIndex(200),
            ))
            .with_children(|row| {
                for (channel, path) in [
                    (AudioChannel::Music, "volume-Sheet.png"),
                    (AudioChannel::Sfx, "sfx-Sheet.png"),
                ] {
                    row.spawn((
                        Button,
                        AudioToggle,
                        channel,
                        ImageNode {
                            image: load(path),
                            rect: Some(icon_rect(true)),
                            ..default()
                        },
                        Node {
                            width: Val::Px(13.0),
                            height: Val::Px(13.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                    ));
                }
            });
    });
}

fn toggle_from_hud(
    buttons: Query<&AudioChannel, (With<AudioToggle>, Added<Pressed>)>,
    mut settings: ResMut<AudioSettings>,
) {
    for &channel in &buttons {
        settings.toggle(channel);
    }
}

fn update_icons(
    settings: Res<AudioSettings>,
    mut buttons: Query<(&AudioChannel, &mut ImageNode), With<AudioToggle>>,
) {
    for (&channel, mut image) in &mut buttons {
        let rect = Some(icon_rect(settings.enabled(channel)));
        if image.rect != rect {
            image.rect = rect;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetApp, AssetPlugin};

    #[test]
    fn hud_toggles_share_settings_and_update_both_icons() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<AudioSettings>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Startup, spawn_toggles)
            .add_systems(Update, (toggle_from_hud, update_icons).chain());
        app.update();
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(Entity, &AudioChannel), With<AudioToggle>>();
        let music = buttons
            .iter(world)
            .find_map(|(entity, channel)| matches!(channel, AudioChannel::Music).then_some(entity))
            .unwrap();
        let sfx = buttons
            .iter(world)
            .find_map(|(entity, channel)| matches!(channel, AudioChannel::Sfx).then_some(entity))
            .unwrap();
        for entity in [music, sfx] {
            let node = world.get::<Node>(entity).unwrap();
            assert_eq!(node.width, Val::Px(13.0));
            assert_eq!(node.height, Val::Px(13.0));
        }
        let row = world.get::<ChildOf>(music).unwrap().parent();
        let node = world.get::<Node>(row).unwrap();
        assert_eq!(node.left, Val::Px(3.0 * crate::GRID_BLOCK_SIZE + 6.0));
        assert_eq!(node.top, Val::Px(6.0));
        world.entity_mut(music).insert(Pressed);
        app.update();
        assert!(!app.world().resource::<AudioSettings>().music);
        assert!(app.world().resource::<AudioSettings>().sfx);
        assert_eq!(
            app.world().get::<ImageNode>(music).unwrap().rect,
            Some(icon_rect(false))
        );
        app.world_mut().resource_mut::<AudioSettings>().sfx = false;
        app.update();
        assert_eq!(
            app.world().get::<ImageNode>(sfx).unwrap().rect,
            Some(icon_rect(false))
        );
        app.world_mut().entity_mut(music).remove::<Pressed>();
        app.update();
        app.world_mut().entity_mut(music).insert(Pressed);
        app.update();
        assert!(app.world().resource::<AudioSettings>().music);
        assert_eq!(
            app.world().get::<ImageNode>(music).unwrap().rect,
            Some(icon_rect(true))
        );
    }
}
