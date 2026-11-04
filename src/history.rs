use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
};
use bevy_aspect_ratio_mask::Hud;

use crate::{BOARD_ROWS, GRID_BLOCK_SIZE, HEALTH_PANEL_INSET, InventoryImages, menu};

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) text: String,
    pub(crate) item: Option<usize>,
}

#[derive(Resource, Default)]
pub(crate) struct ComputerHistory {
    entries: Vec<Entry>,
}

impl ComputerHistory {
    pub(crate) fn add(&mut self, text: String, item: Option<usize>) {
        self.entries.push(Entry { text, item });
    }

    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

pub(crate) fn cell_label(cell: IVec2) -> String {
    format!("{}{}", char::from(b'D' + cell.x as u8), cell.y + 2)
}

#[derive(Component)]
struct HistoryViewport;

#[derive(Component)]
struct HistoryPanel;

#[derive(Component)]
struct HistoryScrollbar;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HistorySystems;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<ComputerHistory>()
        .add_systems(Startup, setup.after(crate::setup))
        .add_systems(
            Update,
            (
                update_panel,
                render,
                scroll::<HistoryViewport>,
                update_scrollbar::<HistoryViewport, HistoryScrollbar>,
            )
                .chain()
                .after(menu::GameplaySystems)
                .in_set(HistorySystems)
                .run_if(menu::in_game),
        );
}

