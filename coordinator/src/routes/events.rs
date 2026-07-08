use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Result,
    middleware::auth::CurrentUser,
    state::AppState,
};

#[derive(Serialize)]
pub struct EventRow {
    pub id:          Uuid,
    pub camera_id:   Uuid,
    pub occurred_at: DateTime<Utc>,
    pub source:      String,
    pub score:       Option<f32>,
}

#[derive(Deserialize)]
pub struct TimeRangeQuery {
    pub from: DateTime<Utc>,
    pub to:   DateTime<Utc>,
}

/// `GET /api/v1/cameras/:id/events?from=ISO&to=ISO`
///
/// Returns motion events for the timeline scrubber markers.  Capped at 5 000
/// results (well above any realistic motion frequency in a query window).
pub async fn list_camera_events(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(camera_id): Path<Uuid>,
    Query(q): Query<TimeRangeQuery>,
) -> Result<Json<Vec<EventRow>>> {
    // payload->>'source' is safe: we always write { "source": "...", "score": ... }
    let rows = sqlx::query_as::<_, (Uuid, DateTime<Utc>, Option<String>, Option<f32>)>(
        "SELECT id, occurred_at,
                payload->>'source' AS source,
                (payload->>'score')::REAL    AS score
           FROM events
          WHERE camera_id  = $1
            AND type       = 'motion'
            AND occurred_at >= $2
            AND occurred_at <  $3
          ORDER BY occurred_at
          LIMIT 5000",
    )
    .bind(camera_id)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.db)
    .await?;

    let events = rows
        .into_iter()
        .map(|(id, occurred_at, source, score)| EventRow {
            id,
            camera_id,
            occurred_at,
            source: source.unwrap_or_else(|| "unknown".into()),
            score,
        })
        .collect();

    Ok(Json(events))
}
