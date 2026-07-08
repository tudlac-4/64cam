use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Envelope for all coordinator↔node WebSocket messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    Heartbeat      { seq: u64, payload: HeartbeatPayload },
    ConfigSync     { seq: u64, payload: ConfigSyncPayload },
    CameraAdded    { seq: u64, payload: NodeCameraConfig },
    CameraUpdated  { seq: u64, payload: NodeCameraConfig },
    CameraRemoved  { seq: u64, camera_id: Uuid },
    SegmentComplete { seq: u64, payload: SegmentPayload },
    DeleteRecording {
        seq:         u64,
        recording_id: Uuid,
        started_at:  DateTime<Utc>,
        storage_uri: String,
    },
    MotionEvent     { seq: u64, payload: EventPayload },
    SegmentMigrated { seq: u64, payload: SegmentMigratedPayload },
    Ping { seq: u64 },
    Pong { seq: u64 },
}

impl WsMessage {
    pub fn config_sync(cameras: Vec<NodeCameraConfig>) -> Self {
        WsMessage::ConfigSync { seq: 0, payload: ConfigSyncPayload { cameras } }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatPayload {
    pub node_id:          Uuid,
    pub hardware:         HardwareProfile,
    pub cameras:          Vec<CameraStatus>,
    pub mediamtx_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HardwareProfile {
    pub cpu_cores:       u32,
    pub cpu_model:       String,
    pub ram_total_mb:    u64,
    pub ram_available_mb: u64,
    pub hw_accel:        Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CameraStatus {
    pub camera_id:   Uuid,
    pub stream_path: String,
    pub connected:   bool,
    pub readers:     u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSyncPayload {
    pub cameras: Vec<NodeCameraConfig>,
}

/// Minimal camera descriptor sent to nodes (no sensitive coordinator internals).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCameraConfig {
    pub id:               Uuid,
    pub name:             String,
    pub rtsp_url:         String,
    pub stream_path:      String,
    pub enabled:          bool,
    pub onvif_url:        Option<String>,
    pub onvif_username:   Option<String>,
    pub onvif_password:   Option<String>,
    pub motion_detection: bool,
}

/// Notification that a segment has been migrated from local storage to S3/MinIO.
/// The coordinator updates the recording's `storage_uri` so playback requests
/// are redirected to a presigned S3 URL instead of the node's local HTTP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMigratedPayload {
    /// Deterministic UUIDv5 — same value as was inserted by the recorder.
    pub recording_id: Uuid,
    /// Partition key; included so the coordinator can do an efficient UPDATE
    /// on the partitioned recordings table without scanning all partitions.
    pub started_at:   DateTime<Utc>,
    /// New URI, e.g. `s3://my-bucket/front-door/20260708_120000.mp4`
    pub storage_uri:  String,
}

/// A motion or analytics event detected on the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub camera_id:   Uuid,
    pub occurred_at: DateTime<Utc>,
    /// "onvif" or "diff"
    pub source:      String,
    /// Normalised pixel-diff score (diff source only)
    pub score:       Option<f32>,
}

/// One fMP4 segment that was closed and is ready for indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentPayload {
    /// Deterministic UUIDv5(NAMESPACE_URL, storage_uri) — stable across retries.
    pub id:           Uuid,
    pub camera_id:    Uuid,
    pub stream_path:  String,
    pub storage_uri:  String,
    pub started_at:   DateTime<Utc>,
    pub ended_at:     DateTime<Utc>,
    pub duration_secs: i32,
    pub size_bytes:   i64,
}
