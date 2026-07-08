use std::path::{Path, PathBuf};
use std::time::Duration;

use sysinfo::Disks;
use walkdir::WalkDir;

const HIGH_WATERMARK: f64 = 85.0;
const LOW_WATERMARK: f64  = 75.0;

pub async fn run(recordings_dir: PathBuf) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        let pct = disk_usage_pct(&recordings_dir);
        if pct > HIGH_WATERMARK {
            tracing::warn!("disk {:.1}% full — culling oldest recordings", pct);
            cull_until(&recordings_dir, LOW_WATERMARK).await;
        }
    }
}

fn disk_usage_pct(path: &Path) -> f64 {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| {
            let total = d.total_space() as f64;
            let avail = d.available_space() as f64;
            if total == 0.0 { 0.0 } else { (1.0 - avail / total) * 100.0 }
        })
        .unwrap_or(0.0)
}

async fn cull_until(recordings_dir: &Path, target_pct: f64) {
    // Collect all .mp4 files with their modification time (oldest first)
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = WalkDir::new(recordings_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((e.into_path(), mtime))
        })
        .collect();

    files.sort_by_key(|(_, mtime)| *mtime);

    for (path, _) in files {
        if disk_usage_pct(recordings_dir) <= target_pct {
            break;
        }
        tracing::info!("watchdog: deleting {}", path.display());
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::warn!("watchdog: failed to delete {}: {e}", path.display());
            continue;
        }
        // Prune empty parent directories (stream_path dir, date dir)
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::remove_dir(parent).await;
            if let Some(grandparent) = parent.parent() {
                let _ = tokio::fs::remove_dir(grandparent).await;
            }
        }
    }
}
