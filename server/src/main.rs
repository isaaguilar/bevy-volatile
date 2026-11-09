mod lobby;

use std::{
    env,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header::ORIGIN},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use lobby::{Client, Lobby};
use tokio::sync::{Mutex, Semaphore, mpsc, watch};
use volatile_online_protocol::{ClientMessage, ServerMessage};

const MAX_CONNECTIONS: usize = 128;
const MAX_MESSAGE_BYTES: usize = 4096;

#[derive(Clone)]
struct AppState {
    lobby: Arc<Mutex<Lobby>>,
    next_id: Arc<AtomicU64>,
    connections: Arc<Semaphore>,
    allowed_origins: Vec<String>,
}

fn parse_allowed_origins(value: &str) -> Result<Vec<String>, &'static str> {
    let origins: Vec<_> = value.split(',').map(str::trim).map(str::to_owned).collect();
    if origins.iter().any(String::is_empty) {
        return Err("VOLATILE_ALLOWED_ORIGIN must contain nonempty origins");
    }
    Ok(origins)
}

async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| {
            state
                .allowed_origins
                .iter()
                .any(|allowed| allowed == origin)
        })
    {
        return (StatusCode::FORBIDDEN, "Origin not allowed").into_response();
    }
    let Ok(permit) = state.connections.clone().try_acquire_owned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Server connection capacity reached",
        )
            .into_response();
    };
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            serve_connection(socket, state).await;
        })
}

