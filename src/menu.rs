use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{Button, ControlOrientation, Scrollbar, ScrollbarThumb},
};
use bevy_aspect_ratio_mask::Hud;

use crate::{
    GRID_BLOCK_SIZE, INVENTORY_ITEMS, INVENTORY_ORDER, InventoryImages, PlayerHealth,
    RESOLUTION_HEIGHT, RESOLUTION_WIDTH,
    audio_settings::{AudioChannel, AudioSettings},
    help, history,
    online::{Connection, Online},
};
use volatile_online_protocol::{ClientMessage, Seat};

const SPLASH_SECONDS: f32 = 1.5;
const RULEBOOK_BASICS: &str = concat!(
    "GOAL: Reduce the computer's health to zero. If both players reach zero together, it is a draw.\n\n",
    "TURNS: Take two actions: move, place an ingredient, or pick up. UNDO reverses your first action. REST ends the turn and restores energy; press R or click REST.\n\n",
    "MOVE: Click MOVE, then a highlighted destination. Walk one tile for free or jump two tiles for one energy; both use one action. Use PICK UP to collect eligible ingredients on your tile.\n\n",
    "INGREDIENTS: Select an item, then a target tile. Your tile and adjacent tiles cost no energy; farther placements cost distance minus one. Distance counts diagonals. A first placement waits until the turn ends, so you can combine two items. Invalid placements do not cost an action, energy, or items.\n\n",
    "ENERGY: Free moves, nearby placements, and pickups restore one energy at the end of a two-action turn. Resting before acting restores all energy; after one action it restores two. Energy is capped at five.\n\n",
    "HOVER: Toggle HOVER without using an action. Hovering uses one magic per turn if you hover or move while hovering; otherwise magic recovers. Hovering avoids some ground effects but not all damage. Ridges and holes require enough magic and safe exits.\n\n",
    "ITEMS AND TERRAIN: Water, Void Dust, and Stone Dust are unlimited. Other items can be collected and return after use. Rain and earthquakes change the board. Select an ingredient below to see its effects and reactions; during a match, select an item and click ? for the same guide."
);
const GOLD: Color = Color::srgb(0.86, 0.68, 0.32);
const TEXT: Color = Color::srgb(1.0, 0.94, 0.78);
const BUTTON_TEXT: Color = Color::srgb(0.08, 0.08, 0.08);
const LOBBY_BACKGROUND: Color = Color::srgb(0.025, 0.045, 0.11);

fn button_rect(selected: bool) -> Rect {
    let top = if selected { 302.0 } else { 0.0 };
    Rect::new(0.0, top, 755.0, top + 302.0)
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GameplaySystems;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FrontendSystems;

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
                .after(crate::audio_settings::AudioHudInput)
                .in_set(FrontendSystems)
                .after(crate::history::HistorySystems),
        )
        .add_systems(
            Update,
            (
                help::render,
                history::scroll::<help::HelpViewport>,
                history::update_scrollbar::<help::HelpViewport, help::HelpScrollbar>,
                history::scroll::<RulebookViewport>,
                history::update_scrollbar::<RulebookViewport, RulebookScrollbar>,
            )
                .chain()
                .after(FrontendSystems)
                .run_if(in_rulebook),
        )
        .add_systems(
            Update,
            (
                history::scroll::<LobbyViewport>,
                history::update_scrollbar::<LobbyViewport, LobbyScrollbar>,
            )
                .chain()
                .after(FrontendSystems)
                .run_if(in_lobby),
        );
}

fn in_lobby(frontend: Res<Frontend>) -> bool {
    frontend.screen == Screen::Lobby
}

fn in_rulebook(frontend: Res<Frontend>) -> bool {
    frontend.screen == Screen::Rulebook
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    BevySplash,
    StudioSplash,
    Title,
    Options,
    Credits,
    Rulebook,
    Lobby,
    Room,
    Pin,
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
    Online,
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
    rulebook_selected: usize,
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
            rulebook_selected: 0,
            root: None,
            dirty: true,
        }
    }
}

impl Frontend {
    pub(crate) fn audio_controls_visible(&self) -> bool {
        !matches!(self.screen, Screen::BevySplash | Screen::StudioSplash)
    }

    pub(crate) fn computer_game(&self) -> bool {
        self.screen == Screen::Game && self.mode == Some(GameMode::Computer)
    }

    pub(crate) fn online_game(&self) -> bool {
        self.mode == Some(GameMode::Online) && self.screen == Screen::Game
    }

    pub(crate) fn online_history_visible(&self) -> bool {
        self.mode == Some(GameMode::Online)
            && matches!(self.screen, Screen::Game | Screen::GameOver)
    }

    pub(crate) fn online_game_over(&self) -> bool {
        self.mode == Some(GameMode::Online) && self.screen == Screen::GameOver
    }

    pub(crate) fn has_online_match(&self) -> bool {
        self.mode == Some(GameMode::Online)
    }

    pub(crate) fn enter_online_match(&mut self) {
        self.mode = Some(GameMode::Online);
        self.outcome = None;
        self.match_in_progress = true;
        self.restart_requested = false;
        self.show(Screen::Game);
    }

    pub(crate) fn end_online_match(&mut self, in_room: bool) {
        self.mode = None;
        self.match_in_progress = false;
        self.show(if in_room { Screen::Room } else { Screen::Lobby });
    }

    #[cfg(test)]
    pub(crate) fn computer_match() -> Self {
        let mut frontend = Self::default();
        frontend.activate(MenuAction::Computer);
        frontend
    }

    fn show(&mut self, screen: Screen) {
        self.screen = screen;
        if matches!(screen, Screen::Title | Screen::Paused | Screen::Options) {
            self.selected = 0;
        }
        self.timer.reset();
        self.dirty = true;
    }

