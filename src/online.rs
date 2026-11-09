use bevy::prelude::*;
use volatile_online_protocol::{
    AnimationEvent, ClientMessage, HistoryEntry, MatchView, RoomInfo, Seat, ServerMessage,
    SoundEffect, TurnClock,
};

#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Connection {
    Disconnected,
    Connecting,
    Connected,
}

#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
#[derive(Resource)]
pub(crate) struct Online {
    pub connection: Connection,
    pub status: String,
    pub rooms: Vec<RoomInfo>,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub credential: Option<String>,
    pub seat: Option<Seat>,
    pub opponent_online: bool,
    pub occupancy: u8,
    pub pin: String,
    pub pin_room: Option<String>,
    pub pending: bool,
    reconnecting: bool,
    pub connect_requested: bool,
    pub outgoing: Vec<ClientMessage>,
    pub game: Option<OnlineMatch>,
    sounds: Vec<SoundEffect>,
    pub(crate) animations: Vec<Vec<AnimationEvent>>,
    pub(crate) history: Vec<HistoryEntry>,
    pub(crate) turn_clock: Option<TurnClock>,
    pub(crate) clock_generation: u64,
    rejected_action: bool,
}

pub(crate) struct OnlineMatch {
    pub revision: u64,
    pub state: MatchView,
}

