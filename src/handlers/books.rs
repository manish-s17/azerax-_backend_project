use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    models::Book,
    utils::errors::{AppError, AppResult},
};

// ─── Request types ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BookListQuery {
    pub page:      Option<i64>,
    pub per_page:  Option<i64>,
    pub genre:     Option<String>,
    pub book_type: Option<String>,
    pub search:    Option<String>,
}

#[derive(Deserialize)]
pub struct CreateBookRequest {
    pub title:       String,
    pub author:      Option<String>,
    pub description: Option<String>,
    pub img_url:     Option<String>,
    pub price:       Option<i32>,
    pub stock:       Option<i32>,
    pub genre:       Option<String>,
    pub tag:         Option<String>,
    pub rating:      Option<f64>,
    pub book_type:   Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBookRequest {
    pub title:       Option<String>,
    pub author:      Option<String>,
    pub description: Option<String>,
    pub img_url:     Option<String>,
    pub price:       Option<i32>,
    pub stock:       Option<i32>,
    pub genre:       Option<String>,
    pub tag:         Option<String>,
    pub rating:      Option<f64>,
    pub is_active:   Option<bool>,
}

#[derive(Deserialize)]
pub struct ChapterRequest {
    pub title: Option<String>,
    pub pages: Vec<String>,  // filenames
}

#[derive(Deserialize)]
pub struct ProgressRequest {
    pub page: i32,
}

// ─── GET /api/manga  (all books, paginated) ────────────────────
pub async fn list_books(
    State(state): State<AppState>,
    Query(q): Query<BookListQuery>,
) -> AppResult<Json<Value>> {
    let page     = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).min(100);
    let offset   = (page - 1) * per_page;

    // Build dynamic WHERE clauses
    let mut conditions = vec!["b.is_active = TRUE".to_string()];
    let mut params: Vec<Box<dyn Send + Sync>> = vec![];
    let mut idx = 1usize;

    if let Some(ref genre) = q.genre {
        params.push(Box::new(genre.clone()));
        conditions.push(format!("b.genre = ${idx}"));
        idx += 1;
    }
    if let Some(ref bt) = q.book_type {
        params.push(Box::new(bt.clone()));
        conditions.push(format!("b.book_type = ${idx}"));
        idx += 1;
    }
    if let Some(ref search) = q.search {
        params.push(Box::new(format!("%{}%", search.to_lowercase())));
        conditions.push(format!(
            "(LOWER(b.title) LIKE ${idx} OR LOWER(b.author) LIKE ${idx})"
        ));
        idx += 1;
    }

    // Simple approach without dynamic binding complexity
    let books = match (q.genre.as_deref(), q.book_type.as_deref(), q.search.as_deref()) {
        (Some(genre), Some(bt), Some(s)) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND genre=$1 AND book_type=$2 AND (LOWER(title) LIKE $3 OR LOWER(author) LIKE $3) ORDER BY created_at DESC LIMIT $4 OFFSET $5"
        ).bind(genre).bind(bt).bind(format!("%{}%",s.to_lowercase())).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (Some(genre), Some(bt), None) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND genre=$1 AND book_type=$2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        ).bind(genre).bind(bt).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (Some(genre), None, Some(s)) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND genre=$1 AND (LOWER(title) LIKE $2 OR LOWER(author) LIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        ).bind(genre).bind(format!("%{}%",s.to_lowercase())).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (None, Some(bt), Some(s)) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND book_type=$1 AND (LOWER(title) LIKE $2 OR LOWER(author) LIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        ).bind(bt).bind(format!("%{}%",s.to_lowercase())).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (Some(genre), None, None) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND genre=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ).bind(genre).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (None, Some(bt), None) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND book_type=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ).bind(bt).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (None, None, Some(s)) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE AND (LOWER(title) LIKE $1 OR LOWER(author) LIKE $1) ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ).bind(format!("%{}%",s.to_lowercase())).bind(per_page).bind(offset).fetch_all(&state.pool).await?,

        (None, None, None) => sqlx::query_as::<_, Book>(
            "SELECT * FROM books WHERE is_active=TRUE ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        ).bind(per_page).bind(offset).fetch_all(&state.pool).await?,
    };

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books WHERE is_active=TRUE")
        .fetch_one(&state.pool)
        .await?;

    let book_list: Vec<Value> = books.iter().map(|b| book_to_json(b)).collect();

    Ok(Json(json!({
        "success": true,
        "books":   book_list,
        "total":   total,
        "page":    page,
        "perPage": per_page,
    })))
}

