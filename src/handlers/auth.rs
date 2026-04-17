use axum::{
    extract::{Extension, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    models::User,
    utils::{
        errors::{AppError, AppResult},
        generate_token,
        jwt::create_token,
        password::{hash_password, verify_password},
    },
};

// ─── Request / Response types ──────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email:    String,
    pub password: Option<String>,  // optional — supports magic-link only
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email:    Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,  // omit → send magic link
}

#[derive(Deserialize)]
pub struct MagicLinkRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub token:    String,
    pub password: String,
}

// ─── Helpers ───────────────────────────────────────────────────

fn user_response(user: &User, token: &str) -> Value {
    json!({
        "success": true,
        "token": token,
        "user": {
            "id":            user.id,
            "username":      user.username,
            "email":         user.email,
            "bio":           user.bio,
            "avatarUrl":     user.avatar_url,
            "role":          user.role,
            "isVerified":    user.is_verified,
            "chaptersRead":  user.chapters_read,
            "libraryCount":  user.library_count,
            "bookmarkCount": user.bookmark_count,
            "createdAt":     user.created_at,
        }
    })
}

// ─── POST /api/auth/register ───────────────────────────────────
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<Json<Value>> {
    // Validation
    let username = body.username.trim().to_string();
    let email    = body.email.trim().to_lowercase();

    if username.len() < 3 {
        return Err(AppError::BadRequest("Username must be at least 3 characters".into()));
    }
    if !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    if let Some(ref pw) = body.password {
        if pw.len() < 6 {
            return Err(AppError::BadRequest("Password must be at least 6 characters".into()));
        }
    }

    // Check duplicates
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE email = $1 OR username = $2"
    )
    .bind(&email)
    .bind(&username)
    .fetch_one(&state.pool)
    .await?;

    if existing > 0 {
        return Err(AppError::Conflict("Email or username already taken".into()));
    }

    // Hash password if provided
    let password_hash = match &body.password {
        Some(pw) => Some(hash_password(pw)?),
        None     => None,
    };

    // Insert user
    let user = sqlx::query_as::<_, User>(
        r#"INSERT INTO users (username, email, password_hash)
           VALUES ($1, $2, $3)
           RETURNING *"#,
    )
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await?;

    // Create & store verification token
    let token_str = generate_token(32);
    let expires   = Utc::now() + chrono::Duration::hours(24);
    sqlx::query(
        "INSERT INTO email_verifications (user_id, token, expires_at) VALUES ($1, $2, $3)"
    )
    .bind(user.id)
    .bind(&token_str)
    .bind(expires)
    .execute(&state.pool)
    .await?;

    // Send verification email (non-fatal)
    if let Err(e) = state.email.send_verification_email(&user.email, &user.username, &token_str).await {
        tracing::warn!("Verification email failed: {e}");
    }

    let jwt = create_token(user.id, &user.email, &user.role, &state.jwt_secret)?;
    Ok(Json(user_response(&user, &jwt)))
}

// ─── POST /api/auth/login ──────────────────────────────────────
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<Value>> {
    // Resolve user by email or username
    let user = if let Some(ref email) = body.email {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email.trim().to_lowercase())
            .fetch_optional(&state.pool)
            .await?
    } else if let Some(ref username) = body.username {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username.trim())
            .fetch_optional(&state.pool)
            .await?
    } else {
        return Err(AppError::BadRequest("Provide email or username".into()));
    };

    let user = user.ok_or_else(|| AppError::Auth("Invalid credentials".into()))?;

    // Magic link login — no password provided
    if body.password.is_none() {
        let token_str = generate_token(32);
        let expires   = Utc::now() + chrono::Duration::minutes(15);
        sqlx::query(
            "INSERT INTO magic_links (user_id, token, expires_at) VALUES ($1, $2, $3)"
        )
        .bind(user.id)
        .bind(&token_str)
        .bind(expires)
        .execute(&state.pool)
        .await?;

        state.email.send_magic_link_email(&user.email, &user.username, &token_str).await?;

        return Ok(Json(json!({
            "success": true,
            "magic_link_sent": true,
            "message": "Magic sign-in link sent to your email"
        })));
    }

    // Password login
    let password = body.password.as_ref().unwrap();
    let hash = user.password_hash.as_deref()
        .ok_or_else(|| AppError::Auth("This account uses magic link login".into()))?;

    if !verify_password(password, hash)? {
        return Err(AppError::Auth("Invalid credentials".into()));
    }

    let jwt = create_token(user.id, &user.email, &user.role, &state.jwt_secret)?;
    Ok(Json(user_response(&user, &jwt)))
}