    fn actions(&self) -> &'static [(MenuAction, &'static str)] {
        match self.screen {
            Screen::Title if self.match_in_progress => &ACTIVE_TITLE_ACTIONS,
            Screen::Title => &TITLE_ACTIONS,
            Screen::Paused if self.has_online_match() => &ONLINE_PAUSE_ACTIONS,
            Screen::Paused => &PAUSE_ACTIONS,
            Screen::Options => &OPTIONS_ACTIONS,
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
            MenuAction::Options => self.show(Screen::Options),
            MenuAction::Rulebook => {
                self.rulebook_selected = 0;
                self.show(Screen::Rulebook);
            }
            MenuAction::Online => self.show(Screen::Lobby),
            MenuAction::Back => self.show(Screen::Title),
            MenuAction::RulebookItem(_)
            | MenuAction::Music
            | MenuAction::Sfx
            | MenuAction::CreateRoom
            | MenuAction::JoinRoom(_)
            | MenuAction::PinDigit(_)
            | MenuAction::PinClear
            | MenuAction::PinSubmit
            | MenuAction::LeaveRoom
            | MenuAction::StartOnlineGame
            | MenuAction::Connect => {}
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

#[derive(Component)]
struct RulebookViewport;

#[derive(Component)]
struct RulebookScrollbar;

#[derive(Component)]
struct LobbyViewport;

#[derive(Component)]
struct LobbyScrollbar;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    Computer,
    Resume,
    #[allow(dead_code)]
    TwoPlayer,
    Credits,
    Options,
    Music,
    Sfx,
    Rulebook,
    Online,
    CreateRoom,
    JoinRoom(u128),
    PinDigit(u8),
    PinClear,
    PinSubmit,
    LeaveRoom,
    StartOnlineGame,
    Connect,
    Back,
    RulebookItem(usize),
}

const TITLE_ACTIONS: [(MenuAction, &str); 5] = [
    (MenuAction::Computer, "PLAY VS COMPUTER"),
    (MenuAction::Credits, "CREDITS"),
    (MenuAction::Rulebook, "RULEBOOK"),
    (MenuAction::Options, "OPTIONS"),
    (MenuAction::Online, "PLAY ONLINE"),
];

const ACTIVE_TITLE_ACTIONS: [(MenuAction, &str); 6] = [
    (MenuAction::Resume, "RESUME"),
    (MenuAction::Computer, "START NEW GAME"),
    (MenuAction::Credits, "CREDITS"),
    (MenuAction::Rulebook, "RULEBOOK"),
    (MenuAction::Options, "OPTIONS"),
    (MenuAction::Online, "PLAY ONLINE"),
];

const OPTIONS_ACTIONS: [(MenuAction, &str); 3] = [
    (MenuAction::Music, "MUSIC"),
    (MenuAction::Sfx, "SFX"),
    (MenuAction::Back, "BACK"),
];

const PAUSE_ACTIONS: [(MenuAction, &str); 2] = [
    (MenuAction::Resume, "RESUME"),
    (MenuAction::Back, "BACK TO MENU"),
];
const ONLINE_PAUSE_ACTIONS: [(MenuAction, &str); 2] = [
    (MenuAction::Resume, "RESUME"),
    (MenuAction::LeaveRoom, "LEAVE ROOM"),
];

pub(crate) fn pause_match(
    keyboard: Res<ButtonInput<KeyCode>>,
    help: Option<Res<crate::help::Help>>,
    mut frontend: ResMut<Frontend>,
) {
    if (frontend.computer_game() || frontend.online_game())
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
    online: Option<Res<Online>>,
    mut frontend: ResMut<Frontend>,
) {
    if frontend.screen != Screen::Game
        || turn
            .as_ref()
            .is_some_and(|turn| turn.effects_active || turn.weather_animation.is_some())
    {
        return;
    }
    let health = if frontend.has_online_match()
        && online.as_ref().and_then(|online| online.seat) == Some(Seat::Guest)
    {
        [health.0[1], health.0[0]]
    } else {
        health.0
    };
    let outcome = match health {
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

pub(crate) fn in_game_or_online_game_over(frontend: Res<Frontend>) -> bool {
    frontend.screen == Screen::Game || frontend.online_game_over()
}

pub(crate) fn in_online_game(frontend: Res<Frontend>) -> bool {
    frontend.online_game()
}

pub(crate) fn in_local_game(frontend: Res<Frontend>) -> bool {
    frontend.screen == Screen::Game && !frontend.has_online_match()
}

type MenuButton = (
    Ref<'static, Hovered>,
    &'static MenuAction,
    &'static mut BackgroundColor,
    Option<&'static Children>,
);

fn handle_frontend(
    input: (Res<Time>, Res<ButtonInput<KeyCode>>),
    state: (ResMut<Frontend>, ResMut<AudioSettings>, ResMut<help::Help>),
    mut buttons: Query<MenuButton, With<Button>>,
    mut button_art: Query<&mut ImageNode, With<MenuButtonArt>>,
    pressed_buttons: Query<&MenuAction, Added<Pressed>>,
    help_close: Query<(), (With<help::HelpClose>, Added<Pressed>)>,
    services: (
        Option<ResMut<crate::sound::SoundCues>>,
        Option<ResMut<Online>>,
    ),
) {
    let (time, keyboard) = input;
    let (mut frontend, mut audio, mut help) = state;
    let (mut sounds, mut online) = services;
    if frontend.screen == Screen::Game {
        return;
    }

    if frontend.screen == Screen::Rulebook && help.is_open() {
        if keyboard.just_pressed(KeyCode::Escape) || !help_close.is_empty() {
            help.close();
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        }
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

    if matches!(
        frontend.screen,
        Screen::Title | Screen::Paused | Screen::Options
    ) {
        if matches!(frontend.screen, Screen::Paused | Screen::Options)
            && keyboard.just_pressed(KeyCode::Escape)
        {
            let action = if frontend.screen == Screen::Paused {
                MenuAction::Resume
            } else {
                MenuAction::Back
            };
            frontend.activate(action);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
            return;
        }
        let actions = frontend.actions();
        let previous_selection = frontend.selected;
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
        } else if keyboard.just_pressed(KeyCode::ArrowUp) {
            frontend.selected = (frontend.selected + actions.len() - 1) % actions.len();
        }
        if frontend.selected != previous_selection {
            crate::sound::request(&mut sounds, crate::sound::Cue::MenuOption);
        }
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            let action = actions[frontend.selected].0;
            activate_action(
                &mut frontend,
                &mut audio,
                &mut help,
                online.as_deref_mut(),
                action,
            );
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
            return;
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        let action = if frontend.screen == Screen::Room {
            MenuAction::LeaveRoom
        } else if frontend.screen == Screen::Pin {
            frontend.show(Screen::Lobby);
            return;
        } else {
            MenuAction::Back
        };
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            online.as_deref_mut(),
            action,
        );
        crate::sound::request(&mut sounds, crate::sound::Cue::Select);
        return;
    } else if frontend.screen == Screen::Rulebook {
        let previous_selection = frontend.rulebook_selected;
        for (hovered, action, _, _) in &mut buttons {
            if hovered.is_changed()
                && hovered.0
                && let MenuAction::RulebookItem(item) = action
                && let Some(index) = INVENTORY_ORDER
                    .iter()
                    .position(|candidate| candidate == item)
            {
                frontend.rulebook_selected = index;
            }
        }
        let count = INVENTORY_ORDER.len();
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            frontend.rulebook_selected = (frontend.rulebook_selected + count - 1) % count;
        } else if keyboard.just_pressed(KeyCode::ArrowRight) {
            frontend.rulebook_selected = (frontend.rulebook_selected + 1) % count;
        } else if keyboard.just_pressed(KeyCode::ArrowUp)
            || keyboard.just_pressed(KeyCode::ArrowDown)
        {
            frontend.rulebook_selected = (frontend.rulebook_selected + count / 2) % count;
        }
        if frontend.rulebook_selected != previous_selection {
            crate::sound::request(&mut sounds, crate::sound::Cue::MenuOption);
        }
        if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
            help.open(INVENTORY_ORDER[frontend.rulebook_selected]);
            crate::sound::request(&mut sounds, crate::sound::Cue::Select);
            return;
        }
    }

    for (hovered, action, mut background, children) in &mut buttons {
        let actions = frontend.actions();
        let highlighted = if frontend.screen == Screen::Rulebook
            && let MenuAction::RulebookItem(item) = action
        {
            INVENTORY_ORDER[frontend.rulebook_selected] == *item
        } else if actions.is_empty() {
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
        if frontend.screen == Screen::Pin
            && !matches!(
                action,
                MenuAction::PinDigit(_)
                    | MenuAction::PinClear
                    | MenuAction::PinSubmit
                    | MenuAction::Back
            )
        {
            return;
        }
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            online.as_deref_mut(),
            *action,
        );
        crate::sound::request(&mut sounds, crate::sound::Cue::Select);
    }
}

