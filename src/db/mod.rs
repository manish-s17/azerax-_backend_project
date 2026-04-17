use sqlx::PgPool;
use crate::utils::email::EmailService;

#[derive(Clone)]
pub struct AppState {
    pub pool:         PgPool,
    pub jwt_secret:   String,
    pub email:        EmailService,
    pub upload_dir:   String,
    pub app_url:      String,
}

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPool::connect(database_url).await
}
