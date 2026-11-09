use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    ListRooms,
    CreateRoom {
        pin: Option<String>,
    },
    JoinRoom {
        room_id: String,
        pin: Option<String>,
    },
    Reconnect {
        credential: String,
    },
    LeaveRoom,
    StartMatch,
    GameAction {
        revision: u64,
        action: GameAction,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum GameAction {
    Move { x: i32, y: i32 },
    Place { ingredient: usize, x: i32, y: i32 },
    Pickup,
    Rest,
    Undo,
    ToggleHover,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoundEffect {
    Pop,
    Whoosh,
    Water,
    Chime,
    Ice,
    Electric,
    Sizzle,
    Explosion,
    Stone,
    Organic,
    Impact,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualEffect {
    Water,
    Acid,
    Poison,
    Fuel,
    Freeze,
    Electric,
    RidgeDust,
    Void,
    Steam,
    Explosion,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnimationEvent {
    pub at_ms: u32,
    #[serde(flatten)]
    pub animation: Animation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "animation", rename_all = "snake_case")]
pub enum Animation {
    MovePlayer {
        seat: Seat,
        x: i32,
        y: i32,
    },
    Throw {
        ingredient: usize,
        from: (i32, i32),
        to: (i32, i32),
        corner: bool,
    },
    Effect {
        effect: VisualEffect,
        x: i32,
        y: i32,
    },
    Serpents {
        visuals: Option<Vec<SerpentView>>,
        impact: Option<(i32, i32)>,
    },
    Aquatic {
        visuals: Option<Vec<AquaticAnimationView>>,
    },
    Weather {
        kind: WeatherEffect,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeatherEffect {
    Rain,
    Earthquake,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AquaticAnimationView {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub lengths: [usize; 4],
    pub sweep_frame: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Solid,
    Water,
    Ridge,
    Hole,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Growth {
    Void,
    Stone,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TileView {
    pub terrain: Terrain,
    pub wet: u8,
    pub oil: bool,
    pub frozen: bool,
    pub poison: bool,
    pub acid: bool,
    pub electrified: bool,
    pub life: bool,
    pub thorns: bool,
    pub growth: Option<Growth>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerView {
    pub x: i32,
    pub y: i32,
    pub health: u8,
    pub energy: u8,
    pub magic: u8,
    pub hovering: bool,
    pub frozen: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemView {
    pub ingredient: usize,
    pub x: i32,
    pub y: i32,
    pub placed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SerpentView {
    pub cells: Vec<(i32, i32)>,
    pub venomous: bool,
    pub straight_frame: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AquaticView {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub lengths: [usize; 4],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MatchView {
    pub active: Seat,
    pub actions_used: u8,
    pub can_undo: bool,
    pub wind: String,
    pub players: [PlayerView; 2],
    pub own_inventory: [usize; 14],
    pub tiles: Vec<TileView>,
    pub items: Vec<ItemView>,
    pub pending_item: Option<ItemView>,
    pub serpents: Vec<SerpentView>,
    pub aquatic: Vec<AquaticView>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub locked: bool,
    pub occupancy: u8,
    pub online: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Seat {
    Host,
    Guest,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub seat: Option<Seat>,
    pub text: String,
    pub item: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TurnClock {
    pub remaining_ms: u32,
    pub starts_in_ms: u32,
    pub running: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Rooms {
        rooms: Vec<RoomInfo>,
    },
    Joined {
        room_id: String,
        room_name: String,
        credential: String,
        seat: Seat,
    },
    RoomStatus {
        room_id: String,
        occupancy: u8,
        opponent_online: bool,
    },
    RoomClosed {
        reason: String,
    },
    LeftRoom,
    MatchState {
        room_id: String,
        revision: u64,
        state: MatchView,
        #[serde(default)]
        sounds: Vec<SoundEffect>,
        #[serde(default)]
        animations: Vec<AnimationEvent>,
        #[serde(default)]
        turn_clock: Option<TurnClock>,
        #[serde(default)]
        history_start: usize,
        #[serde(default)]
        history: Vec<HistoryEntry>,
    },
    MatchEnded {
        reason: String,
    },
    Error {
        message: String,
    },
}
