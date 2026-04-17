use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Email error: {0}")]
    Email(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Token invalid")]
    TokenInvalid,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Auth(msg)        => (StatusCode::UNAUTHORIZED,           msg.clone()),
            AppError::NotFound(msg)    => (StatusCode::NOT_FOUND,              msg.clone()),
            AppError::BadRequest(msg)  => (StatusCode::BAD_REQUEST,            msg.clone()),
            AppError::Conflict(msg)    => (StatusCode::CONFLICT,               msg.clone()),
            AppError::Forbidden(msg)   => (StatusCode::FORBIDDEN,              msg.clone()),
            AppError::TokenExpired     => (StatusCode::UNAUTHORIZED,           "Token expired".to_string()),
            AppError::TokenInvalid     => (StatusCode::UNAUTHORIZED,           "Invalid token".to_string()),
            AppError::Database(e)      => {
                tracing::error!("DB error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::Email(msg)       => {
                tracing::error!("Email error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Email delivery failed".to_string())
            }
            AppError::Internal(msg)    => {
                tracing::error!("Internal: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        (
            status,
            Json(json!({ "success": false, "error": message })),
        )
            .into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
