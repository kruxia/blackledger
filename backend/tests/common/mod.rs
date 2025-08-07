use blackledger::{api::AppState, auth, db};
use sqlx::PgPool;
use std::sync::{Arc, Once};

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

    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://blackledger_test:test@localhost:5434/blackledger_test".to_string()
    });

    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create test database pool");

    // Clean the database before running tests
    clean_database(&pool).await;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

async fn clean_database(pool: &PgPool) {
    // Delete all data in reverse order of dependencies
    sqlx::query("DELETE FROM entry").execute(pool).await.ok();
    sqlx::query("DELETE FROM transaction")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM account").execute(pool).await.ok();
    sqlx::query("DELETE FROM ledger").execute(pool).await.ok();
    sqlx::query("DELETE FROM currency").execute(pool).await.ok();
}

pub async fn setup_test_app_state() -> AppState {
    let pool = setup_test_db().await;

    // Create a mock JWT validator with auth disabled for tests
    let auth_config = auth::AuthConfig {
        enabled: false,
        jwks_url: None,
        audience: None,
        issuer: None,
    };

    let jwt_validator = Arc::new(
        auth::JwtValidator::new(auth_config)
            .await
            .expect("Failed to create JWT validator"),
    );

    AppState {
        pool,
        jwt_validator,
    }
}
