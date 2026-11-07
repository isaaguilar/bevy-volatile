use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Button, ControlOrientation, Scrollbar, ScrollbarThumb},
};
use bevy_aspect_ratio_mask::Hud;

use crate::{
    AQUATIC_ITEM_INDEX, BOARD_BOTTOM, BoardCursor, COATL_EGG_ITEM_INDEX, FROST_CRYSTAL_ITEM_INDEX,
    GRID_BLOCK_SIZE, Ground, HEALTH_PANEL_INSET, INVENTORY_ITEMS, InventoryImages,
    LIFE_ESSENCE_ITEM_INDEX, OBSIDIAN_ITEM_INDEX, PlayerHealth, RESOLUTION_HEIGHT,
    RESOLUTION_WIDTH, STORM_SHARD_ITEM_INDEX, SUNSTONE_ITEM_INDEX, SelectedAction, Selection,
    SerpentSheet, THORNHEART_ITEM_INDEX, WATER_ITEM_INDEX, ai, history, menu,
};

const GOLD: Color = Color::srgb(0.86, 0.68, 0.32);
const TEXT: Color = Color::srgb(1.0, 0.94, 0.78);

struct Guide {
    basics: &'static str,
    reactions: &'static [(&'static str, &'static [ReactionVisual], &'static str)],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactionVisual {
    Ingredient(usize),
    Ground(Ground),
    Frozen,
    Poison,
    PoisonWater,
    Acid,
    AcidRidge,
    Ember,
    Thorns,
    HealingWater,
    Coatl,
    Player,
}