async fn serve_connection(socket: WebSocket, state: AppState) {
    let id = state.next_id.fetch_add(1, Ordering::Relaxed);
    let (outgoing, mut messages) = mpsc::channel::<ServerMessage>(64);
    let (stop, mut stopped) = watch::channel(false);
    state
        .lobby
        .lock()
        .await
        .connect(id, Client { outgoing, stop });
    let (mut writer, mut reader) = socket.split();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    let mut last_seen = Instant::now();
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if last_seen.elapsed() >= Duration::from_secs(60) { break; }
                if !matches!(tokio::time::timeout(Duration::from_secs(10), writer.send(Message::Ping(Vec::new().into()))).await, Ok(Ok(()))) { break; }
            }
            changed = stopped.changed() => {
                if changed.is_err() || *stopped.borrow() { break; }
            }
            outgoing = messages.recv() => {
                let Some(outgoing) = outgoing else { break };
                let text = match serde_json::to_string(&outgoing) {
                    Ok(text) => text,
                    Err(error) => {
                        eprintln!("could not serialize server message: {error}");
                        break;
                    }
                };
                if !matches!(tokio::time::timeout(Duration::from_secs(10), writer.send(Message::Text(text.into()))).await, Ok(Ok(()))) { break; }
            }
            incoming = reader.next() => {
                last_seen = Instant::now();
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(message) => {
                                let lobby = state.lobby.clone();
                                if let Err(error) = tokio::task::spawn_blocking(move || {
                                    lobby.blocking_lock().handle(id, message);
                                }).await {
                                    eprintln!("lobby request failed: {error}");
                                    break;
                                }
                            },
                            Err(_) => {
                                let message = ServerMessage::Error { message: "Invalid message".into() };
                                let text = match serde_json::to_string(&message) {
                                    Ok(text) => text,
                                    Err(error) => {
                                        eprintln!("could not serialize error message: {error}");
                                        break;
                                    }
                                };
                                if !matches!(tokio::time::timeout(Duration::from_secs(10), writer.send(Message::Text(text.into()))).await, Ok(Ok(()))) { break; }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(Message::Binary(_))) => break,
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                }
            }
        }
    }
    state.lobby.lock().await.disconnect(id);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = env::var("VOLATILE_SERVER_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let allowed_origins = parse_allowed_origins(
        &env::var("VOLATILE_ALLOWED_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8000".into()),
    )?;
    if !matches!(address.ip(), IpAddr::V4(ip) if ip.is_loopback())
        && !matches!(address.ip(), IpAddr::V6(ip) if ip.is_loopback())
    {
        return Err(
            "The server must bind to loopback; use a TLS reverse proxy for remote access".into(),
        );
    }
    let state = AppState {
        lobby: Arc::new(Mutex::new(Lobby::default())),
        next_id: Arc::new(AtomicU64::new(1)),
        connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        allowed_origins,
    };
    let lobby = state.lobby.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            lobby.lock().await.expire();
        }
    });
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("volatile lobby listening on {address}");
    let app = Router::new().route("/ws", get(upgrade)).with_state(state);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message as ClientFrame, client::IntoClientRequest, http::HeaderValue},
    };

    #[test]
    fn origin_list_rejects_empty_entries() {
        assert_eq!(
            parse_allowed_origins("http://127.0.0.1:8000, https://game.example").unwrap(),
            vec!["http://127.0.0.1:8000", "https://game.example"]
        );
        for value in ["", "  ", ",https://game.example", "https://game.example,"] {
            assert!(parse_allowed_origins(value).is_err());
        }
    }

    #[tokio::test]
    async fn websocket_clients_create_join_and_reconnect() {
        let state = AppState {
            lobby: Arc::new(Mutex::new(Lobby::default())),
            next_id: Arc::new(AtomicU64::new(1)),
            connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            allowed_origins: parse_allowed_origins("http://127.0.0.1:8000,https://game.example")
                .unwrap(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route("/ws", get(upgrade)).with_state(state);
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let url = format!("ws://{address}/ws");

        for origin in [
            "https://untrusted.example",
            "https://game.example.attacker.invalid",
        ] {
            let mut bad_request = url.clone().into_client_request().unwrap();
            bad_request
                .headers_mut()
                .insert(ORIGIN, HeaderValue::from_static(origin));
            assert!(connect_async(bad_request).await.is_err());
        }
        assert!(connect_async(url.clone()).await.is_err());
        let request = |origin: &'static str| {
            let mut request = url.clone().into_client_request().unwrap();
            request
                .headers_mut()
                .insert(ORIGIN, HeaderValue::from_static(origin));
            request
        };
        let (mut host, _) = connect_async(request("http://127.0.0.1:8000"))
            .await
            .unwrap();
        let (mut guest, _) = connect_async(request("https://game.example"))
            .await
            .unwrap();
        host.send(ClientFrame::Text(
            r#"{"type":"create_room","pin":null}"#.into(),
        ))
        .await
        .unwrap();
        let (room_id, host_credential) = loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), host.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let ClientFrame::Text(text) = frame
                && let ServerMessage::Joined {
                    room_id,
                    credential,
                    seat: volatile_online_protocol::Seat::Host,
                    ..
                } = serde_json::from_str(&text).unwrap()
            {
                break (room_id, credential);
            }
        };
        let join = serde_json::to_string(&ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            pin: None,
        })
        .unwrap();
        guest.send(ClientFrame::Text(join.into())).await.unwrap();
        let guest_credential = loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), guest.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let ClientFrame::Text(text) = frame
                && let ServerMessage::Joined {
                    credential,
                    seat: volatile_online_protocol::Seat::Guest,
                    ..
                } = serde_json::from_str(&text).unwrap()
            {
                break credential;
            }
        };
        assert_ne!(guest_credential, host_credential);
        host.send(ClientFrame::Text(
            serde_json::to_string(&ClientMessage::StartMatch)
                .unwrap()
                .into(),
        ))
        .await
        .unwrap();
        for socket in [&mut host, &mut guest] {
            loop {
                let frame = tokio::time::timeout(Duration::from_secs(3), socket.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                if let ClientFrame::Text(text) = frame
                    && matches!(
                        serde_json::from_str::<ServerMessage>(&text).unwrap(),
                        ServerMessage::MatchState { revision: 0, .. }
                    )
                {
                    break;
                }
            }
        }
        host.send(ClientFrame::Text(
            serde_json::to_string(&ClientMessage::GameAction {
                revision: 0,
                action: volatile_online_protocol::GameAction::Rest,
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
        for socket in [&mut host, &mut guest] {
            loop {
                let frame = tokio::time::timeout(Duration::from_secs(3), socket.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                if let ClientFrame::Text(text) = frame
                    && matches!(serde_json::from_str::<ServerMessage>(&text).unwrap(),
                        ServerMessage::MatchState { revision: 1, state, .. }
                            if state.active == volatile_online_protocol::Seat::Guest)
                {
                    break;
                }
            }
        }
        guest.close(None).await.unwrap();
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), host.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let ClientFrame::Text(text) = frame
                && matches!(
                    serde_json::from_str::<ServerMessage>(&text).unwrap(),
                    ServerMessage::RoomStatus {
                        occupancy: 2,
                        opponent_online: false,
                        ..
                    }
                )
            {
                break;
            }
        }
        let (mut returning, _) = connect_async(request("https://game.example"))
            .await
            .unwrap();
        let reconnect = serde_json::to_string(&ClientMessage::Reconnect {
            credential: guest_credential,
        })
        .unwrap();
        returning
            .send(ClientFrame::Text(reconnect.into()))
            .await
            .unwrap();
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), returning.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let ClientFrame::Text(text) = frame
                && matches!(serde_json::from_str::<ServerMessage>(&text).unwrap(), ServerMessage::Joined { room_id: id, seat: volatile_online_protocol::Seat::Guest, .. } if id == room_id)
            {
                break;
            }
        }
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), returning.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let ClientFrame::Text(text) = frame
                && matches!(serde_json::from_str::<ServerMessage>(&text).unwrap(),
                    ServerMessage::MatchState { revision: 1, state, .. }
                        if state.active == volatile_online_protocol::Seat::Guest)
            {
                break;
            }
        }
        server.abort();
    }
}
