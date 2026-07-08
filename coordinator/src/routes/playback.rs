use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
    storage,
};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordingSegment {
    pub id:            Uuid,
    pub started_at:    DateTime<Utc>,
    pub ended_at:      DateTime<Utc>,
    pub duration_secs: i32,
    pub size_bytes:    i64,
}

#[derive(Deserialize)]
pub struct TimeRangeQuery {
    pub from: DateTime<Utc>,
    pub to:   DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct TokenTimeRangeQuery {
    pub token: String,
    pub from:  DateTime<Utc>,
    pub to:    DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// `GET /api/v1/cameras/:id/recordings?from=ISO&to=ISO`
///
/// Returns segment metadata for the timeline scrubber.  The `from`/`to` range
/// is capped at 10 000 results (≈ 7 days at 60 s segments).
pub async fn list_recordings(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(camera_id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> Result<Json<Vec<RecordingSegment>>> {
    let rows = sqlx::query_as::<_, (Uuid, DateTime<Utc>, DateTime<Utc>, i32, i64)>(
        "SELECT id,
                started_at,
                COALESCE(ended_at, started_at + INTERVAL '60 seconds'),
                COALESCE(duration_secs, 60),
                COALESCE(size_bytes, 0)
           FROM recordings
          WHERE camera_id = $1
            AND started_at >= $2
            AND started_at <  $3
          ORDER BY started_at
          LIMIT 10000",
    )
    .bind(camera_id)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.db)
    .await?;

    let segs = rows
        .into_iter()
        .map(|(id, started_at, ended_at, duration_secs, size_bytes)| RecordingSegment {
            id, started_at, ended_at, duration_secs, size_bytes,
        })
        .collect();

    Ok(Json(segs))
}

/// `GET /api/v1/cameras/:id/segments/:recording_id?token=<jwt>`
///
/// Proxies the segment file from the owning node.  Accepts the JWT in a query
/// param so the browser's `<video src>` tag can authenticate without custom
/// headers. Forwards the `Range` header for in-segment seeking.
pub async fn get_segment(
    State(state): State<AppState>,
    Path((camera_id, recording_id)): Path<(Uuid, Uuid)>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response> {
    state.jwt.decode(&q.token).map_err(|_| AppError::Unauthorized)?;

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT r.storage_uri, CAST(n.ip_addr AS TEXT)
           FROM recordings r
           JOIN nodes n ON n.id = r.node_id
          WHERE r.id = $1 AND r.camera_id = $2
          LIMIT 1",
    )
    .bind(recording_id)
    .bind(camera_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (storage_uri, node_ip) = row;

    // Segments migrated to S3 are served via a presigned redirect instead of
    // proxying through the node (which no longer holds the file on local disk).
    if storage_uri.starts_with("s3://") {
        let presigned = storage::presigned_get(&storage_uri, 3600)
            .ok_or_else(|| AppError::Internal(
                anyhow::anyhow!("S3 not configured on coordinator; cannot serve migrated segment")
            ))?;
        return Ok((
            StatusCode::FOUND,
            [("location", presigned)],
        ).into_response());
    }

    let rel_path = storage_uri.strip_prefix("local://").unwrap_or(&storage_uri);
    let port = node_http_port();
    let node_url = format!("http://{node_ip}:{port}/segments/{rel_path}");

    proxy_get(&node_url, headers.get("range").cloned()).await
}

/// `GET /api/v1/cameras/:id/export?token=<jwt>&from=ISO&to=ISO`
///
/// Proxies the FFmpeg clip export stream from the owning node.  `Content-Disposition`
/// is forwarded so the browser triggers a file download named `clip.mp4`.
pub async fn export_clip(
    State(state): State<AppState>,
    Path(camera_id): Path<Uuid>,
    Query(q): Query<TokenTimeRangeQuery>,
) -> Result<Response> {
    state.jwt.decode(&q.token).map_err(|_| AppError::Unauthorized)?;

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT c.stream_path, CAST(n.ip_addr AS TEXT)
           FROM cameras c
           JOIN nodes n ON n.id = c.node_id
          WHERE c.id = $1",
    )
    .bind(camera_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (stream_path, node_ip) = row;
    let port = node_http_port();
    let from = q.from.to_rfc3339();
    let to   = q.to.to_rfc3339();
    let node_url = format!(
        "http://{node_ip}:{port}/export?stream_path={stream_path}&from={}&to={}",
        encode_ts(&from),
        encode_ts(&to),
    );

    proxy_get(&node_url, None).await
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn proxy_get(node_url: &str, range: Option<axum::http::HeaderValue>) -> Result<Response> {
    let client = reqwest::Client::new();
    let mut req = client.get(node_url);
    if let Some(r) = range {
        if let Ok(v) = r.to_str() {
            req = req.header("Range", v);
        }
    }

    let upstream = req
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("node request failed: {e}")))?;

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);

    // Forward a safe subset of upstream headers
    let mut resp = Response::builder().status(status);
    for name in ["content-type", "content-range", "accept-ranges",
                  "content-length", "content-disposition"] {
        if let Some(v) = upstream.headers().get(name) {
            resp = resp.header(name, v);
        }
    }

    let stream = upstream.bytes_stream();
    Ok(resp
        .body(Body::from_stream(stream))
        .unwrap()
        .into_response())
}

fn node_http_port() -> u16 {
    std::env::var("NODE_HTTP_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8890)
}

/// Encode a timestamp string for use in a query param value.
/// `+` in timezone offsets (e.g. `+00:00`) must be percent-encoded because
/// application/x-www-form-urlencoded treats `+` as a space.
fn encode_ts(s: &str) -> String {
    s.replace('+', "%2B")
}
