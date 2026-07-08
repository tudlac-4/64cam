use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "node_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Pending,
    Approved,
    Rejected,
}

// ip_addr is fetched via CAST(ip_addr AS TEXT) in all queries
#[derive(Debug, Clone, FromRow)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub api_key_hash: String,
    pub status: NodeStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub ip_addr: Option<String>,
    pub version: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NodeResponse {
    pub id: Uuid,
    pub name: String,
    pub status: NodeStatus,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub ip_addr: Option<String>,
    pub version: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Node> for NodeResponse {
    fn from(n: Node) -> Self {
        Self {
            id: n.id,
            name: n.name,
            status: n.status,
            last_seen_at: n.last_seen_at,
            ip_addr: n.ip_addr,
            version: n.version,
            metadata: n.metadata,
            created_at: n.created_at,
            updated_at: n.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNode {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateNodeStatus {
    pub status: NodeStatus,
}
