use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RetentionPolicy {
    pub id: Uuid,
    pub name: String,
    pub keep_days: i32,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRetentionPolicy {
    pub name: String,
    pub keep_days: i32,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRetentionPolicy {
    pub name: Option<String>,
    pub keep_days: Option<i32>,
    pub is_default: Option<bool>,
}
