use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::errors::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // user UUID
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_token(user_id: Uuid, email: &str, role: &str, secret: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub:   user_id.to_string(),
        email: email.to_string(),
        role:  role.to_string(),
        iat:   now.timestamp(),
        exp:   (now + Duration::days(30)).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("JWT encode error: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
        _ => AppError::TokenInvalid,
    })?;
    Ok(data.claims)
}
