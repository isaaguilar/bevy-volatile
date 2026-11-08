use bevy::{picking::hover::Hovered, prelude::*, ui::Pressed, ui_widgets::Button};
use bevy_aspect_ratio_mask::Hud;

use crate::{PlayerHealth, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};

const SPLASH_SECONDS: f32 = 1.5;
const RULE_PAGE_CHARS: usize = 750;
const GOLD: Color = Color::srgb(0.86, 0.68, 0.32);
const TEXT: Color = Color::srgb(1.0, 0.94, 0.78);
const BUTTON_TEXT: Color = Color::srgb(0.08, 0.08, 0.08);

fn button_rect(selected: bool) -> Rect {
    let top = if selected { 302.0 } else { 0.0 };
    Rect::new(0.0, top, 755.0, top + 302.0)
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GameplaySystems;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Frontend>()
        .add_systems(Startup, render_screen.after(crate::setup))
        .add_systems(
            Update,
            pause_match
                .after(crate::reset_match)
                .before(GameplaySystems),
        )
        .add_systems(
            Update,
            (
                detect_game_over,
                handle_frontend,
                render_screen,
                fade_splash,
            )
                .chain()
                .after(crate::history::HistorySystems),
        );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    BevySplash,
    StudioSplash,
    Title,
    Credits,
    Rulebook,
    Game,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Win,
    Lose,
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameMode {
    Computer,
    #[allow(dead_code)]
    TwoPlayer,
}

#[derive(Resource)]
pub(crate) struct Frontend {
    screen: Screen,
    mode: Option<GameMode>,
    outcome: Option<Outcome>,
    match_in_progress: bool,
    pause_opened: bool,
    restart_requested: bool,
    timer: Timer,
    selected: usize,
    page: usize,
    pages: Vec<String>,
    root: Option<Entity>,
    dirty: bool,
}

impl Default for Frontend {
    fn default() -> Self {
        Self {
            screen: Screen::BevySplash,
            mode: None,
            outcome: None,
            match_in_progress: false,
            pause_opened: false,
            restart_requested: false,
            timer: Timer::from_seconds(SPLASH_SECONDS, TimerMode::Once),
            selected: 0,
            page: 0,
            pages: rulebook_pages(include_str!("../RULEBOOK.md")),
            root: None,
            dirty: true,
        }
    }
}

impl Frontend {
    pub(crate) fn computer_game(&self) -> bool {
        self.screen == Screen::Game && self.mode == Some(GameMode::Computer)
    }

    #[cfg(test)]
    pub(crate) fn computer_match() -> Self {
        let mut frontend = Self::default();
        frontend.activate(MenuAction::Computer);
        frontend
    }

    fn show(&mut self, screen: Screen) {
        self.screen = screen;
        if matches!(screen, Screen::Title | Screen::Paused) {
            self.selected = 0;
        }
        self.timer.reset();
        self.dirty = true;
    }

    fn actions(&self) -> &'static [(MenuAction, &'static str)] {
        match self.screen {
            Screen::Title if self.match_in_progress => &ACTIVE_TITLE_ACTIONS,
            Screen::Title => &TITLE_ACTIONS,
            Screen::Paused => &PAUSE_ACTIONS,
            _ => &[],
        }
    }

    fn activate(&mut self, action: MenuAction) {
        match action {
            MenuAction::Computer => {
                self.mode = Some(GameMode::Computer);
                self.outcome = None;
                self.match_in_progress = true;
                self.restart_requested = true;
                self.show(Screen::Game);
            }
            MenuAction::TwoPlayer => {
                self.mode = Some(GameMode::TwoPlayer);
                self.outcome = None;
                self.match_in_progress = true;
                self.restart_requested = true;
                self.show(Screen::Game);
            }
            MenuAction::Resume if self.match_in_progress => self.show(Screen::Game),
            MenuAction::Resume => {}
            MenuAction::Credits => self.show(Screen::Credits),
            MenuAction::Rulebook => {
                self.page = 0;
                self.show(Screen::Rulebook);
            }
            MenuAction::Back => self.show(Screen::Title),
            MenuAction::Previous if self.page > 0 => {
                self.page -= 1;
                self.dirty = true;
            }
            MenuAction::Next if self.page + 1 < self.pages.len() => {
                self.page += 1;
                self.dirty = true;
            }
            MenuAction::Previous | MenuAction::Next => {}
        }
    }

    pub(crate) fn take_restart_request(&mut self) -> bool {
        std::mem::take(&mut self.restart_requested)
    }
}

