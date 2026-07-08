pub mod sigv4;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{NaiveDateTime, TimeZone, Utc};
use common::ws::SegmentMigratedPayload;
use sigv4::S3Config;
use tokio::sync::mpsc;
use uuid::Uuid;
use walkdir::WalkDir;

const SCAN_INTERVAL: Duration = Duration::from_secs(900); // 15 min

/// Returns the default minimum age in seconds before a segment is eligible for S3 migration.
fn migrate_after_secs() -> i64 {
    std::env::var("S3_MIGRATE_AFTER_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(2)
        * 3600
}

/// Background task: periodically migrates old fMP4 segments from local disk to S3.
/// No-ops if `S3_ENDPOINT` / `S3_BUCKET` env vars are not set.
pub async fn run_migration(
    recordings_dir: PathBuf,
    migrate_tx:     mpsc::UnboundedSender<SegmentMigratedPayload>,
) {
    let cfg = match S3Config::from_env() {
        Some(c) => c,
        None => {
            tracing::debug!("S3 not configured — segment migration disabled");
            return;
        }
    };

    tracing::info!("S3 migration enabled → bucket={} endpoint={}", cfg.bucket, cfg.endpoint);

    let mut interval = tokio::time::interval(SCAN_INTERVAL);
    loop {
        interval.tick().await;
        migrate_pass(&recordings_dir, &cfg, &migrate_tx).await;
    }
}

async fn migrate_pass(
    recordings_dir: &Path,
    cfg:            &S3Config,
    migrate_tx:     &mpsc::UnboundedSender<SegmentMigratedPayload>,
) {
    let cutoff = chrono::Duration::seconds(migrate_after_secs());
    let now    = Utc::now();

    let candidates: Vec<PathBuf> = WalkDir::new(recordings_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
        .filter_map(|e| {
            let stem = e.path().file_stem()?.to_str()?;
            let naive = NaiveDateTime::parse_from_str(stem, "%Y%m%d_%H%M%S").ok()?;
            let started = Utc.from_utc_datetime(&naive);
            if now - started >= cutoff {
                Some(e.into_path())
            } else {
                None
            }
        })
        .collect();

    for path in candidates {
        if let Err(e) = migrate_segment(&path, recordings_dir, cfg, migrate_tx).await {
            tracing::warn!("S3 migration failed for {}: {e}", path.display());
        }
    }
}

async fn migrate_segment(
    path:           &Path,
    recordings_dir: &Path,
    cfg:            &S3Config,
    migrate_tx:     &mpsc::UnboundedSender<SegmentMigratedPayload>,
) -> anyhow::Result<()> {
    // Derive the same relative path used for storage_uri by the recorder
    let rel = path.strip_prefix(recordings_dir)?;
    let rel_str = rel.to_string_lossy().replace('\\', "/");

    // Reconstruct the original local storage_uri to reproduce the UUIDv5
    let local_uri     = format!("local://{rel_str}");
    let recording_id  = Uuid::new_v5(&Uuid::NAMESPACE_URL, local_uri.as_bytes());

    // Parse started_at from filename stem
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("bad filename: {}", path.display()))?;
    let naive      = NaiveDateTime::parse_from_str(stem, "%Y%m%d_%H%M%S")?;
    let started_at = Utc.from_utc_datetime(&naive);

    // The S3 object key mirrors the relative path
    let s3_key      = &rel_str;
    let new_uri     = format!("s3://{}/{}", cfg.bucket, s3_key);

    let body = tokio::fs::read(path).await?;
    let headers = sigv4::signed_put_headers(cfg, s3_key, &body);
    let url = cfg.object_url(s3_key);

    let client = reqwest::Client::new();
    let mut req = client.put(&url);
    for (k, v) in &headers {
        req = req.header(k, v);
    }
    let resp = req.body(body).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body   = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("S3 PUT failed ({status}): {body}"));
    }

    tracing::info!("migrated {rel_str} → {new_uri}");

    let _ = migrate_tx.send(SegmentMigratedPayload {
        recording_id,
        started_at,
        storage_uri: new_uri,
    });

    // Delete the local file now that S3 has it
    tokio::fs::remove_file(path).await?;
    // Prune empty parent directories
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::remove_dir(parent).await;
        if let Some(gp) = parent.parent() {
            let _ = tokio::fs::remove_dir(gp).await;
        }
    }

    Ok(())
}