fn activate_action(
    frontend: &mut Frontend,
    audio: &mut AudioSettings,
    help: &mut help::Help,
    online: Option<&mut Online>,
    action: MenuAction,
) {
    if let Some(online) = online {
        match action {
            MenuAction::Online | MenuAction::Connect => {
                if online.connection == Connection::Disconnected {
                    if cfg!(target_family = "wasm") {
                        online.connect_requested = true;
                    } else {
                        online.status = "Online lobby is available in the browser build".into();
                    }
                }
            }
            MenuAction::CreateRoom
                if online.connection == Connection::Connected && !online.pending =>
            {
                online.pin.clear();
                online.pin_room = None;
                online.status.clear();
                frontend.show(Screen::Pin);
            }
            MenuAction::JoinRoom(id)
                if online.connection == Connection::Connected && !online.pending =>
            {
                let room_id = format!("{id:032x}");
                if let Some(room) = online.rooms.iter().find(|room| room.id == room_id) {
                    if room.locked {
                        online.pin.clear();
                        online.pin_room = Some(room_id);
                        online.status = "Enter the room PIN".into();
                        frontend.show(Screen::Pin);
                    } else {
                        online.send(ClientMessage::JoinRoom { room_id, pin: None });
                    }
                }
            }
            MenuAction::PinDigit(digit) if online.pin.len() < 3 => {
                online.pin.push(char::from(b'0' + digit));
            }
            MenuAction::PinClear => {
                online.pin.clear();
            }
            MenuAction::PinSubmit
                if online.pin.len() == 3
                    || (online.pin.is_empty() && online.pin_room.is_none()) =>
            {
                let pin = (!online.pin.is_empty()).then(|| online.pin.clone());
                if let Some(room_id) = online.pin_room.clone() {
                    online.send(ClientMessage::JoinRoom { room_id, pin });
                } else {
                    online.send(ClientMessage::CreateRoom { pin });
                }
            }
            MenuAction::PinSubmit => online.status = "Enter three digits".into(),
            MenuAction::LeaveRoom => {
                if online.connection == Connection::Connected {
                    online.send(ClientMessage::LeaveRoom);
                } else {
                    online.clear_room();
                    online.status = "Left room".into();
                }
            }
            MenuAction::StartOnlineGame
                if frontend.screen == Screen::Room && can_start_online_game(online) =>
            {
                online.send(ClientMessage::StartMatch);
            }
            _ => {}
        }
    }
    match action {
        MenuAction::Music => audio.toggle(AudioChannel::Music),
        MenuAction::Sfx => audio.toggle(AudioChannel::Sfx),
        MenuAction::RulebookItem(index) => help.open(index),
        MenuAction::Back if frontend.screen == Screen::Pin => frontend.show(Screen::Lobby),
        _ => frontend.activate(action),
    }
}

