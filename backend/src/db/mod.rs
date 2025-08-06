use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub mod queries;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pool() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://blackledger_test@localhost:5434/blackledger_test".to_string());
        let pool = create_pool(&database_url).await;
        assert!(pool.is_ok());
    }
}