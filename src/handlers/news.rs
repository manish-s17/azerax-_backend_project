use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    db::AppState,
    utils::errors::AppResult,
};

#[derive(Deserialize)]
pub struct NewsQuery {
    pub category: Option<String>,
    pub page:     Option<i64>,
}

pub async fn get_news(
    State(state): State<AppState>,
    Query(q): Query<NewsQuery>,
) -> AppResult<Json<Value>> {
    let page     = q.page.unwrap_or(1).max(1);
    let per_page = 20i64;
    let offset   = (page - 1) * per_page;

    let news = match q.category.as_deref() {
        Some(cat) => sqlx::query!(
            "SELECT * FROM news WHERE category = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            cat, per_page, offset
        )
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|r| json!({
            "id":        r.id,
            "title":     r.title,
            "summary":   r.summary,
            "cat":       r.category,
            "tag":       r.tag,
            "img":       r.img_url,
            "author":    r.author,
            "hot":       r.is_hot,
            "time":      r.created_at,
        }))
        .collect::<Vec<_>>(),

        None => sqlx::query!(
            "SELECT * FROM news ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            per_page, offset
        )
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|r| json!({
            "id":      r.id,
            "title":   r.title,
            "summary": r.summary,
            "cat":     r.category,
            "tag":     r.tag,
            "img":     r.img_url,
            "author":  r.author,
            "hot":     r.is_hot,
            "time":    r.created_at,
        }))
        .collect::<Vec<_>>(),
    };

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM news")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(json!({ "success": true, "news": news, "total": total, "page": page })))
}