// ─── GET /api/manga/:id ────────────────────────────────────────
pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = $1 AND is_active = TRUE")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    Ok(Json(json!({ "success": true, "book": book_to_json(&book) })))
}

// ─── POST /api/manga  (admin only) ────────────────────────────
pub async fn create_book(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(body): Json<CreateBookRequest>,
) -> AppResult<Json<Value>> {
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::BadRequest("Title is required".into()));
    }

    let book = sqlx::query_as::<_, Book>(
        r#"INSERT INTO books (title, author, description, img_url, price, stock, genre, tag, rating, book_type)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING *"#
    )
    .bind(&title)
    .bind(body.author.as_deref().unwrap_or("Unknown"))
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(body.img_url.as_deref().unwrap_or(""))
    .bind(body.price.unwrap_or(0))
    .bind(body.stock.unwrap_or(0))
    .bind(body.genre.as_deref().unwrap_or("manga"))
    .bind(body.tag.as_deref().unwrap_or(""))
    .bind(body.rating.unwrap_or(0.0))
    .bind(body.book_type.as_deref().unwrap_or("manga"))
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true, "book": book_to_json(&book) })))
}

// ─── PUT /api/manga/:id  (admin only) ─────────────────────────
pub async fn update_book(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBookRequest>,
) -> AppResult<Json<Value>> {
    let book = sqlx::query_as::<_, Book>(
        r#"UPDATE books SET
            title       = COALESCE($1, title),
            author      = COALESCE($2, author),
            description = COALESCE($3, description),
            img_url     = COALESCE($4, img_url),
            price       = COALESCE($5, price),
            stock       = COALESCE($6, stock),
            genre       = COALESCE($7, genre),
            tag         = COALESCE($8, tag),
            is_active   = COALESCE($9, is_active),
            updated_at  = NOW()
           WHERE id = $10
           RETURNING *"#
    )
    .bind(body.title)
    .bind(body.author)
    .bind(body.description)
    .bind(body.img_url)
    .bind(body.price)
    .bind(body.stock)
    .bind(body.genre)
    .bind(body.tag)
    .bind(body.is_active)
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    Ok(Json(json!({ "success": true, "book": book_to_json(&book) })))
}

// ─── DELETE /api/manga/:id  (admin only, soft delete) ─────────
pub async fn delete_book(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let affected = sqlx::query(
        "UPDATE books SET is_active = FALSE, updated_at = NOW() WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound("Book not found".into()));
    }

    Ok(Json(json!({ "success": true })))
}