fn setup(mut commands: Commands, hud: Res<Hud>) {
    commands.entity(hud.0).with_children(|parent| {
        parent
            .spawn((
                HistoryPanel,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(HEALTH_PANEL_INSET),
                    top: Val::Px(GRID_BLOCK_SIZE + HEALTH_PANEL_INSET),
                    width: Val::Px(3.0 * GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                    height: Val::Px(BOARD_ROWS as f32 * GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(3.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.04, 0.03, 0.82)),
                BorderColor::all(Color::srgb(0.86, 0.68, 0.32)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("COMPUTER LOG"),
                    TextFont {
                        font_size: FontSize::Px(9.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.86, 0.68, 0.32)),
                    Pickable::IGNORE,
                ));
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        let viewport = row
                            .spawn((
                                HistoryViewport,
                                Hovered::default(),
                                ScrollPosition::default(),
                                Node {
                                    flex_grow: 1.0,
                                    min_width: Val::Px(0.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(7.0),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .id();
                        row.spawn((
                            HistoryScrollbar,
                            Visibility::Hidden,
                            Node {
                                width: Val::Px(8.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.4, 0.3, 0.18, 0.6)),
                            Scrollbar {
                                orientation: ControlOrientation::Vertical,
                                target: viewport,
                                min_thumb_length: 12.0,
                            },
                        ))
                        .with_children(|scrollbar| {
                            scrollbar.spawn((
                                Hovered::default(),
                                BackgroundColor(Color::srgb(0.86, 0.68, 0.32)),
                                ScrollbarThumb {
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    border: UiRect::ZERO,
                                },
                            ));
                        });
                    });
            });
    });
}

fn update_panel(
    frontend: Res<menu::Frontend>,
    mut panels: Query<&mut Visibility, With<HistoryPanel>>,
) {
    for mut visibility in &mut panels {
        *visibility = if frontend.computer_game() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn render(
    mut commands: Commands,
    history: Res<ComputerHistory>,
    images: Res<InventoryImages>,
    mut viewport: Query<(Entity, Option<&Children>, &mut ScrollPosition), With<HistoryViewport>>,
) {
    if !history.is_changed() {
        return;
    }
    let Ok((entity, children, mut position)) = viewport.single_mut() else {
        return;
    };
    position.y = 0.0;
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    commands.entity(entity).with_children(|viewport| {
        for entry in history.entries.iter().rev() {
            viewport
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_shrink: 0.0,
                        column_gap: Val::Px(4.0),
                        align_items: AlignItems::Start,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    if let Some(index) = entry.item {
                        row.spawn((
                            ImageNode::new(images.0[index].clone()),
                            Node {
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                    row.spawn((
                        Text::new(&entry.text),
                        TextFont {
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.94, 0.78)),
                        Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                });
        }
    });
}

pub(crate) fn scroll<V: Component>(
    mut wheel: MessageReader<MouseWheel>,
    mut viewport: Query<(&Hovered, &ComputedNode, &mut ScrollPosition), With<V>>,
) {
    let Ok((hovered, computed, mut position)) = viewport.single_mut() else {
        return;
    };
    for event in wheel.read() {
        if hovered.0 {
            let scale = if event.unit == MouseScrollUnit::Line {
                24.0
            } else {
                1.0
            };
            let max = ((computed.content_size().y - computed.size().y)
                * computed.inverse_scale_factor())
            .max(0.0);
            position.y = (position.y - event.y * scale).clamp(0.0, max);
        }
    }
}

pub(crate) fn update_scrollbar<V: Component, S: Component>(
    viewport: Query<&ComputedNode, With<V>>,
    mut scrollbar: Query<&mut Visibility, With<S>>,
) {
    let (Ok(computed), Ok(mut visibility)) = (viewport.single(), scrollbar.single_mut()) else {
        return;
    };
    *visibility = if computed.content_size().y > computed.size().y + 0.5 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlayerHealth, SUNSTONE_ITEM_INDEX};

    #[test]
    fn final_action_is_rendered_before_game_over() {
        fn finish_match(mut health: ResMut<PlayerHealth>, mut history: ResMut<ComputerHistory>) {
            history.add(
                "Computer used Sunstone at E5".into(),
                Some(SUNSTONE_ITEM_INDEX),
            );
            health.0[0] = 0;
        }

        let mut app = App::new();
        app.insert_resource(menu::Frontend::computer_match())
            .init_resource::<ComputerHistory>()
            .init_resource::<PlayerHealth>()
            .insert_resource(InventoryImages(std::array::from_fn(|_| Handle::default())))
            .add_systems(Update, finish_match.in_set(menu::GameplaySystems))
            .add_systems(
                Update,
                render.in_set(HistorySystems).after(menu::GameplaySystems),
            )
            .add_systems(Update, menu::detect_game_over.after(HistorySystems));
        app.world_mut()
            .spawn((HistoryViewport, Node::default(), ScrollPosition::default()));
        app.update();
        assert!(!app.world().resource::<menu::Frontend>().computer_game());
        let world = app.world_mut();
        let mut text = world.query::<&Text>();
        assert!(
            text.iter(world)
                .any(|text| text.0 == "Computer used Sunstone at E5")
        );
        let mut images = world.query::<&ImageNode>();
        assert_eq!(images.iter(world).count(), 1);
    }

    #[test]
    fn wheel_scrolls_only_when_hovered_and_scrollbar_tracks_overflow() {
        use bevy::input::touch::TouchPhase;

        let mut app = App::new();
        app.add_message::<MouseWheel>().add_systems(
            Update,
            (
                scroll::<HistoryViewport>,
                update_scrollbar::<HistoryViewport, HistoryScrollbar>,
            )
                .chain(),
        );
        let viewport = app
            .world_mut()
            .spawn((
                HistoryViewport,
                Hovered(true),
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(100.0, 100.0),
                    content_size: Vec2::new(100.0, 200.0),
                    inverse_scale_factor: 1.0,
                    ..default()
                },
            ))
            .id();
        let scrollbar = app
            .world_mut()
            .spawn((HistoryScrollbar, Visibility::Hidden))
            .id();
        let wheel = MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -2.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        };
        app.world_mut().write_message(wheel);
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 48.0);
        assert_eq!(
            *app.world().get::<Visibility>(scrollbar).unwrap(),
            Visibility::Visible
        );
        app.world_mut().entity_mut(viewport).insert(Hovered(false));
        app.world_mut().write_message(wheel);
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 48.0);
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::splat(100.0),
            ..default()
        });
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(scrollbar).unwrap(),
            Visibility::Hidden
        );
    }
}
