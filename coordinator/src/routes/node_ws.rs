use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    http::HeaderMap,
    response::IntoResponse,
};
use common::{
    auth::hash_token,
    models::node::{Node, NodeStatus},
    ws::{EventPayload, NodeCameraConfig, SegmentPayload, SegmentMigratedPayload, WsMessage},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

pub async fn node_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    match authenticate(&state, &headers).await {
        Ok(node_id) => {
            // Honour a reverse-proxy `X-Forwarded-For` if present; otherwise use the
            // TCP peer address.  The stored IP is used by the coordinator to proxy
            // segment/export requests to the node's playback HTTP server on port 8890.
            let ip = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| peer.ip().to_string());

            ws.on_upgrade(move |socket| handle_socket(socket, state, node_id, ip))
        }
        Err(e) => e.into_response(),
    }
}

async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<Uuid, AppError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let key_hash = hash_token(api_key);
    let node = sqlx::query_as::<_, Node>(
        "SELECT id, name, api_key_hash, status, last_seen_at,
                CAST(ip_addr AS TEXT) AS ip_addr, version, metadata, created_at, updated_at
         FROM nodes WHERE api_key_hash = $1",
    )
    .bind(&key_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Sqlx)?
    .ok_or(AppError::Unauthorized)?;

    if node.status == NodeStatus::Rejected {
        return Err(AppError::Forbidden);
    }
    Ok(node.id)
}

async fn fetch_node_cameras(state: &AppState, node_id: Uuid) -> Vec<NodeCameraConfig> {
    sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool,
                          Option<String>, Option<String>, Option<String>, bool)>(
        "SELECT id, name, rtsp_url, stream_path, enabled,
                onvif_url, onvif_username, onvif_password, motion_detection
           FROM cameras WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, name, rtsp_url, stream_path, enabled,
           onvif_url, onvif_username, onvif_password, motion_detection)| NodeCameraConfig {
        id,
        name,
        rtsp_url,
        stream_path: stream_path.unwrap_or_default(),
        enabled,
        onvif_url,
        onvif_username,
        onvif_password,
        motion_detection,
    })
    .collect()
}

async fn handle_socket(socket: WebSocket, state: AppState, node_id: Uuid, ip: String) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut nodes = state.nodes.write().await;
        nodes.insert(node_id, tx);
    }

    let cameras = fetch_node_cameras(&state, node_id).await;
    if let Ok(text) = serde_json::to_string(&WsMessage::config_sync(cameras)) {
        let _ = sink.send(Message::Text(text)).await;
    }

    // Persist last-seen timestamp and the node's current IP address.
    // The IP is used later by the playback proxy to reach this node's HTTP server.
    let _ = sqlx::query(
        "UPDATE nodes
            SET last_seen_at = NOW(),
                ip_addr      = $2::inet,
                updated_at   = NOW()
          WHERE id = $1",
    )
    .bind(node_id)
    .bind(&ip)
    .execute(&state.db)
    .await;

    state
        .broadcast_dashboard(&serde_json::json!({
            "type": "node_status_update",
            "node_id": node_id,
            "online": true,
        }))
        .await;

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_incoming(&state, node_id, &text).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            Some(outgoing) = rx.recv() => {
                if sink.send(Message::Text(outgoing)).await.is_err() {
                    break;
                }
            }
        }
    }

    {
        let mut nodes = state.nodes.write().await;
        nodes.remove(&node_id);
    }
    state
        .broadcast_dashboard(&serde_json::json!({
            "type": "node_status_update",
            "node_id": node_id,
            "online": false,
        }))
        .await;
    tracing::info!("node {node_id} disconnected");
}

