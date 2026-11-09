use std::{
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
    time::{Duration, Instant},
};

use tokio::sync::{mpsc, watch};
use volatile::headless::HeadlessMatch;
use volatile_online_protocol::{
    ClientMessage, GameAction, RoomInfo, Seat, ServerMessage, SoundEffect,
};

const ROOM_NAMES: [&str; 9] = [
    "Calakmul",
    "Chichen Itza",
    "Copan",
    "Monte Alban",
    "Palenque",
    "Tikal",
    "Tulum",
    "Uxmal",
    "Yaxchilan",
];
const ROOM_ADJECTIVES: [&str; 12] = [
    "Adventurous",
    "Bold",
    "Brave",
    "Clever",
    "Curious",
    "Daring",
    "Dreamy",
    "Jolly",
    "Mischievous",
    "Playful",
    "Spirited",
    "Witty",
];
const MAX_ROOMS: usize = 7;
const MAX_CLOSED_ROOMS: usize = 64;
const RECONNECT_GRACE: Duration = Duration::from_secs(5 * 60);
const GAME_ACTION_INTERVAL: Duration = Duration::from_millis(100);

pub struct Client {
    pub outgoing: mpsc::Sender<ServerMessage>,
    pub stop: watch::Sender<bool>,
}

struct Player {
    credential: String,
    connection: Option<u64>,
    disconnected_at: Option<Instant>,
}

struct Room {
    base_name: &'static str,
    name: String,
    pin: Option<String>,
    failed_attempts: u8,
    host: Player,
    guest: Option<Player>,
    game: Option<Match>,
}

struct Match {
    engine: HeadlessMatch,
    revision: u64,
    last_action: [Option<Instant>; 2],
}

#[derive(Default)]
pub struct Lobby {
    clients: HashMap<u64, Client>,
    rooms: HashMap<String, Room>,
    closed: HashMap<String, Instant>,
}