#[derive(Component)]
struct SplashImage;

#[derive(Component)]
struct MenuButtonArt;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    Computer,
    Resume,
    #[allow(dead_code)]
    TwoPlayer,
    Credits,
    Rulebook,
    Back,
    Previous,
    Next,
}

const TITLE_ACTIONS: [(MenuAction, &str); 3] = [
    (MenuAction::Computer, "PLAY VS COMPUTER"),
    (MenuAction::Credits, "CREDITS"),
    (MenuAction::Rulebook, "RULEBOOK"),
];

const ACTIVE_TITLE_ACTIONS: [(MenuAction, &str); 4] = [
    (MenuAction::Resume, "RESUME"),
    (MenuAction::Computer, "START NEW GAME"),
    (MenuAction::Credits, "CREDITS"),
    (MenuAction::Rulebook, "RULEBOOK"),
];

const PAUSE_ACTIONS: [(MenuAction, &str); 2] = [
    (MenuAction::Resume, "RESUME"),
    (MenuAction::Back, "BACK TO MENU"),
];

pub(crate) fn pause_match(
    keyboard: Res<ButtonInput<KeyCode>>,
    help: Option<Res<crate::help::Help>>,
    mut frontend: ResMut<Frontend>,
) {
    if frontend.computer_game()
        && keyboard.just_pressed(KeyCode::Escape)
        && help.is_none_or(crate::help::closed)
    {
        frontend.show(Screen::Paused);
        frontend.pause_opened = true;
    }
}

pub(crate) fn detect_game_over(
    health: Res<PlayerHealth>,
    turn: Option<Res<crate::TurnState>>,
    mut frontend: ResMut<Frontend>,
) {
    if frontend.screen != Screen::Game
        || turn
            .as_ref()
            .is_some_and(|turn| turn.effects_active || turn.weather_animation.is_some())
    {
        return;
    }
    let outcome = match health.0 {
        [0, 0] => Some(Outcome::Draw),
        [0, _] => Some(Outcome::Lose),
        [_, 0] => Some(Outcome::Win),
        _ => None,
    };
    if let Some(outcome) = outcome {
        frontend.outcome = Some(outcome);
        frontend.match_in_progress = false;
        frontend.show(Screen::GameOver);
    }
}

pub(crate) fn in_game(frontend: Res<Frontend>) -> bool {
    frontend.screen == Screen::Game
}

fn rulebook_pages(rulebook: &str) -> Vec<String> {
    let text = rulebook.replace("**", "");
    let mut pages = Vec::new();
    let mut page = String::new();
    for line in text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with("# "))
    {
        let mut first_sentence = true;
        for sentence in line.split_inclusive(". ") {
            let sentence = sentence.trim();
            if !page.is_empty() && page.len() + 2 + sentence.len() > RULE_PAGE_CHARS {
                pages.push(std::mem::take(&mut page));
            }
            if !page.is_empty() {
                page.push_str(if first_sentence { "\n\n" } else { " " });
            }
            page.push_str(sentence);
            first_sentence = false;
        }
    }
    if !page.is_empty() {
        pages.push(page);
    }
    pages
}

type MenuButton = (
    Ref<'static, Hovered>,
    &'static MenuAction,
    &'static mut BackgroundColor,
    Option<&'static Children>,
);

