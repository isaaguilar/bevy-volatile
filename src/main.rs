use bevy::{
    camera::ScalingMode, color::palettes::css::ORANGE, prelude::*, window::WindowResolution,
};
use bevy_aspect_ratio_mask::{AspectRatioPlugin, Hud, Resolution};

const RESOLUTION_WIDTH: f32 = 960.0;
const RESOLUTION_HEIGHT: f32 = 540.0;
const GRID_COLUMNS: u32 = 16;
const GRID_BLOCK_SIZE: f32 = RESOLUTION_WIDTH / GRID_COLUMNS as f32;
const GRID_LINE_WIDTH: f32 = 2.0;
const BOARD_COLUMNS: u32 = 10;
const BOARD_ROWS: u32 = 7;
const BOARD_LEFT: f32 = -RESOLUTION_WIDTH / 2.0 + 3.0 * GRID_BLOCK_SIZE;
const BOARD_TOP: f32 = RESOLUTION_HEIGHT / 2.0 - GRID_BLOCK_SIZE;
const BOARD_RIGHT: f32 = BOARD_LEFT + BOARD_COLUMNS as f32 * GRID_BLOCK_SIZE;
const BOARD_BOTTOM: f32 = BOARD_TOP - BOARD_ROWS as f32 * GRID_BLOCK_SIZE;
const PLAYABLE_LEFT_X: f32 = BOARD_LEFT + GRID_BLOCK_SIZE / 2.0;
const PLAYABLE_RIGHT_X: f32 = BOARD_RIGHT - GRID_BLOCK_SIZE / 2.0;
const PLAYABLE_TOP_Y: f32 = BOARD_TOP - GRID_BLOCK_SIZE / 2.0;
const PLAYABLE_BOTTOM_Y: f32 = BOARD_BOTTOM + GRID_BLOCK_SIZE / 2.0;
const HEALTH_PANEL_INSET: f32 = 6.0;
const HEALTH_PANEL_WIDTH: f32 = 3.0 * GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEALTH_PANEL_HEIGHT: f32 = GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEART_COUNT: usize = 5;
const INVENTORY_ITEMS: [(&str, &str); 14] = [
    ("Water", "items/water.png"),
    ("Salt", "items/salt.png"),
    ("Clay", "items/clay.png"),
    ("Limestone", "items/limestone.png"),
    ("Charcoal", "items/charcoal.png"),
    ("Copper", "items/copper.png"),
    ("Obsidian", "items/obsidian.png"),
    ("Copal", "items/copal.png"),
    ("Cacao", "items/cacao.png"),
    ("Maize", "items/maize.png"),
    ("Chile", "items/chile.png"),
    ("Vanilla", "items/vanilla.png"),
    ("Annatto", "items/annatto.png"),
    ("Honey", "items/honey.png"),
];

#[derive(Component)]
struct Player;

#[derive(Component)]
struct HealthPanel;

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct InventorySlot {
    index: usize,
    draggable: bool,
}

#[derive(Component)]
struct DropFeedback;

#[derive(Component)]
struct BoardItem;

#[derive(Resource, Default)]
struct DragCursor(Option<Vec2>);

#[derive(Resource, Default)]
struct InventoryDrag(Option<DraggedItem>);

#[derive(Resource, Default)]
struct InventoryHover(Option<usize>);

#[derive(Resource)]
struct InventoryImages([Handle<Image>; 14]);

struct DraggedItem {
    index: usize,
    icon: Entity,
    highlight: Entity,
}

#[derive(Debug, PartialEq, Eq)]
enum DropKind {
    Place,
    Throw,
}

