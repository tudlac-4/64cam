use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::models::camera::{Camera, CreateCamera, UpdateCamera};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

const MAX_CAMERAS_PER_NODE: i64 = 64;

#[derive(Debug, Deserialize)]
pub struct CameraFilter {
    pub node_id: Option<Uuid>,
}

/// API response shape — wraps the DB model and adds computed fields.
#[derive(Debug, Serialize)]
pub struct CameraView {
    #[serde(flatten)]
    pub camera: Camera,
    /// Relative path the browser POSTs the SDP offer to: `/api/v1/cameras/{id}/whep`
    pub whep_path: String,
}

impl CameraView {
    fn from(camera: Camera) -> Self {
        let whep_path = format!("/api/v1/cameras/{}/whep", camera.id);
        CameraView { camera, whep_path }
    }
}

fn node_camera_config(row: &Camera) -> common::ws::NodeCameraConfig {
    common::ws::NodeCameraConfig {
        id:               row.id,
        name:             row.name.clone(),
        rtsp_url:         row.rtsp_url.clone(),
        stream_path:      row.stream_path.clone().unwrap_or_default(),
        enabled:          row.enabled,
        onvif_url:        row.onvif_url.clone(),
        onvif_username:   row.onvif_username.clone(),
        onvif_password:   row.onvif_password.clone(),
        motion_detection: row.motion_detection,
    }
}

/// Resolve the target node for a new camera.
/// If `node_id` is explicitly provided, validate it exists and is approved.
/// Otherwise pick the approved node with the fewest cameras that is still under capacity.
async fn resolve_node(state: &AppState, node_id: Option<Uuid>) -> Result<Uuid> {
    match node_id {
        Some(nid) => {
            // Explicit node — must be approved
            let status: Option<(String,)> = sqlx::query_as::<_, (String,)>(
                "SELECT CAST(status AS TEXT) FROM nodes WHERE id = $1",
            )
            .bind(nid)
            .fetch_optional(&state.db)
            .await?;

            match status {
                Some((s,)) if s == "approved" => Ok(nid),
                Some(_) => Err(AppError::UnprocessableEntity(
                    "target node is not in approved status".into(),
                )),
                None => Err(AppError::NotFound),
            }
        }
        None => {
            // Auto-assign: least-loaded approved node under capacity
            let row: Option<(Uuid,)> = sqlx::query_as::<_, (Uuid,)>(
                "SELECT n.id
                   FROM nodes n
                  WHERE n.status = 'approved'
                    AND (SELECT COUNT(*) FROM cameras c WHERE c.node_id = n.id)
                        < $1
                  ORDER BY (SELECT COUNT(*) FROM cameras c WHERE c.node_id = n.id) ASC
                  LIMIT 1",
            )
            .bind(MAX_CAMERAS_PER_NODE)
            .fetch_optional(&state.db)
            .await?;

            row.map(|(id,)| id).ok_or_else(|| {
                AppError::UnprocessableEntity(
                    "no approved node has capacity for an additional camera".into(),
                )
            })
        }
    }
}

/// Hard-check that adding one more camera to `node_id` won't exceed the 64-camera cap.
async fn check_capacity(state: &AppState, node_id: Uuid) -> Result<()> {
    let (count,): (i64,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM cameras WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_one(&state.db)
    .await?;

    if count >= MAX_CAMERAS_PER_NODE {
        return Err(AppError::UnprocessableEntity(format!(
            "node {node_id} is at the {MAX_CAMERAS_PER_NODE}-camera limit"
        )));
    }
    Ok(())
}

#[utoipa::path(get, path = "/api/v1/cameras",
    params(("node_id" = Option<Uuid>, Query, description = "Filter by node")),
    responses((status = 200, body = Vec<Camera>)),
    security(("bearer_token" = []))
)]
pub async fn list_cameras(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(filter): Query<CameraFilter>,
) -> Result<Json<Vec<CameraView>>> {
    let rows: Vec<Camera> = if let Some(nid) = filter.node_id {
        sqlx::query_as::<_, Camera>(
            "SELECT id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at
             FROM cameras WHERE node_id = $1 ORDER BY created_at",
        )
        .bind(nid)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, Camera>(
            "SELECT id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at
             FROM cameras ORDER BY created_at",
        )
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(rows.into_iter().map(CameraView::from).collect()))
}