fn render_screen(
    mut commands: Commands,
    hud: Res<Hud>,
    asset_server: Res<AssetServer>,
    images: Res<InventoryImages>,
    mut frontend: ResMut<Frontend>,
    ui: (
        Res<AudioSettings>,
        Query<&ScrollPosition, With<LobbyViewport>>,
    ),
    online: Option<Res<Online>>,
) {
    let (audio, lobby_scroll) = ui;
    let scroll_position = lobby_scroll
        .single()
        .map(|position| position.y)
        .unwrap_or_default();
    let online_changed = online.as_ref().is_some_and(|online| online.is_changed())
        && matches!(frontend.screen, Screen::Lobby | Screen::Room | Screen::Pin);
    if let Some(online) = online.as_deref() {
        if online.room_id.is_some() && matches!(frontend.screen, Screen::Lobby | Screen::Pin) {
            frontend.show(Screen::Room);
        } else if online.room_id.is_none() && frontend.screen == Screen::Room {
            frontend.show(Screen::Lobby);
        }
    }
    if !frontend.dirty
        && !online_changed
        && !(frontend.screen == Screen::Options && audio.is_changed())
    {
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
                } else if matches!(frontend.screen, Screen::Lobby | Screen::Room | Screen::Pin) {
                    LOBBY_BACKGROUND
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
            Screen::Options => options_screen(parent, &asset_server, &frontend, &audio),
            Screen::Credits => credits_screen(parent, &asset_server),
            Screen::Rulebook => rulebook_screen(parent, &asset_server, &frontend, &images),
            Screen::Lobby => lobby_screen(parent, online.as_deref(), scroll_position),
            Screen::Room => room_screen(parent, online.as_deref()),
            Screen::Pin => {
                lobby_screen(parent, online.as_deref(), scroll_position);
                pin_screen(parent, online.as_deref());
            }
            Screen::Paused => pause_screen(parent, &asset_server, frontend.actions()),
            Screen::GameOver if frontend.has_online_match() => {
                online_game_over_screen(parent, frontend.outcome.expect("game has an outcome"))
            }
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
            padding: UiRect::bottom(Val::Px(24.0)),
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
                        let height = if actions.len() > 4 { 44.0 } else { 52.0 };
                        button(options, assets, action, label, height, index == 0);
                    }
                });
        });
}

fn lobby_screen(parent: &mut ChildSpawnerCommands, online: Option<&Online>, scroll_position: f32) {
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Px(RESOLUTION_WIDTH),
            height: Val::Px(RESOLUTION_HEIGHT),
            ..default()
        })
        .with_children(|content| {
            content
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Px(GRID_BLOCK_SIZE),
                    padding: UiRect::all(Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|header| text(header, "ONLINE LOBBY", 32.0, GOLD));

            content
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(GRID_BLOCK_SIZE),
                    left: Val::Px(0.0),
                    width: Val::Px(3.0 * GRID_BLOCK_SIZE),
                    height: Val::Px(7.0 * GRID_BLOCK_SIZE),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|sidebar| {
                    if let Some(online) = online {
                        if online.connection == Connection::Disconnected {
                            lobby_button(sidebar, MenuAction::Connect, "CONNECT", 160.0, true);
                        } else {
                            lobby_button(
                                sidebar,
                                MenuAction::CreateRoom,
                                "CREATE GAME",
                                160.0,
                                online.connection == Connection::Connected && !online.pending,
                            );
                        }
                    }
                });

            content
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(GRID_BLOCK_SIZE),
                    left: Val::Px(3.0 * GRID_BLOCK_SIZE),
                    width: Val::Px(13.0 * GRID_BLOCK_SIZE),
                    height: Val::Px(7.0 * GRID_BLOCK_SIZE),
                    padding: UiRect::all(Val::Px(10.0)),
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|list| {
                    let viewport = list
                        .spawn((
                            LobbyViewport,
                            Hovered::default(),
                            ScrollPosition(Vec2::new(0.0, scroll_position)),
                            Node {
                                flex_grow: 1.0,
                                min_width: Val::Px(0.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ))
                        .with_children(|rows| {
                            if let Some(online) = online {
                                if online.rooms.is_empty() {
                                    text(rows, "No games available", 18.0, TEXT);
                                }
                                for room in &online.rooms {
                                    if room.id.len() != 32 || !room.id.is_ascii() {
                                        continue;
                                    }
                                    let Ok(id) = u128::from_str_radix(&room.id, 16) else {
                                        continue;
                                    };
                                    rows.spawn((
                                        Button,
                                        Hovered::default(),
                                        MenuAction::JoinRoom(id),
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(50.0),
                                            flex_shrink: 0.0,
                                            padding: UiRect::horizontal(Val::Px(14.0)),
                                            column_gap: Val::Px(12.0),
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.08, 0.06, 0.05)),
                                    ))
                                    .with_children(|entry| {
                                        if room.locked {
                                            lock_icon(entry);
                                        }
                                        entry
                                            .spawn(Node {
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                ..default()
                                            })
                                            .with_children(|name| {
                                                text(name, &room.name, 19.0, TEXT)
                                            });
                                        if room.locked {
                                            text(entry, "LOCKED", 14.0, GOLD);
                                        }
                                        text(
                                            entry,
                                            if room.occupancy < 2 {
                                                "Waiting for opponent"
                                            } else {
                                                "Game in progress"
                                            },
                                            15.0,
                                            TEXT,
                                        );
                                    });
                                }
                            }
                        })
                        .id();
                    list.spawn((
                        LobbyScrollbar,
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

            content
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(8.0 * GRID_BLOCK_SIZE),
                    width: Val::Px(3.0 * GRID_BLOCK_SIZE),
                    height: Val::Px(GRID_BLOCK_SIZE),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|footer| {
                    lobby_button(footer, MenuAction::Back, "BACK", 160.0, true)
                });

            content
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0 * GRID_BLOCK_SIZE),
                    top: Val::Px(8.0 * GRID_BLOCK_SIZE),
                    width: Val::Px(10.0 * GRID_BLOCK_SIZE),
                    height: Val::Px(GRID_BLOCK_SIZE),
                    padding: UiRect::all(Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|footer| {
                    text(
                        footer,
                        online.map_or("Online services unavailable", |online| &online.status),
                        14.0,
                        TEXT,
                    );
                });
        });
}

