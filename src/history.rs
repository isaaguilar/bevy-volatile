use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
};
use bevy_aspect_ratio_mask::Hud;

use crate::{
    BOARD_ROWS, GRID_BLOCK_SIZE, HEALTH_PANEL_INSET, InventoryImages, menu, online, online_game,
};
use volatile_online_protocol::{HistoryEntry, Seat};

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
    format!("{}{}", char::from(b'A' + cell.x as u8), cell.y + 1)
}

#[derive(Component)]
struct HistoryViewport;

#[derive(Component)]
struct HistoryPanel;

#[derive(Component)]
struct HistoryTitle;

#[derive(Component)]
struct HistoryScrollbar;

#[derive(Resource, Default)]
struct OnlineHistory {
    seat: Option<Seat>,
    entries: Vec<HistoryEntry>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HistorySource {
    Computer,
    Online,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HistorySystems;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<ComputerHistory>()
        .init_resource::<OnlineHistory>()
        .add_systems(Startup, setup.after(crate::setup))
        .add_systems(
            Update,
            update_panel
                .after(menu::GameplaySystems)
                .in_set(HistorySystems),
        )
        .add_systems(
            Update,
            (
                sync_online_history,
                render,
                scroll::<HistoryViewport>,
                update_scrollbar::<HistoryViewport, HistoryScrollbar>,
            )
                .chain()
                .after(update_panel)
                .after(menu::GameplaySystems)
                .in_set(HistorySystems)
                .run_if(menu::in_game_or_online_game_over),
        );
}

fn setup(mut commands: Commands, hud: Res<Hud>) {
    commands.entity(hud.0).with_children(|parent| {
        parent
            .spawn((
                HistoryPanel,
                GlobalZIndex(0),
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
                    HistoryTitle,
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
    online: Option<Res<online::Online>>,
    mut panels: Query<(&mut Visibility, &mut Node, &mut GlobalZIndex), With<HistoryPanel>>,
    mut titles: Query<&mut Text, With<HistoryTitle>>,
) {
    let online_visible = frontend.online_history_visible();
    for (mut visibility, mut node, mut z_index) in &mut panels {
        *visibility = if frontend.computer_game() || online_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let left =
            online_visible && online.as_ref().and_then(|online| online.seat) == Some(Seat::Guest);
        node.left = if left {
            Val::Px(HEALTH_PANEL_INSET)
        } else {
            Val::Auto
        };
        node.right = if left {
            Val::Auto
        } else {
            Val::Px(HEALTH_PANEL_INSET)
        };
        *z_index = GlobalZIndex(if frontend.online_game_over() { 101 } else { 0 });
    }
    for mut title in &mut titles {
        let label = if online_visible {
            "GAME LOG"
        } else {
            "COMPUTER LOG"
        };
        if title.0 != label {
            title.0 = label.into();
        }
    }
}

fn sync_online_history(
    frontend: Res<menu::Frontend>,
    online: Option<Res<online::Online>>,
    applied: Option<Res<online_game::AppliedMatch>>,
    mut history: ResMut<OnlineHistory>,
) {
    if !frontend.online_history_visible() {
        return;
    }
    let (Some(online), Some(applied)) = (online, applied) else {
        return;
    };
    if applied.is_current(&online)
        && (history.seat != online.seat || history.entries != online.history)
    {
        history.seat = online.seat;
        history.entries.clone_from(&online.history);
    }
}

fn render(
    mut commands: Commands,
    history: Res<ComputerHistory>,
    frontend: Res<menu::Frontend>,
    online_history: Option<Res<OnlineHistory>>,
    images: Res<InventoryImages>,
    mut last_source: Local<Option<HistorySource>>,
    mut viewport: Query<(Entity, Option<&Children>, &mut ScrollPosition), With<HistoryViewport>>,
) {
    let is_online = frontend.online_history_visible();
    let source = if is_online {
        HistorySource::Online
    } else {
        HistorySource::Computer
    };
    if *last_source == Some(source)
        && !(is_online
            && online_history
                .as_ref()
                .is_some_and(|history| history.is_changed()))
        && !(!is_online && history.is_changed())
    {
        return;
    }
    *last_source = Some(source);
    let entries: Vec<Entry> = if is_online {
        online_history.as_ref().map_or_else(Vec::new, |history| {
            history
                .entries
                .iter()
                .map(|entry| Entry {
                    text: match entry.seat {
                        Some(seat) if Some(seat) == history.seat => format!("You: {}", entry.text),
                        Some(_) => format!("Opponent: {}", entry.text),
                        None => entry.text.clone(),
                    },
                    item: entry.item,
                })
                .collect()
        })
    } else {
        history.entries.clone()
    };
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
        for entry in entries.iter().rev() {
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
                            images.icon(index),
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
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn labels_use_playing_board_coordinates() {
        assert_eq!(cell_label(IVec2::ZERO), "A1");
        assert_eq!(cell_label(IVec2::new(9, 6)), "J7");
    }

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
            .init_resource::<InventoryImages>()
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
        assert_eq!(
            images.single(world).unwrap().rect,
            Some(InventoryImages::rect(SUNSTONE_ITEM_INDEX))
        );
    }

    #[test]
    fn online_history_is_on_the_opponents_side_and_remains_visible_after_game_over() {
        let mut frontend = menu::Frontend::default();
        frontend.enter_online_match();
        let mut app = App::new();
        app.insert_resource(frontend)
            .init_resource::<online::Online>()
            .init_resource::<PlayerHealth>()
            .add_systems(Update, update_panel);
        let panel = app
            .world_mut()
            .spawn((
                HistoryPanel,
                Node::default(),
                Visibility::Hidden,
                GlobalZIndex(0),
            ))
            .id();
        let title = app.world_mut().spawn((HistoryTitle, Text::default())).id();

        app.world_mut().resource_mut::<online::Online>().seat = Some(Seat::Host);
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().right,
            Val::Px(HEALTH_PANEL_INSET)
        );
        assert_eq!(app.world().get::<Node>(panel).unwrap().left, Val::Auto);
        assert_eq!(app.world().get::<Text>(title).unwrap().0, "GAME LOG");

        app.world_mut().resource_mut::<online::Online>().seat = Some(Seat::Guest);
        app.update();
        assert_eq!(
            app.world().get::<Node>(panel).unwrap().left,
            Val::Px(HEALTH_PANEL_INSET)
        );
        assert_eq!(app.world().get::<Node>(panel).unwrap().right, Val::Auto);

        app.world_mut().resource_mut::<PlayerHealth>().0[0] = 0;
        app.world_mut()
            .run_system_once(menu::detect_game_over)
            .unwrap();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(panel).unwrap(),
            Visibility::Visible
        );
        assert_eq!(
            *app.world().get::<GlobalZIndex>(panel).unwrap(),
            GlobalZIndex(101)
        );

        app.world_mut()
            .resource_mut::<menu::Frontend>()
            .end_online_match(true);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(panel).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<GlobalZIndex>(panel).unwrap(),
            GlobalZIndex(0)
        );
    }

    #[test]
    fn online_history_labels_both_players_from_the_local_seat() {
        let mut frontend = menu::Frontend::default();
        frontend.enter_online_match();
        let mut app = App::new();
        app.insert_resource(frontend)
            .init_resource::<ComputerHistory>()
            .init_resource::<InventoryImages>()
            .insert_resource(OnlineHistory {
                seat: Some(Seat::Guest),
                entries: vec![
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
                ],
            })
            .add_systems(Update, render);
        app.world_mut()
            .spawn((HistoryViewport, Node::default(), ScrollPosition::default()));
        app.update();
        let world = app.world_mut();
        let text: Vec<_> = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect();
        assert!(text.contains(&"Opponent: moved to D4".into()));
        assert!(text.contains(&"You: rested at H4".into()));
    }

    #[test]
    fn frontend_changes_do_not_reset_the_history_scroll() {
        let mut frontend = menu::Frontend::default();
        frontend.enter_online_match();
        let mut app = App::new();
        app.insert_resource(frontend)
            .init_resource::<ComputerHistory>()
            .init_resource::<InventoryImages>()
            .init_resource::<OnlineHistory>()
            .add_systems(Update, render);
        let viewport = app
            .world_mut()
            .spawn((HistoryViewport, Node::default(), ScrollPosition::default()))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<ScrollPosition>(viewport)
            .unwrap()
            .y = 24.0;
        app.world_mut()
            .resource_mut::<menu::Frontend>()
            .set_changed();
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 24.0);
    }

    #[test]
    fn final_online_event_is_rendered_before_game_over() {
        fn finish_match(mut health: ResMut<PlayerHealth>) {
            health.0[1] = 0;
        }
        let mut frontend = menu::Frontend::default();
        frontend.enter_online_match();
        let mut app = App::new();
        app.insert_resource(frontend)
            .init_resource::<ComputerHistory>()
            .init_resource::<PlayerHealth>()
            .init_resource::<InventoryImages>()
            .insert_resource(OnlineHistory {
                seat: Some(Seat::Host),
                entries: vec![HistoryEntry {
                    seat: Some(Seat::Host),
                    text: "won the match".into(),
                    item: None,
                }],
            })
            .add_systems(Update, finish_match.in_set(menu::GameplaySystems))
            .add_systems(
                Update,
                render.in_set(HistorySystems).after(menu::GameplaySystems),
            )
            .add_systems(Update, menu::detect_game_over.after(HistorySystems));
        app.world_mut()
            .spawn((HistoryViewport, Node::default(), ScrollPosition::default()));
        app.update();
        assert!(app.world().resource::<menu::Frontend>().online_game_over());
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "You: won the match")
        );
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