fn random_hex(bytes: usize) -> Result<String, getrandom::Error> {
    let mut random = vec![0; bytes];
    getrandom::fill(&mut random)?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn valid_pin(pin: &str) -> bool {
    pin.len() == 3 && pin.bytes().all(|byte| byte.is_ascii_digit())
}

impl Lobby {
    pub fn connect(&mut self, id: u64, client: Client) {
        self.clients.insert(id, client);
        self.send_rooms(id);
    }
}

impl Lobby {
    fn send(&self, id: u64, message: ServerMessage) {
        if let Some(client) = self.clients.get(&id)
            && client.outgoing.try_send(message).is_err()
        {
            eprintln!("closing slow or disconnected client");
            let _ = client.stop.send(true);
        }
    }

    fn error(&self, id: u64, message: &str) {
        self.send(
            id,
            ServerMessage::Error {
                message: message.into(),
            },
        );
    }

    fn room_info(&self) -> Vec<RoomInfo> {
        let mut rooms: Vec<_> = self
            .rooms
            .iter()
            .filter(|(_, room)| {
                room.host.connection.is_some()
                    || room
                        .guest
                        .as_ref()
                        .is_some_and(|guest| guest.connection.is_some())
            })
            .map(|(id, room)| RoomInfo {
                id: id.clone(),
                name: room.name.clone(),
                locked: room.pin.is_some(),
                occupancy: 1 + u8::from(room.guest.is_some()),
                online: u8::from(room.host.connection.is_some())
                    + u8::from(
                        room.guest
                            .as_ref()
                            .is_some_and(|guest| guest.connection.is_some()),
                    ),
            })
            .collect();
        rooms.sort_by(|a, b| a.name.cmp(&b.name));
        rooms
    }

    fn send_rooms(&self, id: u64) {
        self.send(
            id,
            ServerMessage::Rooms {
                rooms: self.room_info(),
            },
        );
    }

    fn broadcast_rooms(&self) {
        let rooms = self.room_info();
        for id in self.clients.keys() {
            self.send(
                *id,
                ServerMessage::Rooms {
                    rooms: rooms.clone(),
                },
            );
        }
    }

    fn notify_room(&self, room_id: &str) {
        let Some(room) = self.rooms.get(room_id) else {
            return;
        };
        let occupancy = 1 + u8::from(room.guest.is_some());
        if let Some(id) = room.host.connection {
            self.send(
                id,
                ServerMessage::RoomStatus {
                    room_id: room_id.into(),
                    occupancy,
                    opponent_online: room
                        .guest
                        .as_ref()
                        .is_some_and(|guest| guest.connection.is_some()),
                },
            );
        }
        if let Some(id) = room.guest.as_ref().and_then(|guest| guest.connection) {
            self.send(
                id,
                ServerMessage::RoomStatus {
                    room_id: room_id.into(),
                    occupancy,
                    opponent_online: room.host.connection.is_some(),
                },
            );
        }
    }

    fn room_for_connection(&self, id: u64) -> Option<String> {
        self.rooms.iter().find_map(|(room_id, room)| {
            (room.host.connection == Some(id)
                || room
                    .guest
                    .as_ref()
                    .is_some_and(|guest| guest.connection == Some(id)))
            .then(|| room_id.clone())
        })
    }

    pub fn handle(&mut self, id: u64, message: ClientMessage) {
        match message {
            ClientMessage::ListRooms => self.send_rooms(id),
            ClientMessage::CreateRoom { pin } => self.create(id, pin),
            ClientMessage::JoinRoom { room_id, pin } => self.join(id, room_id, pin),
            ClientMessage::Reconnect { credential } => self.reconnect(id, credential),
            ClientMessage::LeaveRoom => self.leave(id),
            ClientMessage::StartMatch => self.start_match(id),
            ClientMessage::GameAction { revision, action } => {
                self.game_action(id, revision, action)
            }
        }
    }

    fn create(&mut self, id: u64, pin: Option<String>) {
        if self.room_for_connection(id).is_some() {
            return self.error(id, "Leave your current room first");
        }
        if pin.as_deref().is_some_and(|pin| !valid_pin(pin)) {
            return self.error(id, "PIN must contain exactly three digits");
        }
        if self.rooms.len() >= MAX_ROOMS {
            return self.error(id, "Server room capacity reached");
        }
        let available: Vec<_> = ROOM_NAMES
            .iter()
            .copied()
            .filter(|name| !self.rooms.values().any(|room| room.base_name == *name))
            .collect();
        if available.is_empty() {
            return self.error(id, "Server room capacity reached");
        }
        let mut random = [0; 2];
        if getrandom::fill(&mut random).is_err() {
            return self.error(id, "Could not choose a room name");
        }
        let base_name = available[usize::from(random[0]) % available.len()];
        let name = format!(
            "{} {base_name}",
            ROOM_ADJECTIVES[usize::from(random[1]) % ROOM_ADJECTIVES.len()]
        );
        let (room_id, credential) = match (random_hex(16), random_hex(32)) {
            (Ok(room_id), Ok(credential)) => (room_id, credential),
            _ => return self.error(id, "Could not create secure room credentials"),
        };
        if self.rooms.contains_key(&room_id) {
            return self.error(id, "Could not create a unique room");
        }
        self.rooms.insert(
            room_id.clone(),
            Room {
                base_name,
                name: name.clone(),
                pin,
                failed_attempts: 0,
                host: Player {
                    credential: credential.clone(),
                    connection: Some(id),
                    disconnected_at: None,
                },
                guest: None,
                game: None,
            },
        );
        self.send(
            id,
            ServerMessage::Joined {
                room_id: room_id.clone(),
                room_name: name,
                credential,
                seat: Seat::Host,
            },
        );
        self.notify_room(&room_id);
        self.broadcast_rooms();
    }

    fn join(&mut self, id: u64, room_id: String, pin: Option<String>) {
        if self.room_for_connection(id).is_some() {
            return self.error(id, "Leave your current room first");
        }

        let Some(room) = self.rooms.get_mut(&room_id) else {
            return self.error(id, "Room not found");
        };
        if room.guest.is_some() {
            return self.error(id, "Room is full");
        }
        if room
            .pin
            .as_ref()
            .is_some_and(|expected| pin.as_ref() != Some(expected))
        {
            room.failed_attempts += 1;
            if room.failed_attempts >= 5 {
                let room = self.rooms.remove(&room_id).unwrap();
                let host = room.host.connection;
                if let Some(host) = host {
                    self.send(
                        host,
                        ServerMessage::RoomClosed {
                            reason: "Five incorrect PIN attempts".into(),
                        },
                    );
                } else {
                    if self.closed.len() >= MAX_CLOSED_ROOMS
                        && let Some(oldest) = self
                            .closed
                            .iter()
                            .min_by_key(|(_, at)| *at)
                            .map(|(key, _)| key.clone())
                    {
                        self.closed.remove(&oldest);
                    }
                    self.closed.insert(room.host.credential, Instant::now());
                }
                self.error(id, "Room closed after five incorrect PIN attempts");
                self.broadcast_rooms();
            } else {
                self.error(id, "Incorrect PIN");
            }
            return;
        }
        let credential = match random_hex(32) {
            Ok(credential) => credential,
            Err(_) => return self.error(id, "Could not create secure room credentials"),
        };
        let room = self.rooms.get_mut(&room_id).unwrap();
        let room_name = room.name.clone();
        room.guest = Some(Player {
            credential: credential.clone(),
            connection: Some(id),
            disconnected_at: None,
        });
        self.send(
            id,
            ServerMessage::Joined {
                room_id: room_id.clone(),
                room_name,
                credential,
                seat: Seat::Guest,
            },
        );
        self.notify_room(&room_id);
        self.broadcast_rooms();
        self.send_match_views(&room_id, &[], &[], 0);
    }

    fn start_match(&mut self, id: u64) {
        let Some(room_id) = self.room_for_connection(id) else {
            return self.error(id, "Join a room first");
        };
        let room = self.rooms.get_mut(&room_id).expect("connected room exists");
        if room.host.connection != Some(id) {
            return self.error(id, "Only the host can start the match");
        }
        if room.game.is_some() {
            return self.error(id, "The match has already started");
        }
        if room
            .guest
            .as_ref()
            .and_then(|guest| guest.connection)
            .is_none()
        {
            return self.error(id, "Wait for the opponent to connect");
        }
        let mut seed = [0; 8];
        if getrandom::fill(&mut seed).is_err() {
            return self.error(id, "Could not initialize the match");
        }
        let engine = match catch_unwind(AssertUnwindSafe(|| HeadlessMatch::new(seed))) {
            Ok(engine) => engine,
            Err(_) => {
                eprintln!("match initialization panicked for room {room_id}");
                return self.error(id, "Could not initialize the match");
            }
        };
        room.game = Some(Match {
            engine,
            revision: 0,
            last_action: [None, None],
        });
        self.send_match_views(&room_id, &[], &[], 0);
    }

    fn send_match_views(
        &mut self,
        room_id: &str,
        sounds: &[SoundEffect],
        animations: &[volatile_online_protocol::AnimationEvent],
        history_start: usize,
    ) {
        let Some(room) = self.rooms.get_mut(room_id) else {
            return;
        };
        let Some(game) = &mut room.game else {
            return;
        };
        let views = catch_unwind(AssertUnwindSafe(|| {
            let host = room.host.connection.map(|id| {
                (
                    id,
                    ServerMessage::MatchState {
                        room_id: room_id.into(),
                        revision: game.revision,
                        state: game.engine.view(Seat::Host),
                        sounds: sounds.to_vec(),
                        animations: animations.to_vec(),
                        history_start,
                        history: game.engine.history()[history_start..].to_vec(),
                    },
                )
            });
            let guest = room
                .guest
                .as_ref()
                .and_then(|guest| guest.connection)
                .map(|id| {
                    (
                        id,
                        ServerMessage::MatchState {
                            room_id: room_id.into(),
                            revision: game.revision,
                            state: game.engine.view(Seat::Guest),
                            sounds: sounds.to_vec(),
                            animations: animations.to_vec(),
                            history_start,
                            history: game.engine.history()[history_start..].to_vec(),
                        },
                    )
                });
            (host, guest)
        }));
        let (host, guest) = match views {
            Ok(views) => views,
            Err(_) => {
                eprintln!("match snapshot panicked for room {room_id}");
                self.end_match(room_id, "The match could not continue");
                return;
            }
        };
        for (id, message) in host.into_iter().chain(guest) {
            self.send(id, message);
        }
    }

    fn end_match(&mut self, room_id: &str, reason: &str) {
        let Some(room) = self.rooms.get_mut(room_id) else {
            return;
        };
        room.game = None;
        let host = room.host.connection;
        let guest = room.guest.as_ref().and_then(|guest| guest.connection);
        for id in host.into_iter().chain(guest) {
            self.send(
                id,
                ServerMessage::MatchEnded {
                    reason: reason.into(),
                },
            );
        }
    }

    fn game_action(&mut self, id: u64, revision: u64, action: GameAction) {
        let Some(room_id) = self.room_for_connection(id) else {
            return self.error(id, "Join a room first");
        };
        let room = self.rooms.get_mut(&room_id).expect("connected room exists");
        let seat = if room.host.connection == Some(id) {
            Seat::Host
        } else {
            Seat::Guest
        };
        if room.host.connection.is_none()
            || room
                .guest
                .as_ref()
                .and_then(|guest| guest.connection)
                .is_none()
        {
            return self.error(id, "Wait for the opponent to reconnect");
        }
        let Some(game) = &mut room.game else {
            return self.error(id, "The match has not started");
        };
        if game.revision != revision {
            self.error(id, "Match state changed; try again");
            self.send_match_views(&room_id, &[], &[], 0);
            return;
        }
        let index = if seat == Seat::Host { 0 } else { 1 };
        let now = Instant::now();
        if game.last_action[index]
            .is_some_and(|last| now.duration_since(last) < GAME_ACTION_INTERVAL)
        {
            return self.error(id, "Please wait before another action");
        }
        game.last_action[index] = Some(now);
        let history_start = game.engine.history().len();
        let result = catch_unwind(AssertUnwindSafe(|| game.engine.perform(seat, action)));
        let result = match result {
            Ok(result) => result,
            Err(_) => {
                eprintln!("match simulation panicked for room {room_id}");
                self.end_match(&room_id, "The match could not continue");
                return;
            }
        };
        if let Err(error) = result {
            if game.engine.is_broken() {
                self.end_match(&room_id, &error);
            } else {
                self.error(id, &error);
            }
            return;
        }
        let Some(revision) = game.revision.checked_add(1) else {
            self.end_match(&room_id, "The match could not continue");
            return;
        };
        game.revision = revision;
        let sounds = game.engine.take_sounds();
        let animations = game.engine.take_animations();
        self.send_match_views(&room_id, &sounds, &animations, history_start);
    }

    fn reconnect(&mut self, id: u64, credential: String) {
        if self.room_for_connection(id).is_some() {
            return self.error(id, "Leave your current room first");
        }
        let found = self.rooms.iter().find_map(|(room_id, room)| {
            if room.host.credential == credential {
                Some((room_id.clone(), Seat::Host))
            } else if room
                .guest
                .as_ref()
                .is_some_and(|guest| guest.credential == credential)
            {
                Some((room_id.clone(), Seat::Guest))
            } else {
                None
            }
        });
        let Some((room_id, seat)) = found else {
            if self
                .closed
                .remove(&credential)
                .is_some_and(|at| at.elapsed() < RECONNECT_GRACE)
            {
                self.send(
                    id,
                    ServerMessage::RoomClosed {
                        reason: "Five incorrect PIN attempts".into(),
                    },
                );
                return;
            }
            return self.error(id, "Invalid or expired reconnect credential");
        };
        let room = self.rooms.get_mut(&room_id).unwrap();
        let room_name = room.name.clone();
        let match_active = room.game.is_some();
        let player = match seat {
            Seat::Host => &mut room.host,
            Seat::Guest => room.guest.as_mut().unwrap(),
        };
        if player
            .disconnected_at
            .is_some_and(|at| at.elapsed() >= RECONNECT_GRACE)
        {
            return self.error(id, "Invalid or expired reconnect credential");
        }
        let previous = player.connection;
        player.connection = Some(id);
        player.disconnected_at = None;
        if let Some(previous) = previous
            && let Some(client) = self.clients.get(&previous)
        {
            let _ = client.stop.send(true);
        }
        self.send(
            id,
            ServerMessage::Joined {
                room_id: room_id.clone(),
                room_name,
                credential,
                seat,
            },
        );
        self.notify_room(&room_id);
        self.broadcast_rooms();
        if match_active {
            self.send_match_views(&room_id, &[], &[], 0);
        } else {
            self.send(
                id,
                ServerMessage::MatchEnded {
                    reason: "The match ended while you were disconnected".into(),
                },
            );
        }
    }

    fn leave(&mut self, id: u64) {
        let Some(room_id) = self.room_for_connection(id) else {
            return self.error(id, "Not in a room");
        };
        let room = self.rooms.get_mut(&room_id).unwrap();
        if room.host.connection == Some(id) {
            let guest = room.guest.as_ref().and_then(|guest| guest.connection);
            self.rooms.remove(&room_id);
            if let Some(guest) = guest {
                self.send(
                    guest,
                    ServerMessage::RoomClosed {
                        reason: "Host left the room".into(),
                    },
                );
            }
        } else {
            room.guest = None;
            let ended = room.game.take().is_some();
            let host = room.host.connection;
            if ended && let Some(host) = host {
                self.send(
                    host,
                    ServerMessage::MatchEnded {
                        reason: "Opponent left the room".into(),
                    },
                );
            }
            self.notify_room(&room_id);
        }
        self.send(id, ServerMessage::LeftRoom);
        self.broadcast_rooms();
    }

    pub fn disconnect(&mut self, id: u64) {
        self.clients.remove(&id);
        if let Some(room_id) = self.room_for_connection(id) {
            let room = self.rooms.get_mut(&room_id).unwrap();
            let player = if room.host.connection == Some(id) {
                &mut room.host
            } else {
                room.guest.as_mut().unwrap()
            };
            player.connection = None;
            player.disconnected_at = Some(Instant::now());
            self.notify_room(&room_id);
            self.broadcast_rooms();
        }
    }

    pub fn expire(&mut self) {
        self.closed.retain(|_, at| at.elapsed() < RECONNECT_GRACE);
        let expired = |player: &Player| {
            player
                .disconnected_at
                .is_some_and(|at| at.elapsed() >= RECONNECT_GRACE)
        };
        let mut closed = Vec::new();
        let mut changed = Vec::new();
        for (id, room) in &mut self.rooms {
            if expired(&room.host) {
                closed.push((
                    id.clone(),
                    room.guest.as_ref().and_then(|guest| guest.connection),
                ));
            } else if room.guest.as_ref().is_some_and(expired) {
                room.guest = None;
                changed.push((id.clone(), room.game.take().is_some()));
            }
        }
        let any = !closed.is_empty() || !changed.is_empty();
        for (id, guest) in closed {
            self.rooms.remove(&id);
            if let Some(guest) = guest {
                self.send(
                    guest,
                    ServerMessage::RoomClosed {
                        reason: "Host reconnect time expired".into(),
                    },
                );
            }
        }
        for (id, ended) in changed {
            if ended && let Some(host) = self.rooms[&id].host.connection {
                self.send(
                    host,
                    ServerMessage::MatchEnded {
                        reason: "Opponent reconnect time expired".into(),
                    },
                );
            }
            self.notify_room(&id);
        }
        if any {
            self.broadcast_rooms();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(
        lobby: &mut Lobby,
        id: u64,
    ) -> (mpsc::Receiver<ServerMessage>, watch::Receiver<bool>) {
        let (outgoing, receiver) = mpsc::channel(64);
        let (stop, stopped) = watch::channel(false);
        lobby.connect(id, Client { outgoing, stop });
        (receiver, stopped)
    }

    fn joined(receiver: &mut mpsc::Receiver<ServerMessage>) -> (String, String, Seat) {
        while let Ok(message) = receiver.try_recv() {
            if let ServerMessage::Joined {
                room_id,
                credential,
                seat,
                ..
            } = message
            {
                return (room_id, credential, seat);
            }
        }
        panic!("no joined message");
    }

    fn drain(receiver: &mut mpsc::Receiver<ServerMessage>) -> Vec<ServerMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    #[test]
    fn both_players_receive_animations_only_for_the_new_action() {
        use volatile_online_protocol::Animation;

        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut guest, _) = client(&mut lobby, 2);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, _, _) = joined(&mut host);
        lobby.handle(
            2,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                pin: None,
            },
        );
        let (_, guest_token, _) = joined(&mut guest);
        lobby.handle(1, ClientMessage::StartMatch);
        let initial = drain(&mut host);
        let start = initial
            .iter()
            .find_map(|message| match message {
                ServerMessage::MatchState { state, .. } => {
                    Some((state.players[0].x, state.players[0].y))
                }
                _ => None,
            })
            .unwrap();
        drain(&mut guest);
        lobby.handle(
            1,
            ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Place {
                    ingredient: 0,
                    x: start.0,
                    y: start.1,
                },
            },
        );
        let host_first = drain(&mut host);
        let guest_first = drain(&mut guest);
        let first_history = |messages: &[ServerMessage]| {
            messages
                .iter()
                .find_map(|message| match message {
                    ServerMessage::MatchState {
                        revision: 1,
                        history_start: 0,
                        history,
                        ..
                    } => Some(history.clone()),
                    _ => None,
                })
                .expect("first history delta")
        };
        let first = first_history(&host_first);
        assert_eq!(first, first_history(&guest_first));
        assert!(first.iter().any(|entry| entry.text.contains("queued")));
        lobby
            .rooms
            .get_mut(&room_id)
            .unwrap()
            .game
            .as_mut()
            .unwrap()
            .last_action = [None, None];
        lobby.handle(
            1,
            ClientMessage::GameAction {
                revision: 1,
                action: GameAction::Move {
                    x: start.0 + 1,
                    y: start.1,
                },
            },
        );
        let events = |messages: Vec<ServerMessage>| {
            messages
                .into_iter()
                .find_map(|message| match message {
                    ServerMessage::MatchState {
                        revision: 2,
                        animations,
                        history_start,
                        history,
                        ..
                    } => Some((animations, history_start, history)),
                    _ => None,
                })
                .expect("action snapshot")
        };
        let (host_events, delta_start, second) = events(drain(&mut host));
        let (guest_events, guest_start, guest_history) = events(drain(&mut guest));
        assert_eq!(host_events, guest_events);
        assert_eq!(delta_start, first.len());
        assert_eq!(guest_start, delta_start);
        assert_eq!(guest_history, second);
        assert!(second.iter().any(|entry| entry.text.contains("moved")));
        assert!(second.iter().any(|entry| entry.text.contains("threw")));
        let serialized = serde_json::to_string(&host_events).unwrap();
        let decoded: Vec<volatile_online_protocol::AnimationEvent> =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded, host_events);
        assert!(matches!(
            host_events.first().map(|event| &event.animation),
            Some(Animation::MovePlayer { seat: Seat::Host, x, y })
                if *x == start.0 + 1 && *y == start.1
        ));
        assert!(host_events.iter().any(
            |event| matches!(event.animation, Animation::Throw { from, .. }
                    if from == (start.0 + 1, start.1))
        ));
        assert!(
            host_events
                .iter()
                .any(|event| matches!(event.animation, Animation::Effect { .. }))
        );

        lobby.disconnect(2);
        let (mut returning, _) = client(&mut lobby, 3);
        lobby.handle(
            3,
            ClientMessage::Reconnect {
                credential: guest_token,
            },
        );
        let messages = drain(&mut returning);
        let restored = messages
            .iter()
            .find_map(|message| match message {
                ServerMessage::MatchState {
                    revision: 2,
                    animations,
                    history_start: 0,
                    history,
                    ..
                } if animations.is_empty() => Some(history),
                _ => None,
            })
            .expect("reconnect snapshot");
        assert_eq!(
            *restored,
            first.into_iter().chain(second).collect::<Vec<_>>()
        );
    }

    #[test]
    fn match_actions_are_authoritative_and_resume_after_reconnect() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut guest, _) = client(&mut lobby, 2);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, _, _) = joined(&mut host);
        lobby.handle(1, ClientMessage::StartMatch);
        assert!(drain(&mut host).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("opponent")
        )));
        lobby.handle(
            2,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                pin: None,
            },
        );
        let (_, guest_token, _) = joined(&mut guest);
        lobby.handle(2, ClientMessage::StartMatch);
        assert!(drain(&mut guest).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("host")
        )));
        lobby.handle(1, ClientMessage::StartMatch);
        assert!(
            drain(&mut host)
                .iter()
                .any(|message| matches!(message, ServerMessage::MatchState { revision: 0, .. }))
        );
        assert!(
            drain(&mut guest)
                .iter()
                .any(|message| matches!(message, ServerMessage::MatchState { revision: 0, .. }))
        );
        lobby.handle(
            2,
            ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Rest,
            },
        );
        assert!(drain(&mut guest).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("turn")
        )));
        lobby.handle(
            1,
            ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Move { x: -1, y: 0 },
            },
        );
        assert!(drain(&mut host).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("outside")
        )));
        lobby
            .rooms
            .get_mut(&room_id)
            .unwrap()
            .game
            .as_mut()
            .unwrap()
            .last_action = [None, None];
        lobby.handle(
            1,
            ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Rest,
            },
        );
        assert!(drain(&mut host).iter().any(|message| matches!(
            message, ServerMessage::MatchState { revision: 1, state, sounds, .. }
                if state.active == Seat::Guest && sounds.contains(&SoundEffect::Chime)
        )));
        assert!(drain(&mut guest).iter().any(|message| matches!(
            message, ServerMessage::MatchState { revision: 1, state, sounds, .. }
                if state.active == Seat::Guest && sounds.contains(&SoundEffect::Chime)
        )));
        lobby.handle(
            2,
            ClientMessage::GameAction {
                revision: 0,
                action: GameAction::Rest,
            },
        );
        assert!(drain(&mut guest).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("changed")
        )));
        lobby.disconnect(2);
        lobby.handle(
            1,
            ClientMessage::GameAction {
                revision: 1,
                action: GameAction::Rest,
            },
        );
        assert!(drain(&mut host).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("reconnect")
        )));
        let (mut returning, _) = client(&mut lobby, 3);
        lobby.handle(
            3,
            ClientMessage::Reconnect {
                credential: guest_token,
            },
        );
        assert!(drain(&mut returning).iter().any(|message| matches!(
            message, ServerMessage::MatchState { revision: 1, state, sounds, .. }
                if state.active == Seat::Guest && sounds.is_empty()
        )));
        lobby.handle(3, ClientMessage::LeaveRoom);
        assert!(lobby.rooms[&room_id].game.is_none());
        assert!(
            drain(&mut host)
                .iter()
                .any(|message| matches!(message, ServerMessage::MatchEnded { .. }))
        );
    }

    #[test]
    fn reconnecting_host_is_told_when_match_ended_while_offline() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut guest, _) = client(&mut lobby, 2);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, host_token, _) = joined(&mut host);
        lobby.handle(2, ClientMessage::JoinRoom { room_id, pin: None });
        joined(&mut guest);
        lobby.handle(1, ClientMessage::StartMatch);
        lobby.disconnect(1);
        lobby.handle(2, ClientMessage::LeaveRoom);
        let (mut returning, _) = client(&mut lobby, 3);
        lobby.handle(
            3,
            ClientMessage::Reconnect {
                credential: host_token,
            },
        );
        assert!(drain(&mut returning).iter().any(|message| matches!(
            message, ServerMessage::MatchEnded { reason } if reason.contains("disconnected")
        )));
    }

    #[test]
    fn room_names_are_unique_and_capacity_is_seven() {
        let mut lobby = Lobby::default();
        let mut names = std::collections::HashSet::new();
        let mut first_room = None;
        for id in 1..=7 {
            let (mut host, _) = client(&mut lobby, id);
            lobby.handle(id, ClientMessage::CreateRoom { pin: None });
            let (room_id, _, _) = joined(&mut host);
            let room = &lobby.rooms[&room_id];
            assert!(names.insert(room.base_name));
            assert!(room.name.ends_with(room.base_name));
            if id == 1 {
                first_room = Some((room_id, room.base_name));
            }
        }
        let (mut extra, _) = client(&mut lobby, 8);
        lobby.handle(8, ClientMessage::CreateRoom { pin: None });
        assert_eq!(lobby.rooms.len(), 7);
        assert!(drain(&mut extra).iter().any(|message| matches!(
            message, ServerMessage::Error { message } if message.contains("capacity")
        )));

        let (old_id, released_name) = first_room.unwrap();
        lobby.handle(1, ClientMessage::LeaveRoom);
        assert!(!lobby.rooms.contains_key(&old_id));
        assert!(names.remove(released_name));
        lobby.handle(8, ClientMessage::CreateRoom { pin: None });
        let (new_id, _, _) = joined(&mut extra);
        assert!(names.insert(lobby.rooms[&new_id].base_name));
        assert_eq!(lobby.rooms.len(), 7);
    }

    #[test]
    fn joined_and_reconnected_players_see_the_same_name_as_the_room_list() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, room_name, credential) = drain(&mut host)
            .into_iter()
            .find_map(|message| match message {
                ServerMessage::Joined {
                    room_id,
                    room_name,
                    credential,
                    ..
                } => Some((room_id, room_name, credential)),
                _ => None,
            })
            .unwrap();
        assert_eq!(lobby.room_info()[0].name, room_name);
        lobby.disconnect(1);
        let (mut returning, _) = client(&mut lobby, 2);
        lobby.handle(2, ClientMessage::Reconnect { credential });
        assert!(drain(&mut returning).iter().any(|message| matches!(
            message, ServerMessage::Joined { room_id: id, room_name: name, .. }
            if id == &room_id && name == &room_name
        )));
    }

    #[test]
    fn rooms_with_no_online_players_are_hidden_until_a_player_reconnects() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut guest, _) = client(&mut lobby, 2);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, host_token, _) = joined(&mut host);
        lobby.handle(
            2,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                pin: None,
            },
        );
        joined(&mut guest);

        lobby.disconnect(1);
        assert_eq!(lobby.room_info().len(), 1);
        lobby.disconnect(2);
        assert!(lobby.room_info().is_empty());
        assert!(lobby.rooms.contains_key(&room_id));

        let (mut returning, _) = client(&mut lobby, 3);
        lobby.handle(
            3,
            ClientMessage::Reconnect {
                credential: host_token,
            },
        );
        assert_eq!(joined(&mut returning).0, room_id);
        assert_eq!(lobby.room_info()[0].id, room_id);
    }

    #[test]
    fn two_players_join_disconnect_and_reconnect_without_leaking_credentials() {
        let mut lobby = Lobby::default();
        let (mut host, mut old_host_stop) = client(&mut lobby, 1);
        let (mut guest, _) = client(&mut lobby, 2);
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, host_token, seat) = joined(&mut host);
        assert_eq!(seat, Seat::Host);
        lobby.handle(
            2,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                pin: None,
            },
        );
        let (_, guest_token, seat) = joined(&mut guest);
        assert_eq!(seat, Seat::Guest);
        assert_ne!(host_token, guest_token);
        assert!(drain(&mut host).iter().all(|message| {
            !serde_json::to_string(message)
                .unwrap()
                .contains(&guest_token)
        }));
        lobby.disconnect(2);
        assert!(drain(&mut host).iter().any(|message| matches!(
            message,
            ServerMessage::RoomStatus {
                occupancy: 2,
                opponent_online: false,
                ..
            }
        )));
        let (mut reconnecting, _) = client(&mut lobby, 3);
        lobby.handle(
            3,
            ClientMessage::Reconnect {
                credential: guest_token,
            },
        );
        assert_eq!(joined(&mut reconnecting).2, Seat::Guest);
        assert!(drain(&mut host).iter().any(|message| matches!(
            message,
            ServerMessage::RoomStatus {
                occupancy: 2,
                opponent_online: true,
                ..
            }
        )));
        let (mut replacement, _) = client(&mut lobby, 4);
        lobby.handle(
            4,
            ClientMessage::Reconnect {
                credential: host_token,
            },
        );
        assert_eq!(joined(&mut replacement).2, Seat::Host);
        assert!(*old_host_stop.borrow_and_update());
        lobby.disconnect(1);
        assert_eq!(lobby.rooms[&room_id].host.connection, Some(4));
    }

    #[test]
    fn pin_failures_are_shared_and_the_fifth_closes_the_room() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut visitor, _) = client(&mut lobby, 2);
        let (mut visitor_two, _) = client(&mut lobby, 3);
        lobby.handle(
            1,
            ClientMessage::CreateRoom {
                pin: Some("012".into()),
            },
        );
        let (room_id, _, _) = joined(&mut host);
        for id in [2, 3, 2, 3] {
            lobby.handle(
                id,
                ClientMessage::JoinRoom {
                    room_id: room_id.clone(),
                    pin: Some("999".into()),
                },
            );
        }
        assert!(lobby.rooms.contains_key(&room_id));
        lobby.handle(
            2,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                pin: Some("999".into()),
            },
        );
        assert!(!lobby.rooms.contains_key(&room_id));
        assert!(
            drain(&mut host)
                .iter()
                .any(|message| matches!(message, ServerMessage::RoomClosed { .. }))
        );
        assert!(drain(&mut visitor).iter().any(|message| matches!(message, ServerMessage::Error { message } if message.contains("closed"))));
        assert!(
            !drain(&mut visitor_two)
                .iter()
                .any(|message| matches!(message, ServerMessage::Joined { .. }))
        );
    }

    #[test]
    fn disconnected_host_is_notified_of_pin_closure_on_reconnect() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        let (mut visitor, _) = client(&mut lobby, 2);
        lobby.handle(
            1,
            ClientMessage::CreateRoom {
                pin: Some("123".into()),
            },
        );
        let (room_id, credential, _) = joined(&mut host);
        lobby.disconnect(1);
        for _ in 0..5 {
            lobby.handle(
                2,
                ClientMessage::JoinRoom {
                    room_id: room_id.clone(),
                    pin: None,
                },
            );
        }
        drain(&mut visitor);
        let (mut returning, _) = client(&mut lobby, 3);
        lobby.handle(3, ClientMessage::Reconnect { credential });
        assert!(drain(&mut returning).iter().any(|message| matches!(
            message, ServerMessage::RoomClosed { reason } if reason.contains("PIN")
        )));
    }

    #[test]
    fn invalid_pins_and_expired_credentials_are_rejected() {
        let mut lobby = Lobby::default();
        let (mut host, _) = client(&mut lobby, 1);
        lobby.handle(
            1,
            ClientMessage::CreateRoom {
                pin: Some("12x".into()),
            },
        );
        assert!(lobby.rooms.is_empty());
        assert!(
            drain(&mut host)
                .iter()
                .any(|message| matches!(message, ServerMessage::Error { .. }))
        );
        lobby.handle(1, ClientMessage::CreateRoom { pin: None });
        let (room_id, credential, _) = joined(&mut host);
        lobby.disconnect(1);
        lobby.rooms.get_mut(&room_id).unwrap().host.disconnected_at =
            Some(Instant::now() - RECONNECT_GRACE);
        let (mut returning, _) = client(&mut lobby, 2);
        lobby.handle(2, ClientMessage::Reconnect { credential });
        assert!(
            drain(&mut returning)
                .iter()
                .any(|message| matches!(message, ServerMessage::Error { .. }))
        );
        lobby.expire();
        assert!(!lobby.rooms.contains_key(&room_id));
    }
}