fn handle_frontend(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut frontend: ResMut<Frontend>,
    mut buttons: Query<MenuButton, With<Button>>,
    mut button_art: Query<&mut ImageNode, With<MenuButtonArt>>,
    pressed_buttons: Query<&MenuAction, Added<Pressed>>,
    mut sounds: Option<ResMut<crate::sound::SoundCues>>,
) {
    if frontend.screen == Screen::Game {
        return;
    }

    if matches!(frontend.screen, Screen::BevySplash | Screen::StudioSplash) {
        if keyboard.just_pressed(KeyCode::Space)
            || keyboard.just_pressed(KeyCode::Enter)
            || keyboard.just_pressed(KeyCode::Escape)
        {
            frontend.show(Screen::Title);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        } else if frontend.timer.tick(time.delta()).is_finished() {
            let next = if frontend.screen == Screen::BevySplash {
                Screen::StudioSplash
            } else {
                Screen::Title
            };
            frontend.show(next);
        }
        return;
    }

    if std::mem::take(&mut frontend.pause_opened) {
        return;
    }

    if matches!(frontend.screen, Screen::Title | Screen::Paused) {
        if frontend.screen == Screen::Paused && keyboard.just_pressed(KeyCode::Escape) {
            frontend.activate(MenuAction::Resume);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
            return;
        }
        let actions = frontend.actions();
        for (hovered, action, _, _) in &mut buttons {
            if hovered.is_changed()
                && hovered.0
                && let Some(index) = actions.iter().position(|(item, _)| item == action)
            {
                frontend.selected = index;
            }
        }
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            frontend.selected = (frontend.selected + 1) % actions.len();
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        } else if keyboard.just_pressed(KeyCode::ArrowUp) {
            frontend.selected = (frontend.selected + actions.len() - 1) % actions.len();
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        }
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            let action = actions[frontend.selected].0;
            frontend.activate(action);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
            return;
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        frontend.activate(MenuAction::Back);
        crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        return;
    } else if frontend.screen == Screen::Rulebook {
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            frontend.activate(MenuAction::Previous);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        } else if keyboard.just_pressed(KeyCode::ArrowRight) {
            frontend.activate(MenuAction::Next);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        }
    }

    for (hovered, action, mut background, children) in &mut buttons {
        let actions = frontend.actions();
        let highlighted = if actions.is_empty() {
            hovered.0
        } else {
            actions[frontend.selected].0 == *action
        };
        let mut styled = false;
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut image) = button_art.get_mut(child) {
                    image.rect = Some(button_rect(highlighted));
                    styled = true;
                }
            }
        }
        background.0 = if styled {
            Color::NONE
        } else if highlighted {
            Color::srgb(0.40, 0.28, 0.14)
        } else {
            Color::srgba(0.08, 0.06, 0.05, 0.94)
        };
    }
    if let Some(action) = pressed_buttons.iter().next() {
        frontend.activate(*action);
        crate::sound::request(&mut sounds, crate::sound::Cue::Select);
    }
}

