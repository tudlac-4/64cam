use axum::{extract::State, http::StatusCode, Json};
use common::auth::{generate_refresh_token, hash_token, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::CurrentUser,
    state::AppState,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(FromRow)]
struct LoginRow {
    id: Uuid,
    email: String,
    password_hash: String,
    role_name: String,
}

#[derive(FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    email: String,
    role_name: String,
}

#[utoipa::path(post, path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses((status = 200, body = TokenResponse), (status = 401, description = "Bad credentials"))
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>> {
    let row = sqlx::query_as::<_, LoginRow>(
        "SELECT u.id, u.email, u.password_hash, r.name AS role_name
         FROM users u JOIN roles r ON r.id = u.role_id
         WHERE u.email = $1",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Sqlx)?
    .ok_or(AppError::Unauthorized)?;

    if !verify_password(&body.password, &row.password_hash).map_err(anyhow::Error::from)? {
        return Err(AppError::Unauthorized);
    }

    let claims = state.jwt.make_claims(row.id, &row.email, &row.role_name);
    let access_token = state.jwt.encode(&claims).map_err(anyhow::Error::from)?;
    let refresh_token = generate_refresh_token();
    let token_hash = hash_token(&refresh_token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(row.id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .map_err(AppError::Sqlx)?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".into(),
        expires_in: state.jwt.access_ttl_secs,
    }))
}

#[utoipa::path(post, path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses((status = 200, body = TokenResponse), (status = 401, description = "Invalid or expired token"))
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>> {
    let token_hash = hash_token(&body.refresh_token);

    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT s.id, u.id AS user_id, u.email, r.name AS role_name
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         JOIN roles r ON r.id = u.role_id
         WHERE s.token_hash = $1 AND s.expires_at > NOW()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Sqlx)?
    .ok_or(AppError::Unauthorized)?;

    let new_refresh = generate_refresh_token();
    let new_hash = hash_token(&new_refresh);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query("UPDATE sessions SET token_hash = $1, expires_at = $2 WHERE id = $3")
        .bind(&new_hash)
        .bind(expires_at)
        .bind(session.id)
        .execute(&state.db)
        .await
        .map_err(AppError::Sqlx)?;

    let claims = state.jwt.make_claims(session.user_id, &session.email, &session.role_name);
    let access_token = state.jwt.encode(&claims).map_err(anyhow::Error::from)?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh,
        token_type: "Bearer".into(),
        expires_in: state.jwt.access_ttl_secs,
    }))
}

#[utoipa::path(post, path = "/api/v1/auth/logout",
    responses((status = 204, description = "Logged out"), (status = 401, description = "Unauthorized")),
    security(("bearer_token" = []))
)]
pub async fn logout(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<StatusCode> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await
        .map_err(AppError::Sqlx)?;
    Ok(StatusCode::NO_CONTENT)
}
