use blackledger::db;
use sqlx::PgPool;
use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("blackledger=debug,sqlx=warn")
            .with_test_writer()
            .init();
    });
}

pub async fn setup_test_db() -> PgPool {
    setup_test_logging();
    
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://blackledger_test:test@localhost:5434/blackledger_test".to_string());
    
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create test database pool");
    
    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    
    pool
}