fn render_screen(
    mut commands: Commands,
    hud: Res<Hud>,
    asset_server: Res<AssetServer>,
    mut frontend: ResMut<Frontend>,
) {
    if !frontend.dirty {
        return;
    }
    frontend.dirty = false;
    if let Some(root) = frontend.root.take() {
        commands.entity(root).despawn();
    }
    if frontend.screen == Screen::Game {
        return;
    }

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(
                if matches!(frontend.screen, Screen::GameOver | Screen::Paused) {
                    Color::srgba(0.0, 0.0, 0.0, 0.65)
                } else {
                    Color::BLACK
                },
            ),
            GlobalZIndex(100),
        ))
        .id();
    commands.entity(hud.0).add_child(root);
    commands
        .entity(root)
        .with_children(|parent| match frontend.screen {
            Screen::BevySplash | Screen::StudioSplash => {
                let path = if frontend.screen == Screen::BevySplash {
                    "menu/bevy-splash.png"
                } else {
                    "menu/studio-splash.png"
                };
                parent.spawn((
                    SplashImage,
                    ImageNode::new(asset_server.load(path)),
                    Node {
                        width: Val::Px(RESOLUTION_HEIGHT * 1.6),
                        height: Val::Px(RESOLUTION_HEIGHT),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            Screen::Title => title_screen(parent, &asset_server, frontend.actions()),
            Screen::Credits => credits_screen(parent, &asset_server),
            Screen::Rulebook => rulebook_screen(parent, &asset_server, &frontend),
            Screen::Paused => pause_screen(parent, &asset_server),
            Screen::GameOver => game_over_screen(
                parent,
                &asset_server,
                frontend.outcome.expect("game has an outcome"),
            ),
            Screen::Game => unreachable!(),
        });
    frontend.root = Some(root);
}

fn fade_splash(frontend: Res<Frontend>, mut images: Query<&mut ImageNode, With<SplashImage>>) {
    if !matches!(frontend.screen, Screen::BevySplash | Screen::StudioSplash) {
        return;
    }
    let elapsed = frontend.timer.elapsed_secs();
    let alpha = (elapsed / 0.4)
        .min((SPLASH_SECONDS - elapsed) / 0.4)
        .clamp(0.0, 1.0);
    for mut image in &mut images {
        image.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}

fn title_screen(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    actions: &[(MenuAction, &str)],
) {
    parent.spawn((
        ImageNode::new(assets.load("volatile.png")),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
    ));
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            padding: UiRect::bottom(Val::Px(if actions.len() > 3 { 24.0 } else { 52.0 })),
            ..default()
        })
        .with_children(|screen| {
            screen
                .spawn(Node {
                    width: Val::Px(276.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|options| {
                    for (index, &(action, label)) in actions.iter().enumerate() {
                        button(options, assets, action, label, 54.0, index == 0);
                    }
                });
        });
}

fn pause_screen(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Node {
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(24.0)),
                border: UiRect::all(Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.96)),
            BorderColor::all(GOLD),
        ))
        .with_children(|content| {
            text(content, "PAUSED", 52.0, GOLD);
            for (index, (action, label)) in PAUSE_ACTIONS.into_iter().enumerate() {
                button(content, assets, action, label, 46.0, index == 0);
            }
        });
}

fn game_over_screen(parent: &mut ChildSpawnerCommands, assets: &AssetServer, outcome: Outcome) {
    parent
        .spawn((
            Node {
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(24.0)),
                border: UiRect::all(Val::Px(3.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.96)),
            BorderColor::all(GOLD),
        ))
        .with_children(|content| {
            text(
                content,
                match outcome {
                    Outcome::Win => "YOU WIN!",
                    Outcome::Lose => "YOU LOSE",
                    Outcome::Draw => "DRAW",
                },
                52.0,
                GOLD,
            );
            button(
                content,
                assets,
                MenuAction::Computer,
                "NEW GAME",
                46.0,
                false,
            );
            button(
                content,
                assets,
                MenuAction::Back,
                "BACK TO MENU",
                46.0,
                false,
            );
        });
}

fn credits_screen(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(24.0),
            ..default()
        })
        .with_children(|content| {
            text(content, "CREDITS", 42.0, GOLD);
            text(content, "Isa-Metric Games", 26.0, TEXT);
            text(content, "Made with Bevy", 21.0, TEXT);
            button(content, assets, MenuAction::Back, "BACK", 42.0, false);
        });
}

fn rulebook_screen(parent: &mut ChildSpawnerCommands, assets: &AssetServer, frontend: &Frontend) {
    parent
        .spawn(Node {
            width: Val::Px(RESOLUTION_WIDTH - 100.0),
            height: Val::Px(RESOLUTION_HEIGHT - 60.0),
            padding: UiRect::all(Val::Px(22.0)),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            ..default()
        })
        .insert((
            BackgroundColor(Color::srgb(0.08, 0.06, 0.05)),
            BorderColor::all(GOLD),
        ))
        .with_children(|content| {
            text(content, "RULEBOOK", 30.0, GOLD);
            text(
                content,
                &format!("Page {} / {}", frontend.page + 1, frontend.pages.len()),
                14.0,
                GOLD,
            );
            content
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|body| {
                    text(body, &frontend.pages[frontend.page], 16.0, TEXT);
                });
            content
                .spawn(Node {
                    height: Val::Px(42.0),
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|row| {
                    button(row, assets, MenuAction::Back, "BACK", 42.0, false);
                    if frontend.page > 0 {
                        button(row, assets, MenuAction::Previous, "PREVIOUS", 42.0, false);
                    }
                    if frontend.page + 1 < frontend.pages.len() {
                        button(row, assets, MenuAction::Next, "NEXT", 42.0, false);
                    }
                });
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
        Pickable::IGNORE,
    ));
}