const GUIDES: [Guide; 14] = [
    Guide {
        basics: "Unlimited. Turns solid ground into permanent water. Cannot be placed on water or a hole.",
        reactions: &[
            (
                "Solid Ground",
                &[ReactionVisual::Ground(Ground::Solid)],
                "Creates a permanent water tile.",
            ),
            (
                "Ember Tile",
                &[ReactionVisual::Ember],
                "On solid ground, creates water and consumes the ember to produce steam. The center takes 1 heart and other affected tiles take half a heart; wind extends the steam.",
            ),
            (
                "Frost Crystal",
                &[ReactionVisual::Ingredient(FROST_CRYSTAL_ITEM_INDEX)],
                "On solid ground, creates water and freezes it and adjacent water. Grounded players on affected water freeze for two turns.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "On solid ground, creates water and an electrical pulse through connected water. Grounded players on water lose 1 heart; grounded neighbors lose half a heart.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "On solid ground, creates healing water. Grounded players on it recover half a heart per completed turn for three turns.",
            ),
            (
                "Life Essence + Ahuizotl Totem",
                &[
                    ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX),
                    ReactionVisual::Ingredient(AQUATIC_ITEM_INDEX),
                ],
                "On solid ground, creates water and awakens Ahuizotl.",
            ),
            (
                "Acidic Solid Ground",
                &[ReactionVisual::Acid],
                "Creates permanent acidic water.",
            ),
            (
                "Acidic Ridge",
                &[ReactionVisual::AcidRidge],
                "Levels the ridge to solid ground; the acid remains temporary.",
            ),
            (
                "Poisoned Solid Ground",
                &[ReactionVisual::Poison],
                "Creates water, pauses that poison's remaining duration for the action, and allows it to spread through connected water on later turns.",
            ),
            (
                "Thorned Solid Ground",
                &[ReactionVisual::Thorns],
                "Creates water. If this is the base of an actively growing Thorn Heart, its opposite vine can grow or resume.",
            ),
        ],
    },
    Guide {
        basics: "A limited ingredient. Sunstone can thaw a frozen player early.",
        reactions: &[
            (
                "Frozen Water Tile",
                &[ReactionVisual::Frozen],
                "Thaws the water and any frozen player on it without steam, unless another reaction takes priority.",
            ),
            (
                "Frozen Player",
                &[ReactionVisual::Player],
                "Thaws the player early, including on solid ground.",
            ),
            (
                "Ember Tile",
                &[ReactionVisual::Ember],
                "Ignites connected ember. Each burning tile deals 1 heart at its center and half a heart to neighbors.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates a cross-shaped explosion up to two tiles away, dealing 1.5 hearts on each affected tile.",
            ),
        ],
    },
    Guide {
        basics: "Freezes grounded players for two turns. A missed crystal on solid ground waits for a player to step or land on it; hovering does not trigger it. Cannot be placed on ember.",
        reactions: &[
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Freezes the target and adjacent water, including diagonals. Grounded players on affected water freeze for two turns.",
            ),
            (
                "Poisoned Water Tile",
                &[ReactionVisual::PoisonWater],
                "Freezes the water and pauses its poison until the tile thaws.",
            ),
            (
                "Grounded Player",
                &[ReactionVisual::Player],
                "A direct hit freezes the player for two turns.",
            ),
        ],
    },
    Guide {
        basics: "A dormant Thorn Heart remains collectible. Active thorns deal 1 heart per completed turn, even to hovering players. It cannot be placed on acid.",
        reactions: &[
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Activates a vine that grows for up to seven completed turns and then retracts. A water base adds an opposite vine.",
            ),
            (
                "Healing Water Tile",
                &[ReactionVisual::HealingWater],
                "Uses the active Life Essence to grow vines, including an opposite vine on water.",
            ),
        ],
    },
    Guide {
        basics: "A direct hit deals half a heart, including to a hovering player.",
        reactions: &[
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Electrifies connected water. Grounded players on it lose 1 heart; grounded players on neighboring tiles lose half a heart.",
            ),
            (
                "Acidic Tile",
                &[ReactionVisual::Acid],
                "Electrifies acid for five completed turns, adding 1 heart per turn to its normal damage. Acidic water also conducts the usual water pulse.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates an X-shaped explosion up to two tiles away, dealing 1.5 hearts on each affected tile.",
            ),
            (
                "Player",
                &[ReactionVisual::Player],
                "A direct hit deals half a heart, including to a hovering player.",
            ),
        ],
    },
    Guide {
        basics: "Unlimited. On water, it can only be placed where Obsidian or Life Essence is present. Terrain limits can prevent new holes.",
        reactions: &[
            (
                "Solid Ground",
                &[ReactionVisual::Ground(Ground::Solid)],
                "Creates a hole and removes items and tile effects from that tile.",
            ),
            (
                "Ridge",
                &[ReactionVisual::Ground(Ground::Ridge)],
                "Levels the ridge to solid ground.",
            ),
            (
                "Thorned Solid Ground",
                &[ReactionVisual::Thorns],
                "Creates a hole and dissolves the active vine touching it.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates an explosion on the center and eight neighbors, then creates holes in a cross. The blast deals 1 heart at the center and half a heart nearby.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Starts two opposite arms of growth; after four growth turns, the arms and center become holes where the terrain limit allows.",
            ),
            (
                "Healing Water Tile",
                &[ReactionVisual::HealingWater],
                "Consumes the active Life Essence and starts the hole-growing reaction.",
            ),
        ],
    },
    Guide {
        basics: "Cannot be placed in a hole. Explosions can ignite ember and other Obsidian; ridges can block blasts.",
        reactions: &[
            (
                "Sunstone",
                &[ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX)],
                "Creates a cross-shaped explosion up to two tiles away, dealing 1.5 hearts on each affected tile.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "Creates an X-shaped explosion up to two tiles away, dealing 1.5 hearts on each affected tile.",
            ),
        ],
    },
    Guide {
        basics: "Unlimited. On solid ground or a ridge, it requires acid, poison, Obsidian, or Life Essence.",
        reactions: &[
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Turns the water into solid ground, unless a higher-priority reaction such as Life Essence applies.",
            ),
            (
                "Frozen Water Tile",
                &[ReactionVisual::Frozen],
                "Turns the water into solid ground and removes its ice.",
            ),
            (
                "Hole",
                &[ReactionVisual::Ground(Ground::Hole)],
                "Fills the hole with solid ground.",
            ),
            (
                "Poisoned Tile",
                &[ReactionVisual::Poison],
                "Clears poison from the targeted tile. If the tile is water, also turns it into solid ground.",
            ),
            (
                "Acidic Tile",
                &[ReactionVisual::Acid],
                "Makes the acid temporary. If the tile is water, also turns it into solid ground.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates an explosion on the center and eight neighbors, then creates ridges in a cross. The blast deals 1 heart at the center and half a heart nearby.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Starts two opposite arms of growth; after four growth turns, the arms and center become ridges where the terrain limit allows.",
            ),
            (
                "Healing Water Tile",
                &[ReactionVisual::HealingWater],
                "Consumes the active Life Essence and starts the ridge-growing reaction.",
            ),
        ],
    },
    Guide {
        basics: "A limited ingredient. Leaves a layer on solid ground or a ridge. Cannot be placed on frozen water, a Frost Crystal, a hole, or an existing ember layer.",
        reactions: &[
            (
                "Solid Ground",
                &[ReactionVisual::Ground(Ground::Solid)],
                "Leaves an ember layer until it is ignited or the tile becomes a hole.",
            ),
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Consumes the ember to produce steam; the water remains. The center takes 1 heart and other affected tiles take half a heart; wind extends the steam.",
            ),
            (
                "Ridge",
                &[ReactionVisual::Ground(Ground::Ridge)],
                "Leaves an ember layer until it is ignited or the tile becomes a hole.",
            ),
            (
                "Sunstone",
                &[ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX)],
                "Ignites connected ember. Each burning tile deals 1 heart at its center and half a heart to neighbors.",
            ),
        ],
    },
    Guide {
        basics: "Acid damages grounded players and lasts five turns on solid ground or a ridge; it is permanent on water.",
        reactions: &[
            (
                "Solid Ground",
                &[ReactionVisual::Ground(Ground::Solid)],
                "Releases acid for five completed turns. Grounded players on it lose half a heart per turn.",
            ),
            (
                "Ridge",
                &[ReactionVisual::Ground(Ground::Ridge)],
                "Releases acid for five completed turns. Grounded players on it lose half a heart per turn.",
            ),
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Releases permanent acid. Grounded players on it lose half a heart per turn.",
            ),
            (
                "Poisoned Tile",
                &[ReactionVisual::Poison],
                "On a non-frozen tile, spreads acid and poison to the center and diagonal neighbors; holes are excluded and frozen tiles receive no poison.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "Consumes the placed shard and electrifies the acid for five completed turns.",
            ),
            (
                "Thorn Heart",
                &[ReactionVisual::Ingredient(THORNHEART_ITEM_INDEX)],
                "Dissolves the dormant Thorn Heart.",
            ),
            (
                "Thorned Tile",
                &[ReactionVisual::Thorns],
                "Dissolves the entire active vine touching the acid.",
            ),
            (
                "Tlaloc's Water on a Ridge",
                &[
                    ReactionVisual::Ingredient(WATER_ITEM_INDEX),
                    ReactionVisual::Ground(Ground::Ridge),
                ],
                "Acid and the water already on the ridge turn it into solid ground.",
            ),
            (
                "Coatl",
                &[ReactionVisual::Coatl],
                "Acid on any part of the summoned Coatl kills it during turn resolution.",
            ),
        ],
    },
    Guide {
        basics: "Poison lasts two completed turns. Grounded players on active poison lose half a heart per turn. Poison cannot be placed on frozen tiles.",
        reactions: &[
            (
                "Solid Ground",
                &[ReactionVisual::Ground(Ground::Solid)],
                "Releases poison for two completed turns. Grounded players on active poison lose half a heart per turn.",
            ),
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Releases poison that spreads through connected unfrozen water on later completed turns.",
            ),
            (
                "Acidic Tile",
                &[ReactionVisual::Acid],
                "On a non-frozen solid or water tile, spreads acid and poison to the center and diagonal neighbors; holes are excluded and frozen tiles receive no poison.",
            ),
            (
                "Coatl",
                &[ReactionVisual::Coatl],
                "Poison on the Coatl's tile can make its hit lethal if touched during the hit turn or the preceding completed turn.",
            ),
        ],
    },
    Guide {
        basics: "The egg remains collectible until activated. Coatl dies on acid or holes. Its normal hit deals 1.5 hearts but cannot reduce a living player below half a heart.",
        reactions: &[
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Summons a three-cell Coatl. It moves up to two cells after each completed turn toward a player.",
            ),
            (
                "Healing Water Tile",
                &[ReactionVisual::HealingWater],
                "Consumes the active Life Essence and summons a three-cell Coatl.",
            ),
        ],
    },
    Guide {
        basics: "On solid ground or a ridge, Life Essence remains collectible until activated.",
        reactions: &[
            (
                "Water Tile",
                &[ReactionVisual::Ground(Ground::Water)],
                "Creates healing water. Grounded players on the tile recover half a heart per completed turn for three turns.",
            ),
            (
                "Acidic Tile",
                &[ReactionVisual::Acid],
                "Removes the acid and its electrification, then behaves normally on the remaining terrain.",
            ),
            (
                "Thorn Heart",
                &[ReactionVisual::Ingredient(THORNHEART_ITEM_INDEX)],
                "Activates a thorn vine for up to seven turns; a water base adds an opposite vine.",
            ),
            (
                "Coatl's Egg",
                &[ReactionVisual::Ingredient(COATL_EGG_ITEM_INDEX)],
                "Summons Coatl.",
            ),
            (
                "Ahuizotl Totem on Water",
                &[
                    ReactionVisual::Ingredient(AQUATIC_ITEM_INDEX),
                    ReactionVisual::Ground(Ground::Water),
                ],
                "Awakens Ahuizotl.",
            ),
        ],
    },
    Guide {
        basics: "The dormant totem remains collectible. On water, it awakens with Life Essence. Ahuizotl hits hovering players too: 1 heart during growth, 1.5 during its sweep, at most once per turn.",
        reactions: &[
            (
                "Life Essence on Water",
                &[
                    ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX),
                    ReactionVisual::Ground(Ground::Water),
                ],
                "Awakens Ahuizotl. Four arms grow for three turns, then sweep to an X and retract.",
            ),
            (
                "Healing Water Tile",
                &[ReactionVisual::HealingWater],
                "Consumes the active Life Essence and awakens Ahuizotl.",
            ),
        ],
    },
];

