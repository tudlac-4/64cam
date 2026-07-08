mod diff;
mod onvif;

use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use common::ws::{EventPayload, HardwareProfile, NodeCameraConfig};
use tokio::{
    sync::{mpsc, RwLock},
    task::AbortHandle,
};
use uuid::Uuid;

struct CameraTask {
    abort:      AbortHandle,
    /// Fingerprint for change detection: rtsp_url + onvif_url (or "diff")
    sig:        String,
}

fn task_sig(cam: &NodeCameraConfig) -> String {
    format!(
        "{}|{}",
        cam.rtsp_url,
        cam.onvif_url.as_deref().unwrap_or("diff"),
    )
}

/// Spawn and manage per-camera motion detection tasks.
///
/// Checks the camera list every 10 s.  For each enabled camera with motion
/// detection, starts the appropriate task (ONVIF if configured, otherwise
/// keyframe-diff if the node has sufficient CPU).  Aborts tasks for cameras
/// that are removed, disabled, or whose connection config has changed.
pub async fn run(
    cameras: Arc<RwLock<Vec<NodeCameraConfig>>>,
    hw:      HardwareProfile,
    tx:      mpsc::UnboundedSender<EventPayload>,
) {
    let diff_capable = hw.cpu_cores >= 2 || !hw.hw_accel.is_empty();
    let mut running: HashMap<Uuid, CameraTask> = HashMap::new();

    loop {
        let list = cameras.read().await.clone();

        // Stop tasks for cameras that are gone, disabled, or reconfigured
        let wanted: HashMap<Uuid, String> = list
            .iter()
            .filter(|c| c.enabled && c.motion_detection)
            .map(|c| (c.id, task_sig(c)))
            .collect();

        running.retain(|id, task| {
            match wanted.get(id) {
                Some(sig) if sig == &task.sig => true,  // still valid
                _ => {
                    task.abort.abort();
                    false
                }
            }
        });

        // Start tasks for cameras not yet tracked
        for cam in list.iter().filter(|c| c.enabled && c.motion_detection) {
            if running.contains_key(&cam.id) {
                continue;
            }

            let sig = task_sig(cam);

            let handle = if cam.onvif_url.is_some() {
                let c = cam.clone();
                let t = tx.clone();
                tokio::spawn(async move { onvif::run(c, t).await })
            } else if diff_capable {
                let c = cam.clone();
                let t = tx.clone();
                tokio::spawn(async move { diff::run(c, t).await })
            } else {
                continue;
            };

            running.insert(cam.id, CameraTask { abort: handle.abort_handle(), sig });
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
