use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::models::retention::{CreateRetentionPolicy, RetentionPolicy, UpdateRetentionPolicy};
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

#[utoipa::path(get, path = "/api/v1/retention-policies",
    responses((status = 200, body = Vec<RetentionPolicy>)),
    security(("bearer_token" = []))
)]
pub async fn list_retention_policies(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<Vec<RetentionPolicy>>> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let rows = sqlx::query_as::<_, RetentionPolicy>(
        "SELECT id, name, keep_days, is_default, created_at
         FROM retention_policies ORDER BY is_default DESC, name",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[utoipa::path(post, path = "/api/v1/retention-policies",
    request_body = CreateRetentionPolicy,
    responses((status = 201, body = RetentionPolicy), (status = 409, description = "Name already exists")),
    security(("bearer_token" = []))
)]
pub async fn create_retention_policy(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateRetentionPolicy>,
) -> Result<(StatusCode, Json<RetentionPolicy>)> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    if body.keep_days <= 0 {
        return Err(AppError::BadRequest("keep_days must be > 0".into()));
    }
    let is_default = body.is_default.unwrap_or(false);
    let mut tx = state.db.begin().await?;
    if is_default {
        sqlx::query("UPDATE retention_policies SET is_default = false WHERE is_default = true")
            .execute(&mut *tx)
            .await?;
    }
    let row = sqlx::query_as::<_, RetentionPolicy>(
        "INSERT INTO retention_policies (name, keep_days, is_default)
         VALUES ($1, $2, $3)
         RETURNING id, name, keep_days, is_default, created_at",
    )
    .bind(&body.name)
    .bind(body.keep_days)
    .bind(is_default)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(row)))
}

#[utoipa::path(get, path = "/api/v1/retention-policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    responses((status = 200, body = RetentionPolicy), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn get_retention_policy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RetentionPolicy>> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let row = sqlx::query_as::<_, RetentionPolicy>(
        "SELECT id, name, keep_days, is_default, created_at FROM retention_policies WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

#[utoipa::path(patch, path = "/api/v1/retention-policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    request_body = UpdateRetentionPolicy,
    responses((status = 200, body = RetentionPolicy), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn update_retention_policy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRetentionPolicy>,
) -> Result<Json<RetentionPolicy>> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    if let Some(d) = body.keep_days {
        if d <= 0 {
            return Err(AppError::BadRequest("keep_days must be > 0".into()));
        }
    }
    let mut tx = state.db.begin().await?;
    if body.is_default == Some(true) {
        sqlx::query(
            "UPDATE retention_policies SET is_default = false WHERE is_default = true AND id != $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    let row = sqlx::query_as::<_, RetentionPolicy>(
        "UPDATE retention_policies SET
            name       = COALESCE($2, name),
            keep_days  = COALESCE($3, keep_days),
            is_default = COALESCE($4, is_default)
         WHERE id = $1
         RETURNING id, name, keep_days, is_default, created_at",
    )
    .bind(id)
    .bind(body.name.as_deref())
    .bind(body.keep_days)
    .bind(body.is_default)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    tx.commit().await?;
    Ok(Json(row))
}

#[utoipa::path(delete, path = "/api/v1/retention-policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn delete_retention_policy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let result = sqlx::query("DELETE FROM retention_policies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