#[derive(Resource, Default)]
pub(crate) struct Help {
    item: Option<usize>,
    root: Option<Entity>,
    dirty: bool,
}

impl Help {
    pub(crate) fn reset(&mut self, commands: &mut Commands) {
        if let Some(root) = self.root.take() {
            commands.entity(root).despawn();
        }
        self.item = None;
        self.dirty = false;
    }

    #[cfg(test)]
    pub(crate) fn item(&self) -> Option<usize> {
        self.item
    }
}

#[derive(Component)]
struct HelpTarget;

#[derive(Component)]
struct HelpClose;

#[derive(Component)]
struct HelpViewport;

#[derive(Component)]
struct HelpScrollbar;

#[derive(Component)]
struct ReactionIcon;

pub(crate) fn closed(help: Res<Help>) -> bool {
    help.item.is_none()
}

pub(crate) fn contains_target(position: Vec2) -> bool {
    let left = -RESOLUTION_WIDTH / 2.0 + 2.0 * GRID_BLOCK_SIZE + HEALTH_PANEL_INSET;
    let right = left + GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET;
    position.is_finite()
        && (left..right).contains(&position.x)
        && (-RESOLUTION_HEIGHT / 2.0 + HEALTH_PANEL_INSET..BOARD_BOTTOM - HEALTH_PANEL_INSET)
            .contains(&position.y)
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Help>()
        .add_systems(Startup, setup.after(crate::setup))
        .add_systems(
            Update,
            (
                input,
                update_target,
                render,
                history::scroll::<HelpViewport>,
                history::update_scrollbar::<HelpViewport, HelpScrollbar>,
            )
                .chain()
                .after(menu::GameplaySystems)
                .before(history::HistorySystems)
                .run_if(menu::in_game),
        );
}