fn lobby_button(
    parent: &mut ChildSpawnerCommands,
    action: MenuAction,
    label: &str,
    width: f32,
    enabled: bool,
) {
    let mut button = parent.spawn((
        Node {
            width: Val::Px(width),
            height: Val::Px(40.0),
            border: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(if enabled {
            Color::srgb(0.08, 0.06, 0.05)
        } else {
            Color::srgb(0.13, 0.14, 0.17)
        }),
        BorderColor::all(if enabled {
            GOLD
        } else {
            Color::srgb(0.3, 0.3, 0.3)
        }),
    ));
    if enabled {
        button.insert((Button, Hovered::default(), action));
    } else {
        button.insert(Pickable::IGNORE);
    }
    button.with_children(|content| {
        text(
            content,
            label,
            16.0,
            if enabled {
                TEXT
            } else {
                Color::srgb(0.5, 0.5, 0.5)
            },
        )
    });
}

fn lock_icon(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(20.0),
                height: Val::Px(24.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|icon| {
            icon.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(0.0),
                    width: Val::Px(12.0),
                    height: Val::Px(15.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BorderColor::all(GOLD),
                Pickable::IGNORE,
            ));
            icon.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1.0),
                    top: Val::Px(10.0),
                    width: Val::Px(18.0),
                    height: Val::Px(13.0),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(GOLD),
                Pickable::IGNORE,
            ));
        });
}

fn can_start_online_game(online: &Online) -> bool {
    online.seat == Some(Seat::Host)
        && online.room_id.is_some()
        && online.connection == Connection::Connected
        && !online.pending
        && online.occupancy == 2
        && online.opponent_online
}

fn room_screen(parent: &mut ChildSpawnerCommands, online: Option<&Online>) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|content| {
            text(content, "ONLINE ROOM", 32.0, GOLD);
            if let Some(online) = online {
                if let Some(name) = &online.room_name {
                    text(content, name, 25.0, GOLD);
                }
                text(content, &online.status, 18.0, TEXT);
                text(
                    content,
                    &format!("Players: {}/2", online.occupancy),
                    20.0,
                    TEXT,
                );
                text(
                    content,
                    if online.opponent_online {
                        "Opponent online"
                    } else {
                        "Opponent offline"
                    },
                    20.0,
                    TEXT,
                );
                if online.seat == Some(Seat::Host) {
                    lobby_button(
                        content,
                        MenuAction::StartOnlineGame,
                        "START GAME",
                        160.0,
                        can_start_online_game(online),
                    );
                }
            }
            lobby_button(content, MenuAction::LeaveRoom, "LEAVE ROOM", 160.0, true);
        });
}

fn pin_screen(parent: &mut ChildSpawnerCommands, online: Option<&Online>) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ZIndex(2),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(580.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(LOBBY_BACKGROUND),
                    BorderColor::all(GOLD),
                ))
                .with_children(|content| {
                    let joining = online.is_some_and(|online| online.pin_room.is_some());
                    text(
                        content,
                        if joining {
                            "ENTER ROOM PIN"
                        } else {
                            "CREATE GAME"
                        },
                        28.0,
                        GOLD,
                    );
                    if !joining {
                        text(
                            content,
                            "No PIN: open game. Three-digit PIN: locked game.",
                            16.0,
                            TEXT,
                        );
                    }
                    if let Some(online) = online {
                        text(
                            content,
                            &format!(
                                "{}{}",
                                "*".repeat(online.pin.len()),
                                "_".repeat(3 - online.pin.len())
                            ),
                            24.0,
                            TEXT,
                        );
                        text(content, &online.status, 16.0, TEXT);
                    }
                    for digits in [0..5, 5..10] {
                        content
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for digit in digits {
                                    row.spawn((
                                        Button,
                                        Hovered::default(),
                                        MenuAction::PinDigit(digit),
                                        Node {
                                            width: Val::Px(60.0),
                                            height: Val::Px(36.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.08, 0.06, 0.05)),
                                    ))
                                    .with_children(
                                        |digit_button| {
                                            text(digit_button, &digit.to_string(), 20.0, TEXT)
                                        },
                                    );
                                }
                            });
                    }
                    content
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|row| {
                            lobby_button(row, MenuAction::PinClear, "CLEAR", 140.0, true);
                            let enabled = online.is_some_and(|online| {
                                !online.pending
                                    && online.connection == Connection::Connected
                                    && (online.pin.len() == 3
                                        || (!joining && online.pin.is_empty()))
                            });
                            let label = if joining {
                                "JOIN GAME"
                            } else if online.is_some_and(|online| online.pin.is_empty()) {
                                "OPEN GAME"
                            } else {
                                "START GAME"
                            };
                            lobby_button(row, MenuAction::PinSubmit, label, 160.0, enabled);
                        });
                    lobby_button(content, MenuAction::Back, "CANCEL", 140.0, true);
                });
        });
}

fn options_screen(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    frontend: &Frontend,
    audio: &AudioSettings,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(18.0),
            ..default()
        })
        .with_children(|content| {
            text(content, "OPTIONS", 42.0, GOLD);
            for (index, (action, label)) in OPTIONS_ACTIONS.into_iter().enumerate() {
                let label = match action {
                    MenuAction::Music => {
                        format!("{label}: {}", if audio.music { "ON" } else { "OFF" })
                    }
                    MenuAction::Sfx => format!("{label}: {}", if audio.sfx { "ON" } else { "OFF" }),
                    _ => label.to_owned(),
                };
                button(
                    content,
                    assets,
                    action,
                    &label,
                    46.0,
                    index == frontend.selected,
                );
            }
        });
}

fn pause_screen(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    actions: &[(MenuAction, &str)],
) {
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
            for (index, &(action, label)) in actions.iter().enumerate() {
                button(content, assets, action, label, 46.0, index == 0);
            }
        });
}

fn online_game_over_screen(parent: &mut ChildSpawnerCommands, outcome: Outcome) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(24.0),
            ..default()
        })
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
            lobby_button(content, MenuAction::LeaveRoom, "LEAVE ROOM", 160.0, true);
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
            row_gap: Val::Px(14.0),
            ..default()
        })
        .with_children(|content| {
            text(content, "CREDITS", 42.0, GOLD);
            text(content, "Made with Bevy", 26.0, TEXT);
            text(content, "ART, MUSIC, SFX", 22.0, GOLD);
            text(content, "Isa Aguilar", 21.0, TEXT);
            text(content, "ENGINEERING", 22.0, GOLD);
            text(content, "AI Assisted", 21.0, TEXT);
            text(content, "Isa Aguilar", 21.0, TEXT);
            button(content, assets, MenuAction::Back, "BACK", 42.0, false);
        });
}

