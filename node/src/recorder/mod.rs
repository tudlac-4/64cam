pub mod queue;
pub mod watchdog;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{NaiveDateTime, TimeZone, Utc};
use common::ws::{NodeCameraConfig, SegmentPayload};
use notify::{
    event::{AccessKind, AccessMode, CreateKind},
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use self::queue::SegmentQueue;

const SEGMENT_SECS: i64 = 60;

/// Watches `recordings_dir` for completed fMP4 segments, appends them to the durable
/// JSONL queue, and signals `flush_tx` so the WS client can backfill immediately.
pub async fn run(
    recordings_dir: PathBuf,
    cameras: Arc<RwLock<Vec<NodeCameraConfig>>>,
    node_id: Uuid,
    queue: Arc<SegmentQueue>,
    flush_tx: mpsc::UnboundedSender<()>,
) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(&recordings_dir).await?;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(256);

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| {
            let _ = event_tx.blocking_send(result);
        },
        Config::default(),
    )?;
    watcher.watch(&recordings_dir, RecursiveMode::Recursive)?;
    tracing::info!("recording watcher active: {}", recordings_dir.display());

    while let Some(result) = event_rx.recv().await {
        match result {
            Ok(event) if is_segment_complete(&event) => {
                for path in &event.paths {
                    if path.extension().and_then(|e| e.to_str()) == Some("mp4") {
                        if let Err(e) =
                            handle_segment(path, &recordings_dir, &cameras, node_id, &queue, &flush_tx)
                                .await
                        {
                            tracing::warn!("segment handling error for {}: {e}", path.display());
                        }
                    }
                }
            }
            Err(e) => tracing::error!("fs watch error: {e}"),
            _ => {}
        }
    }
    Ok(())
}

fn is_segment_complete(event: &notify::Event) -> bool {
    matches!(
        event.kind,
        // inotify CLOSE_WRITE — fires when MediaMTX closes the file after finishing a segment
        EventKind::Access(AccessKind::Close(AccessMode::Write))
        // kqueue / FSEvents fallback on macOS (dev machines)
        | EventKind::Create(CreateKind::File)
    )
}

async fn handle_segment(
    path: &Path,
    recordings_dir: &Path,
    cameras: &RwLock<Vec<NodeCameraConfig>>,
    _node_id: Uuid,
    queue: &SegmentQueue,
    flush_tx: &mpsc::UnboundedSender<()>,
) -> anyhow::Result<()> {
    // First path component after recordings_dir is the stream_path
    let rel = path.strip_prefix(recordings_dir)?;
    let stream_path = rel
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot extract stream_path from {}", path.display()))?
        .to_owned();

    // Filename stem encodes start time: YYYYMMDD_HHMMSS (set by MediaMTX recordPath format)
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("bad segment filename: {}", path.display()))?;

    let naive = NaiveDateTime::parse_from_str(stem, "%Y%m%d_%H%M%S")
        .map_err(|e| anyhow::anyhow!("cannot parse timestamp '{stem}': {e}"))?;

    let started_at = Utc.from_utc_datetime(&naive);
    let ended_at   = started_at + chrono::Duration::seconds(SEGMENT_SECS);

    let size_bytes = tokio::fs::metadata(path).await.map(|m| m.len() as i64).unwrap_or(0);

    let camera_id = {
        let list = cameras.read().await;
        list.iter().find(|c| c.stream_path == stream_path).map(|c| c.id)
    };
    let Some(camera_id) = camera_id else {
        tracing::debug!("segment for unknown stream_path '{stream_path}' — skipping");
        return Ok(());
    };

    // Relative URI: local://{stream_path}/{filename}.mp4
    // The coordinator strips "local://" to build the redirect URL to the node's
    // playback HTTP server.  Using a relative path keeps node-local absolute paths
    // out of the database and makes the coordinator topology-agnostic.
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(stem);
    let storage_uri = format!("local://{stream_path}/{filename}");
    // Deterministic UUIDv5 from the URI so duplicate events produce the same ID
    let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, storage_uri.as_bytes());

    let payload = SegmentPayload {
        id,
        camera_id,
        stream_path,
        storage_uri,
        started_at,
        ended_at,
        duration_secs: SEGMENT_SECS as i32,
        size_bytes,
    };

    queue.push(&payload).await?;
    let _ = flush_tx.send(());
    tracing::debug!("queued segment {id}");
    Ok(())
}
