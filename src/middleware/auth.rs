use axum::{
    body::Body,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    db::AppState,
    utils::{errors::AppError, jwt::verify_token},
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id:    Uuid,
    pub email: String,
    pub role:  String,
}

/// Extracts and validates Bearer token — injects AuthUser into extensions
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Auth("Missing or malformed Authorization header".to_string()))?;

    let claims = verify_token(token, &state.jwt_secret)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::TokenInvalid)?;

    req.extensions_mut().insert(AuthUser {
        id:    user_id,
        email: claims.email,
        role:  claims.role,
    });

    Ok(next.run(req).await)
}

/// Middleware that requires admin role
pub async fn require_admin(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Auth("Missing Authorization header".to_string()))?;

    let claims = verify_token(token, &state.jwt_secret)?;

    if claims.role != "admin" {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::TokenInvalid)?;
    req.extensions_mut().insert(AuthUser {
        id:    user_id,
        email: claims.email,
        role:  claims.role,
    });

    Ok(next.run(req).await)
}
