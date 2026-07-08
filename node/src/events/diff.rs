use std::time::{Duration, Instant};

use chrono::Utc;
use common::ws::{EventPayload, NodeCameraConfig};
use tokio::{
    io::AsyncReadExt,
    sync::mpsc,
};

const WIDTH:       usize  = 160;
const HEIGHT:      usize  = 90;
const FRAME_BYTES: usize  = WIDTH * HEIGHT;
const THRESHOLD:   f32    = 0.02; // 2 % of pixels changed at max intensity
const COOLDOWN:    Duration = Duration::from_secs(10);

pub async fn run(cam: NodeCameraConfig, tx: mpsc::UnboundedSender<EventPayload>) {
    loop {
        if let Err(e) = diff_loop(&cam, &tx).await {
            tracing::warn!("diff [{}] error: {e} — retrying in 15s", cam.name);
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn diff_loop(
    cam: &NodeCameraConfig,
    tx: &mpsc::UnboundedSender<EventPayload>,
) -> anyhow::Result<()> {
    let mut child = tokio::process::Command::new("ffmpeg")
        .args([
            "-loglevel", "quiet",
            "-rtsp_transport", "tcp",
            "-i", &cam.rtsp_url,
            "-vf", &format!("fps=2,scale={WIDTH}:{HEIGHT}"),
            "-f", "rawvideo",
            "-pix_fmt", "gray",
            "pipe:1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;

    let mut prev_frame = vec![0u8; FRAME_BYTES];
    let mut curr_frame = vec![0u8; FRAME_BYTES];
    let mut first      = true;
    let mut last_event = Instant::now() - COOLDOWN * 2;

    loop {
        stdout.read_exact(&mut curr_frame).await?;

        if !first {
            let score = frame_diff(&prev_frame, &curr_frame);
            if score >= THRESHOLD && last_event.elapsed() >= COOLDOWN {
                last_event = Instant::now();
                let _ = tx.send(EventPayload {
                    camera_id:   cam.id,
                    occurred_at: Utc::now(),
                    source:      "diff".into(),
                    score:       Some(score),
                });
            }
        }

        std::mem::swap(&mut prev_frame, &mut curr_frame);
        first = false;
    }
}

fn frame_diff(a: &[u8], b: &[u8]) -> f32 {
    let sum: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64)
        .sum();
    sum as f32 / (FRAME_BYTES as f32 * 255.0)
}