impl Default for Online {
    fn default() -> Self {
        Self {
            connection: Connection::Disconnected,
            status: "Not connected".into(),
            rooms: Vec::new(),
            room_id: None,
            room_name: None,
            credential: None,
            seat: None,
            opponent_online: false,
            occupancy: 0,
            pin: String::new(),
            pin_room: None,
            pending: false,
            reconnecting: false,
            connect_requested: false,
            outgoing: Vec::new(),
            game: None,
            sounds: Vec::new(),
            animations: Vec::new(),
            history: Vec::new(),
            turn_clock: None,
            clock_generation: 0,
            rejected_action: false,
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
impl Online {
    pub fn send(&mut self, message: ClientMessage) {
        if self.connection != Connection::Connected {
            self.status = "Not connected to the server".into();
        } else if self.pending {
            self.status = "Waiting for the server".into();
        } else {
            self.pending = true;
            self.reconnecting = matches!(message, ClientMessage::Reconnect { .. });
            self.outgoing.push(message);
        }
    }

    fn receive(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Rooms { rooms } => {
                self.rooms = rooms;
            }
            ServerMessage::Joined {
                room_id,
                room_name,
                credential,
                seat,
            } => {
                self.room_id = Some(room_id);
                self.room_name = Some(room_name);
                self.credential = Some(credential);
                self.seat = Some(seat);
                self.pending = false;
                self.reconnecting = false;
                self.status = "Joined room".into();
            }
            ServerMessage::RoomStatus {
                room_id,
                occupancy,
                opponent_online,
            } => {
                if self.room_id.as_deref() == Some(&room_id) {
                    self.occupancy = occupancy;
                    self.opponent_online = opponent_online;
                }
            }
            ServerMessage::RoomClosed { reason } => {
                self.clear_room();
                self.status = format!("Room closed: {reason}");
            }
            ServerMessage::LeftRoom => {
                self.clear_room();
                self.status = "Left room".into();
            }
            ServerMessage::MatchState {
                room_id,
                revision,
                state,
                sounds,
                animations,
                turn_clock,
                history_start,
                history,
            } => {
                if self.room_id.as_deref() == Some(&room_id) {
                    if self
                        .game
                        .as_ref()
                        .is_some_and(|game| revision < game.revision)
                    {
                        return;
                    }
                    if history_start > self.history.len()
                        || history.iter().any(|entry| {
                            entry
                                .item
                                .is_some_and(|item| item >= crate::INVENTORY_ITEMS.len())
                        })
                    {
                        self.pending = false;
                        self.status =
                            "Invalid or incomplete match history; reconnect to resynchronize"
                                .into();
                        return;
                    }
                    self.history.truncate(history_start);
                    self.history.extend(history);
                    if self
                        .game
                        .as_ref()
                        .is_some_and(|game| revision > game.revision)
                    {
                        self.sounds.extend(sounds);
                        if !animations.is_empty() {
                            self.animations.push(animations);
                        }
                    }
                    self.game = Some(OnlineMatch { revision, state });
                    self.turn_clock = turn_clock;
                    self.clock_generation = self.clock_generation.wrapping_add(1);
                    self.pending = false;
                    self.status.clear();
                }
            }
            ServerMessage::MatchEnded { reason } => {
                let was_playing = self.game.take().is_some();
                self.animations.clear();
                self.history.clear();
                self.turn_clock = None;
                let was_starting = self.pending && !self.reconnecting;
                self.pending = false;
                if was_playing || was_starting {
                    self.status = format!("Match ended: {reason}");
                }
            }
            ServerMessage::Error { message } => {
                self.rejected_action |= self.pending && self.game.is_some() && !self.reconnecting;
                if self.reconnecting {
                    self.clear_room();
                }
                self.pending = false;
                self.status = message;
            }
        }
    }

    pub fn clear_room(&mut self) {
        self.room_id = None;
        self.room_name = None;
        self.credential = None;
        self.seat = None;
        self.opponent_online = false;
        self.occupancy = 0;
        self.pending = false;
        self.reconnecting = false;
        self.game = None;
        self.sounds.clear();
        self.animations.clear();
        self.history.clear();
        self.turn_clock = None;
        self.rejected_action = false;
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Online>()
        .init_resource::<crate::online_game::AppliedMatch>()
        .init_resource::<crate::online_game::AnimationPlayback>()
        .add_systems(
            Update,
            crate::online_game::play_animations
                .before(crate::online_game::apply_snapshot)
                .before(crate::online_game::update_controls)
                .before(crate::menu::GameplaySystems),
        )
        .add_systems(
            Update,
            crate::online_game::apply_snapshot.before(crate::menu::GameplaySystems),
        )
        .add_systems(
            Update,
            crate::online_game::update_controls
                .after(crate::online_game::apply_snapshot)
                .before(crate::menu::GameplaySystems),
        )
        .add_systems(
            Update,
            play_gameplay_sounds.before(crate::menu::GameplaySystems),
        )
        .add_systems(Update, crate::online_game::update_side_avatars);
    app.add_systems(
        Update,
        crate::online_game::update_turn_timer
            .after(crate::online_game::play_animations)
            .after(crate::online_game::apply_snapshot),
    );
    #[cfg(target_family = "wasm")]
    app.insert_non_send(browser::Transport::default())
        .add_systems(
            Update,
            browser::update
                .before(crate::menu::FrontendSystems)
                .before(crate::online_game::apply_snapshot)
                .before(crate::online_game::play_animations)
                .before(play_gameplay_sounds),
        );
}

fn play_gameplay_sounds(
    mut online: ResMut<Online>,
    sounds: Option<ResMut<crate::sound::SoundCues>>,
) {
    let Some(mut sounds) = sounds else {
        return;
    };
    for event in online.sounds.drain(..) {
        sounds.request_gameplay(event);
    }
    if std::mem::take(&mut online.rejected_action) {
        sounds.request(crate::sound::Cue::Reject);
    }
}

#[cfg(target_family = "wasm")]
mod browser {
    use std::{cell::RefCell, rc::Rc};

    use bevy::prelude::*;
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::{Event, MessageEvent, WebSocket};

    use super::{ClientMessage, Connection, Online, ServerMessage};

    enum EventMessage {
        Opened,
        Message(String),
        Closed,
        Error,
    }

    #[derive(Default)]
    pub struct Transport {
        socket: Option<WebSocket>,
        events: Rc<RefCell<Vec<EventMessage>>>,
        open: Option<Closure<dyn FnMut(Event)>>,
        message: Option<Closure<dyn FnMut(MessageEvent)>>,
        close: Option<Closure<dyn FnMut(Event)>>,
        error: Option<Closure<dyn FnMut(Event)>>,
        retry: f32,
    }

    impl Transport {
        fn clear(&mut self) {
            if let Some(socket) = self.socket.take() {
                socket.set_onopen(None);
                socket.set_onmessage(None);
                socket.set_onclose(None);
                socket.set_onerror(None);
                let _ = socket.close();
            }
            self.open = None;
            self.message = None;
            self.close = None;
            self.error = None;
        }

        fn connect(&mut self) -> Result<(), String> {
            let url = option_env!("VOLATILE_SERVER_URL").unwrap_or("ws://127.0.0.1:3000/ws");
            if !url.starts_with("wss://")
                && !url.starts_with("ws://127.0.0.1:")
                && !url.starts_with("ws://localhost:")
            {
                return Err("Server URL must use WSS except on localhost".into());
            }
            let socket = WebSocket::new(url)
                .map_err(|error| format!("Could not open WebSocket: {error:?}"))?;
            let events = self.events.clone();
            let open = Closure::<dyn FnMut(Event)>::new(move |_| {
                events.borrow_mut().push(EventMessage::Opened)
            });
            let events = self.events.clone();
            let message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                if let Some(text) = event.data().as_string() {
                    events.borrow_mut().push(EventMessage::Message(text));
                } else {
                    events.borrow_mut().push(EventMessage::Error);
                }
            });
            let events = self.events.clone();
            let close = Closure::<dyn FnMut(Event)>::new(move |_| {
                events.borrow_mut().push(EventMessage::Closed)
            });
            let events = self.events.clone();
            let error = Closure::<dyn FnMut(Event)>::new(move |_| {
                events.borrow_mut().push(EventMessage::Error)
            });
            socket.set_onopen(Some(open.as_ref().unchecked_ref()));
            socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
            socket.set_onclose(Some(close.as_ref().unchecked_ref()));
            socket.set_onerror(Some(error.as_ref().unchecked_ref()));
            self.socket = Some(socket);
            self.open = Some(open);
            self.message = Some(message);
            self.close = Some(close);
            self.error = Some(error);
            Ok(())
        }
    }

    pub fn update(
        time: Res<Time>,
        mut online: ResMut<Online>,
        mut transport: NonSendMut<Transport>,
    ) {
        if online.connect_requested {
            online.connect_requested = false;
            if online.connection == Connection::Disconnected {
                online.connection = Connection::Connecting;
                online.status = "Connecting...".into();
                if let Err(error) = transport.connect() {
                    online.connection = Connection::Disconnected;
                    online.status = error;
                }
            }
        }
        let events: Vec<_> = transport.events.borrow_mut().drain(..).collect();
        for event in events {
            match event {
                EventMessage::Opened => {
                    online.connection = Connection::Connected;
                    online.status = "Connected".into();
                    online.pending = false;
                    if let Some(credential) = online.credential.clone() {
                        online.send(ClientMessage::Reconnect { credential });
                    }
                }
                EventMessage::Message(text) => match serde_json::from_str::<ServerMessage>(&text) {
                    Ok(message) => online.receive(message),
                    Err(_) => online.status = "Invalid message from server".into(),
                },
                EventMessage::Closed | EventMessage::Error => {
                    transport.clear();
                    online.connection = Connection::Disconnected;
                    online.pending = false;
                    online.outgoing.clear();
                    online.opponent_online = false;
                    online.status = "Disconnected from server".into();
                    transport.retry = 2.0;
                }
            }
        }
        if online.connection == Connection::Disconnected && online.credential.is_some() {
            transport.retry -= time.delta_secs();
            if transport.retry <= 0.0 {
                online.connect_requested = true;
            }
        }
        if online.connection == Connection::Connected && !online.outgoing.is_empty() {
            for message in std::mem::take(&mut online.outgoing) {
                let result = serde_json::to_string(&message)
                    .map_err(|error| error.to_string())
                    .and_then(|text| {
                        transport
                            .socket
                            .as_ref()
                            .ok_or_else(|| "WebSocket is not available".to_owned())?
                            .send_with_str(&text)
                            .map_err(|error| format!("Could not send message: {error:?}"))
                    });
                if let Err(error) = result {
                    online.pending = false;
                    online.status = error;
                    transport.clear();
                    online.connection = Connection::Disconnected;
                    transport.retry = 2.0;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_failure_clears_stale_room_and_exposes_the_error() {
        let mut online = Online {
            connection: Connection::Connected,
            ..default()
        };
        online.receive(ServerMessage::Joined {
            room_id: "room".into(),
            room_name: "Jade Tikal".into(),
            credential: "secret".into(),
            seat: Seat::Host,
        });
        online.send(ClientMessage::Reconnect {
            credential: "secret".into(),
        });
        online.receive(ServerMessage::Error {
            message: "Invalid or expired reconnect credential".into(),
        });
        assert!(online.room_id.is_none());
        assert!(online.credential.is_none());
        assert!(!online.pending);
        assert!(online.status.contains("expired"));
    }

    #[test]
    fn room_closure_clears_credentials_and_reports_reason() {
        let mut online = Online::default();
        online.receive(ServerMessage::Joined {
            room_id: "room".into(),
            room_name: "Jade Tikal".into(),
            credential: "secret".into(),
            seat: Seat::Host,
        });
        online.receive(ServerMessage::RoomClosed {
            reason: "Five incorrect PIN attempts".into(),
        });
        assert!(online.room_id.is_none());
        assert!(online.credential.is_none());
        assert!(online.status.contains("PIN"));
    }

    #[test]
    fn reconnect_reports_a_match_that_ended_while_offline() {
        let mut engine = crate::headless::HeadlessMatch::new([7; 8]);
        let mut online = Online {
            game: Some(OnlineMatch {
                revision: 1,
                state: engine.view(Seat::Host),
            }),
            ..default()
        };
        online.receive(ServerMessage::MatchEnded {
            reason: "Opponent left".into(),
        });
        assert!(online.game.is_none());
        assert!(online.status.contains("Opponent left"));
        online.status.clear();
        online.receive(ServerMessage::MatchEnded {
            reason: "No active match".into(),
        });
        assert!(online.status.is_empty());
        online.pending = true;
        online.receive(ServerMessage::MatchEnded {
            reason: "Could not start".into(),
        });
        assert!(online.status.contains("Could not start"));
    }

    #[test]
    fn match_sounds_play_once_for_each_new_revision() {
        let mut engine = crate::headless::HeadlessMatch::new([7; 8]);
        let view = engine.view(Seat::Host);
        let mut online = Online {
            room_id: Some("room".into()),
            ..default()
        };
        let snapshot = |revision, sounds| ServerMessage::MatchState {
            room_id: "room".into(),
            revision,
            state: view.clone(),
            sounds,
            animations: Vec::new(),
            turn_clock: None,
            history_start: 0,
            history: Vec::new(),
        };
        online.receive(snapshot(0, vec![SoundEffect::Water]));
        assert!(online.sounds.is_empty());
        online.receive(snapshot(1, vec![SoundEffect::Water]));
        online.receive(snapshot(1, vec![SoundEffect::Water]));
        online.receive(snapshot(2, vec![SoundEffect::Impact]));
        assert_eq!(online.sounds, vec![SoundEffect::Water, SoundEffect::Impact]);
        let mut app = App::new();
        app.insert_resource(online)
            .init_resource::<crate::sound::SoundCues>()
            .add_systems(Update, play_gameplay_sounds);
        app.update();
        let mut cues = app.world_mut().resource_mut::<crate::sound::SoundCues>();
        assert!(cues.take(crate::sound::Cue::Water));
        assert!(cues.take(crate::sound::Cue::Impact));
        drop(cues);
        app.update();
        assert!(
            !app.world_mut()
                .resource_mut::<crate::sound::SoundCues>()
                .take(crate::sound::Cue::Water)
        );
    }

    #[test]
    fn clock_resynchronizes_on_same_revision_and_ignores_stale_snapshots() {
        let mut engine = crate::headless::HeadlessMatch::new([7; 8]);
        let view = engine.view(Seat::Host);
        let mut online = Online {
            room_id: Some("room".into()),
            ..default()
        };
        let snapshot = |revision, remaining_ms, running| ServerMessage::MatchState {
            room_id: "room".into(),
            revision,
            state: view.clone(),
            sounds: vec![],
            animations: vec![],
            turn_clock: Some(TurnClock {
                remaining_ms,
                starts_in_ms: 0,
                running,
            }),
            history_start: 0,
            history: vec![],
        };
        online.receive(snapshot(1, 25_000, true));
        let generation = online.clock_generation;
        online.receive(snapshot(1, 20_000, false));
        assert_ne!(online.clock_generation, generation);
        assert_eq!(online.turn_clock.as_ref().unwrap().remaining_ms, 20_000);
        assert!(!online.turn_clock.as_ref().unwrap().running);
        online.receive(snapshot(0, 30_000, true));
        assert_eq!(online.turn_clock.as_ref().unwrap().remaining_ms, 20_000);
        online.clear_room();
        assert!(online.turn_clock.is_none());
    }

    #[test]
    fn animation_events_are_queued_once_and_cleared_on_room_exit() {
        use volatile_online_protocol::{Animation, AnimationEvent, VisualEffect};

        let mut engine = crate::headless::HeadlessMatch::new([7; 8]);
        let view = engine.view(Seat::Host);
        let mut online = Online {
            room_id: Some("room".into()),
            ..default()
        };
        let event = AnimationEvent {
            at_ms: 450,
            animation: Animation::Effect {
                effect: VisualEffect::Steam,
                x: 2,
                y: 3,
            },
        };
        let snapshot = |revision, animations| ServerMessage::MatchState {
            room_id: "room".into(),
            revision,
            state: view.clone(),
            sounds: vec![],
            animations,
            turn_clock: None,
            history_start: 0,
            history: Vec::new(),
        };
        online.receive(snapshot(0, vec![event.clone()]));
        assert!(online.animations.is_empty());
        online.receive(snapshot(1, vec![event.clone()]));
        online.receive(snapshot(1, vec![event.clone()]));
        online.receive(snapshot(0, vec![event.clone()]));
        online.receive(snapshot(2, vec![event.clone()]));
        assert_eq!(online.animations, vec![vec![event.clone()], vec![event]]);
        assert_eq!(online.game.as_ref().unwrap().revision, 2);
        online.clear_room();
        assert!(online.animations.is_empty());
    }

    #[test]
    fn history_reconciles_deltas_duplicates_and_reconnect_snapshots() {
        let mut engine = crate::headless::HeadlessMatch::new([7; 8]);
        let view = engine.view(Seat::Host);
        let mut online = Online {
            room_id: Some("room".into()),
            ..default()
        };
        let entry = |text: &str| HistoryEntry {
            seat: Some(Seat::Host),
            text: text.into(),
            item: None,
        };
        let snapshot = |revision, history_start, history| ServerMessage::MatchState {
            room_id: "room".into(),
            revision,
            state: view.clone(),
            sounds: vec![],
            animations: vec![],
            turn_clock: None,
            history_start,
            history,
        };
        online.receive(snapshot(0, 0, vec![]));
        online.receive(snapshot(1, 0, vec![entry("first")]));
        online.receive(snapshot(1, 0, vec![entry("first")]));
        online.receive(snapshot(2, 1, vec![entry("second")]));
        online.receive(snapshot(1, 0, vec![entry("stale")]));
        assert_eq!(online.history, vec![entry("first"), entry("second")]);
        online.receive(snapshot(3, 5, vec![entry("missing")]));
        assert_eq!(online.game.as_ref().unwrap().revision, 2);
        assert!(online.status.contains("reconnect"));
        online.receive(snapshot(
            3,
            0,
            vec![entry("first"), entry("second"), entry("third")],
        ));
        assert_eq!(
            online.history,
            vec![entry("first"), entry("second"), entry("third")]
        );
        online.clear_room();
        assert!(online.history.is_empty());
    }
}
