use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Button, ControlOrientation, Scrollbar, ScrollbarThumb},
};
use bevy_aspect_ratio_mask::Hud;

use crate::{
    ACID_ITEM_INDEX, AQUATIC_ITEM_INDEX, BOARD_BOTTOM, BoardCursor, COATL_EGG_ITEM_INDEX,
    FROST_CRYSTAL_ITEM_INDEX, GRID_BLOCK_SIZE, Ground, HEALTH_PANEL_INSET, INVENTORY_ITEMS,
    InventoryImages, LIFE_ESSENCE_ITEM_INDEX, OBSIDIAN_ITEM_INDEX, OIL_ITEM_INDEX,
    POISON_ITEM_INDEX, PlayerHealth, RESOLUTION_HEIGHT, RESOLUTION_WIDTH, STONE_DUST_ITEM_INDEX,
    STORM_SHARD_ITEM_INDEX, SUNSTONE_ITEM_INDEX, SelectedAction, Selection, THORNHEART_ITEM_INDEX,
    VOID_DUST_ITEM_INDEX, WATER_ITEM_INDEX, ai, history, menu,
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
    WetGround,
    Frozen,
    Poison,
    Acid,
    Explosion,
    Electricity,
    Aquatic,
    Player,
}

const GUIDES: [Guide; 14] = [
    Guide {
        basics: "Unlimited. Turns solid ground into permanent water. Cannot be placed on water or a hole.",
        reactions: &[
            (
                "Sunstone",
                &[ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX)],
                "Creates steam: 1 heart on the target, half a heart nearby. Wind extends it; ridges block it.",
            ),
            (
                "Frost Crystal",
                &[ReactionVisual::Ingredient(FROST_CRYSTAL_ITEM_INDEX)],
                "Freezes this tile and adjacent water. Grounded players there are frozen for two turns.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "Electrifies connected water: 1 heart on water, half a heart next to it. Hovering prevents the shock.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Creates healing water: grounded players recover half a heart per completed turn for three turns. With an Aquatic Creature and Life Essence, awakens the creature.",
            ),
            (
                "Toad Wart / poison",
                &[
                    ReactionVisual::Ingredient(ACID_ITEM_INDEX),
                    ReactionVisual::Poison,
                ],
                "Makes acid on ground permanent and lets poison spread through connected water. Water and acid on a ridge level it.",
            ),
            (
                "Stone Dust",
                &[ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX)],
                "Dries water to solid ground. Stone Dust with Life Essence instead grows ridges.",
            ),
            (
                "Void Dust",
                &[ReactionVisual::Ingredient(VOID_DUST_ITEM_INDEX)],
                "Does not react. Void Dust cannot be placed on natural water without Obsidian or Life Essence.",
            ),
        ],
    },
    Guide {
        basics: "A limited ingredient. Place it on your own frozen player to thaw them.",
        reactions: &[
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "Creates steam: 1 heart on the target, half a heart nearby. Wind extends it; ridges block it.",
            ),
            (
                "Frozen water",
                &[ReactionVisual::Frozen],
                "Thaws the ice and frozen players on the tile without steam, unless another reaction occurs.",
            ),
            (
                "Tlexochtli Ember",
                &[ReactionVisual::Ingredient(OIL_ITEM_INDEX)],
                "Ignites connected ember. Each burning tile deals 1 heart at its center and half a heart to neighbors.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates a cross-shaped blast up to two tiles away, dealing 1.5 hearts per affected player.",
            ),
        ],
    },
    Guide {
        basics: "Freezes grounded players for two turns. On dry ground, a missed crystal waits until a player steps or lands on it; hovering does not trigger it.",
        reactions: &[
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "Freezes the tile and adjacent water. Grounded players on it are frozen for two turns.",
            ),
            (
                "Frozen tiles",
                &[ReactionVisual::Frozen],
                "A grounded player ending their own turn on ice loses half a heart, whether frozen or not.",
            ),
            (
                "Sunstone",
                &[ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX)],
                "Thaws a frozen player and, on frozen water, the tile without steam.",
            ),
            (
                "Poison",
                &[ReactionVisual::Poison],
                "Ice pauses poison on the tile until it thaws. Frost does not remove acid.",
            ),
            (
                "Stone Dust / explosions",
                &[
                    ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX),
                    ReactionVisual::Explosion,
                ],
                "Stone Dust turns frozen water to ground; explosions thaw ice. A new hole or ridge frees a frozen player on it.",
            ),
        ],
    },
    Guide {
        basics: "Dormant Thorn Heart remains collectible. Active thorns deal 1 heart per completed turn, even to hovering players.",
        reactions: &[
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Grows a thorn vine for up to seven turns, then retracts. Water or wet ground adds an opposite vine.",
            ),
            (
                "Frost",
                &[ReactionVisual::Ingredient(FROST_CRYSTAL_ITEM_INDEX)],
                "Frozen tiles block a growing vine until they thaw.",
            ),
            (
                "Toad Wart",
                &[ReactionVisual::Ingredient(ACID_ITEM_INDEX)],
                "Acid dissolves dormant Thorn Heart or an entire active vine; it cannot be placed on acid.",
            ),
            (
                "Explosions / holes",
                &[
                    ReactionVisual::Explosion,
                    ReactionVisual::Ground(Ground::Hole),
                ],
                "Explosions remove thorns in their footprint. A new hole touching an active vine dissolves it.",
            ),
        ],
    },
    Guide {
        basics: "A direct hit deals half a heart, including to a hovering player.",
        reactions: &[
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "Electrifies connected water: 1 heart on water and half a heart on neighboring tiles. Hovering prevents the shock.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Creates an X-shaped blast up to two tiles away, dealing 1.5 hearts per affected player.",
            ),
            (
                "Toad Wart",
                &[ReactionVisual::Ingredient(ACID_ITEM_INDEX)],
                "Electrifies acid for five turns, adding 1 heart per turn to its normal half-heart damage.",
            ),
            (
                "Poison / frost",
                &[ReactionVisual::Poison, ReactionVisual::Frozen],
                "No special reaction. Electricity conducts through poisoned water and does not thaw ice.",
            ),
        ],
    },
    Guide {
        basics: "Unlimited. On solid ground makes a hole; on a ridge makes solid ground. On natural water it needs Obsidian or Life Essence.",
        reactions: &[
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Explodes at the center and eight neighbors, then creates holes in a cross. The blast deals 1 heart at the center and half a heart nearby.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Grows two opposite arms over four turns; on the following turn their tiles become holes, while the center stays unchanged.",
            ),
            (
                "Other ingredients",
                &[
                    ReactionVisual::Ingredient(WATER_ITEM_INDEX),
                    ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX),
                    ReactionVisual::Ingredient(FROST_CRYSTAL_ITEM_INDEX),
                ],
                "No special reaction. Items and tile effects on newly created holes are removed.",
            ),
            (
                "Terrain limit",
                &[
                    ReactionVisual::Ground(Ground::Ridge),
                    ReactionVisual::Ground(Ground::Hole),
                ],
                "Immediate holes that exceed the combined ridge and hole limit are rejected. Delayed growth stops where the limit prevents it.",
            ),
        ],
    },
    Guide {
        basics: "Cannot be placed in a hole. Explosions can ignite ember and other Obsidian.",
        reactions: &[
            (
                "Sunstone",
                &[ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX)],
                "Creates a cross-shaped blast up to two tiles away, dealing 1.5 hearts on each affected tile.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "Creates an X-shaped blast up to two tiles away, dealing 1.5 hearts on each affected tile. Electricity reaching Obsidian through water also triggers it; the water shock adds its normal damage.",
            ),
            (
                "Void Dust",
                &[ReactionVisual::Ingredient(VOID_DUST_ITEM_INDEX)],
                "Explodes at the center and eight neighbors, then makes holes in a cross.",
            ),
            (
                "Stone Dust",
                &[ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX)],
                "Explodes at the center and eight neighbors, then makes ridges in a cross.",
            ),
            (
                "Ridges / explosions",
                &[
                    ReactionVisual::Ground(Ground::Ridge),
                    ReactionVisual::Explosion,
                ],
                "A blast starting off a ridge stops before ridges. An Obsidian hit by any explosion detonates in a cross.",
            ),
        ],
    },
    Guide {
        basics: "Unlimited. Cannot be placed on untainted dry ground or a ridge unless Obsidian or Life Essence is present.",
        reactions: &[
            (
                "Water / ice / holes",
                &[
                    ReactionVisual::Ground(Ground::Water),
                    ReactionVisual::Frozen,
                    ReactionVisual::Ground(Ground::Hole),
                ],
                "Turns water or frozen water into dry solid ground, and fills a hole with solid ground.",
            ),
            (
                "Poison / acid",
                &[ReactionVisual::Poison, ReactionVisual::Acid],
                "Clears poison. Makes acid temporary; on acidic water it also creates solid ground.",
            ),
            (
                "Obsidian",
                &[ReactionVisual::Ingredient(OBSIDIAN_ITEM_INDEX)],
                "Explodes at the center and eight neighbors, then makes ridges in a cross. The blast deals 1 heart at the center and half a heart nearby.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Grows two opposite arms over four turns; on the following turn their tiles become ridges, while the center stays unchanged.",
            ),
            (
                "Void Dust / Sunstone",
                &[
                    ReactionVisual::Ingredient(VOID_DUST_ITEM_INDEX),
                    ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX),
                ],
                "No special reaction.",
            ),
        ],
    },
    Guide {
        basics: "Unlimited. Creates a permanent ember layer on any tile except a hole.",
        reactions: &[
            (
                "Sunstone / explosions",
                &[
                    ReactionVisual::Ingredient(SUNSTONE_ITEM_INDEX),
                    ReactionVisual::Explosion,
                ],
                "Ignites and consumes connected ember. Each burning tile deals 1 heart at its center and half a heart to neighbors; overlapping blasts add.",
            ),
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "An ember explosion centered on unfrozen water dries it to solid ground. The explosion does not produce steam.",
            ),
            (
                "Frost / electricity",
                &[ReactionVisual::Frozen, ReactionVisual::Electricity],
                "Ember does not change freezing or electricity and remains until ignited or its tile becomes a hole.",
            ),
        ],
    },
    Guide {
        basics: "Releases acid: half a heart per completed turn to grounded players. Acid lasts five turns on ground or a ridge and is permanent on water.",
        reactions: &[
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "Makes acid on ground permanent. Water and acid on a ridge level it to solid ground.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "Electrifies acid for five turns, adding 1 heart per turn to its normal damage.",
            ),
            (
                "Poison Frog Sphere",
                &[ReactionVisual::Ingredient(POISON_ITEM_INDEX)],
                "Spreads acid and poison in an X on the center and diagonal neighbors, excluding holes; frozen tiles receive no poison.",
            ),
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Removes acid and electrification from the tile.",
            ),
            (
                "Stone Dust",
                &[ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX)],
                "Makes acid temporary; on acidic water it also creates solid ground.",
            ),
            (
                "Thorn Heart",
                &[ReactionVisual::Ingredient(THORNHEART_ITEM_INDEX)],
                "Dissolves dormant Thorn Heart or an entire active vine.",
            ),
        ],
    },
    Guide {
        basics: "Releases poison for two completed turns. Grounded players on active poison lose half a heart per turn.",
        reactions: &[
            (
                "Water / wet ground",
                &[
                    ReactionVisual::Ground(Ground::Water),
                    ReactionVisual::WetGround,
                ],
                "Poison spreads one tile per completed turn through connected unfrozen wet tiles, including diagonals.",
            ),
            (
                "Toad Wart",
                &[ReactionVisual::Ingredient(ACID_ITEM_INDEX)],
                "Spreads acid and poison in an X on the center and diagonal neighbors, excluding holes; frozen tiles receive no poison.",
            ),
            (
                "Frost",
                &[ReactionVisual::Ingredient(FROST_CRYSTAL_ITEM_INDEX)],
                "Poison on ice pauses without damage or spreading until thawed. Poison cannot be placed on a frozen tile.",
            ),
            (
                "Stone Dust",
                &[ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX)],
                "Clears poison from the targeted tile.",
            ),
            (
                "Coatl's Egg",
                &[ReactionVisual::Ingredient(COATL_EGG_ITEM_INDEX)],
                "A summoned Coatl that touched active poison on its hit turn or the previous turn reduces its target to zero hearts.",
            ),
            (
                "Storm Shard",
                &[ReactionVisual::Ingredient(STORM_SHARD_ITEM_INDEX)],
                "No special reaction; electricity still conducts through poisoned water.",
            ),
        ],
    },
    Guide {
        basics: "The egg remains collectible until it reacts.",
        reactions: &[
            (
                "Life Essence",
                &[ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX)],
                "Summons a three-cell Coatl. It moves after each completed turn toward the player with the most health.",
            ),
            (
                "Poison",
                &[ReactionVisual::Poison],
                "If the Coatl touched active poison on its hit turn or the previous turn, its hit reduces the target to zero hearts.",
            ),
            (
                "Acid / holes",
                &[ReactionVisual::Acid, ReactionVisual::Ground(Ground::Hole)],
                "A Coatl dies on contact with acid or a hole.",
            ),
            (
                "Normal hit",
                &[ReactionVisual::Player],
                "Deals 1.5 hearts, but cannot reduce a living player below half a heart. A Coatl disappears after hitting.",
            ),
        ],
    },
    Guide {
        basics: "On dry ground or a ridge, Life Essence remains collectible until activated.",
        reactions: &[
            (
                "Water",
                &[ReactionVisual::Ground(Ground::Water)],
                "Heals grounded players on the tile by half a heart per completed turn for three turns.",
            ),
            (
                "Toad Wart",
                &[ReactionVisual::Ingredient(ACID_ITEM_INDEX)],
                "Removes acid and electrification, then behaves normally on the remaining terrain.",
            ),
            (
                "Thorn Heart",
                &[ReactionVisual::Ingredient(THORNHEART_ITEM_INDEX)],
                "Grows thorns for up to seven turns; water or wet ground adds an opposite vine.",
            ),
            (
                "Void Dust",
                &[ReactionVisual::Ingredient(VOID_DUST_ITEM_INDEX)],
                "Grows two opposite arms that become holes along with the center after four growth turns.",
            ),
            (
                "Stone Dust",
                &[ReactionVisual::Ingredient(STONE_DUST_ITEM_INDEX)],
                "Grows two opposite arms that become ridges along with the center after four growth turns.",
            ),
            (
                "Coatl's Egg",
                &[ReactionVisual::Ingredient(COATL_EGG_ITEM_INDEX)],
                "Summons a three-cell Coatl.",
            ),
            (
                "Aquatic Creature",
                &[ReactionVisual::Ingredient(AQUATIC_ITEM_INDEX)],
                "Together on water, awakens an aquatic creature. Adding water to both on dry ground also awakens it.",
            ),
        ],
    },
    Guide {
        basics: "A dormant Aquatic Creature remains collectible until activated.",
        reactions: &[
            (
                "Life Essence + water",
                &[
                    ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX),
                    ReactionVisual::Ground(Ground::Water),
                ],
                "Awakens the creature. Water can be added after both ingredients are on dry ground.",
            ),
            (
                "Growth",
                &[ReactionVisual::Aquatic],
                "Four arms grow in a cross for three turns. Ridges stop an arm during growth.",
            ),
            (
                "Rotation",
                &[ReactionVisual::Aquatic],
                "The arms sweep clockwise for two turns, leveling ridges and removing collectibles in their path.",
            ),
            (
                "Player contact",
                &[ReactionVisual::Player],
                "Deals 1 heart per growth turn or 1.5 hearts per rotation turn, even to hovering players. Each creature hits a player at most once per turn.",
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
                            ImageNode::new(images.0[index].clone()),
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
                                                        spawn_visual(icons, visual, &images);
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
                                                    align_items: AlignItems::Center,
                                                    ..default()
                                                },
                                                Pickable::IGNORE,
                                            ))
                                            .with_children(|cell| {
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
        parent.spawn((
            ReactionIcon,
            ImageNode::new(images.0[index].clone()),
            node,
            Pickable::IGNORE,
        ));
        return;
    }

    let (ground, overlay) = match visual {
        ReactionVisual::Ground(ground) => (ground.color(), None),
        ReactionVisual::WetGround => (Ground::Solid.color(), None),
        ReactionVisual::Frozen => (
            Ground::Water.color(),
            Some(Color::srgba(0.7, 0.94, 1.0, 0.8)),
        ),
        ReactionVisual::Poison => (
            Ground::Solid.color(),
            Some(Color::srgba(0.65, 0.1, 0.75, 0.55)),
        ),
        ReactionVisual::Acid => (
            Ground::Solid.color(),
            Some(Color::srgba(0.25, 0.8, 0.1, 0.55)),
        ),
        ReactionVisual::Explosion => (
            Ground::Solid.color(),
            Some(Color::srgba(1.0, 0.35, 0.0, 0.65)),
        ),
        ReactionVisual::Electricity => (
            Ground::Water.color(),
            Some(Color::srgba(1.0, 0.9, 0.1, 0.65)),
        ),
        ReactionVisual::Aquatic => (
            Ground::Water.color(),
            Some(Color::srgba(0.08, 0.63, 0.75, 0.85)),
        ),
        ReactionVisual::Player => (Ground::Solid.color(), Some(Color::srgb(0.15, 0.45, 0.9))),
        ReactionVisual::Ingredient(_) => unreachable!(),
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
            } else if visual == ReactionVisual::WetGround {
                tile.spawn((
                    inset,
                    ImageNode::new(images.0[WATER_ITEM_INDEX].clone()),
                    Pickable::IGNORE,
                ));
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

    #[test]
    fn every_reaction_has_valid_visuals_and_compound_reactions_show_each_part() {
        for guide in &GUIDES {
            for &(name, visuals, _) in guide.reactions {
                assert!(!visuals.is_empty(), "{name} has no visual");
                assert!(
                    visuals.len() <= 3,
                    "{name} has too many visuals for its cell"
                );
                for visual in visuals {
                    if let ReactionVisual::Ingredient(index) = visual {
                        assert!(*index < INVENTORY_ITEMS.len(), "{name} has an invalid icon");
                    }
                }
            }
        }

        assert_eq!(
            GUIDES[7].reactions[0].1,
            &[
                ReactionVisual::Ground(Ground::Water),
                ReactionVisual::Frozen,
                ReactionVisual::Ground(Ground::Hole),
            ]
        );
        assert_eq!(
            GUIDES[0].reactions[4].1,
            &[
                ReactionVisual::Ingredient(ACID_ITEM_INDEX),
                ReactionVisual::Poison,
            ]
        );
        assert_eq!(
            GUIDES[13].reactions[0].1,
            &[
                ReactionVisual::Ingredient(LIFE_ESSENCE_ITEM_INDEX),
                ReactionVisual::Ground(Ground::Water),
            ]
        );
    }

    #[test]
    fn rendering_spawns_images_and_terrain_visuals_for_each_reaction() {
        let mut app = App::new();
        let hud = app.world_mut().spawn_empty().id();
        app.insert_resource(Hud(hud))
            .insert_resource(InventoryImages(std::array::from_fn(|_| Handle::default())))
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