#[utoipa::path(post, path = "/api/v1/cameras",
    request_body = CreateCamera,
    responses(
        (status = 201, body = Camera),
        (status = 422, description = "Node at 64-camera capacity or no approved node available"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_camera(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateCamera>,
) -> Result<(StatusCode, Json<CameraView>)> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }

    let target_node = resolve_node(&state, body.node_id).await?;
    check_capacity(&state, target_node).await?;

    let row = sqlx::query_as::<_, Camera>(
        "INSERT INTO cameras
             (node_id, name, rtsp_url, stream_path,
              onvif_url, onvif_username, onvif_password, motion_detection)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at,
                   onvif_url, onvif_username, onvif_password, motion_detection",
    )
    .bind(target_node)
    .bind(&body.name)
    .bind(&body.rtsp_url)
    .bind(body.stream_path.as_deref())
    .bind(body.onvif_url.as_deref())
    .bind(body.onvif_username.as_deref())
    .bind(body.onvif_password.as_deref())
    .bind(body.motion_detection.unwrap_or(true))
    .fetch_one(&state.db)
    .await?;

    let cfg = node_camera_config(&row);
    state
        .push_to_node(target_node, common::ws::WsMessage::CameraAdded { seq: 0, payload: cfg })
        .await;

    Ok((StatusCode::CREATED, Json(CameraView::from(row))))
}

#[utoipa::path(get, path = "/api/v1/cameras/{id}",
    params(("id" = Uuid, Path, description = "Camera ID")),
    responses((status = 200, body = Camera), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn get_camera(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CameraView>> {
    let row = sqlx::query_as::<_, Camera>(
        "SELECT id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at
         FROM cameras WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(CameraView::from(row)))
}

#[utoipa::path(patch, path = "/api/v1/cameras/{id}",
    params(("id" = Uuid, Path, description = "Camera ID")),
    request_body = UpdateCamera,
    responses((status = 200, body = Camera), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn update_camera(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCamera>,
) -> Result<Json<CameraView>> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }
    let row = sqlx::query_as::<_, Camera>(
        "UPDATE cameras SET
            name             = COALESCE($2, name),
            rtsp_url         = COALESCE($3, rtsp_url),
            stream_path      = COALESCE($4, stream_path),
            enabled          = COALESCE($5, enabled),
            onvif_url        = COALESCE($6, onvif_url),
            onvif_username   = COALESCE($7, onvif_username),
            onvif_password   = COALESCE($8, onvif_password),
            motion_detection = COALESCE($9, motion_detection),
            updated_at       = NOW()
         WHERE id = $1
         RETURNING id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at,
                   onvif_url, onvif_username, onvif_password, motion_detection",
    )
    .bind(id)
    .bind(body.name.as_deref())
    .bind(body.rtsp_url.as_deref())
    .bind(body.stream_path.as_deref())
    .bind(body.enabled)
    .bind(body.onvif_url.as_deref())
    .bind(body.onvif_username.as_deref())
    .bind(body.onvif_password.as_deref())
    .bind(body.motion_detection)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let cfg = node_camera_config(&row);
    state
        .push_to_node(row.node_id, common::ws::WsMessage::CameraUpdated { seq: 0, payload: cfg })
        .await;

    Ok(Json(CameraView::from(row)))
}

#[utoipa::path(delete, path = "/api/v1/cameras/{id}",
    params(("id" = Uuid, Path, description = "Camera ID")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn delete_camera(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }
    let cam = sqlx::query_as::<_, Camera>(
        "DELETE FROM cameras WHERE id = $1
         RETURNING id, node_id, name, rtsp_url, stream_path, enabled, metadata, created_at, updated_at",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    state
        .push_to_node(cam.node_id, common::ws::WsMessage::CameraRemoved { seq: 0, camera_id: id })
        .await;
    Ok(StatusCode::NO_CONTENT)
}
