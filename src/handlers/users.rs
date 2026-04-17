use axum::{
    extract::{Extension, Multipart, Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::fs;
use uuid::Uuid;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    models::User,
    utils::errors::{AppError, AppResult},
};

// ─── GET /api/users/:id ────────────────────────────────────────
pub async fn get_user(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
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

// ─── PUT /api/users/:id ────────────────────────────────────────
#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub bio:      Option<String>,
}

pub async fn update_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> AppResult<Json<Value>> {
    if auth.id != id && auth.role != "admin" {
        return Err(AppError::Forbidden("Cannot update another user's profile".into()));
    }

    if let Some(ref u) = body.username {
        if u.trim().len() < 3 {
            return Err(AppError::BadRequest("Username must be at least 3 characters".into()));
        }
    }

    let user = sqlx::query_as::<_, User>(
        r#"UPDATE users SET
            username   = COALESCE($1, username),
            bio        = COALESCE($2, bio),
            updated_at = NOW()
           WHERE id = $3
           RETURNING *"#,
    )
    .bind(body.username.as_deref().map(str::trim))
    .bind(body.bio.as_deref())
    .bind(id)
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

// ─── PUT /api/users/:id/avatar ────────────────────────────────
pub async fn upload_avatar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    if auth.id != id && auth.role != "admin" {
        return Err(AppError::Forbidden("Cannot update another user's avatar".into()));
    }

    let dir = format!("{}/avatars", state.upload_dir);
    fs::create_dir_all(&dir).await
        .map_err(|e| AppError::Internal(format!("Cannot create upload dir: {e}")))?;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() != Some("avatar") { continue; }

        let ext = field.file_name()
            .and_then(|n| n.rsplit('.').next())
            .map(|e| e.to_lowercase())
            .filter(|e| matches!(e.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif"))
            .unwrap_or("jpg".to_string());

        let filename = format!("user_{}_{}.{}", id, chrono::Utc::now().timestamp(), ext);
        let filepath = format!("{}/{}", dir, filename);
        let url      = format!("/uploads/avatars/{}", filename);

        let bytes = field.bytes().await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        fs::write(&filepath, &bytes).await
            .map_err(|e| AppError::Internal(format!("File write failed: {e}")))?;

        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2 RETURNING *"
        )
        .bind(&url)
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

        return Ok(Json(json!({
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
        })));
    }

    Err(AppError::BadRequest("No avatar field found in multipart form".into()))
}

// ─── GET /api/users/:id/library ───────────────────────────────
pub async fn get_library(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    if auth.id != id && auth.role != "admin" {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let rows = sqlx::query!(
        r#"SELECT ul.id, ul.book_id, ul.last_chapter, ul.added_at,
                  b.title, b.img_url, b.genre
           FROM user_library ul
           JOIN books b ON b.id = ul.book_id
           WHERE ul.user_id = $1
           ORDER BY ul.added_at DESC"#,
        id
    )
    .fetch_all(&state.pool)
    .await?;

    let library: Vec<Value> = rows.iter().map(|r| json!({
        "id":          r.id,
        "bookId":      r.book_id,
        "title":       r.title,
        "img":         r.img_url,
        "genre":       r.genre,
        "lastChapter": r.last_chapter,
        "addedAt":     r.added_at,
    })).collect();

    Ok(Json(json!({ "success": true, "library": library })))
}

// ─── POST /api/users/:id/library ──────────────────────────────
#[derive(Deserialize)]
pub struct AddLibraryRequest {
    #[serde(rename = "bookId")]
    pub book_id: Uuid,
}

pub async fn add_to_library(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<AddLibraryRequest>,
) -> AppResult<Json<Value>> {
    if auth.id != id {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    // Check book exists
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM books WHERE id = $1)")
        .bind(body.book_id)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(AppError::NotFound("Book not found".into()));
    }

    sqlx::query(
        "INSERT INTO user_library (user_id, book_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(id)
    .bind(body.book_id)
    .execute(&state.pool)
    .await?;

    // Update count
    sqlx::query(
        "UPDATE users SET library_count = (SELECT COUNT(*) FROM user_library WHERE user_id = $1) WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true })))
}

// ─── DELETE /api/users/:id/library/:book_id ───────────────────
pub async fn remove_from_library(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((id, book_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    if auth.id != id {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    sqlx::query("DELETE FROM user_library WHERE user_id = $1 AND book_id = $2")
        .bind(id)
        .bind(book_id)
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "UPDATE users SET library_count = (SELECT COUNT(*) FROM user_library WHERE user_id = $1) WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "success": true })))
}

// ─── GET /api/users/:id/history ───────────────────────────────
pub async fn get_history(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    if auth.id != id && auth.role != "admin" {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let rows = sqlx::query!(
        r#"SELECT rh.id, rh.book_id, rh.chapter_num, rh.page_num, rh.read_at,
                  b.title, b.img_url
           FROM reading_history rh
           JOIN books b ON b.id = rh.book_id
           WHERE rh.user_id = $1
           ORDER BY rh.read_at DESC"#,
        id
    )
    .fetch_all(&state.pool)
    .await?;

    let history: Vec<Value> = rows.iter().map(|r| json!({
        "id":      r.id,
        "bookId":  r.book_id,
        "title":   r.title,
        "img":     r.img_url,
        "chapter": r.chapter_num,
        "page":    r.page_num,
        "readAt":  r.read_at,
    })).collect();

    Ok(Json(json!({ "success": true, "history": history })))
}

// ─── GET /api/users/:id/bookmarks ─────────────────────────────
pub async fn get_bookmarks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    if auth.id != id && auth.role != "admin" {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let rows = sqlx::query!(
        r#"SELECT bm.id, bm.book_id, bm.chapter_num, bm.page_num, bm.created_at,
                  b.title, b.img_url
           FROM bookmarks bm
           JOIN books b ON b.id = bm.book_id
           WHERE bm.user_id = $1
           ORDER BY bm.created_at DESC"#,
        id
    )
    .fetch_all(&state.pool)
    .await?;

    let bookmarks: Vec<Value> = rows.iter().map(|r| json!({
        "id":        r.id,
        "bookId":    r.book_id,
        "title":     r.title,
        "img":       r.img_url,
        "chapter":   r.chapter_num,
        "page":      r.page_num,
        "createdAt": r.created_at,
    })).collect();

    Ok(Json(json!({ "success": true, "bookmarks": bookmarks })))
}

// ─── POST /api/users/:id/bookmarks ────────────────────────────
#[derive(Deserialize)]
pub struct AddBookmarkRequest {
    #[serde(rename = "bookId")]
    pub book_id:     Uuid,
    pub chapter:     i32,
    pub page:        i32,
}

pub async fn add_bookmark(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<AddBookmarkRequest>,
) -> AppResult<Json<Value>> {
    if auth.id != id {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let bm = sqlx::query!(
        r#"INSERT INTO bookmarks (user_id, book_id, chapter_num, page_num)
           VALUES ($1, $2, $3, $4) RETURNING id, created_at"#,
        id, body.book_id, body.chapter, body.page
    )
    .fetch_one(&state.pool)
    .await?;

    sqlx::query(
        "UPDATE users SET bookmark_count = (SELECT COUNT(*) FROM bookmarks WHERE user_id = $1) WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "bookmark": { "id": bm.id, "bookId": body.book_id, "chapter": body.chapter, "page": body.page, "createdAt": bm.created_at }
    })))
}

// ─── DELETE /api/users/:id/bookmarks/:bm_id ───────────────────
pub async fn remove_bookmark(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((id, bm_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    if auth.id != id {
        return Err(AppError::Forbidden("Access denied".into()));
    }
    sqlx::query("DELETE FROM bookmarks WHERE id = $1 AND user_id = $2")
        .bind(bm_id)
        .bind(id)
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "UPDATE users SET bookmark_count = (SELECT COUNT(*) FROM bookmarks WHERE user_id = $1) WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "success": true })))
}
