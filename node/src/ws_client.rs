use std::sync::Arc;

use common::ws::{
    CameraStatus, EventPayload, HeartbeatPayload, HardwareProfile, NodeCameraConfig,
    SegmentMigratedPayload, WsMessage,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    config::NodeConfig,
    identity::NodeIdentity,
    recorder::queue::SegmentQueue,
};

pub async fn run(
    config: &NodeConfig,
    identity: &NodeIdentity,
    hardware: HardwareProfile,
    mediamtx_config_tx: mpsc::UnboundedSender<Vec<NodeCameraConfig>>,
    cameras: Arc<RwLock<Vec<NodeCameraConfig>>>,
    queue: Arc<SegmentQueue>,
    mut flush_rx:   mpsc::UnboundedReceiver<()>,
    mut event_rx:   mpsc::UnboundedReceiver<EventPayload>,
    mut migrate_rx: mpsc::UnboundedReceiver<SegmentMigratedPayload>,
) {
    let ws_url = config
        .coordinator_url
        .replace("https://", "wss://")
        .replace("http://", "ws://")
        + "/api/v1/ws/node";

    loop {
        tracing::info!("connecting to coordinator: {ws_url}");
        match connect_and_run(
            &ws_url,
            identity,
            &hardware,
            &mediamtx_config_tx,
            &cameras,
            &queue,
            &mut flush_rx,
            &mut event_rx,
            &mut migrate_rx,
            config,
        )
        .await
        {
            Ok(_) => tracing::info!("WS disconnected cleanly"),
            Err(e) => tracing::warn!("WS error: {e}"),
        }
        tracing::info!("reconnecting in 5s");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn connect_and_run(
    ws_url: &str,
    identity: &NodeIdentity,
    hardware: &HardwareProfile,
    mediamtx_config_tx: &mpsc::UnboundedSender<Vec<NodeCameraConfig>>,
    cameras: &Arc<RwLock<Vec<NodeCameraConfig>>>,
    queue: &Arc<SegmentQueue>,
    flush_rx:   &mut mpsc::UnboundedReceiver<()>,
    event_rx:   &mut mpsc::UnboundedReceiver<EventPayload>,
    migrate_rx: &mut mpsc::UnboundedReceiver<SegmentMigratedPayload>,
    cfg: &NodeConfig,
) -> anyhow::Result<()> {
    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(ws_url)
        .header("x-api-key", &identity.api_key)
        .body(())?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request).await?;
    tracing::info!("WS connected");

    let (mut sink, mut stream) = ws_stream.split();
    let mut heartbeat = tokio::time::interval(
        tokio::time::Duration::from_secs(cfg.heartbeat_secs),
    );
    let mut seq: u64 = 0;

    // Drain stale flush notifications from while we were disconnected
    while flush_rx.try_recv().is_ok() {}

    // Backfill: send all queued segments that accumulated while offline
    flush_queue(queue, &mut sink, &mut seq).await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                seq += 1;
                let cam_list = cameras.read().await.clone();
                let statuses = poll_mediamtx(cfg.mediamtx_api_port, &cam_list).await;
                let msg = WsMessage::Heartbeat {
                    seq,
                    payload: HeartbeatPayload {
                        node_id: identity.node_id,
                        hardware: hardware.clone(),
                        cameras: statuses,
                        mediamtx_version: None,
                    },
                };
                sink.send(Message::Text(serde_json::to_string(&msg)?)).await?;
            }

            // New segment ready — flush queue immediately
            Some(_) = flush_rx.recv() => {
                // Drain burst (multiple segments may arrive together)
                while flush_rx.try_recv().is_ok() {}
                flush_queue(queue, &mut sink, &mut seq).await;
            }

            // Motion event from ONVIF or diff detector — fire-and-forget, no queue
            Some(evt) = event_rx.recv() => {
                seq += 1;
                let msg = WsMessage::MotionEvent { seq, payload: evt };
                if let Ok(text) = serde_json::to_string(&msg) {
                    let _ = sink.send(Message::Text(text)).await;
                }
            }

            // Segment migrated to S3 — notify coordinator to update DB
            Some(payload) = migrate_rx.recv() => {
                seq += 1;
                let msg = WsMessage::SegmentMigrated { seq, payload };
                if let Ok(text) = serde_json::to_string(&msg) {
                    let _ = sink.send(Message::Text(text)).await;
                }
            }

            Some(raw) = stream.next() => {
                match raw? {
                    Message::Text(text) => {
                        handle_incoming(
                            &text,
                            mediamtx_config_tx,
                            cameras,
                            &mut sink,
                            &mut seq,
                            &cfg.recordings_dir(),
                        )
                        .await?;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn flush_queue<S>(
    queue: &SegmentQueue,
    sink: &mut S,
    seq: &mut u64,
) where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let entries = queue.read_all().await;
    if entries.is_empty() {
        return;
    }
    tracing::info!("backfilling {} queued segments", entries.len());
    let mut all_sent = true;
    for payload in entries {
        *seq += 1;
        let msg = WsMessage::SegmentComplete { seq: *seq, payload };
        if let Ok(text) = serde_json::to_string(&msg) {
            if sink.send(Message::Text(text)).await.is_err() {
                all_sent = false;
                break;
            }
        }
    }
    if all_sent {
        queue.clear().await;
    }
}

async fn handle_incoming<S>(
    text: &str,
    mediamtx_config_tx: &mpsc::UnboundedSender<Vec<NodeCameraConfig>>,
    cameras: &Arc<RwLock<Vec<NodeCameraConfig>>>,
    sink: &mut S,
    seq: &mut u64,
    recordings_dir: &std::path::Path,
) -> anyhow::Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let msg: WsMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    match msg {
        WsMessage::ConfigSync { payload, .. } => {
            let mut list = cameras.write().await;
            *list = payload.cameras.clone();
            let _ = mediamtx_config_tx.send(payload.cameras);
        }
        WsMessage::CameraAdded { payload, .. } => {
            let mut list = cameras.write().await;
            list.retain(|c| c.id != payload.id);
            list.push(payload.clone());
            let snap = list.clone();
            drop(list);
            let _ = mediamtx_config_tx.send(snap);
        }
        WsMessage::CameraUpdated { payload, .. } => {
            let mut list = cameras.write().await;
            if let Some(c) = list.iter_mut().find(|c| c.id == payload.id) {
                *c = payload;
            }
            let snap = list.clone();
            drop(list);
            let _ = mediamtx_config_tx.send(snap);
        }
        WsMessage::CameraRemoved { camera_id, .. } => {
            let mut list = cameras.write().await;
            list.retain(|c| c.id != camera_id);
            let snap = list.clone();
            drop(list);
            let _ = mediamtx_config_tx.send(snap);
        }
        WsMessage::DeleteRecording { storage_uri, .. } => {
            // storage_uri format: "local://stream_path/filename.mp4"
            // Resolve relative to recordings_dir so the node doesn't need to know
            // the absolute path that's stored in the DB.
            if let Some(rel) = storage_uri.strip_prefix("local://") {
                let path = recordings_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                match tokio::fs::remove_file(&path).await {
                    Ok(_)  => tracing::info!("deleted recording: {}", path.display()),
                    Err(e) => tracing::warn!("delete recording failed ({}): {e}", path.display()),
                }
            }
        }
        WsMessage::Ping { seq: s } => {
            *seq += 1;
            let pong = serde_json::to_string(&WsMessage::Pong { seq: s })?;
            sink.send(Message::Text(pong)).await?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Deserialize)]
struct MtxPathsResponse {
    items: Vec<MtxPath>,
}

#[derive(Deserialize)]
struct MtxPath {
    name:    String,
    ready:   bool,
    readers: Vec<serde_json::Value>,
}

async fn poll_mediamtx(api_port: u16, cameras: &[NodeCameraConfig]) -> Vec<CameraStatus> {
    let url = format!("http://127.0.0.1:{api_port}/v3/paths/list");
    let paths: Vec<MtxPath> = match reqwest::get(&url).await {
        Ok(r) => r.json::<MtxPathsResponse>().await.map(|r| r.items).unwrap_or_default(),
        Err(_) => vec![],
    };

    cameras
        .iter()
        .map(|cam| {
            let p = paths.iter().find(|p| p.name == cam.stream_path);
            CameraStatus {
                camera_id:   cam.id,
                stream_path: cam.stream_path.clone(),
                connected:   p.map(|p| p.ready).unwrap_or(false),
                readers:     p.map(|p| p.readers.len() as u32).unwrap_or(0),
            }
        })
        .collect()
}
