use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    UnprocessableEntity(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::UnprocessableEntity(m) => (StatusCode::UNPROCESSABLE_ENTITY, m.clone()),
            AppError::Sqlx(e) => {
                if let sqlx::Error::Database(db) = e {
                    if db.code().as_deref() == Some("23505") {
                        return (
                            StatusCode::CONFLICT,
                            Json(json!({"error": "already exists"})),
                        )
                            .into_response();
                    }
                }
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into()),
        };
        (status, Json(json!({"error": msg}))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
