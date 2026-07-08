use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::{
    auth::hash_password,
    models::user::{CreateUser, UpdateUser, User, UserResponse},
};
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

#[utoipa::path(get, path = "/api/v1/users",
    responses((status = 200, body = Vec<UserResponse>)),
    security(("bearer_token" = []))
)]
pub async fn list_users(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<Vec<UserResponse>>> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let rows = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, role_id, created_at, updated_at
         FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/api/v1/users",
    request_body = CreateUser,
    responses((status = 201, body = UserResponse), (status = 409, description = "Email already exists")),
    security(("bearer_token" = []))
)]
pub async fn create_user(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserResponse>)> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let hash = hash_password(&body.password).map_err(anyhow::Error::from)?;
    let row = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, role_id)
         VALUES ($1, $2, $3)
         RETURNING id, email, password_hash, role_id, created_at, updated_at",
    )
    .bind(&body.email)
    .bind(&hash)
    .bind(body.role_id)
    .fetch_one(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(get, path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses((status = 200, body = UserResponse), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn get_user(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>> {
    if !user.is_admin() && user.id != id {
        return Err(AppError::Forbidden);
    }
    let row = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, role_id, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}

#[utoipa::path(patch, path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = UpdateUser,
    responses((status = 200, body = UserResponse), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn update_user(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUser>,
) -> Result<Json<UserResponse>> {
    if !current.is_admin() && current.id != id {
        return Err(AppError::Forbidden);
    }
    let new_hash = body
        .password
        .as_deref()
        .map(hash_password)
        .transpose()
        .map_err(anyhow::Error::from)?;

    let row = sqlx::query_as::<_, User>(
        "UPDATE users SET
            email         = COALESCE($2, email),
            password_hash = COALESCE($3, password_hash),
            role_id       = COALESCE($4, role_id),
            updated_at    = NOW()
         WHERE id = $1
         RETURNING id, email, password_hash, role_id, created_at, updated_at",
    )
    .bind(id)
    .bind(body.email.as_deref())
    .bind(new_hash.as_deref())
    .bind(body.role_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}

#[utoipa::path(delete, path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")),
    security(("bearer_token" = []))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if !user.is_admin() {
        return Err(AppError::Forbidden);
    }
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
