use chrono::{DateTime, Utc};
use tokio::time::Duration;
use uuid::Uuid;

use common::ws::WsMessage;

use crate::state::AppState;

#[derive(sqlx::FromRow)]
struct ExpiredRecording {
    id:          Uuid,
    started_at:  DateTime<Utc>,
    node_id:     Uuid,
    storage_uri: String,
}

/// Runs every hour. Deletes recordings whose retention period has expired,
/// then tells each node to remove the corresponding file via WS (best-effort).
pub async fn run_enforcer(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.tick().await; // skip the immediate first tick
    loop {
        interval.tick().await;
        let expired = query_expired(&state.db).await;
        delete_and_notify(&state, expired).await;
    }
}

/// Single retention pass against the supplied pool — no WS notifications.
/// Exposed for integration tests; production code always uses `run_enforcer`.
pub async fn run_single_pass(db: &sqlx::PgPool) {
    let expired = query_expired(db).await;
    for rec in &expired {
        let _ = sqlx::query("DELETE FROM recordings WHERE id = $1 AND started_at = $2")
            .bind(rec.id)
            .bind(rec.started_at)
            .execute(db)
            .await;
    }
    if !expired.is_empty() {
        tracing::info!("retention pass: {} recordings deleted", expired.len());
    }
}

async fn query_expired(db: &sqlx::PgPool) -> Vec<ExpiredRecording> {
    sqlx::query_as::<_, ExpiredRecording>(
        "SELECT r.id, r.started_at, r.node_id, r.storage_uri
         FROM recordings r
         LEFT JOIN retention_policies rp  ON rp.id = r.retention_policy_id
         LEFT JOIN LATERAL (
             SELECT keep_days FROM retention_policies WHERE is_default = true LIMIT 1
         ) AS def ON true
         WHERE COALESCE(rp.keep_days, def.keep_days) IS NOT NULL
           AND r.started_at < NOW()
                 - (COALESCE(rp.keep_days, def.keep_days) || ' days')::INTERVAL
         ORDER BY r.started_at
         LIMIT 500",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

async fn delete_and_notify(state: &AppState, expired: Vec<ExpiredRecording>) {
    if expired.is_empty() {
        return;
    }
    tracing::info!("retention enforcer: {} expired recordings", expired.len());
    for rec in expired {
        let _ = sqlx::query("DELETE FROM recordings WHERE id = $1 AND started_at = $2")
            .bind(rec.id)
            .bind(rec.started_at)
            .execute(&state.db)
            .await;

        state
            .push_to_node(
                rec.node_id,
                WsMessage::DeleteRecording {
                    seq:          0,
                    recording_id: rec.id,
                    started_at:   rec.started_at,
                    storage_uri:  rec.storage_uri,
                },
            )
            .await;
    }
}