// ─── GET /api/manga/:bookId/chapters ──────────────────────────
pub async fn get_chapters(
    State(state): State<AppState>,
    Path(book_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let chapters = sqlx::query!(
        "SELECT chapter_num FROM manga_chapters WHERE book_id = $1 ORDER BY chapter_num",
        book_id
    )
    .fetch_all(&state.pool)
    .await?;

    let nums: Vec<i32> = chapters.iter().map(|r| r.chapter_num).collect();
    Ok(Json(json!({ "success": true, "bookId": book_id, "chapters": nums })))
}

// ─── GET /api/manga/:bookId/chapter/:chapter ──────────────────
pub async fn get_chapter(
    State(state): State<AppState>,
    Path((book_id, chapter_num)): Path<(Uuid, i32)>,
) -> AppResult<Json<Value>> {
    let book = sqlx::query!("SELECT title FROM books WHERE id = $1", book_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    let chapter = sqlx::query!(
        "SELECT id, title, pages FROM manga_chapters WHERE book_id = $1 AND chapter_num = $2",
        book_id, chapter_num
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Chapter {} not found", chapter_num)))?;

    let base_url = format!("/manga-pages/{}/{}", book_id, chapter_num);
    let pages_raw = chapter.pages.as_array().cloned().unwrap_or_default();
    let total   = pages_raw.len();

    let pages: Vec<Value> = pages_raw.iter().enumerate().map(|(i, filename)| {
        let fname = filename.as_str().unwrap_or("001.jpg");
        json!({
            "page": i + 1,
            "url":  format!("{}/{}", base_url, fname),
            "alt":  format!("{} Ch{} P{}", book.title, chapter_num, i + 1),
        })
    }).collect();

    Ok(Json(json!({
        "success":    true,
        "bookId":     book_id,
        "chapter":    chapter_num,
        "totalPages": total,
        "bookTitle":  book.title,
        "pages":      pages,
    })))
}

// ─── POST /api/manga/:bookId/chapters  (admin) ────────────────
pub async fn add_chapter(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(book_id): Path<Uuid>,
    Json(body): Json<ChapterRequest>,
) -> AppResult<Json<Value>> {
    // Get next chapter number
    let next_num: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(chapter_num), 0) + 1 FROM manga_chapters WHERE book_id = $1"
    )
    .bind(book_id)
    .fetch_one(&state.pool)
    .await?;

    let pages_json = serde_json::to_value(&body.pages)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query(
        r#"INSERT INTO manga_chapters (book_id, chapter_num, title, pages)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (book_id, chapter_num) DO UPDATE SET title=$3, pages=$4"#
    )
    .bind(book_id)
    .bind(next_num)
    .bind(body.title.as_deref().unwrap_or(""))
    .bind(&pages_json)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true, "chapterNum": next_num })))
}

// ─── POST /api/manga/:bookId/chapter/:chapter/progress  ───────
pub async fn save_progress(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((book_id, chapter_num)): Path<(Uuid, i32)>,
    Json(body): Json<ProgressRequest>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        r#"INSERT INTO reading_history (user_id, book_id, chapter_num, page_num)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, book_id, chapter_num)
           DO UPDATE SET page_num = $4, read_at = NOW()"#
    )
    .bind(auth.id)
    .bind(book_id)
    .bind(chapter_num)
    .bind(body.page)
    .execute(&state.pool)
    .await?;

    // Update last chapter in library if present
    sqlx::query(
        r#"UPDATE user_library SET last_chapter = GREATEST(last_chapter, $3)
           WHERE user_id = $1 AND book_id = $2"#
    )
    .bind(auth.id)
    .bind(book_id)
    .bind(chapter_num)
    .execute(&state.pool)
    .await?;

    // Update chapters_read count
    sqlx::query(
        "UPDATE users SET chapters_read = (SELECT COUNT(DISTINCT book_id || chapter_num::text) FROM reading_history WHERE user_id = $1) WHERE id = $1"
    )
    .bind(auth.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true })))
}

// ─── GET /api/trending ────────────────────────────────────────
pub async fn get_trending(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    // In production replace with real analytics query
    let books = sqlx::query_as::<_, Book>(
        "SELECT * FROM books WHERE is_active=TRUE ORDER BY rating DESC, created_at DESC LIMIT 8"
    )
    .fetch_all(&state.pool)
    .await?;

    let trending: Vec<Value> = books.iter().enumerate().map(|(i, b)| json!({
        "rank":   i + 1,
        "bookId": b.id,
        "title":  b.title,
        "rating": b.rating,
        "img":    b.img_url,
        "change": if i == 0 { "★ NEW" } else { "—" },
    })).collect();

    Ok(Json(json!({ "success": true, "trending": trending })))
}

// ─── Helper ────────────────────────────────────────────────────
fn book_to_json(b: &Book) -> Value {
    json!({
        "id":          b.id,
        "title":       b.title,
        "author":      b.author,
        "description": b.description,
        "img":         b.img_url,
        "price":       b.price,
        "stock":       b.stock,
        "genre":       b.genre,
        "tag":         b.tag,
        "rating":      b.rating,
        "bookType":    b.book_type,
        "createdAt":   b.created_at,
    })
}