async fn handle_incoming(state: &AppState, node_id: Uuid, text: &str) {
    let msg: WsMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => return,
    };

    match msg {
        WsMessage::Heartbeat { payload, .. } => {
            let metadata = serde_json::json!({
                "hardware": payload.hardware,
                "cameras":  payload.cameras,
            });
            let _ = sqlx::query(
                "UPDATE nodes SET last_seen_at = NOW(), metadata = $2, version = $3, updated_at = NOW()
                 WHERE id = $1",
            )
            .bind(node_id)
            .bind(metadata)
            .bind(payload.mediamtx_version)
            .execute(&state.db)
            .await;

            // Forward camera statuses to all connected dashboard browsers
            let dash = serde_json::json!({
                "type": "camera_status_update",
                "node_id": node_id,
                "cameras": payload.cameras.iter().map(|s| serde_json::json!({
                    "camera_id": s.camera_id,
                    "stream_path": s.stream_path,
                    "connected": s.connected,
                    "readers": s.readers,
                })).collect::<Vec<_>>()
            });
            state.broadcast_dashboard(&dash).await;
        }

        WsMessage::SegmentComplete { payload, .. } => {
            index_segment(state, node_id, payload).await;
        }

        WsMessage::MotionEvent { payload, .. } => {
            index_event(state, node_id, payload).await;
        }

        WsMessage::SegmentMigrated { payload, .. } => {
            update_segment_uri(state, payload).await;
        }

        WsMessage::Ping { seq } => {
            state.push_to_node(node_id, WsMessage::Pong { seq }).await;
        }

        _ => {}
    }
}

/// Update the storage_uri for a recording that has been migrated to S3.
/// Uses `started_at` in the WHERE clause for partition-efficient updates.
async fn update_segment_uri(state: &AppState, p: SegmentMigratedPayload) {
    let res = sqlx::query(
        "UPDATE recordings
            SET storage_uri = $3
          WHERE id = $1 AND started_at = $2",
    )
    .bind(p.recording_id)
    .bind(p.started_at)
    .bind(&p.storage_uri)
    .execute(&state.db)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::debug!("recording {} migrated → {}", p.recording_id, p.storage_uri);
        }
        Ok(_) => {
            tracing::warn!("segment_migrated: no row found for recording {}", p.recording_id);
        }
        Err(e) => {
            tracing::error!("segment_migrated DB error ({}): {e}", p.recording_id);
        }
    }
}

async fn index_event(state: &AppState, node_id: Uuid, evt: EventPayload) {
    let res = sqlx::query(
        "INSERT INTO events (camera_id, node_id, type, payload, occurred_at)
         VALUES ($1, $2, 'motion', $3, $4)",
    )
    .bind(evt.camera_id)
    .bind(node_id)
    .bind(serde_json::json!({ "source": evt.source, "score": evt.score }))
    .bind(evt.occurred_at)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => {
            let dash = serde_json::json!({
                "type":        "motion_event",
                "camera_id":   evt.camera_id,
                "occurred_at": evt.occurred_at,
                "source":      evt.source,
                "score":       evt.score,
            });
            state.broadcast_dashboard(&dash).await;
        }
        Err(e) => tracing::error!("failed to index event (camera {}): {e}", evt.camera_id),
    }
}

async fn index_segment(state: &AppState, node_id: Uuid, seg: SegmentPayload) {
    // Resolve retention_policy_id: use camera's policy, fall back to default
    let retention_id: Option<Uuid> = sqlx::query_as::<_, (Option<Uuid>,)>(
        "SELECT COALESCE(c.retention_policy_id, rp.id)
         FROM cameras c
         LEFT JOIN retention_policies rp ON rp.is_default = true
         WHERE c.id = $1
         LIMIT 1",
    )
    .bind(seg.camera_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|(id,)| id);

    let res = sqlx::query(
        "INSERT INTO recordings
             (id, camera_id, node_id, storage_uri, started_at, ended_at,
              duration_secs, size_bytes, retention_policy_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (id, started_at) DO NOTHING",
    )
    .bind(seg.id)
    .bind(seg.camera_id)
    .bind(node_id)
    .bind(&seg.storage_uri)
    .bind(seg.started_at)
    .bind(seg.ended_at)
    .bind(seg.duration_secs)
    .bind(seg.size_bytes)
    .bind(retention_id)
    .execute(&state.db)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::debug!("indexed segment {} (camera {})", seg.id, seg.camera_id);
        }
        Ok(_) => {
            tracing::debug!("duplicate segment {} ignored", seg.id);
        }
        Err(e) => {
            tracing::error!("failed to index segment {}: {e}", seg.id);
        }
    }
}
