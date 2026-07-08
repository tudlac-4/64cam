pub mod config;

use std::path::PathBuf;
use tokio::sync::mpsc;
use common::ws::NodeCameraConfig;

pub async fn run_supervisor(
    mut config_rx: mpsc::UnboundedReceiver<Vec<NodeCameraConfig>>,
    binary: PathBuf,
    config_path: PathBuf,
    recordings_dir: PathBuf,
    api_port: u16,
    rtsp_port: u16,
) {
    let mut process: Option<tokio::process::Child> = None;

    loop {
        // Check process health
        if let Some(ref mut p) = process {
            match p.try_wait() {
                Ok(Some(status)) => {
                    tracing::warn!("mediamtx exited: {status}");
                    process = None;
                    if config_path.exists() {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        process = spawn(&binary, &config_path);
                    }
                }
                Ok(None) => {}
                Err(e) => tracing::error!("process wait: {e}"),
            }
        }

        // Wait for config update (poll health every 5s via timeout)
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            config_rx.recv(),
        )
        .await
        {
            Ok(Some(cameras)) => {
                let yaml = config::generate(&cameras, &recordings_dir, api_port, rtsp_port);
                if let Err(e) = tokio::fs::write(&config_path, yaml).await {
                    tracing::error!("config write: {e}");
                    continue;
                }
                if process.is_some() {
                    reload(&process);
                } else {
                    process = spawn(&binary, &config_path);
                }
            }
            Ok(None) => {
                tracing::info!("config channel closed, stopping supervisor");
                break;
            }
            Err(_) => {} // timeout — loop to check process health
        }
    }
}

fn spawn(binary: &PathBuf, config_path: &PathBuf) -> Option<tokio::process::Child> {
    match tokio::process::Command::new(binary).arg(config_path).spawn() {
        Ok(child) => {
            tracing::info!("mediamtx started (pid={})", child.id().unwrap_or(0));
            Some(child)
        }
        Err(e) => {
            tracing::error!("mediamtx spawn: {e}");
            None
        }
    }
}

fn reload(process: &Option<tokio::process::Child>) {
    #[cfg(unix)]
    {
        if let Some(pid) = process.as_ref().and_then(|p| p.id()) {
            use nix::{sys::signal::Signal, unistd::Pid};
            match nix::sys::signal::kill(Pid::from_raw(pid as i32), Signal::SIGHUP) {
                Ok(_) => tracing::info!("SIGHUP sent to mediamtx (pid={pid})"),
                Err(e) => tracing::warn!("SIGHUP failed: {e}"),
            }
        }
    }
    #[cfg(not(unix))]
    tracing::warn!("config reload via SIGHUP not supported on this platform");
}
