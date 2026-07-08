use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Camera {
    pub id:               Uuid,
    pub node_id:          Uuid,
    pub name:             String,
    pub rtsp_url:         String,
    pub stream_path:      Option<String>,
    pub enabled:          bool,
    pub metadata:         serde_json::Value,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
    #[sqlx(default)]
    pub onvif_url:        Option<String>,
    #[sqlx(default)]
    pub onvif_username:   Option<String>,
    /// Never serialised to API responses — omitted via skip
    #[serde(skip_serializing)]
    #[sqlx(default)]
    pub onvif_password:   Option<String>,
    #[sqlx(default)]
    pub motion_detection: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCamera {
    /// Target node. Omit to auto-assign to the approved node with the most headroom.
    pub node_id:          Option<Uuid>,
    pub name:             String,
    pub rtsp_url:         String,
    pub stream_path:      Option<String>,
    pub onvif_url:        Option<String>,
    pub onvif_username:   Option<String>,
    pub onvif_password:   Option<String>,
    pub motion_detection: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCamera {
    pub name:             Option<String>,
    pub rtsp_url:         Option<String>,
    pub stream_path:      Option<String>,
    pub enabled:          Option<bool>,
    pub onvif_url:        Option<String>,
    pub onvif_username:   Option<String>,
    pub onvif_password:   Option<String>,
    pub motion_detection: Option<bool>,
}
