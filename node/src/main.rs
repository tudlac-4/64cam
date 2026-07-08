mod config;
mod events;
mod hardware;
mod identity;
mod mediamtx;
mod playback;
mod recorder;
mod registration;
mod storage;
mod ws_client;

use std::sync::Arc;

use anyhow::Result;
use config::NodeConfig;
use identity::NodeIdentity;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cfg = NodeConfig::from_env();
    std::fs::create_dir_all(&cfg.data_dir)?;

    // Load or register identity
    let identity = match NodeIdentity::load(&cfg.data_dir) {
        Some(id) => {
            tracing::info!("node identity loaded: {}", id.node_id);
            id
        }
        None => {
            tracing::info!("no identity found — registering with coordinator");
            let hw = hardware::detect();
            let id = registration::register(&cfg, hw).await?;
            id.save(&cfg.data_dir)?;
            tracing::info!(
                "registered as {} (status=pending, awaiting admin approval)",
                id.node_id
            );
            id
        }
    };

    let hw = hardware::detect();
    tracing::info!(
        cores    = hw.cpu_cores,
        ram_mb   = hw.ram_total_mb,
        accel    = ?hw.hw_accel,
        "hardware detected"
    );

    // Shared camera list — updated by WS client, read by recorder and heartbeat
    let cameras: Arc<RwLock<Vec<common::ws::NodeCameraConfig>>> =
        Arc::new(RwLock::new(Vec::new()));

    // Channel: WS client → MediaMTX supervisor (delivers updated camera list on every change)
    let (mediamtx_config_tx, mediamtx_config_rx) = tokio::sync::mpsc::unbounded_channel();

    // Channel: recorder → WS client ("flush the queue now")
    let (flush_tx, flush_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Channel: events module → WS client (motion events, real-time only)
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<common::ws::EventPayload>();

    // Channel: storage migration → WS client (segment URI updates)
    let (migrate_tx, migrate_rx) =
        tokio::sync::mpsc::unbounded_channel::<common::ws::SegmentMigratedPayload>();

    let recordings_dir = cfg.recordings_dir();
    let queue = Arc::new(recorder::queue::SegmentQueue::new(cfg.segment_queue_path()));

    // Spawn MediaMTX supervisor
    tokio::spawn(mediamtx::run_supervisor(
        mediamtx_config_rx,
        cfg.mediamtx_binary.clone(),
        cfg.mediamtx_config_path(),
        recordings_dir.clone(),
        cfg.mediamtx_api_port,
        cfg.mediamtx_rtsp_port,
    ));

    // Spawn recording FS watcher
    {
        let cameras_ref  = cameras.clone();
        let queue_ref    = queue.clone();
        let node_id      = identity.node_id;
        let rec_dir      = recordings_dir.clone();
        tokio::spawn(async move {
            if let Err(e) = recorder::run(rec_dir, cameras_ref, node_id, queue_ref, flush_tx).await {
                tracing::error!("recorder exited: {e}");
            }
        });
    }

    // Spawn disk watchdog
    tokio::spawn(recorder::watchdog::run(recordings_dir.clone()));

    // Spawn S3 migration task (no-op if S3_ENDPOINT not set)
    {
        let rec_dir = recordings_dir.clone();
        tokio::spawn(async move {
            storage::run_migration(rec_dir, migrate_tx).await;
        });
    }

    // Spawn playback HTTP server (segment serving + clip export)
    {
        let rec_dir = recordings_dir.clone();
        let port    = cfg.http_port;
        tokio::spawn(async move {
            if let Err(e) = playback::serve(rec_dir, port).await {
                tracing::error!("playback server exited: {e}");
            }
        });
    }

    // Spawn per-camera motion detection (ONVIF / keyframe-diff)
    {
        let cameras_ref = cameras.clone();
        let hw_ref      = hw.clone();
        tokio::spawn(async move {
            events::run(cameras_ref, hw_ref, event_tx).await;
        });
    }

    // WS client — blocks here, auto-reconnects
    ws_client::run(
        &cfg,
        &identity,
        hw,
        mediamtx_config_tx,
        cameras,
        queue,
        flush_rx,
        event_rx,
        migrate_rx,
    )
    .await;

    Ok(())
}
