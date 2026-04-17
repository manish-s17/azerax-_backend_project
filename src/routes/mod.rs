use axum::{
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::json;

use crate::{
    db::AppState,
    handlers::{auth, books, news, orders, users},
    middleware::auth::{require_admin, require_auth},
};

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .merge(auth_routes(state.clone()))
        .merge(user_routes(state.clone()))
        .merge(book_routes(state.clone()))
        .merge(cart_routes(state.clone()))
        .merge(order_routes(state.clone()))
        .merge(news_routes(state.clone()))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "aezarx-backend", "version": env!("CARGO_PKG_VERSION") }))
}

fn auth_routes(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/auth/register",            post(auth::register))
        .route("/api/auth/login",               post(auth::login))
        .route("/auth/magic-login",         get(auth::magic_login))
        .route("/api/auth/verify-email",        get(auth::verify_email))
        .route("/api/auth/forgot-password",     post(auth::forgot_password))
        .route("/api/auth/reset-password",      post(auth::reset_password))
        .route("/api/auth/logout",              post(auth::logout));

    let protected = Router::new()
        .route("/api/auth/me",                  get(auth::me))
        .route("/api/auth/resend-verification", post(auth::resend_verification))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new().merge(public).merge(protected).with_state(state)
}

fn user_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/users/{id}",                     get(users::get_user))
        .route("/api/users/{id}",                     put(users::update_user))
        .route("/api/users/{id}/avatar",              put(users::upload_avatar))
        .route("/api/users/{id}/library",             get(users::get_library))
        .route("/api/users/{id}/library",             post(users::add_to_library))
        .route("/api/users/{id}/library/{book_id}",   delete(users::remove_from_library))
        .route("/api/users/{id}/history",             get(users::get_history))
        .route("/api/users/{id}/bookmarks",           get(users::get_bookmarks))
        .route("/api/users/{id}/bookmarks",           post(users::add_bookmark))
        .route("/api/users/{id}/bookmarks/{bm_id}",   delete(users::remove_bookmark))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

fn book_routes(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/manga",                                   get(books::list_books))
        .route("/api/manga/{id}",                              get(books::get_book))
        .route("/api/manga/{book_id}/chapters",                get(books::get_chapters))
        .route("/api/manga/{book_id}/chapter/{chapter}",       get(books::get_chapter))
        .route("/api/trending",                                get(books::get_trending));

    let admin = Router::new()
        .route("/api/manga",                                   post(books::create_book))
        .route("/api/manga/{id}",                              put(books::update_book))
        .route("/api/manga/{id}",                              delete(books::delete_book))
        .route("/api/manga/{book_id}/chapters",                post(books::add_chapter))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let protected = Router::new()
        .route("/api/manga/{book_id}/chapter/{chapter}/progress", post(books::save_progress))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new().merge(public).merge(admin).merge(protected).with_state(state)
}
fn cart_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/cart",            get(orders::get_cart))
        .route("/api/cart",            post(orders::add_to_cart))
        .route("/api/cart/{item_id}",  delete(orders::remove_from_cart))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

fn order_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/orders", post(orders::checkout))
        .route("/api/orders", get(orders::get_orders))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

fn news_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/news", get(news::get_news))
        .with_state(state)
}


