use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;

/// Creates a connection pool tuned for Neon serverless.
///
/// Statement caching is disabled because `DATABASE_URL` points to Neon's
/// PgBouncer pooler (transaction mode), which drops prepared statements
/// between transactions.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let options = PgConnectOptions::from_str(database_url)?.statement_cache_capacity(0);
    PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}
