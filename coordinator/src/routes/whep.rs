use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

#[derive(sqlx::FromRow)]
struct WhepTarget {
    stream_path: Option<String>,
    node_ip:     Option<String>,
}

/// `POST /api/v1/cameras/:id/whep`
///
/// Proxies the browser's SDP offer to the node's MediaMTX WHEP endpoint and
/// returns the SDP answer. Keeps node IPs off the browser and avoids CORS.
pub async fn whep_proxy(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(camera_id): Path<Uuid>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response> {
    let target = sqlx::query_as::<_, WhepTarget>(
        "SELECT c.stream_path, CAST(n.ip_addr AS TEXT) AS node_ip
         FROM cameras c
         JOIN nodes n ON n.id = c.node_id
         WHERE c.id = $1",
    )
    .bind(camera_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let stream_path = target
        .stream_path
        .ok_or_else(|| AppError::BadRequest("camera has no stream_path configured".into()))?;
    let node_ip = target
        .node_ip
        .ok_or_else(|| AppError::BadRequest("node IP not yet known".into()))?;

    // Default MediaMTX WebRTC port; override via MEDIAMTX_WEBRTC_PORT env var
    let webrtc_port: u16 = std::env::var("MEDIAMTX_WEBRTC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8889);

    let whep_url = format!("http://{node_ip}:{webrtc_port}/{stream_path}/whep");

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/sdp");

    let sdp_body = axum::body::to_bytes(body, 64 * 1024)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let upstream = reqwest::Client::new()
        .post(&whep_url)
        .header("Content-Type", content_type)
        .body(sdp_body)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("WHEP upstream error: {e}")))?;

    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut resp_headers = HeaderMap::new();
    for (k, v) in upstream.headers() {
        if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes()) {
            resp_headers.insert(name, v.clone());
        }
    }
    resp_headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );

    let answer = upstream
        .bytes()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("WHEP read: {e}")))?;

    let mut response = Response::new(Body::from(answer));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    Ok(response)
}

/// `OPTIONS /api/v1/cameras/:id/whep` — preflight for browser CORS
pub async fn whep_preflight() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [
            ("Access-Control-Allow-Origin",  "*"),
            ("Access-Control-Allow-Methods", "POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type, Authorization"),
        ],
    )
}