fn setup(mut commands: Commands, hud: Res<Hud>) {
    commands.entity(hud.0).with_children(|parent| {
        parent
            .spawn((
                HelpTarget,
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(2.0 * GRID_BLOCK_SIZE + HEALTH_PANEL_INSET),
                    top: Val::Px(8.0 * GRID_BLOCK_SIZE + HEALTH_PANEL_INSET),
                    width: Val::Px(GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                    height: Val::Px(GRID_BLOCK_SIZE - 2.0 * HEALTH_PANEL_INSET),
                    border: UiRect::all(Val::Px(2.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.04, 0.03, 0.82)),
                BorderColor::all(GOLD),
                Pickable::IGNORE,
            ))
            .with_children(|target| {
                text(target, "?", 25.0, GOLD);
                text(target, "HELP", 9.0, TEXT);
            });
    });
}

fn input(
    controls: (
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
        Res<BoardCursor>,
    ),
    selection: Res<Selection>,
    bot: Res<ai::BotController>,
    health: Res<PlayerHealth>,
    pressed: Query<(), (With<HelpClose>, Added<Pressed>)>,
    mut help: ResMut<Help>,
) {
    let (mouse, keyboard, cursor) = controls;
    if help.item.is_some() {
        if keyboard.just_pressed(KeyCode::Escape) || !pressed.is_empty() {
            help.item = None;
            help.dirty = true;
        }
        return;
    }
    if !bot.bot_turn
        && health.0.iter().all(|&health| health > 0)
        && mouse.just_pressed(MouseButton::Left)
        && cursor.0.is_some_and(contains_target)
        && let Some(SelectedAction::Item { index, .. }) = &selection.0
    {
        help.item = Some(*index);
        help.dirty = true;
    }
}

fn update_target(
    selection: Res<Selection>,
    cursor: Res<BoardCursor>,
    bot: Res<ai::BotController>,
    help: Res<Help>,
    mut targets: Query<(&mut Visibility, &mut BackgroundColor), With<HelpTarget>>,
) {
    for (mut visibility, mut background) in &mut targets {
        *visibility = if !bot.bot_turn
            && help.item.is_none()
            && matches!(selection.0, Some(SelectedAction::Item { .. }))
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        background.0 = if cursor.0.is_some_and(contains_target) {
            Color::srgba(0.4, 0.28, 0.14, 0.94)
        } else {
            Color::srgba(0.06, 0.04, 0.03, 0.82)
        };
    }
}

fn render(
    mut commands: Commands,
    hud: Res<Hud>,
    images: Res<InventoryImages>,
    serpent: Res<SerpentSheet>,
    mut help: ResMut<Help>,
) {
    if !help.dirty {
        return;
    }
    help.dirty = false;
    if let Some(root) = help.root.take() {
        commands.entity(root).despawn();
    }
    let Some(index) = help.item else {
        return;
    };
    let guide = &GUIDES[index];
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
            GlobalZIndex(90),
        ))
        .id();
    commands.entity(hud.0).add_child(root);
    commands.entity(root).with_children(|overlay| {
        overlay
            .spawn((
                Node {
                    width: Val::Px(760.0),
                    height: Val::Px(RESOLUTION_HEIGHT - 50.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    border: UiRect::all(Val::Px(3.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.98)),
                BorderColor::all(GOLD),
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|heading| {
                        heading.spawn((
                            images.icon(index),
                            Node {
                                width: Val::Px(42.0),
                                height: Val::Px(42.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        text(heading, INVENTORY_ITEMS[index].0, 25.0, GOLD);
                    });
                text(panel, guide.basics, 14.0, TEXT);
                text(panel, "REACTIONS", 14.0, GOLD);
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        column_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|row| {
                        let viewport = row
                            .spawn((
                                HelpViewport,
                                Hovered::default(),
                                ScrollPosition::default(),
                                Node {
                                    flex_grow: 1.0,
                                    min_width: Val::Px(0.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .with_children(|body| {
                                for &(name, visuals, description) in guide.reactions {
                                    body.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            min_height: Val::Px(70.0),
                                            flex_shrink: 0.0,
                                            border: UiRect {
                                                bottom: Val::Px(1.0),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                        BorderColor::all(Color::srgba(0.86, 0.68, 0.32, 0.4)),
                                        Pickable::IGNORE,
                                    ))
                                    .with_children(|entry| {
                                        entry
                                            .spawn((
                                                Node {
                                                    width: Val::Px(135.0),
                                                    flex_shrink: 0.0,
                                                    padding: UiRect::all(Val::Px(5.0)),
                                                    border: UiRect {
                                                        right: Val::Px(1.0),
                                                        ..default()
                                                    },
                                                    flex_direction: FlexDirection::Column,
                                                    align_items: AlignItems::Center,
                                                    justify_content: JustifyContent::Center,
                                                    row_gap: Val::Px(3.0),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgba(0.2, 0.15, 0.1, 0.45)),
                                                BorderColor::all(Color::srgba(
                                                    0.86, 0.68, 0.32, 0.4,
                                                )),
                                                Pickable::IGNORE,
                                            ))
                                            .with_children(|cell| {
                                                cell.spawn((
                                                    Node {
                                                        column_gap: Val::Px(4.0),
                                                        justify_content: JustifyContent::Center,
                                                        ..default()
                                                    },
                                                    Pickable::IGNORE,
                                                ))
                                                .with_children(|icons| {
                                                    for &visual in visuals {
                                                        spawn_visual(
                                                            icons, visual, &images, &serpent,
                                                        );
                                                    }
                                                });
                                                text(cell, name, 12.0, GOLD);
                                            });
                                        entry
                                            .spawn((
                                                Node {
                                                    flex_grow: 1.0,
                                                    min_width: Val::Px(0.0),
                                                    padding: UiRect::all(Val::Px(8.0)),
                                                    flex_direction: FlexDirection::Column,
                                                    justify_content: JustifyContent::Center,
                                                    row_gap: Val::Px(4.0),
                                                    ..default()
                                                },
                                                Pickable::IGNORE,
                                            ))
                                            .with_children(|cell| {
                                                text(
                                                    cell,
                                                    &format!(
                                                        "{} + {name}",
                                                        INVENTORY_ITEMS[index].0
                                                    ),
                                                    15.0,
                                                    GOLD,
                                                );
                                                text(cell, description, 14.0, TEXT);
                                            });
                                    });
                                }
                            })
                            .id();
                        row.spawn((
                            HelpScrollbar,
                            Visibility::Hidden,
                            Node {
                                width: Val::Px(10.0),
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
                        .with_children(|bar| {
                            bar.spawn((
                                Hovered::default(),
                                BackgroundColor(GOLD),
                                ScrollbarThumb {
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    border: UiRect::ZERO,
                                },
                            ));
                        });
                    });
                panel
                    .spawn((
                        HelpClose,
                        Button,
                        Hovered::default(),
                        Node {
                            align_self: AlignSelf::Center,
                            min_width: Val::Px(160.0),
                            height: Val::Px(40.0),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.94)),
                        BorderColor::all(GOLD),
                    ))
                    .with_children(|button| text(button, "CLOSE (ESC)", 15.0, TEXT));
            });
    });
    help.root = Some(root);
}

fn spawn_visual(
    parent: &mut ChildSpawnerCommands,
    visual: ReactionVisual,
    images: &InventoryImages,
    serpent: &SerpentSheet,
) {
    let node = Node {
        width: Val::Px(30.0),
        height: Val::Px(30.0),
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    if let ReactionVisual::Ingredient(index) = visual {
        parent.spawn((ReactionIcon, images.icon(index), node, Pickable::IGNORE));
        return;
    }
    if visual == ReactionVisual::Coatl {
        parent.spawn((
            ReactionIcon,
            ImageNode {
                image: serpent.0.clone(),
                rect: Some(Rect::new(0.0, 0.0, 96.0, 68.0)),
                ..default()
            },
            node,
            Pickable::IGNORE,
        ));
        return;
    }

    let (ground, overlay) = match visual {
        ReactionVisual::Ground(ground) => (ground.color(), None),
        ReactionVisual::Frozen => (
            Ground::Water.color(),
            Some(Color::srgba(0.7, 0.94, 1.0, 0.8)),
        ),
        ReactionVisual::Poison => (
            Ground::Solid.color(),
            Some(Color::srgba(0.65, 0.1, 0.75, 0.55)),
        ),
        ReactionVisual::PoisonWater => (
            Ground::Water.color(),
            Some(Color::srgba(0.65, 0.1, 0.75, 0.55)),
        ),
        ReactionVisual::Acid => (
            Ground::Solid.color(),
            Some(Color::srgba(0.25, 0.8, 0.1, 0.55)),
        ),
        ReactionVisual::AcidRidge => (
            Ground::Ridge.color(),
            Some(Color::srgba(0.25, 0.8, 0.1, 0.55)),
        ),
        ReactionVisual::Ember => (
            Ground::Solid.color(),
            Some(Color::srgba(1.0, 0.35, 0.0, 0.65)),
        ),
        ReactionVisual::HealingWater => (
            Ground::Water.color(),
            Some(Color::srgba(0.25, 0.9, 0.55, 0.65)),
        ),
        ReactionVisual::Thorns => (
            Ground::Solid.color(),
            Some(Color::srgba(0.2, 0.55, 0.15, 0.85)),
        ),
        ReactionVisual::Player => (Ground::Solid.color(), Some(Color::srgb(0.15, 0.45, 0.9))),
        ReactionVisual::Ingredient(_) | ReactionVisual::Coatl => unreachable!(),
    };
    parent
        .spawn((
            ReactionIcon,
            node,
            BackgroundColor(ground),
            Pickable::IGNORE,
        ))
        .with_children(|tile| {
            let inset = Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                ..default()
            };
            if let Some(color) = overlay {
                tile.spawn((inset, BackgroundColor(color), Pickable::IGNORE));
            }
        });
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Node {
            max_width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ACID_ITEM_INDEX, POISON_ITEM_INDEX, STONE_DUST_ITEM_INDEX, VOID_DUST_ITEM_INDEX};

    #[test]
    fn guides_list_directional_placements_with_valid_visuals() {
        assert_eq!(
            GUIDES
                .iter()
                .map(|guide| guide.reactions.len())
                .sum::<usize>(),
            65
        );
        for (index, guide) in GUIDES.iter().enumerate() {
            for &(name, visuals, description) in guide.reactions {
                assert!(!name.is_empty() && !description.is_empty());
                assert!(!name.starts_with(INVENTORY_ITEMS[index].0));
                assert!(!visuals.is_empty(), "{name} has no visual");
                assert!(
                    visuals.len() <= 2,
                    "{name} has too many visuals for its cell"
                );
                for visual in visuals {
                    if let ReactionVisual::Ingredient(index) = visual {
                        assert!(*index < INVENTORY_ITEMS.len(), "{name} has an invalid icon");
                    }
                }
            }
        }

        let targets = |index: usize| {
            GUIDES[index]
                .reactions
                .iter()
                .map(|row| row.0)
                .collect::<Vec<_>>()
        };
        assert!(targets(STONE_DUST_ITEM_INDEX).contains(&"Poisoned Tile"));
        assert!(targets(STONE_DUST_ITEM_INDEX).contains(&"Acidic Tile"));
        assert!(!targets(STONE_DUST_ITEM_INDEX).contains(&"Poison Frog Sphere"));
        assert!(!targets(LIFE_ESSENCE_ITEM_INDEX).contains(&"Void Dust"));
        assert!(!targets(OBSIDIAN_ITEM_INDEX).contains(&"Void Dust"));
        assert!(targets(VOID_DUST_ITEM_INDEX).contains(&"Thorned Solid Ground"));
        assert!(targets(ACID_ITEM_INDEX).contains(&"Thorned Tile"));
        assert_eq!(
            GUIDES[POISON_ITEM_INDEX]
                .reactions
                .iter()
                .find(|row| row.0 == "Coatl")
                .unwrap()
                .1,
            &[ReactionVisual::Coatl]
        );
        assert_eq!(INVENTORY_ITEMS[AQUATIC_ITEM_INDEX].0, "Ahuizotl Totem");
        assert!(targets(LIFE_ESSENCE_ITEM_INDEX).contains(&"Ahuizotl Totem on Water"));
    }

    #[test]
    fn rendering_shows_full_equations_and_descriptions_for_every_guide() {
        let mut app = App::new();
        let hud = app.world_mut().spawn_empty().id();
        app.insert_resource(Hud(hud))
            .init_resource::<InventoryImages>()
            .init_resource::<SerpentSheet>()
            .init_resource::<Help>()
            .add_systems(Update, render);
        for (index, guide) in GUIDES.iter().enumerate() {
            let mut help = app.world_mut().resource_mut::<Help>();
            help.item = Some(index);
            help.dirty = true;
            app.update();

            let world = app.world_mut();
            let mut labels = world.query::<&Text>();
            let labels: Vec<_> = labels.iter(world).map(|text| text.0.as_str()).collect();
            for &(target, _, description) in guide.reactions {
                assert!(
                    labels.contains(&format!("{} + {target}", INVENTORY_ITEMS[index].0).as_str())
                );
                assert!(labels.contains(&description));
            }
        }
    }

    #[test]
    fn rendering_spawns_images_and_terrain_visuals_for_each_reaction() {
        let mut app = App::new();
        let hud = app.world_mut().spawn_empty().id();
        app.insert_resource(Hud(hud))
            .init_resource::<InventoryImages>()
            .init_resource::<SerpentSheet>()
            .insert_resource(Help {
                item: Some(STONE_DUST_ITEM_INDEX),
                dirty: true,
                ..default()
            })
            .add_systems(Update, render);
        app.update();

        let world = app.world_mut();
        let mut icons = world.query_filtered::<(&Node, Option<&ImageNode>, Option<&BackgroundColor>), With<ReactionIcon>>();
        let icons: Vec<_> = icons.iter(world).collect();
        let expected = GUIDES[STONE_DUST_ITEM_INDEX]
            .reactions
            .iter()
            .map(|(_, visuals, _)| visuals.len())
            .sum::<usize>();
        assert_eq!(icons.len(), expected);
        assert!(icons.iter().any(|(_, image, _)| image.is_some()));
        assert!(icons.iter().any(|(_, _, background)| background.is_some()));
        let expected_rects: Vec<_> = GUIDES[STONE_DUST_ITEM_INDEX]
            .reactions
            .iter()
            .flat_map(|(_, visuals, _)| visuals.iter())
            .filter_map(|visual| match visual {
                ReactionVisual::Ingredient(index) => Some(InventoryImages::rect(*index)),
                _ => None,
            })
            .collect();
        let rects: Vec<_> = icons
            .iter()
            .filter_map(|(_, image, _)| image.and_then(|image| image.rect))
            .collect();
        assert_eq!(rects, expected_rects);

        let viewport = world
            .query_filtered::<Entity, With<HelpViewport>>()
            .single(world)
            .unwrap();
        let mut pending = vec![viewport];
        while let Some(parent) = pending.pop() {
            if let Some(children) = world.get::<Children>(parent) {
                for child in children.iter() {
                    let pickable = world.get::<Pickable>(child).unwrap();
                    assert!(!pickable.should_block_lower);
                    assert!(!pickable.is_hoverable);
                    pending.push(child);
                }
            }
        }
    }

    #[test]
    fn close_button_dismisses_the_guide_without_clearing_selection() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<ai::BotController>()
            .init_resource::<PlayerHealth>()
            .insert_resource(Help {
                item: Some(1),
                ..default()
            })
            .add_systems(Update, input);
        let world = app.world_mut();
        let icon = world.spawn_empty().id();
        let highlight = world.spawn_empty().id();
        world.resource_mut::<Selection>().0 = Some(SelectedAction::Item {
            index: 1,
            icon,
            highlight,
        });
        world.spawn((HelpClose, Pressed));
        app.update();
        assert_eq!(app.world().resource::<Help>().item, None);
        assert!(matches!(
            app.world().resource::<Selection>().0,
            Some(SelectedAction::Item { index: 1, .. })
        ));
    }

    #[test]
    fn escape_closes_help_before_pausing_the_match() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<BoardCursor>()
            .init_resource::<Selection>()
            .init_resource::<ai::BotController>()
            .init_resource::<PlayerHealth>()
            .insert_resource(menu::Frontend::computer_match())
            .insert_resource(Help {
                item: Some(1),
                ..default()
            })
            .add_systems(
                Update,
                (menu::pause_match, input.run_if(menu::in_game)).chain(),
            );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert_eq!(app.world().resource::<Help>().item, None);
        assert!(app.world().resource::<menu::Frontend>().computer_game());
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        keys.press(KeyCode::Escape);
        app.update();
        assert!(!app.world().resource::<menu::Frontend>().computer_game());
    }
}
