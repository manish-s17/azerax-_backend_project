use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    utils::errors::{AppError, AppResult},
};

// ─── GET /api/cart ─────────────────────────────────────────────
pub async fn get_cart(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Value>> {
    let rows = sqlx::query!(
        r#"SELECT ci.id, ci.book_id, ci.quantity, ci.added_at,
                  b.title, b.price, b.img_url
           FROM cart_items ci
           JOIN books b ON b.id = ci.book_id
           WHERE ci.user_id = $1
           ORDER BY ci.added_at DESC"#,
        auth.id
    )
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<Value> = rows.iter().map(|r| json!({
        "id":       r.id,
        "bookId":   r.book_id,
        "title":    r.title,
        "price":    r.price,
        "img":      r.img_url,
        "quantity": r.quantity,
        "addedAt":  r.added_at,
    })).collect();

    let total: i64 = rows.iter()
        .map(|r| r.price as i64 * r.quantity as i64)
        .sum();

    Ok(Json(json!({ "success": true, "items": items, "total": total })))
}

// ─── POST /api/cart ────────────────────────────────────────────
#[derive(Deserialize)]
pub struct AddCartRequest {
    #[serde(rename = "bookId")]
    pub book_id:  Uuid,
    pub quantity: Option<i32>,
}

pub async fn add_to_cart(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<AddCartRequest>,
) -> AppResult<Json<Value>> {
    let quantity = body.quantity.unwrap_or(1).max(1);

    // Verify book exists and has stock
    let book = sqlx::query!("SELECT stock, title FROM books WHERE id = $1 AND is_active = TRUE", body.book_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    if book.stock < quantity {
        return Err(AppError::BadRequest(format!("Only {} in stock", book.stock)));
    }

    sqlx::query(
        r#"INSERT INTO cart_items (user_id, book_id, quantity)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id, book_id)
           DO UPDATE SET quantity = cart_items.quantity + $3"#
    )
    .bind(auth.id)
    .bind(body.book_id)
    .bind(quantity)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({ "success": true, "message": format!("'{}' added to cart", book.title) })))
}

// ─── DELETE /api/cart/:item_id ─────────────────────────────────
pub async fn remove_from_cart(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(item_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let affected = sqlx::query(
        "DELETE FROM cart_items WHERE id = $1 AND user_id = $2"
    )
    .bind(item_id)
    .bind(auth.id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound("Cart item not found".into()));
    }

    Ok(Json(json!({ "success": true })))
}

// ─── POST /api/orders (checkout) ──────────────────────────────
pub async fn checkout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Value>> {
    // Fetch cart
    let cart = sqlx::query!(
        r#"SELECT ci.id as cart_id, ci.book_id, ci.quantity, b.price, b.title, b.stock
           FROM cart_items ci
           JOIN books b ON b.id = ci.book_id
           WHERE ci.user_id = $1"#,
        auth.id
    )
    .fetch_all(&state.pool)
    .await?;

    if cart.is_empty() {
        return Err(AppError::BadRequest("Cart is empty".into()));
    }

    // Check stock
    for item in &cart {
        if item.stock < item.quantity {
            return Err(AppError::BadRequest(
                format!("'{}' only has {} in stock", item.title, item.stock)
            ));
        }
    }

    let total_price: i64 = cart.iter().map(|i| i.price as i64 * i.quantity as i64).sum();

    // Begin transaction
    let mut tx = state.pool.begin().await?;

    // Create order
    let order = sqlx::query!(
        "INSERT INTO orders (user_id, total_price) VALUES ($1, $2) RETURNING id, created_at",
        auth.id, total_price as i32
    )
    .fetch_one(&mut *tx)
    .await?;

    // Create order items & decrement stock
    for item in &cart {
        sqlx::query(
            "INSERT INTO order_items (order_id, book_id, quantity, unit_price) VALUES ($1, $2, $3, $4)"
        )
        .bind(order.id)
        .bind(item.book_id)
        .bind(item.quantity)
        .bind(item.price)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE books SET stock = stock - $1 WHERE id = $2")
            .bind(item.quantity)
            .bind(item.book_id)
            .execute(&mut *tx)
            .await?;
    }

    // Clear cart
    sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(auth.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "success":    true,
        "orderId":    order.id,
        "totalPrice": total_price,
        "createdAt":  order.created_at,
        "message":    "Order placed successfully",
    })))
}

// ─── GET /api/orders ───────────────────────────────────────────
pub async fn get_orders(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Value>> {
    let orders = sqlx::query!(
        "SELECT id, total_price, status, created_at FROM orders WHERE user_id = $1 ORDER BY created_at DESC",
        auth.id
    )
    .fetch_all(&state.pool)
    .await?;

    let mut order_list = Vec::new();
    for order in &orders {
        let items = sqlx::query!(
            r#"SELECT oi.quantity, oi.unit_price, b.title, b.img_url
               FROM order_items oi JOIN books b ON b.id = oi.book_id
               WHERE oi.order_id = $1"#,
            order.id
        )
        .fetch_all(&state.pool)
        .await?;

        let items_json: Vec<Value> = items.iter().map(|i| json!({
            "title":     i.title,
            "img":       i.img_url,
            "quantity":  i.quantity,
            "unitPrice": i.unit_price,
        })).collect();

        order_list.push(json!({
            "id":         order.id,
            "totalPrice": order.total_price,
            "status":     order.status,
            "items":      items_json,
            "createdAt":  order.created_at,
        }));
    }

    Ok(Json(json!({ "success": true, "orders": order_list })))
}