#[derive(Debug, PartialEq, Eq)]
struct DropTarget {
    cell: IVec2,
    distance: u32,
    kind: DropKind,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Jam 8".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AspectRatioPlugin {
            resolution: Resolution {
                width: RESOLUTION_WIDTH,
                height: RESOLUTION_HEIGHT,
            },
            ..default()
        })
        .init_resource::<DragCursor>()
        .init_resource::<InventoryDrag>()
        .init_resource::<InventoryHover>()
        .add_systems(Startup, (setup, spawn_board_grid))
        .add_systems(
            Update,
            (
                move_sprite,
                update_drag_cursor,
                highlight_inventory_hover,
                drag_inventory,
            )
                .chain(),
        )
        .run();
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
                    Text::new("Bevy 0.20 + Aspect Ratio Mask | Board: 10 x 7"),
                    TextFont {
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                ));
            });
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    bottom: Val::Px(8.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|parent| {
                parent.spawn((
                    DropFeedback,
                    Text::new("Drag an item onto the board"),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.1, 0.07, 0.04)),
                    Pickable::IGNORE,
                ));
            });
    });

    spawn_health_displays(&mut commands, hud.0, asset_server.load("heart.png"));
    let item_images = INVENTORY_ITEMS.map(|(_, path)| asset_server.load(path));
    spawn_inventories(&mut commands, hud.0, &item_images);
    commands.insert_resource(InventoryImages(item_images));

    commands.spawn((
        Sprite {
            image: asset_server.load("backgroundboard.png"),
            custom_size: Some(Vec2::new(RESOLUTION_WIDTH, RESOLUTION_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -2.0),
    ));

    commands.spawn((
        Sprite {
            color: ORANGE.into(),
            custom_size: Some(Vec2::splat(48.0)),
            ..default()
        },
        Transform::from_xyz(GRID_BLOCK_SIZE / 2.0, 0.0, 0.0),
        Player,
    ));
}

fn spawn_inventories(commands: &mut Commands, hud: Entity, images: &[Handle<Image>; 14]) {
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
                    InventoryPanel,
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
                    for (index, ((label, _), image)) in
                        INVENTORY_ITEMS.iter().zip(images.iter()).enumerate()
                    {
                        let column = index % 2;
                        let row = index / 2;
                        let left_inset = if column == 0 { HEALTH_PANEL_INSET } else { 0.0 };
                        let top_inset = if row == 0 { HEALTH_PANEL_INSET } else { 0.0 };
                        let bottom_inset = if row == BOARD_ROWS as usize - 1 {
                            HEALTH_PANEL_INSET
                        } else {
                            0.0
                        };
                        inventory
                            .spawn((
                                Name::new(*label),
                                InventorySlot {
                                    index,
                                    draggable: is_left,
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
                                    ImageNode::new(image.clone()),
                                    Node {
                                        width: Val::Px(31.0),
                                        height: Val::Px(31.0),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                slot.spawn((
                                    Text::new(*label),
                                    TextFont {
                                        font_size: FontSize::Px(10.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.94, 0.78)),
                                    Pickable::IGNORE,
                                ));
                            });
                    }
                });
        }
    });
}

