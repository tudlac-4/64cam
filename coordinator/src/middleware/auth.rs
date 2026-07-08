use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
    pub fn is_operator_or_above(&self) -> bool {
        self.role == "admin" || self.role == "operator"
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        let app = AppState::from_ref(state);
        let token = bearer_token(&parts.headers).ok_or(AppError::Unauthorized)?;
        let claims = app.jwt.decode(token).map_err(|_| AppError::Unauthorized)?;
        Ok(Self {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
