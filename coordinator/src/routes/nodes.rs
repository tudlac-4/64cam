use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::{
    auth::{generate_refresh_token, hash_token},
    models::node::{CreateNode, Node, NodeResponse, UpdateNodeStatus},
    ws::{HardwareProfile, NodeCameraConfig},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

const MAX_CAMERAS_PER_NODE: i64 = 64;

#[derive(Serialize, ToSchema)]
pub struct RegisterNodeResponse {
    pub node: NodeResponse,
    pub api_key: String,
}

/// Public self-registration — no auth required.
/// Node is created in `pending` status; admin must approve before cameras are synced.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SelfRegisterRequest {
    pub name: String,
    pub hardware: Option<HardwareProfile>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NodeCapacity {
    pub node_id:      Uuid,
    pub camera_count: i64,
    pub max_cameras:  i64,
    pub headroom:     i64,
}

const NODE_SELECT: &str =
    "SELECT id, name, api_key_hash, status, last_seen_at,
            CAST(ip_addr AS TEXT) AS ip_addr, version, metadata, created_at, updated_at
     FROM nodes";

/// Fetch all cameras for a node including ONVIF fields, using the same shape as
/// the `node_camera_config()` helper in cameras.rs.
async fn fetch_node_cameras_full(state: &AppState, node_id: Uuid) -> Vec<NodeCameraConfig> {
    sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool,
                          Option<String>, Option<String>, Option<String>, bool)>(
        "SELECT id, name, rtsp_url, stream_path, enabled,
                onvif_url, onvif_username, onvif_password, motion_detection
           FROM cameras WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, name, rtsp_url, stream_path, enabled,
           onvif_url, onvif_username, onvif_password, motion_detection)| NodeCameraConfig {
        id, name, rtsp_url,
        stream_path: stream_path.unwrap_or_default(),
        enabled,
        onvif_url,
        onvif_username,
        onvif_password,
        motion_detection,
    })
    .collect()
}

#[utoipa::path(post, path = "/api/v1/nodes/register",
    request_body = SelfRegisterRequest,
    responses((status = 201, body = RegisterNodeResponse), (status = 409, description = "Name already taken"))
)]
pub async fn self_register(
    State(state): State<AppState>,
    Json(body): Json<SelfRegisterRequest>,
) -> Result<(StatusCode, Json<RegisterNodeResponse>)> {
    let api_key = generate_refresh_token();
    let key_hash = hash_token(&api_key);
    let metadata = serde_json::json!({ "hardware": body.hardware });

    let row = sqlx::query_as::<_, Node>(&format!(
        "INSERT INTO nodes (name, api_key_hash, metadata)
         VALUES ($1, $2, $3)
         RETURNING id, name, api_key_hash, status, last_seen_at,
                   CAST(ip_addr AS TEXT) AS ip_addr, version, metadata, created_at, updated_at"
    ))
    .bind(&body.name)
    .bind(&key_hash)
    .bind(metadata)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterNodeResponse { node: row.into(), api_key }),
    ))
}

#[utoipa::path(get, path = "/api/v1/nodes",
    responses((status = 200, body = Vec<NodeResponse>)),
    security(("bearer_token" = []))
)]
pub async fn list_nodes(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<Vec<NodeResponse>>> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }
    let rows = sqlx::query_as::<_, Node>(&format!("{NODE_SELECT} ORDER BY created_at DESC"))
        .fetch_all(&state.db)
        .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/api/v1/nodes",
    request_body = CreateNode,
    responses((status = 201, body = RegisterNodeResponse)),
    security(("bearer_token" = []))
)]
pub async fn create_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateNode>,
) -> Result<(StatusCode, Json<RegisterNodeResponse>)> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let api_key = generate_refresh_token();
    let key_hash = hash_token(&api_key);

    let row = sqlx::query_as::<_, Node>(&format!(
        "INSERT INTO nodes (name, api_key_hash, status)
         VALUES ($1, $2, 'approved')
         RETURNING id, name, api_key_hash, status, last_seen_at,
                   CAST(ip_addr AS TEXT) AS ip_addr, version, metadata, created_at, updated_at"
    ))
    .bind(&body.name)
    .bind(&key_hash)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterNodeResponse { node: row.into(), api_key }),
    ))
}

#[utoipa::path(get, path = "/api/v1/nodes/{id}",
    params(("id" = Uuid, Path, description = "Node ID")),
    responses((status = 200, body = NodeResponse), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn get_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<NodeResponse>> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }
    let row = sqlx::query_as::<_, Node>(&format!("{NODE_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}

#[utoipa::path(get, path = "/api/v1/nodes/{id}/capacity",
    params(("id" = Uuid, Path, description = "Node ID")),
    responses((status = 200, body = NodeCapacity), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn get_node_capacity(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<NodeCapacity>> {
    if !user.is_operator_or_above() {
        return Err(AppError::Forbidden);
    }
    // Verify node exists
    let exists: Option<(i32,)> = sqlx::query_as::<_, (i32,)>(
        "SELECT 1 FROM nodes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let (count,): (i64,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM cameras WHERE node_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(NodeCapacity {
        node_id:      id,
        camera_count: count,
        max_cameras:  MAX_CAMERAS_PER_NODE,
        headroom:     (MAX_CAMERAS_PER_NODE - count).max(0),
    }))
}

#[utoipa::path(patch, path = "/api/v1/nodes/{id}/status",
    params(("id" = Uuid, Path, description = "Node ID")),
    request_body = UpdateNodeStatus,
    responses((status = 200, body = NodeResponse), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn update_node_status(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNodeStatus>,
) -> Result<Json<NodeResponse>> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let row = sqlx::query_as::<_, Node>(&format!(
        "UPDATE nodes SET status = $2, updated_at = NOW()
         WHERE id = $1
         RETURNING id, name, api_key_hash, status, last_seen_at,
                   CAST(ip_addr AS TEXT) AS ip_addr, version, metadata, created_at, updated_at"
    ))
    .bind(id)
    .bind(body.status)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // If just approved and the node is online, push a full ConfigSync with
    // all ONVIF fields included so motion detection starts immediately.
    if matches!(row.status, common::models::node::NodeStatus::Approved) {
        let cameras = fetch_node_cameras_full(&state, id).await;
        state.push_to_node(id, common::ws::WsMessage::config_sync(cameras)).await;
    }

    Ok(Json(row.into()))
}

#[utoipa::path(delete, path = "/api/v1/nodes/{id}",
    params(("id" = Uuid, Path, description = "Node ID")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn delete_node(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let result = sqlx::query("DELETE FROM nodes WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
