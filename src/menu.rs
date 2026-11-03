use bevy::{picking::hover::Hovered, prelude::*, ui::Pressed, ui_widgets::Button};
use bevy_aspect_ratio_mask::Hud;

use crate::{PlayerHealth, RESOLUTION_HEIGHT, RESOLUTION_WIDTH};

const SPLASH_SECONDS: f32 = 1.5;
const RULE_PAGE_CHARS: usize = 750;
const GOLD: Color = Color::srgb(0.86, 0.68, 0.32);
const TEXT: Color = Color::srgb(1.0, 0.94, 0.78);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GameplaySystems;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Frontend>()
        .add_systems(Startup, render_screen.after(crate::setup))
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
        self.timer.reset();
        self.dirty = true;
    }

    fn activate(&mut self, action: MenuAction) {
        match action {
            MenuAction::Computer => {
                self.mode = Some(GameMode::Computer);
                self.outcome = None;
                self.restart_requested = true;
                self.show(Screen::Game);
            }
            MenuAction::TwoPlayer => {
                self.mode = Some(GameMode::TwoPlayer);
                self.outcome = None;
                self.restart_requested = true;
                self.show(Screen::Game);
            }
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

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    Computer,
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

pub(crate) fn detect_game_over(health: Res<PlayerHealth>, mut frontend: ResMut<Frontend>) {
    if frontend.screen != Screen::Game {
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

fn handle_frontend(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut frontend: ResMut<Frontend>,
    mut buttons: Query<(Ref<Hovered>, &MenuAction, &mut BackgroundColor), With<Button>>,
    pressed_buttons: Query<&MenuAction, Added<Pressed>>,
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

    if frontend.screen == Screen::Title {
        for (hovered, action, _) in &mut buttons {
            if hovered.is_changed()
                && hovered.0
                && let Some(index) = TITLE_ACTIONS.iter().position(|(item, _)| item == action)
            {
                frontend.selected = index;
            }
        }
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            frontend.selected = (frontend.selected + 1) % TITLE_ACTIONS.len();
        } else if keyboard.just_pressed(KeyCode::ArrowUp) {
            frontend.selected = (frontend.selected + TITLE_ACTIONS.len() - 1) % TITLE_ACTIONS.len();
        }
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            let action = TITLE_ACTIONS[frontend.selected].0;
            frontend.activate(action);
            return;
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        frontend.activate(MenuAction::Back);
        return;
    } else if frontend.screen == Screen::Rulebook {
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            frontend.activate(MenuAction::Previous);
        } else if keyboard.just_pressed(KeyCode::ArrowRight) {
            frontend.activate(MenuAction::Next);
        }
    }

    for (hovered, action, mut background) in &mut buttons {
        let selected =
            frontend.screen == Screen::Title && TITLE_ACTIONS[frontend.selected].0 == *action;
        let highlighted = if frontend.screen == Screen::Title {
            selected
        } else {
            hovered.0
        };
        background.0 = if highlighted {
            Color::srgb(0.40, 0.28, 0.14)
        } else {
            Color::srgba(0.08, 0.06, 0.05, 0.94)
        };
    }
    if let Some(action) = pressed_buttons.iter().next() {
        frontend.activate(*action);
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
            BackgroundColor(if frontend.screen == Screen::GameOver {
                Color::srgba(0.0, 0.0, 0.0, 0.65)
            } else {
                Color::BLACK
            }),
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
            Screen::Title => title_screen(parent, &asset_server),
            Screen::Credits => credits_screen(parent),
            Screen::Rulebook => rulebook_screen(parent, &frontend),
            Screen::GameOver => {
                game_over_screen(parent, frontend.outcome.expect("game has an outcome"))
            }
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

fn title_screen(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    let mut background = ImageNode::new(assets.load("backgroundboard.png"));
    background.color = Color::srgb(0.19, 0.16, 0.12);
    parent.spawn((
        background,
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
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                width: Val::Px(430.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|left| {
                text(left, "ISA-METRIC GAMES", 19.0, GOLD);
                text(left, "BEVY JAM 8", 52.0, TEXT);
                text(left, "A game of elements and alchemy", 17.0, TEXT);
            });
            row.spawn(Node {
                width: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|options| {
                for (action, label) in TITLE_ACTIONS {
                    button(options, action, label, 46.0);
                }
            });
        });
}

fn game_over_screen(parent: &mut ChildSpawnerCommands, outcome: Outcome) {
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
            button(content, MenuAction::Computer, "NEW GAME", 46.0);
            button(content, MenuAction::Back, "BACK TO MENU", 46.0);
        });
}

fn credits_screen(parent: &mut ChildSpawnerCommands) {
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
            button(content, MenuAction::Back, "BACK", 42.0);
        });
}

fn rulebook_screen(parent: &mut ChildSpawnerCommands, frontend: &Frontend) {
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
                    button(row, MenuAction::Back, "BACK", 42.0);
                    if frontend.page > 0 {
                        button(row, MenuAction::Previous, "PREVIOUS", 42.0);
                    }
                    if frontend.page + 1 < frontend.pages.len() {
                        button(row, MenuAction::Next, "NEXT", 42.0);
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

fn button(parent: &mut ChildSpawnerCommands, action: MenuAction, label: &str, height: f32) {
    parent
        .spawn((
            Button,
            Hovered::default(),
            action,
            Node {
                min_width: Val::Px(150.0),
                height: Val::Px(height),
                padding: UiRect::horizontal(Val::Px(14.0)),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.94)),
            BorderColor::all(GOLD),
        ))
        .with_children(|button| text(button, label, 17.0, TEXT));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
