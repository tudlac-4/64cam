use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use tokio::process::Command;
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct PlaybackState {
    pub recordings_dir: PathBuf,
}

/// Starts the node's playback HTTP server (segment serving + clip export).
pub async fn serve(recordings_dir: PathBuf, port: u16) -> anyhow::Result<()> {
    let state = PlaybackState { recordings_dir: recordings_dir.clone() };

    let app = Router::new()
        .route("/export", get(export_clip))
        // ServeDir handles Range headers automatically — enables browser seeking
        .nest_service("/segments", ServeDir::new(&recordings_dir))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("playback server listening on :{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Deserialize)]
struct ExportParams {
    stream_path: String,
    from:        chrono::DateTime<Utc>,
    to:          chrono::DateTime<Utc>,
}

/// `GET /export?stream_path=X&from=ISO&to=ISO`
///
/// Finds segments in the time range, concatenates them with FFmpeg, streams
/// the result as `video/mp4`. The coordinator redirects the browser here.
async fn export_clip(
    State(state): State<PlaybackState>,
    Query(params): Query<ExportParams>,
) -> impl IntoResponse {
    let dir = state.recordings_dir.join(&params.stream_path);
    let files = segments_in_range(&dir, params.from, params.to);

    if files.is_empty() {
        return (StatusCode::NOT_FOUND, "no recordings found in the requested range")
            .into_response();
    }

    // Write a temporary FFmpeg concat file list
    let list_path = std::env::temp_dir().join(format!("64cam_export_{}.txt", uuid::Uuid::new_v4()));
    let list_content = files
        .iter()
        .map(|p| format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''")))
        .collect::<String>();

    if let Err(e) = tokio::fs::write(&list_path, &list_content).await {
        tracing::error!("export: failed to write filelist: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut child = match Command::new("ffmpeg")
        .args(["-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args([
            "-c", "copy",
            // frag_keyframe+empty_moov produces a streamable MP4 without seeking to end
            "-movflags", "frag_keyframe+empty_moov",
            "-f", "mp4",
            "pipe:1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("export: ffmpeg spawn failed: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg not available").into_response();
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Clean up the filelist and wait for FFmpeg after the stream completes
    let list_clone = list_path.clone();
    tokio::spawn(async move {
        let _ = child.wait().await;
        let _ = tokio::fs::remove_file(&list_clone).await;
    });

    let stream = ReaderStream::new(stdout);
    Response::builder()
        .status(200)
        .header("Content-Type",        "video/mp4")
        .header("Content-Disposition", "attachment; filename=\"clip.mp4\"")
        .header("Cache-Control",       "no-store")
        .body(Body::from_stream(stream))
        .unwrap()
        .into_response()
}

/// Returns paths to segments whose time range overlaps `[from, to)`.
fn segments_in_range(dir: &Path, from: chrono::DateTime<Utc>, to: chrono::DateTime<Utc>) -> Vec<PathBuf> {
    let mut files: Vec<(PathBuf, chrono::DateTime<Utc>)> = walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mp4"))
        .filter_map(|e| {
            let stem = e.path().file_stem()?.to_str()?;
            let naive = NaiveDateTime::parse_from_str(stem, "%Y%m%d_%H%M%S").ok()?;
            let started = Utc.from_utc_datetime(&naive);
            // Segment overlaps request window if it starts before `to` and ends after `from`
            let ended = started + chrono::Duration::seconds(60);
            if started < to && ended > from {
                Some((e.into_path(), started))
            } else {
                None
            }
        })
        .collect();

    files.sort_by_key(|(_, t)| *t);
    files.into_iter().map(|(p, _)| p).collect()
}
