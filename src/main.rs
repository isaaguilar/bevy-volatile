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
const HEALTH_PANEL_INSET: f32 = 6.0;
const HEALTH_PANEL_WIDTH: f32 = 3.0 * GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEALTH_PANEL_HEIGHT: f32 = GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
const HEALTH_PANEL_BORDER: f32 = 3.0;
const ENERGY_BAR_SIDE_PADDING: f32 = 6.0;
const HEART_COUNT: usize = 5;
const MAX_STRENGTH: u8 = 5;
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
struct Player(PlayerId);

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
    acted_this_frame: bool,
}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            active: PlayerId::One,
            strength: [MAX_STRENGTH; 2],
            acted_this_frame: false,
        }
    }
}

impl TurnState {
    fn finish_action(&mut self) {
        self.active = self.active.other();
        self.acted_this_frame = true;
    }
}

#[derive(Component)]
struct HealthPanel;

#[derive(Component)]
struct EnergyFill(PlayerId);

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct InventorySlot {
    index: usize,
    owner: PlayerId,
}

#[derive(Component)]
struct TurnStatus;

#[derive(Component)]
struct DropFeedback;

#[derive(Component)]
struct BoardItem;

#[derive(Resource, Default)]
struct BoardCursor(Option<Vec2>);

#[derive(Resource, Default)]
struct Selection(Option<SelectedAction>);

#[derive(Resource, Default)]
struct InventoryHover {
    hovered: Option<(PlayerId, usize)>,
    selected: Option<(PlayerId, usize)>,
}