fn button(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    action: MenuAction,
    label: &str,
    height: f32,
    selected: bool,
) {
    parent
        .spawn((
            Button,
            Hovered::default(),
            action,
            Node {
                min_width: Val::Px(210.0),
                height: Val::Px(height),
                padding: UiRect::horizontal(Val::Px(14.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|button| {
            button.spawn((
                MenuButtonArt,
                ImageNode {
                    image: assets.load("button.png"),
                    rect: Some(button_rect(selected)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(-13.0),
                    top: Val::Px(-height * 85.0 / 115.0),
                    width: Val::Percent(129.0),
                    height: Val::Px(height * 302.0 / 115.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            text(button, label, 19.0, BUTTON_TEXT);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetApp, AssetPlugin};
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct GameTicks(u32);

    fn tick_game(mut ticks: ResMut<GameTicks>) {
        ticks.0 += 1;
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    #[test]
    fn title_uses_cover_and_compact_buttons_with_selection_art() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Update, (handle_frontend, render_screen).chain());
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Title);
        app.update();

        let world = app.world_mut();
        let assets = world.resource::<AssetServer>();
        let cover: Handle<Image> = assets.load("volatile.png");
        let button_sheet: Handle<Image> = assets.load("button.png");
        let mut images = world.query::<(&ImageNode, &Node)>();
        let cover_node = images
            .iter(world)
            .find(|(image, _)| image.image == cover)
            .unwrap()
            .1;
        assert_eq!(cover_node.width, Val::Percent(100.0));
        assert_eq!(cover_node.height, Val::Percent(100.0));
        let mut buttons = world.query_filtered::<(&MenuAction, &Node, &Children), With<Button>>();
        let buttons: Vec<_> = buttons.iter(world).collect();
        assert_eq!(buttons.len(), 3);
        for (action, node, children) in buttons {
            assert_eq!(node.height, Val::Px(54.0));
            let image = children
                .iter()
                .find_map(|child| world.get::<ImageNode>(child))
                .unwrap();
            assert_eq!(image.image, button_sheet);
            assert_eq!(
                image.rect,
                Some(button_rect(*action == MenuAction::Computer))
            );
        }

        tap(&mut app, KeyCode::ArrowDown);
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(&MenuAction, &Children), With<Button>>();
        for (action, children) in buttons.iter(world) {
            let image = children
                .iter()
                .find_map(|child| world.get::<ImageNode>(child))
                .unwrap();
            assert_eq!(
                image.rect,
                Some(button_rect(*action == MenuAction::Credits))
            );
        }
        let rulebook = world
            .query_filtered::<(Entity, &MenuAction), With<Button>>()
            .iter(world)
            .find_map(|(entity, action)| (*action == MenuAction::Rulebook).then_some(entity))
            .unwrap();
        world.entity_mut(rulebook).insert(Hovered(true));
        app.update();
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(&MenuAction, &Children), With<Button>>();
        for (action, children) in buttons.iter(world) {
            let image = children
                .iter()
                .find_map(|child| world.get::<ImageNode>(child))
                .unwrap();
            assert_eq!(
                image.rect,
                Some(button_rect(*action == MenuAction::Rulebook))
            );
        }

        {
            let mut frontend = app.world_mut().resource_mut::<Frontend>();
            frontend.match_in_progress = true;
            frontend.show(Screen::Title);
        }
        app.update();
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(&MenuAction, &Node, &Children), With<Button>>();
        let buttons: Vec<_> = buttons.iter(world).collect();
        assert_eq!(buttons.len(), 4);
        for (action, node, children) in buttons {
            assert_eq!(node.height, Val::Px(54.0));
            let image = children
                .iter()
                .find_map(|child| world.get::<ImageNode>(child))
                .unwrap();
            assert_eq!(image.rect, Some(button_rect(*action == MenuAction::Resume)));
        }
    }

    #[test]
    fn pause_and_menu_preserve_a_match_until_resume_or_new_game() {
        let mut frontend = Frontend::computer_match();
        assert!(frontend.take_restart_request());
        let mut app = App::new();
        app.insert_resource(frontend)
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(PlayerHealth([7, 4]))
            .init_resource::<GameTicks>()
            .add_systems(
                Update,
                (
                    pause_match,
                    tick_game.run_if(in_game),
                    detect_game_over,
                    handle_frontend,
                )
                    .chain(),
            );
        app.update();
        assert_eq!(app.world().resource::<GameTicks>().0, 1);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Paused);
        assert_eq!(app.world().resource::<GameTicks>().0, 1);
        app.update();
        assert_eq!(app.world().resource::<GameTicks>().0, 1);

        tap(&mut app, KeyCode::ArrowDown);
        tap(&mut app, KeyCode::Enter);
        let frontend = app.world().resource::<Frontend>();
        assert_eq!(frontend.screen, Screen::Title);
        assert_eq!(frontend.actions(), ACTIVE_TITLE_ACTIONS);
        assert_eq!(app.world().resource::<GameTicks>().0, 1);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 4]);

        tap(&mut app, KeyCode::Enter);
        let frontend = app.world_mut().resource_mut::<Frontend>();
        assert_eq!(frontend.screen, Screen::Game);
        assert!(!frontend.restart_requested);
        assert_eq!(app.world().resource::<PlayerHealth>().0, [7, 4]);
        app.update();
        assert_eq!(app.world().resource::<GameTicks>().0, 2);

        tap(&mut app, KeyCode::Escape);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Game);
        assert_eq!(app.world().resource::<GameTicks>().0, 2);
        tap(&mut app, KeyCode::Escape);
        tap(&mut app, KeyCode::ArrowDown);
        tap(&mut app, KeyCode::Enter);
        tap(&mut app, KeyCode::ArrowDown);
        tap(&mut app, KeyCode::Enter);
        let mut frontend = app.world_mut().resource_mut::<Frontend>();
        assert_eq!(frontend.screen, Screen::Game);
        assert!(frontend.take_restart_request());
    }

    #[test]
    fn finished_match_restores_the_normal_title_menu() {
        let mut app = App::new();
        app.insert_resource(Frontend::computer_match())
            .insert_resource(PlayerHealth([0, 5]))
            .add_systems(Update, detect_game_over);
        app.update();
        let mut frontend = app.world_mut().resource_mut::<Frontend>();
        frontend.activate(MenuAction::Back);
        assert_eq!(frontend.screen, Screen::Title);
        assert_eq!(frontend.actions(), TITLE_ACTIONS);
        frontend.activate(MenuAction::Resume);
        assert_eq!(frontend.screen, Screen::Title);
    }

    #[test]
    fn active_match_remains_resumable_after_visiting_rulebook_and_credits() {
        let mut frontend = Frontend::computer_match();
        frontend.take_restart_request();
        frontend.show(Screen::Paused);
        frontend.activate(MenuAction::Back);
        for action in [MenuAction::Rulebook, MenuAction::Credits] {
            frontend.activate(action);
            frontend.activate(MenuAction::Back);
            assert_eq!(frontend.actions(), ACTIVE_TITLE_ACTIONS);
        }
        frontend.activate(MenuAction::Resume);
        assert_eq!(frontend.screen, Screen::Game);
        assert!(!frontend.take_restart_request());
    }

    #[test]
    fn computer_game_requests_a_fresh_match() {
        let mut frontend = Frontend::default();
        frontend.activate(MenuAction::Computer);
        assert_eq!(frontend.screen, Screen::Game);
        assert!(frontend.take_restart_request());
        assert!(!frontend.take_restart_request());
    }

    #[test]
    fn zero_health_ends_the_match_and_preserves_the_first_result() {
        for (health, expected) in [
            ([0, 5], Outcome::Lose),
            ([5, 0], Outcome::Win),
            ([0, 0], Outcome::Draw),
        ] {
            let mut app = App::new();
            app.insert_resource(Frontend::computer_match())
                .insert_resource(PlayerHealth(health))
                .add_systems(Update, detect_game_over);
            assert!(
                app.world_mut()
                    .resource_mut::<Frontend>()
                    .take_restart_request()
            );
            app.update();
            assert_eq!(app.world().resource::<Frontend>().screen, Screen::GameOver);
            assert_eq!(app.world().resource::<Frontend>().outcome, Some(expected));
            app.world_mut().resource_mut::<PlayerHealth>().0 = [0, 0];
            app.update();
            assert_eq!(app.world().resource::<Frontend>().outcome, Some(expected));
            let mut frontend = app.world_mut().resource_mut::<Frontend>();
            frontend.activate(MenuAction::Back);
            assert_eq!(frontend.screen, Screen::Title);
            frontend.activate(MenuAction::Computer);
            assert_eq!(frontend.screen, Screen::Game);
            assert!(frontend.take_restart_request());
            assert_eq!(frontend.outcome, None);
        }
    }

    #[test]
    fn game_over_waits_until_reaction_animations_finish() {
        let mut app = App::new();
        let turn = crate::TurnState {
            effects_active: true,
            ..default()
        };
        app.insert_resource(Frontend::computer_match())
            .insert_resource(PlayerHealth([0, 5]))
            .insert_resource(turn)
            .add_systems(Update, detect_game_over);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Game);
        app.world_mut()
            .resource_mut::<crate::TurnState>()
            .effects_active = false;
        app.update();
        assert_eq!(
            app.world().resource::<Frontend>().outcome,
            Some(Outcome::Lose)
        );
    }

    #[test]
    fn multiplayer_remains_available_internally_but_not_in_the_title_menu() {
        assert!(
            TITLE_ACTIONS
                .iter()
                .all(|(action, _)| *action != MenuAction::TwoPlayer)
        );
        let mut frontend = Frontend::default();
        frontend.activate(MenuAction::TwoPlayer);
        assert_eq!(frontend.screen, Screen::Game);
        assert!(!frontend.computer_game());
    }

    #[test]
    fn rulebook_pages_preserve_all_words_and_navigation_is_bounded() {
        let source = include_str!("../RULEBOOK.md").replace("**", "");
        let source = source
            .lines()
            .filter(|line| !line.starts_with("# "))
            .collect::<Vec<_>>()
            .join(" ");
        let pages = rulebook_pages(include_str!("../RULEBOOK.md"));
        assert!(pages.len() > 1);
        assert!(pages.iter().all(|page| page.len() <= RULE_PAGE_CHARS));
        assert_eq!(
            pages
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            source.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        let mut frontend = Frontend::default();
        frontend.activate(MenuAction::Rulebook);
        frontend.activate(MenuAction::Previous);
        assert_eq!(frontend.page, 0);
        for _ in 0..frontend.pages.len() + 1 {
            frontend.activate(MenuAction::Next);
        }
        assert_eq!(frontend.page, frontend.pages.len() - 1);
        frontend.activate(MenuAction::Back);
        assert_eq!(frontend.screen, Screen::Title);
    }

    #[test]
    fn splash_sequence_and_keyboard_selection_work() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .add_systems(Update, handle_frontend);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(SPLASH_SECONDS));
        app.update();
        assert_eq!(
            app.world().resource::<Frontend>().screen,
            Screen::StudioSplash
        );
        app.update();
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Title);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        let frontend = app.world().resource::<Frontend>();
        assert_eq!(frontend.screen, Screen::Credits);
    }

    #[test]
    fn hover_selection_persists_after_exit_and_keyboard_continues_from_it() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .add_systems(Update, handle_frontend);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Title);
        let credits = app
            .world_mut()
            .spawn((
                Button,
                Hovered(false),
                MenuAction::Credits,
                BackgroundColor(Color::BLACK),
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(credits).insert(Hovered(true));
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
        app.world_mut().entity_mut(credits).insert(Hovered(false));
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
        assert_eq!(
            app.world().get::<BackgroundColor>(credits).unwrap().0,
            Color::srgb(0.40, 0.28, 0.14)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Rulebook);
    }

    #[test]
    fn keyboard_selection_is_not_overridden_by_stationary_hover() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .add_systems(Update, handle_frontend);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Title);
        let credits = app
            .world_mut()
            .spawn((
                Button,
                Hovered(true),
                MenuAction::Credits,
                BackgroundColor(Color::BLACK),
            ))
            .id();
        let computer = app
            .world_mut()
            .spawn((
                Button,
                Hovered(false),
                MenuAction::Computer,
                BackgroundColor(Color::BLACK),
            ))
            .id();
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowUp);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 0);
        assert_eq!(
            app.world().get::<BackgroundColor>(computer).unwrap().0,
            Color::srgb(0.40, 0.28, 0.14)
        );
        assert_ne!(
            app.world().get::<BackgroundColor>(credits).unwrap().0,
            Color::srgb(0.40, 0.28, 0.14)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut().entity_mut(credits).insert(Hovered(false));
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 0);
        app.world_mut().entity_mut(credits).insert(Hovered(true));
        app.update();
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
    }

    #[test]
    fn pointer_press_opens_credits_and_back_returns_to_title() {
        use bevy::{
            camera::NormalizedRenderTarget,
            picking::{
                backend::HitData,
                events::{Pointer, PointerPress},
                pointer::{Location, PointerButton, PointerId},
            },
            ui_widgets::ButtonPlugin,
        };

        let mut app = App::new();
        app.add_plugins(ButtonPlugin)
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .add_systems(Update, handle_frontend);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Title);
        let button = app
            .world_mut()
            .spawn((
                Button,
                Hovered(true),
                MenuAction::Credits,
                BackgroundColor(Color::BLACK),
            ))
            .id();
        app.world_mut().trigger(PointerPress {
            entity: button,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::None {
                        width: 960,
                        height: 540,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(button, 0.0, None, None),
            count: 1,
        });
        app.update();
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Credits);
        app.world_mut().entity_mut(button).despawn();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Title);
    }

    #[test]
    fn game_over_buttons_respond_to_pointer_presses() {
        use bevy::{
            camera::NormalizedRenderTarget,
            picking::{
                backend::HitData,
                events::{Pointer, PointerPress},
                pointer::{Location, PointerButton, PointerId},
            },
            ui_widgets::ButtonPlugin,
        };

        for (action, expected) in [
            (MenuAction::Computer, Screen::Game),
            (MenuAction::Back, Screen::Title),
        ] {
            let mut app = App::new();
            app.add_plugins(ButtonPlugin)
                .init_resource::<Time>()
                .init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<Frontend>()
                .add_systems(Update, handle_frontend);
            app.world_mut()
                .resource_mut::<Frontend>()
                .show(Screen::GameOver);
            let button = app
                .world_mut()
                .spawn((Button, Hovered(true), action, BackgroundColor(Color::BLACK)))
                .id();
            app.world_mut().trigger(PointerPress {
                entity: button,
                pointer: Pointer::new(
                    PointerId::Mouse,
                    Location {
                        target: NormalizedRenderTarget::None {
                            width: 960,
                            height: 540,
                        },
                        position: Vec2::ZERO,
                    },
                ),
                button: PointerButton::Primary,
                hit: HitData::new(button, 0.0, None, None),
                count: 1,
            });
            app.update();
            let mut frontend = app.world_mut().resource_mut::<Frontend>();
            assert_eq!(frontend.screen, expected);
            assert_eq!(
                frontend.take_restart_request(),
                action == MenuAction::Computer
            );
        }
    }
}