fn rulebook_screen(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    frontend: &Frontend,
    images: &InventoryImages,
) {
    parent
        .spawn(Node {
            width: Val::Px(RESOLUTION_WIDTH - 100.0),
            height: Val::Px(RESOLUTION_HEIGHT - 60.0),
            padding: UiRect::all(Val::Px(22.0)),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .insert((
            BackgroundColor(Color::srgb(0.08, 0.06, 0.05)),
            BorderColor::all(GOLD),
        ))
        .with_children(|content| {
            text(content, "RULEBOOK", 30.0, GOLD);
            content
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
                            RulebookViewport,
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
                        .with_children(|body| text(body, RULEBOOK_BASICS, 15.0, TEXT))
                        .id();
                    row.spawn((
                        RulebookScrollbar,
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
            text(content, "INGREDIENTS AND REACTIONS", 15.0, GOLD);
            for (row_index, row_items) in INVENTORY_ORDER.chunks(7).enumerate() {
                content
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (column, &item) in row_items.iter().enumerate() {
                            let selected = frontend.rulebook_selected == row_index * 7 + column;
                            row.spawn((
                                Button,
                                Hovered::default(),
                                MenuAction::RulebookItem(item),
                                Node {
                                    flex_grow: 1.0,
                                    flex_basis: Val::Px(0.0),
                                    min_width: Val::Px(0.0),
                                    height: Val::Px(58.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(if selected {
                                    Color::srgb(0.40, 0.28, 0.14)
                                } else {
                                    Color::srgba(0.08, 0.06, 0.05, 0.94)
                                }),
                                BorderColor::all(GOLD),
                            ))
                            .with_children(|entry| {
                                entry.spawn((
                                    images.icon(item),
                                    Node {
                                        width: Val::Px(30.0),
                                        height: Val::Px(30.0),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                text(entry, INVENTORY_ITEMS[item].0, 10.0, TEXT);
                            });
                        }
                    });
            }
            content
                .spawn(Node {
                    height: Val::Px(42.0),
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|row| {
                    button(row, assets, MenuAction::Back, "BACK", 42.0, false);
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
    fn lobby_pin_actions_queue_a_locked_room_and_cannot_start_a_match() {
        let mut frontend = Frontend::default();
        let mut audio = AudioSettings::default();
        let mut help = help::Help::default();
        let mut online = Online::default();
        online.connection = Connection::Connected;
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::Online,
        );
        assert_eq!(frontend.screen, Screen::Lobby);
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::CreateRoom,
        );
        assert_eq!(frontend.screen, Screen::Pin);
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinDigit(0),
        );
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinSubmit,
        );
        assert_eq!(online.status, "Enter three digits");
        assert!(online.outgoing.is_empty());
        for digit in [1, 2] {
            activate_action(
                &mut frontend,
                &mut audio,
                &mut help,
                Some(&mut online),
                MenuAction::PinDigit(digit),
            );
        }
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinSubmit,
        );
        assert!(
            matches!(online.outgoing.as_slice(), [ClientMessage::CreateRoom { pin: Some(pin) }] if pin == "012")
        );
        assert!(!frontend.match_in_progress);
    }

    #[test]
    fn room_start_button_tracks_host_and_opponent_and_leave_uses_lobby_style() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<InventoryImages>()
            .init_resource::<Online>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Update, render_screen);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Room);
        let online = &mut *app.world_mut().resource_mut::<Online>();
        online.connection = Connection::Connected;
        online.room_id = Some("room".into());
        online.seat = Some(Seat::Host);
        online.occupancy = 1;
        app.update();

        let start_count = |app: &mut App| {
            let world = app.world_mut();
            world
                .query_filtered::<&MenuAction, With<Button>>()
                .iter(world)
                .filter(|action| **action == MenuAction::StartOnlineGame)
                .count()
        };
        let start_label_count = |app: &mut App| {
            let world = app.world_mut();
            world
                .query::<&Text>()
                .iter(world)
                .filter(|text| text.0 == "START GAME")
                .count()
        };
        assert_eq!(start_label_count(&mut app), 1);
        assert_eq!(start_count(&mut app), 0);

        let world = app.world_mut();
        let leave = world
            .query::<(&MenuAction, &Children)>()
            .iter(world)
            .find_map(|(action, children)| {
                (*action == MenuAction::LeaveRoom).then(|| children.to_vec())
            })
            .unwrap();
        assert!(
            leave
                .iter()
                .all(|child| world.get::<MenuButtonArt>(*child).is_none())
        );

        let online = &mut *world.resource_mut::<Online>();
        online.occupancy = 2;
        online.opponent_online = true;
        app.update();
        assert_eq!(start_count(&mut app), 1);

        app.world_mut().resource_mut::<Online>().opponent_online = false;
        app.update();
        assert_eq!(start_label_count(&mut app), 1);
        assert_eq!(start_count(&mut app), 0);

        let online = &mut *app.world_mut().resource_mut::<Online>();
        online.opponent_online = true;
        online.seat = Some(Seat::Guest);
        app.update();
        assert_eq!(start_label_count(&mut app), 0);
        assert_eq!(start_count(&mut app), 0);
    }

    #[test]
    fn starting_online_game_requests_the_server_without_starting_a_local_match() {
        let mut frontend = Frontend::default();
        frontend.show(Screen::Room);
        let mut audio = AudioSettings::default();
        let mut help = help::Help::default();
        let mut online = Online::default();
        online.connection = Connection::Connected;
        online.room_id = Some("room".into());
        online.seat = Some(Seat::Host);
        online.occupancy = 1;
        online.status.clear();
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::StartOnlineGame,
        );
        assert!(online.status.is_empty());

        online.occupancy = 2;
        online.opponent_online = true;
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::StartOnlineGame,
        );
        assert!(matches!(
            online.outgoing.as_slice(),
            [ClientMessage::StartMatch]
        ));
        assert_eq!(frontend.screen, Screen::Room);
        assert!(!frontend.match_in_progress);
    }

    #[test]
    fn clearing_pin_allows_open_creation_but_join_requires_three_digits() {
        let mut frontend = Frontend::default();
        let mut audio = AudioSettings::default();
        let mut help = help::Help::default();
        let mut online = Online::default();
        online.connection = Connection::Connected;
        online.pin = "12".into();
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinClear,
        );
        assert!(online.pin.is_empty());
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinSubmit,
        );
        assert!(matches!(
            online.outgoing.as_slice(),
            [ClientMessage::CreateRoom { pin: None }]
        ));

        online.outgoing.clear();
        online.pending = false;
        online.pin_room = Some("room".into());
        activate_action(
            &mut frontend,
            &mut audio,
            &mut help,
            Some(&mut online),
            MenuAction::PinSubmit,
        );
        assert!(online.outgoing.is_empty());
        assert_eq!(online.status, "Enter three digits");
    }

    #[test]
    fn create_dialog_submit_is_only_clickable_for_an_empty_or_complete_pin() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<InventoryImages>()
            .init_resource::<Online>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Update, render_screen);
        app.world_mut().resource_mut::<Frontend>().show(Screen::Pin);
        app.world_mut().resource_mut::<Online>().connection = Connection::Connected;
        app.update();
        let submit_count = |app: &mut App| {
            let world = app.world_mut();
            world
                .query_filtered::<&MenuAction, With<Button>>()
                .iter(world)
                .filter(|action| **action == MenuAction::PinSubmit)
                .count()
        };
        assert_eq!(submit_count(&mut app), 1);
        app.world_mut().resource_mut::<Online>().pin = "12".into();
        app.update();
        assert_eq!(submit_count(&mut app), 0);
        app.world_mut().resource_mut::<Online>().pin = "123".into();
        app.update();
        assert_eq!(submit_count(&mut app), 1);
    }

    #[test]
    fn lobby_renders_all_rooms_and_preserves_scroll_when_the_list_updates() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<InventoryImages>()
            .init_resource::<Online>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Update, render_screen);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Lobby);
        let online = &mut *app.world_mut().resource_mut::<Online>();
        online.connection = Connection::Connected;
        online.status = "Lobby status".into();
        online.rooms = (0..7)
            .map(|index| volatile_online_protocol::RoomInfo {
                id: format!("{index:032x}"),
                name: format!("Room {index}"),
                locked: index == 0,
                occupancy: if index == 1 { 2 } else { 1 },
                online: 1,
            })
            .collect();
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&MenuAction, With<Button>>()
                .iter(world)
                .filter(|action| matches!(action, MenuAction::JoinRoom(_)))
                .count(),
            7
        );
        let texts: Vec<_> = world
            .query::<&Text>()
            .iter(world)
            .map(|t| t.0.as_str())
            .collect();
        assert_eq!(
            texts
                .iter()
                .filter(|text| **text == "Waiting for opponent")
                .count(),
            6
        );
        assert_eq!(
            texts
                .iter()
                .filter(|text| **text == "Game in progress")
                .count(),
            1
        );
        assert!(
            !texts
                .iter()
                .any(|text| *text == "1/2" || *text == "1 online")
        );
        let status_parent = world
            .query::<(&Text, &ChildOf)>()
            .iter(world)
            .find_map(|(text, parent)| (text.0 == "Lobby status").then(|| parent.parent()))
            .unwrap();
        let footer = world.get::<Node>(status_parent).unwrap();
        assert_eq!(footer.left, Val::Px(3.0 * GRID_BLOCK_SIZE));
        assert_eq!(footer.top, Val::Px(8.0 * GRID_BLOCK_SIZE));
        assert_eq!(footer.width, Val::Px(10.0 * GRID_BLOCK_SIZE));
        assert_eq!(footer.justify_content, JustifyContent::Center);
        let viewport = world
            .query_filtered::<Entity, With<LobbyViewport>>()
            .single(world)
            .unwrap();
        world.get_mut::<ScrollPosition>(viewport).unwrap().y = 120.0;
        let mut online = world.resource_mut::<Online>();
        online.status = "Updated".into();
        online.rooms[1].occupancy = 1;
        app.update();
        let world = app.world_mut();
        let viewport = world
            .query_filtered::<&ScrollPosition, With<LobbyViewport>>()
            .single(world)
            .unwrap();
        assert_eq!(viewport.y, 120.0);
        assert_eq!(
            world
                .query::<&Text>()
                .iter(world)
                .filter(|text| text.0 == "Waiting for opponent")
                .count(),
            7
        );
        assert!(
            !world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "Game in progress")
        );
    }

    #[test]
    fn options_support_keyboard_pointer_and_shared_settings_across_screens() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
            .init_resource::<InventoryImages>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_systems(Update, (handle_frontend, render_screen).chain());
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Options);
        app.update();
        tap(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<AudioSettings>().music);
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "MUSIC: OFF")
        );

        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(Entity, &MenuAction), With<Button>>();
        let sfx = buttons
            .iter(world)
            .find_map(|(entity, action)| (*action == MenuAction::Sfx).then_some(entity))
            .unwrap();
        world.entity_mut(sfx).insert(Pressed);
        app.update();
        assert!(!app.world().resource::<AudioSettings>().sfx);
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "SFX: OFF")
        );

        app.world_mut().resource_mut::<AudioSettings>().music = true;
        app.update();
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "MUSIC: ON")
        );
        tap(&mut app, KeyCode::ArrowDown);
        assert_eq!(app.world().resource::<Frontend>().selected, 1);
        tap(&mut app, KeyCode::Enter);
        assert!(app.world().resource::<AudioSettings>().sfx);
        tap(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<AudioSettings>().sfx);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Title);
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Credits);
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<&Text>();
        let credits: Vec<_> = query.iter(world).map(|text| text.0.as_str()).collect();
        assert_eq!(
            credits
                .iter()
                .filter(|&&text| text == "Made with Bevy")
                .count(),
            1
        );
        assert!(!credits.contains(&"Isa-Metric Games"));
        assert!(credits.contains(&"ART, MUSIC, SFX"));
        assert!(credits.contains(&"ENGINEERING"));
        assert!(credits.contains(&"AI Assisted"));
        assert_eq!(
            credits
                .iter()
                .filter(|&&text| text == "Isa Aguilar")
                .count(),
            2
        );
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Back);
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Computer);
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Paused);
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Back);
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Computer);
        assert!(app.world().resource::<AudioSettings>().music);
        assert!(!app.world().resource::<AudioSettings>().sfx);
    }

    #[test]
    fn title_uses_cover_and_compact_buttons_with_selection_art() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
            .init_resource::<InventoryImages>();
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
        assert_eq!(buttons.len(), TITLE_ACTIONS.len());
        for (action, node, children) in buttons {
            assert_eq!(node.height, Val::Px(44.0));
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
        assert_eq!(buttons.len(), ACTIVE_TITLE_ACTIONS.len());
        for (action, node, children) in buttons {
            assert_eq!(node.height, Val::Px(44.0));
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
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
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
    fn online_outcome_is_relative_to_the_receiving_seat() {
        for (seat, expected) in [(Seat::Host, Outcome::Lose), (Seat::Guest, Outcome::Win)] {
            let mut frontend = Frontend::default();
            frontend.enter_online_match();
            let mut online = Online::default();
            online.seat = Some(seat);
            let mut app = App::new();
            app.insert_resource(frontend)
                .insert_resource(online)
                .insert_resource(PlayerHealth([0, 5]))
                .add_systems(Update, detect_game_over);
            app.update();
            assert_eq!(app.world().resource::<Frontend>().outcome, Some(expected));
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
    fn rulebook_opens_reaction_guides_and_returns_to_basics() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
        use bevy::input::touch::TouchPhase;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_message::<MouseWheel>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<InventoryImages>()
            .init_resource::<PlayerHealth>()
            .insert_resource(crate::SerpentSheet(Handle::default()));
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_plugins((help::plugin, plugin));
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Rulebook);
        app.update();
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == RULEBOOK_BASICS)
        );
        let mut buttons = world.query::<&MenuAction>();
        assert_eq!(
            buttons
                .iter(world)
                .filter(|action| matches!(action, MenuAction::RulebookItem(_)))
                .count(),
            INVENTORY_ORDER.len()
        );
        let viewport = world
            .query_filtered::<Entity, With<RulebookViewport>>()
            .single(world)
            .unwrap();
        world.entity_mut(viewport).insert((
            Hovered(true),
            ComputedNode {
                size: Vec2::new(100.0, 100.0),
                content_size: Vec2::new(100.0, 200.0),
                inverse_scale_factor: 1.0,
                ..default()
            },
        ));
        world.write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -2.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 48.0);

        tap(&mut app, KeyCode::ArrowRight);
        tap(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<help::Help>().item(),
            Some(INVENTORY_ORDER[1])
        );
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0.starts_with("Tlexochtli Ember +"))
        );
        tap(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<help::Help>().is_open());
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Rulebook);
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<(Entity, &MenuAction), With<Button>>();
        let water = buttons
            .iter(world)
            .find_map(|(entity, action)| {
                (*action == MenuAction::RulebookItem(INVENTORY_ORDER[0])).then_some(entity)
            })
            .unwrap();
        world.entity_mut(water).insert(Pressed);
        app.update();
        assert_eq!(
            app.world().resource::<help::Help>().item(),
            Some(INVENTORY_ORDER[0])
        );
        let world = app.world_mut();
        let close = world
            .query_filtered::<Entity, With<help::HelpClose>>()
            .single(world)
            .unwrap();
        world.entity_mut(close).insert(Pressed);
        app.update();
        assert!(!app.world().resource::<help::Help>().is_open());
        assert_eq!(app.world().get::<ScrollPosition>(viewport).unwrap().y, 48.0);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Rulebook);
        tap(&mut app, KeyCode::Escape);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Title);
    }

    #[test]
    fn splash_sequence_and_keyboard_selection_work() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
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
    fn audio_toggles_appear_when_splashes_end() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<help::Help>();
        let hud = app.world_mut().spawn(Node::default()).id();
        app.insert_resource(Hud(hud))
            .add_plugins(crate::audio_settings::plugin)
            .add_systems(Update, handle_frontend.in_set(FrontendSystems));
        app.update();
        let world = app.world_mut();
        let row = world
            .query_filtered::<Entity, With<crate::audio_settings::AudioToggleRow>>()
            .single(world)
            .unwrap();
        assert_eq!(*world.get::<Visibility>(row).unwrap(), Visibility::Hidden);
        world.resource_mut::<Frontend>().show(Screen::StudioSplash);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(row).unwrap(),
            Visibility::Hidden
        );
        tap(&mut app, KeyCode::Enter);
        assert_eq!(app.world().resource::<Frontend>().screen, Screen::Title);
        assert_eq!(
            *app.world().get::<Visibility>(row).unwrap(),
            Visibility::Inherited
        );
        app.world_mut()
            .resource_mut::<Frontend>()
            .show(Screen::Game);
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(row).unwrap(),
            Visibility::Inherited
        );
    }

    #[test]
    fn hover_selection_persists_after_exit_and_keyboard_continues_from_it() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
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
    fn selection_changes_use_menu_option_sound_for_keyboard_and_pointer() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
            .init_resource::<crate::sound::SoundCues>()
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
        assert!(
            !app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        tap(&mut app, KeyCode::ArrowDown);
        assert!(
            app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        assert!(
            !app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::Select)
        );
        app.world_mut().entity_mut(credits).insert(Hovered(true));
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        tap(&mut app, KeyCode::ArrowDown);
        assert!(
            app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        app.world_mut().entity_mut(credits).insert(Hovered(false));
        app.update();
        app.world_mut().entity_mut(credits).insert(Hovered(true));
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        tap(&mut app, KeyCode::Enter);
        assert!(
            app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::Select)
        );
        assert!(
            !app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
        app.world_mut()
            .resource_mut::<Frontend>()
            .activate(MenuAction::Rulebook);
        tap(&mut app, KeyCode::ArrowRight);
        assert!(
            app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::MenuOption)
        );
    }

    #[test]
    fn keyboard_selection_is_not_overridden_by_stationary_hover() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Frontend>()
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
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
            .init_resource::<AudioSettings>()
            .init_resource::<help::Help>()
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
                .init_resource::<AudioSettings>()
                .init_resource::<help::Help>()
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