#[derive(Resource)]
struct InventoryImages([Handle<Image>; 14]);

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
        .init_resource::<BoardCursor>()
        .init_resource::<Selection>()
        .init_resource::<InventoryHover>()
        .init_resource::<TurnState>()
        .add_systems(Startup, (setup, spawn_board_grid))
        .add_systems(
            Update,
            (
                begin_turn_frame,
                rest,
                update_board_cursor,
                handle_selection,
                highlight_inventory_hover,
                update_turn_status,
                update_energy_bars,
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
                    TurnStatus,
                    Text::new("P1 (ORANGE) 5/5 | P2 (BLUE) 5/5 | P1 TURN | R: REST"),
                    TextFont {
                        font_size: FontSize::Px(17.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.1, 0.07, 0.04)),
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
                    Text::new("Click your character or an item to select it"),
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
        Transform::from_xyz(-150.0, 0.0, 0.0),
        Player(PlayerId::One),
    ));
    commands.spawn((
        Sprite {
            color: Color::srgb(0.15, 0.45, 0.9),
            custom_size: Some(Vec2::splat(48.0)),
            ..default()
        },
        Transform::from_xyz(150.0, 0.0, 0.0),
        Player(PlayerId::Two),
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
                                height: Val::Px(32.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceEvenly,
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|row| {
                            for _ in 0..HEART_COUNT {
                                row.spawn((
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
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(
                                    HEALTH_PANEL_WIDTH
                                        - 2.0 * (HEALTH_PANEL_BORDER + ENERGY_BAR_SIDE_PADDING),
                                ),
                                height: Val::Px(7.0),
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
    Some(row * 2 + column)
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
    selection: Res<Selection>,
    turn: Res<TurnState>,
    mut hover: ResMut<InventoryHover>,
    mut slots: Query<(
        &InventorySlot,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
    )>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
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
        && let Ok(mut feedback) = feedback.single_mut()
    {
        feedback.0 = match hovered {
            Some((_, index)) => format!("{} - click to select", INVENTORY_ITEMS[index].0),
            None => "Click your character or an item to select it".into(),
        };
    }
    hover.hovered = hovered;
    hover.selected = selected;
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

fn select_movement(commands: &mut Commands, player: Vec2, opponent: Vec2) -> SelectedAction {
    let mut highlights = Vec::new();
    if let Some(cell) = board_cell(player) {
        for x in -1..=1 {
            for y in -1..=1 {
                if x == 0 && y == 0 {
                    continue;
                }
                let center = cell_center(cell + IVec2::new(x, y));
                if board_cell(center).is_none() || center == opponent {
                    continue;
                }
                highlights.push(
                    commands
                        .spawn((
                            Sprite {
                                color: Color::srgba(0.2, 0.8, 0.3, 0.35),
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
    input: (
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
        Res<BoardCursor>,
    ),
    assets: (Res<InventoryImages>, Res<Hud>),
    mut selection: ResMut<Selection>,
    mut turn: ResMut<TurnState>,
    mut players: Query<(Entity, &Player, &mut Transform)>,
    mut feedback: Query<&mut Text, With<DropFeedback>>,
) {
    let (mouse, keyboard, cursor) = input;
    let (images, hud) = assets;
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

    if keyboard.just_pressed(KeyCode::Escape) && selection.0.is_some() {
        clear_selection(&mut commands, &mut selection);
        feedback.0 = "Selection cancelled".into();
        return;
    }

    let cell = cursor.0.and_then(board_cell);
    let target = cursor.0.and_then(|position| drop_target(player, position));
    if mouse.just_pressed(MouseButton::Left) && !turn.acted_this_frame {
        if let Some(index) = cursor
            .0
            .and_then(|position| inventory_item(position, turn.active))
        {
            let already_selected = matches!(selection.0, Some(SelectedAction::Item { index: current, .. }) if current == index);
            clear_selection(&mut commands, &mut selection);
            if already_selected {
                feedback.0 = "Selection cancelled".into();
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
                        ImageNode::new(images.0[index].clone()),
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
            }
            return;
        }
        if cell.is_some() && cell == board_cell(player) {
            let already_selected = matches!(selection.0, Some(SelectedAction::Movement { .. }));
            clear_selection(&mut commands, &mut selection);
            if already_selected {
                feedback.0 = "Selection cancelled".into();
            } else {
                selection.0 = Some(select_movement(&mut commands, player, opponent));
                feedback.0 =
                    "Click a highlighted cell to walk; click character or Esc to cancel".into();
            }
            return;
        }
        match (&selection.0, &target) {
            (Some(SelectedAction::Item { index, .. }), Some(target))
                if target.kind == DropKind::Place || turn.strength[turn.active.index()] > 0 =>
            {
                let center = cell_center(target.cell);
                commands.spawn((
                    Name::new(INVENTORY_ITEMS[*index].0),
                    BoardItem,
                    Sprite {
                        image: images.0[*index].clone(),
                        custom_size: Some(Vec2::splat(44.0)),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, 0.1),
                ));
                if target.kind == DropKind::Throw {
                    let index = turn.active.index();
                    turn.strength[index] -= 1;
                }
                clear_selection(&mut commands, &mut selection);
                turn.finish_action();
                feedback.0 = "Item used - next player's turn".into();
                return;
            }
            (Some(SelectedAction::Movement { .. }), Some(target))
                if target.distance == 1 && cell_center(target.cell) != opponent =>
            {
                if let Ok((_, _, mut transform)) = players.get_mut(player_entity) {
                    let center = cell_center(target.cell);
                    transform.translation.x = center.x;
                    transform.translation.y = center.y;
                    clear_selection(&mut commands, &mut selection);
                    turn.finish_action();
                    feedback.0 = "Walk complete - next player's turn".into();
                }
                return;
            }
            _ => {}
        }
    }

    match &selection.0 {
        Some(SelectedAction::Item {
            index,
            icon,
            highlight,
        }) => {
            let can_throw = turn.strength[turn.active.index()] > 0;
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
                    Some(DropKind::Place) => Color::srgba(0.2, 0.8, 0.3, 0.4),
                    Some(DropKind::Throw) if can_throw => Color::srgba(1.0, 0.7, 0.1, 0.4),
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
            feedback.0 = match &target {
                Some(target) if target.kind == DropKind::Throw && !can_throw => {
                    "Not enough energy - Esc to cancel, R to rest".into()
                }
                Some(target) => format!(
                    "{}: {} - {} grid {} - click to confirm",
                    INVENTORY_ITEMS[*index].0,
                    if target.kind == DropKind::Place {
                        "PLACE"
                    } else {
                        "THROW"
                    },
                    target.distance,
                    if target.distance == 1 {
                        "step"
                    } else {
                        "steps"
                    }
                ),
                None if cell.is_some() => "Click your character to select movement".into(),
                None => "Click a board cell, or click the item again / Esc to cancel".into(),
            };
        }
        Some(SelectedAction::Movement { .. }) => {
            feedback.0 = match &target {
                Some(target) if target.distance > 1 => "Jumping is not available yet".into(),
                Some(target) if cell_center(target.cell) == opponent => {
                    "Cell occupied by opponent".into()
                }
                Some(_) => "Click to walk - 1 grid step".into(),
                None => "Click a highlighted cell to walk; click character or Esc to cancel".into(),
            };
        }
        None => {}
    }
}

fn begin_turn_frame(mut turn: ResMut<TurnState>) {
    turn.acted_this_frame = false;
}

fn update_turn_status(turn: Res<TurnState>, mut status: Query<&mut Text, With<TurnStatus>>) {
    if let Ok(mut status) = status.single_mut() {
        status.0 = format!(
            "P1 (ORANGE) {}/{} | P2 (BLUE) {}/{} | {} TURN | R: REST",
            turn.strength[0],
            MAX_STRENGTH,
            turn.strength[1],
            MAX_STRENGTH,
            match turn.active {
                PlayerId::One => "P1",
                PlayerId::Two => "P2",
            },
        );
    }
}

fn update_energy_bars(turn: Res<TurnState>, mut bars: Query<(&EnergyFill, &mut Node)>) {
    for (fill, mut node) in &mut bars {
        node.width = Val::Percent(
            100.0 * f32::from(turn.strength[fill.0.index()]) / f32::from(MAX_STRENGTH),
        );
    }
}

fn rest(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<Selection>,
    mut turn: ResMut<TurnState>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) && !turn.acted_this_frame && selection.0.is_none() {
        let index = turn.active.index();
        turn.strength[index] = (turn.strength[index] + 1).min(MAX_STRENGTH);
        turn.finish_action();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(children.len(), 2);
            let hearts = world.get::<Children>(children[0]).unwrap();
            assert_eq!(hearts.len(), 5);
            for child in hearts.iter() {
                assert!(world.get::<ImageNode>(child).is_some());
            }
            assert_eq!(node.align_items, AlignItems::Center);
            assert_eq!(
                world.get::<Node>(children[1]).unwrap().width,
                Val::Px(150.0)
            );
            let bar = world.get::<Children>(children[1]).unwrap();
            assert_eq!(bar.len(), 1);
            assert!(world.get::<EnergyFill>(bar[0]).is_some());
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
    fn hover_highlights_only_active_inventory_item_and_shows_hint() {
        fn setup_inventory(mut commands: Commands) {
            let hud = commands.spawn(Node::default()).id();
            spawn_inventories(
                &mut commands,
                hud,
                &std::array::from_fn(|_| Handle::default()),
            );
            commands.spawn((
                DropFeedback,
                Text::new("Click your character or an item to select it"),
            ));
        }

        let mut app = App::new();
        app.init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<InventoryHover>()
            .init_resource::<TurnState>()
            .add_systems(Startup, setup_inventory)
            .add_systems(Update, highlight_inventory_hover);
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(-450.0, 180.0));
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
        assert_eq!(feedback.single(world).unwrap().0, "Water - click to select");

        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(450.0, 180.0));
        app.update();
        let world = app.world_mut();
        let mut slots = world.query::<(&InventorySlot, &BackgroundColor)>();
        assert!(slots.iter(world).all(|(_, color)| color.0 == Color::NONE));
        let mut feedback = world.query_filtered::<&Text, With<DropFeedback>>();
        assert_eq!(
            feedback.single(world).unwrap().0,
            "Click your character or an item to select it"
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

    fn interaction_app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<InventoryHover>()
            .init_resource::<TurnState>()
            .insert_resource(InventoryImages(std::array::from_fn(|_| Handle::default())))
            .add_systems(
                Update,
                (
                    begin_turn_frame,
                    rest,
                    handle_selection,
                    highlight_inventory_hover,
                    update_energy_bars,
                )
                    .chain(),
            );
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud));
        app.add_systems(Startup, |mut commands: Commands, hud: Res<Hud>| {
            spawn_inventories(
                &mut commands,
                hud.0,
                &std::array::from_fn(|_| Handle::default()),
            );
            spawn_health_displays(&mut commands, hud.0, Handle::default());
        });
        app.world_mut()
            .spawn((Player(PlayerId::One), Transform::from_xyz(30.0, 0.0, 0.0)));
        app.world_mut().spawn((
            Player(PlayerId::Two),
            Transform::from_xyz(150.0, -180.0, 0.0),
        ));
        app.world_mut().spawn((DropFeedback, Text::new("")));
        app
    }

    fn click(app: &mut App, position: Vec2) {
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
    }

    fn item_count(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut items = world.query_filtered::<Entity, With<BoardItem>>();
        items.iter(world).count()
    }

    #[test]
    fn item_requires_two_clicks_and_previews_after_release() {
        let mut app = interaction_app();
        click(&mut app, Vec2::new(-450.0, 180.0));
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
        assert_eq!(item_count(&mut app), 1);
        assert!(app.world().resource::<Selection>().0.is_none());
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [5, 5]);
    }

    #[test]
    fn selected_item_preview_is_centered_on_cursor_in_hud() {
        let mut app = interaction_app();
        click(&mut app, Vec2::new(-450.0, 180.0));
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
        click(&mut app, Vec2::new(-450.0, 180.0));
        assert_eq!(
            slot_border(&mut app, PlayerId::One, 0),
            UiRect::all(Val::Px(3.0))
        );
        app.world_mut().resource_mut::<BoardCursor>().0 = Some(Vec2::new(-390.0, 180.0));
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

        click(&mut app, Vec2::new(-450.0, 180.0));
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
        click(&mut app, Vec2::new(-450.0, 180.0));
        click(&mut app, Vec2::new(-450.0, 180.0));
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, Vec2::new(-450.0, 180.0));
        click(&mut app, Vec2::new(-390.0, 180.0));
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
    fn switching_between_item_and_character_clears_previews() {
        let mut app = interaction_app();
        click(&mut app, Vec2::new(-450.0, 180.0));
        let (icon, highlight) = match &app.world().resource::<Selection>().0 {
            Some(SelectedAction::Item {
                icon, highlight, ..
            }) => (*icon, *highlight),
            _ => panic!("item should be selected"),
        };
        click(&mut app, Vec2::new(30.0, 0.0));
        assert!(app.world().get_entity(icon).is_err());
        assert!(app.world().get_entity(highlight).is_err());
        let highlights = match &app.world().resource::<Selection>().0 {
            Some(SelectedAction::Movement { highlights }) => highlights.clone(),
            _ => panic!("character should be selected"),
        };
        assert_eq!(highlights.len(), 8);
        click(&mut app, Vec2::new(-450.0, 180.0));
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
    }

    #[test]
    fn player_selection_walks_diagonally_and_rejects_jumps_and_occupied_cells() {
        let mut app = interaction_app();
        click(&mut app, Vec2::new(30.0, 0.0));
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
            "Jumping is not available yet"
        );

        click(&mut app, Vec2::new(30.0, 0.0));
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, Vec2::new(30.0, 0.0));
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
        click(&mut app, Vec2::new(30.0, 0.0));
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
        click(&mut app, Vec2::new(450.0, 180.0));
        assert!(app.world().resource::<Selection>().0.is_none());
        click(&mut app, Vec2::new(-450.0, 180.0));
        click(&mut app, Vec2::new(-270.0, 180.0));
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
        assert_eq!(app.world().resource::<TurnState>().strength, [4, 5]);
        click(&mut app, Vec2::new(450.0, 180.0));
        click(&mut app, Vec2::new(-270.0, 180.0));
        assert_eq!(item_count(&mut app), 2);
        assert_eq!(app.world().resource::<TurnState>().strength, [4, 4]);
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

        click(&mut app, Vec2::new(-450.0, 180.0));
        click(&mut app, Vec2::new(-270.0, 180.0));
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(80.0)
        );
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::Two),
            Val::Percent(100.0)
        );

        press_key(&mut app, KeyCode::KeyR);
        assert_eq!(
            energy_bar_width(&mut app, PlayerId::One),
            Val::Percent(80.0)
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
            Val::Percent(20.0)
        );
    }

    #[test]
    fn insufficient_strength_keeps_selection_until_cancelled() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        click(&mut app, Vec2::new(-450.0, 180.0));
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
        assert_eq!(app.world().resource::<TurnState>().strength, [1, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }

    #[test]
    fn exhausted_player_can_still_place_and_arrow_keys_do_not_move() {
        let mut app = interaction_app();
        app.world_mut().resource_mut::<TurnState>().strength[0] = 0;
        press_key(&mut app, KeyCode::ArrowRight);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::One);
        click(&mut app, Vec2::new(-450.0, 180.0));
        click(&mut app, Vec2::new(90.0, 60.0));
        assert_eq!(item_count(&mut app), 1);
        assert_eq!(app.world().resource::<TurnState>().strength, [0, 5]);
        assert_eq!(app.world().resource::<TurnState>().active, PlayerId::Two);
    }
}
