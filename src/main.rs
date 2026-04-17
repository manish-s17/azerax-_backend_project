use axum::Router;
use dotenvy::dotenv;
use std::{env, net::SocketAddr};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod handlers;
mod middleware;
mod models;
mod routes;
mod utils;

use db::{create_pool, AppState};
use utils::email::EmailService;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "aezarx_backend=debug,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret   = env::var("JWT_SECRET").unwrap_or_else(|_| "aezarx-change-this-in-production-secret".into());
    let app_url      = env::var("APP_URL").unwrap_or_else(|_| "http://localhost:4000".into());
    let port: u16    = env::var("PORT").unwrap_or_else(|_| "4000".into()).parse().expect("PORT must be a number");
    let upload_dir   = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".into());

    let smtp_host     = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".into());
    let smtp_port: u16= env::var("SMTP_PORT").unwrap_or_else(|_| "587".into()).parse().unwrap_or(587);
    let smtp_user     = env::var("SMTP_USER").unwrap_or_default();
    let smtp_pass     = env::var("SMTP_PASS").unwrap_or_default();
    let from_email    = env::var("FROM_EMAIL").unwrap_or_else(|_| format!("AEZARX <{}>", smtp_user));

    tracing::info!("Connecting to PostgreSQL…");
    let pool = create_pool(&database_url).await.expect("Failed to connect to database");

    tracing::info!("Running migrations…");
    sqlx::migrate!("./migrations").run(&pool).await.expect("Migration failed");

    std::fs::create_dir_all(format!("{}/avatars", upload_dir)).expect("Failed to create upload dirs");

    let email_service = EmailService::new(smtp_host, smtp_port, smtp_user, smtp_pass, from_email, app_url.clone());

    let state = AppState {
        pool,
        jwt_secret,
        email: email_service,
        upload_dir: upload_dir.clone(),
        app_url: app_url.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

let app = Router::new()
    .merge(routes::all_routes(state.clone()))
    .nest_service("/uploads",     ServeDir::new(&upload_dir))
    .nest_service("/manga-pages", ServeDir::new("./manga-pages"))
    .nest_service("/images",      ServeDir::new("./public/images"))
    .fallback_service(ServeDir::new("./public")) // ✅ FIXED
    .layer(cors)
    .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🚀 AEZARX backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