// ─── GET /api/auth/magic-login?token=XYZ ──────────────────────
pub async fn magic_login(
    State(state): State<AppState>,
    Query(q): Query<TokenQuery>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query!(
        r#"SELECT ml.id, ml.user_id, ml.expires_at, ml.used_at
           FROM magic_links ml
           WHERE ml.token = $1"#,
        &q.token
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::TokenInvalid)?;

    if row.used_at.is_some() {
        return Err(AppError::Auth("Magic link already used".into()));
    }
    if row.expires_at < Utc::now() {
        return Err(AppError::TokenExpired);
    }

    // Mark as used
    sqlx::query("UPDATE magic_links SET used_at = NOW() WHERE id = $1")
        .bind(row.id)
        .execute(&state.pool)
        .await?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(row.user_id)
        .fetch_one(&state.pool)
        .await?;

    let jwt = create_token(user.id, &user.email, &user.role, &state.jwt_secret)?;
    Ok(Json(user_response(&user, &jwt)))
}

// ─── GET /api/auth/verify-email?token=XYZ ─────────────────────
pub async fn verify_email(
    State(state): State<AppState>,
    Query(q): Query<TokenQuery>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query!(
        "SELECT id, user_id, expires_at, used_at FROM email_verifications WHERE token = $1",
        &q.token
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::TokenInvalid)?;

    if row.used_at.is_some() {
        return Err(AppError::Auth("Verification token already used".into()));
    }
    if row.expires_at < Utc::now() {
        return Err(AppError::TokenExpired);
    }

    sqlx::query("UPDATE email_verifications SET used_at = NOW() WHERE id = $1")
        .bind(row.id)
        .execute(&state.pool)
        .await?;

    sqlx::query("UPDATE users SET is_verified = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(row.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({ "success": true, "message": "Email verified successfully" })))
}

// ─── POST /api/auth/resend-verification ───────────────────────
pub async fn resend_verification(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Value>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if user.is_verified {
        return Err(AppError::BadRequest("Email already verified".into()));
    }

    // Invalidate old tokens
    sqlx::query("DELETE FROM email_verifications WHERE user_id = $1")
        .bind(auth.id)
        .execute(&state.pool)
        .await?;

    let token_str = generate_token(32);
    let expires   = Utc::now() + chrono::Duration::hours(24);
    sqlx::query(
        "INSERT INTO email_verifications (user_id, token, expires_at) VALUES ($1, $2, $3)"
    )
    .bind(auth.id)
    .bind(&token_str)
    .bind(expires)
    .execute(&state.pool)
    .await?;

    state.email.send_verification_email(&user.email, &user.username, &token_str).await?;
    Ok(Json(json!({ "success": true, "message": "Verification email resent" })))
}

// ─── POST /api/auth/forgot-password ───────────────────────────
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> AppResult<Json<Value>> {
    let email = body.email.trim().to_lowercase();
    let user  = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;

    // Always respond success to prevent email enumeration
    if let Some(user) = user {
        let token_str = generate_token(32);
        let expires   = Utc::now() + chrono::Duration::hours(1);

        sqlx::query("DELETE FROM password_resets WHERE user_id = $1")
            .bind(user.id)
            .execute(&state.pool)
            .await?;

        sqlx::query(
            "INSERT INTO password_resets (user_id, token, expires_at) VALUES ($1, $2, $3)"
        )
        .bind(user.id)
        .bind(&token_str)
        .bind(expires)
        .execute(&state.pool)
        .await?;

        if let Err(e) = state.email.send_password_reset_email(&user.email, &user.username, &token_str).await {
            tracing::warn!("Password reset email failed: {e}");
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "If this email exists, a reset link has been sent"
    })))
}

// ─── POST /api/auth/reset-password ────────────────────────────
pub async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetPasswordRequest>,
) -> AppResult<Json<Value>> {
    if body.password.len() < 6 {
        return Err(AppError::BadRequest("Password must be at least 6 characters".into()));
    }

    let row = sqlx::query!(
        "SELECT id, user_id, expires_at, used_at FROM password_resets WHERE token = $1",
        &body.token
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::TokenInvalid)?;

    if row.used_at.is_some() {
        return Err(AppError::Auth("Reset token already used".into()));
    }
    if row.expires_at < Utc::now() {
        return Err(AppError::TokenExpired);
    }

    let hash = hash_password(&body.password)?;

    sqlx::query("UPDATE password_resets SET used_at = NOW() WHERE id = $1")
        .bind(row.id)
        .execute(&state.pool)
        .await?;

    sqlx::query(
        "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(&hash)
    .bind(row.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true, "message": "Password reset successfully" })))
}

// ─── POST /api/auth/logout ─────────────────────────────────────
pub async fn logout() -> Json<Value> {
    // Stateless JWT: client discards token. Nothing server-side needed.
    Json(json!({ "success": true }))
}

// ─── GET /api/auth/me ─────────────────────────────────────────
pub async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Value>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    Ok(Json(json!({
        "success": true,
        "user": {
            "id":            user.id,
            "username":      user.username,
            "email":         user.email,
            "bio":           user.bio,
            "avatarUrl":     user.avatar_url,
            "role":          user.role,
            "isVerified":    user.is_verified,
            "chaptersRead":  user.chapters_read,
            "libraryCount":  user.library_count,
            "bookmarkCount": user.bookmark_count,
            "createdAt":     user.created_at,
        }
    })))
}
