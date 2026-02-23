use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Creates a connection pool tuned for Neon serverless (low max connections).
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