fn spawn_health_displays(commands: &mut Commands, hud: Entity, heart: Handle<Image>) {
    commands.entity(hud).with_children(|parent| {
        for is_left in [true, false] {
            let mut node = Node {
                position_type: PositionType::Absolute,
                top: Val::Px(HEALTH_PANEL_INSET),
                width: Val::Px(HEALTH_PANEL_WIDTH),
                height: Val::Px(HEALTH_PANEL_HEIGHT),
                border: UiRect::all(Val::Px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
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
                    for _ in 0..HEART_COUNT {
                        panel.spawn((
                            ImageNode::new(heart.clone()),
                            Node {
                                width: Val::Px(27.0),
                                height: Val::Px(27.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
        }
    });
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

fn drop_target(player: Vec2, position: Vec2) -> Option<DropTarget> {
    let player_cell = board_cell(player)?;
    let cell = board_cell(position)?;
    let delta = (cell - player_cell).abs();
    let distance = delta.x.max(delta.y) as u32;
    let kind = match distance {
        0 => return None,
        1 => DropKind::Place,
        _ => DropKind::Throw,
    };
    Some(DropTarget {
        cell,
        distance,
        kind,
    })
}

fn left_inventory_item(position: Vec2) -> Option<usize> {
    let x = position.x + RESOLUTION_WIDTH / 2.0;
    let y = BOARD_TOP - position.y;
    if !position.is_finite()
        || !(0.0..2.0 * GRID_BLOCK_SIZE).contains(&x)
        || !(0.0..BOARD_ROWS as f32 * GRID_BLOCK_SIZE).contains(&y)
    {
        return None;
    }
    let column = (x / GRID_BLOCK_SIZE).floor() as usize;
    let row = (y / GRID_BLOCK_SIZE).floor() as usize;
    Some(row * 2 + column)
}

fn update_drag_cursor(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor: ResMut<DragCursor>,
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
    cursor: Res<DragCursor>,
    drag: Res<InventoryDrag>,
    mut hover: ResMut<InventoryHover>,
    mut slots: Query<(
        &InventorySlot,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
    )>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    let hovered = if drag.0.is_none() {
        cursor.0.and_then(left_inventory_item)
    } else {
        None
    };
    if hover.0 == hovered {
        return;
    }

    for (slot, mut background, mut border, mut node) in &mut slots {
        if !slot.draggable {
            continue;
        }
        let is_hovered = hovered == Some(slot.index);
        background.0 = if is_hovered {
            Color::srgba(0.86, 0.68, 0.32, 0.35)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(if is_hovered {
            Color::srgb(1.0, 0.85, 0.45)
        } else {
            Color::srgba(0.86, 0.68, 0.32, 0.4)
        });
        node.border = UiRect::all(Val::Px(if is_hovered { 2.0 } else { 0.5 }));
    }

    if drag.0.is_none()
        && let Ok(mut feedback) = feedback.single_mut()
    {
        feedback.0 = match hovered {
            Some(index) => format!("{} - drag onto the board", INVENTORY_ITEMS[index].0),
            None => "Drag an item onto the board".into(),
        };
    }
    hover.0 = hovered;
}

fn drag_inventory(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Res<DragCursor>,
    images: Res<InventoryImages>,
    mut drag: ResMut<InventoryDrag>,
    player: Query<&Transform, With<Player>>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    let (Ok(player), Ok(mut feedback)) = (player.single(), feedback.single_mut()) else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left)
        && drag.0.is_none()
        && let Some(index) = cursor.0.and_then(left_inventory_item)
    {
        let highlight = commands
            .spawn((
                Sprite {
                    color: Color::srgba(0.2, 0.8, 0.3, 0.4),
                    custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let icon = commands
            .spawn((
                Sprite {
                    image: images.0[index].clone(),
                    custom_size: Some(Vec2::splat(44.0)),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        drag.0 = Some(DraggedItem {
            index,
            icon,
            highlight,
        });
    }

    let Some(item) = &drag.0 else {
        return;
    };
    let cell = cursor.0.and_then(board_cell);
    let target = cursor
        .0
        .and_then(|position| drop_target(player.translation.truncate(), position));

    if let Some(position) = cursor.0 {
        let preview = cell.map(cell_center).unwrap_or(position);
        commands.entity(item.icon).insert((
            Transform::from_xyz(preview.x, preview.y, 2.0),
            Visibility::Visible,
        ));
    } else {
        commands.entity(item.icon).insert(Visibility::Hidden);
    }
    if let Some(cell) = cell {
        let center = cell_center(cell);
        let color = match target.as_ref().map(|target| &target.kind) {
            Some(DropKind::Place) => Color::srgba(0.2, 0.8, 0.3, 0.4),
            Some(DropKind::Throw) => Color::srgba(1.0, 0.7, 0.1, 0.4),
            None => Color::srgba(0.9, 0.15, 0.1, 0.4),
        };
        commands.entity(item.highlight).insert((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(GRID_BLOCK_SIZE - 4.0)),
                ..default()
            },
            Transform::from_xyz(center.x, center.y, 1.0),
            Visibility::Visible,
        ));
    } else {
        commands.entity(item.highlight).insert(Visibility::Hidden);
    }

    feedback.0 = match &target {
        Some(target) => format!(
            "{}: {} - {} grid {}",
            INVENTORY_ITEMS[item.index].0,
            match target.kind {
                DropKind::Place => "PLACE",
                DropKind::Throw => "THROW",
            },
            target.distance,
            if target.distance == 1 {
                "step"
            } else {
                "steps"
            }
        ),
        None if cell.is_some() => "Cannot use the character's cell".into(),
        None => "Drag onto a board cell".into(),
    };

    if mouse.just_released(MouseButton::Left) || !mouse.pressed(MouseButton::Left) {
        if let Some(target) = target {
            let center = cell_center(target.cell);
            commands.spawn((
                Name::new(INVENTORY_ITEMS[item.index].0),
                BoardItem,
                Sprite {
                    image: images.0[item.index].clone(),
                    custom_size: Some(Vec2::splat(44.0)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, 0.1),
            ));
        } else {
            feedback.0 = "Drop cancelled - drag an item onto the board".into();
        }
        commands.entity(item.icon).despawn();
        commands.entity(item.highlight).despawn();
        drag.0 = None;
    }
}

fn move_sprite(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sprite: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = sprite.single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }

    transform.translation.x = (transform.translation.x + direction.x * GRID_BLOCK_SIZE)
        .clamp(PLAYABLE_LEFT_X, PLAYABLE_RIGHT_X);
    transform.translation.y = (transform.translation.y + direction.y * GRID_BLOCK_SIZE)
        .clamp(PLAYABLE_BOTTOM_Y, PLAYABLE_TOP_Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag_app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<DragCursor>()
            .init_resource::<InventoryDrag>()
            .init_resource::<InventoryHover>()
            .insert_resource(InventoryImages(std::array::from_fn(|_| Handle::default())))
            .add_systems(Update, (highlight_inventory_hover, drag_inventory).chain());
        app.world_mut()
            .spawn((Player, Transform::from_xyz(GRID_BLOCK_SIZE / 2.0, 0.0, 0.0)));
        app.world_mut().spawn((DropFeedback, Text::new("")));
        app
    }

    fn move_once(start: Vec2, key: KeyCode) -> Vec2 {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, move_sprite);
        let player = app
            .world_mut()
            .spawn((Player, Transform::from_xyz(start.x, start.y, 0.0)))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world()
            .get::<Transform>(player)
            .unwrap()
            .translation
            .truncate()
    }

    #[test]
    fn arrows_move_one_cell() {
        let start = Vec2::new(GRID_BLOCK_SIZE / 2.0, 0.0);
        for (key, direction) in [
            (KeyCode::ArrowLeft, Vec2::NEG_X),
            (KeyCode::ArrowRight, Vec2::X),
            (KeyCode::ArrowUp, Vec2::Y),
            (KeyCode::ArrowDown, Vec2::NEG_Y),
        ] {
            assert_eq!(move_once(start, key), start + direction * GRID_BLOCK_SIZE);
        }
    }

    #[test]
    fn movement_stays_within_d2_to_m8() {
        let d2 = Vec2::new(-270.0, 180.0);
        let m8 = Vec2::new(270.0, -180.0);
        assert_eq!(move_once(d2, KeyCode::ArrowLeft), d2);
        assert_eq!(move_once(d2, KeyCode::ArrowUp), d2);
        assert_eq!(move_once(m8, KeyCode::ArrowRight), m8);
        assert_eq!(move_once(m8, KeyCode::ArrowDown), m8);
        assert_eq!(move_once(d2, KeyCode::ArrowRight), d2 + Vec2::X * 60.0);
        assert_eq!(move_once(m8, KeyCode::ArrowUp), m8 + Vec2::Y * 60.0);
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
            assert_eq!(children.len(), 5);
            for child in children.iter() {
                assert!(world.get::<ImageNode>(child).is_some());
            }
        }
    }

    #[test]
    fn inventories_fill_rows_two_through_eight_on_both_sides() {
        fn setup_inventory(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &std::array::from_fn(|_| Handle::default()),
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
                    ))
                })
                .collect();
            assert_eq!(slots.len(), 14);
            for (index, ((slot, name), (label, _))) in
                slots.iter().zip(INVENTORY_ITEMS.iter()).enumerate()
            {
                let expected_x = if index % 2 == 0 { 6.0 } else { 60.0 };
                let expected_y =
                    (index / 2) as f32 * GRID_BLOCK_SIZE + if index < 2 { 6.0 } else { 0.0 };
                assert_eq!(slot.left, Val::Px(expected_x));
                assert_eq!(slot.top, Val::Px(expected_y));
                assert_eq!(name.as_str(), *label);
            }
        }
    }

    #[test]
    fn hover_highlights_only_draggable_item_and_shows_hint() {
        fn setup_inventory(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &std::array::from_fn(|_| Handle::default()),
            );
            commands.spawn((DropFeedback, Text::new("Drag an item onto the board")));
        }

        let mut app = App::new();
        app.init_resource::<DragCursor>()
            .init_resource::<InventoryDrag>()
            .init_resource::<InventoryHover>()
            .add_systems(Startup, setup_inventory)
            .add_systems(Update, highlight_inventory_hover);
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(-450.0, 180.0));
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &BackgroundColor, &Node)>();
        for (slot, background, node) in slots.iter(world) {
            let highlighted = slot.draggable && slot.index == 0;
            assert_eq!(background.0 == Color::NONE, !highlighted);
            assert_eq!(
                node.border,
                UiRect::all(Val::Px(if highlighted { 2.0 } else { 0.5 }))
            );
        }
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Water - drag onto the board"
        );

        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(450.0, 180.0));
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &BackgroundColor)>();
        assert!(slots.iter(world).all(|(_, color)| color.0 == Color::NONE));
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Drag an item onto the board"
        );
    }

    #[test]
    fn diagonal_is_placeable_and_further_cells_are_throwable() {
        let player = Vec2::new(30.0, 0.0);
        let diagonal = drop_target(player, Vec2::new(90.0, 60.0)).unwrap();
        assert_eq!(diagonal.kind, DropKind::Place);
        assert_eq!(diagonal.distance, 1);
        let far = drop_target(player, Vec2::new(-270.0, 180.0)).unwrap();
        assert_eq!(far.kind, DropKind::Throw);
        assert_eq!(far.distance, 5);
        assert_eq!(drop_target(player, player), None);
        assert_eq!(drop_target(player, Vec2::new(300.0, 0.0)), None);
        assert_eq!(board_cell(Vec2::new(0.0, f32::NAN)), None);
    }

    #[test]
    fn dragging_from_left_inventory_previews_and_places_an_item() {
        let mut app = drag_app();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(-450.0, 180.0));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<InventoryDrag>().0.is_some());

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(90.0, 60.0));
        app.update();
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Water: PLACE - 1 grid step"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        let world = app.world_mut();
        assert!(world.resource::<InventoryDrag>().0.is_none());
        let mut items = world.query_filtered::<(&Name, &Transform), With<BoardItem>>();
        let (name, transform) = items.single(world).unwrap();
        assert_eq!(name.as_str(), "Water");
        assert_eq!(transform.translation.truncate(), Vec2::new(90.0, 60.0));
    }

    #[test]
    fn dragging_to_a_distant_cell_previews_and_throws_an_item() {
        let mut app = drag_app();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(-390.0, 180.0));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(-270.0, 180.0));
        app.update();
        let world = app.world_mut();
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Salt: THROW - 5 grid steps"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        let world = app.world_mut();
        let mut items = world.query_filtered::<(&Name, &Transform), With<BoardItem>>();
        let (name, transform) = items.single(world).unwrap();
        assert_eq!(name.as_str(), "Salt");
        assert_eq!(transform.translation.truncate(), Vec2::new(-270.0, 180.0));
    }

    #[test]
    fn right_inventory_and_invalid_drops_do_not_create_items() {
        let mut app = drag_app();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(450.0, 180.0));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<InventoryDrag>().0.is_none());

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(-450.0, 180.0));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut().resource_mut::<DragCursor>().0 = Some(Vec2::new(30.0, 0.0));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        let world = app.world_mut();
        assert!(world.resource::<InventoryDrag>().0.is_none());
        let mut items = world.query_filtered::<Entity, With<BoardItem>>();
        assert_eq!(items.iter(world).count(), 0);
    }
}
