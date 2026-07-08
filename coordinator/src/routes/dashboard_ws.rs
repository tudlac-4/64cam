use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

/// `GET /api/v1/ws/dashboard?token=<jwt>`
///
/// Browser WebSocket — cannot set `Authorization` header, so JWT is passed as a
/// query param. Only sends outbound messages (status updates); browser sends nothing.
pub async fn dashboard_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
) -> impl IntoResponse {
    // Validate JWT
    match state.jwt.decode(&q.token) {
        Ok(_) => ws.on_upgrade(move |socket| handle_dashboard(socket, state)),
        Err(_) => AppError::Unauthorized.into_response(),
    }
}

async fn handle_dashboard(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let session_id = Uuid::new_v4();

    {
        let mut dashboards = state.dashboards.write().await;
        dashboards.insert(session_id, tx);
    }

    // Send current node/camera status snapshot on connect
    if let Ok(snapshot) = build_snapshot(&state).await {
        if let Ok(text) = serde_json::to_string(&snapshot) {
            let _ = sink.send(Message::Text(text)).await;
        }
    }

    loop {
        tokio::select! {
            // Outbound: coordinator pushes status updates
            Some(msg) = rx.recv() => {
                if sink.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            // Inbound: browser only sends pings / close frames; we don't act on anything else
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    let mut dashboards = state.dashboards.write().await;
    dashboards.remove(&session_id);
}

/// Initial snapshot sent when a dashboard client connects.
#[derive(serde::Serialize)]
struct SnapshotMessage {
    #[serde(rename = "type")]
    msg_type: &'static str,
    nodes:    Vec<NodeSnapshot>,
}

#[derive(serde::Serialize, sqlx::FromRow)]
struct NodeSnapshot {
    id:           Uuid,
    name:         String,
    status:       String,
    last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn build_snapshot(state: &AppState) -> anyhow::Result<SnapshotMessage> {
    let nodes = sqlx::query_as::<_, NodeSnapshot>(
        "SELECT id, name, status::TEXT AS status, last_seen_at FROM nodes ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(SnapshotMessage { msg_type: "snapshot", nodes })
}
