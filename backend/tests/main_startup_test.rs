use blackledger::{api, auth, config, db};
use std::sync::Arc;

/// Test that main components can be initialized successfully
#[tokio::test]
async fn test_main_components_initialization() {
    // Set up test environment
    unsafe {
        std::env::set_var(
            "DATABASE_URL",
            "postgresql://blackledger_test:test@localhost:5434/blackledger_test",
        );
        std::env::set_var("PORT", "8000");
        std::env::set_var("AUTH_ENABLED", "false");
    }

    // Test config loading
    let config = config::Config::from_env().expect("Failed to load config");
    assert_eq!(config.port, 8000);
    assert_eq!(config.auth_enabled, false);

    // Test database pool creation
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    // Test migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Test JWT validator creation
    let auth_config = auth::AuthConfig {
        enabled: config.auth_enabled,
        jwks_url: config.jwks_url.clone(),
        audience: None,
        issuer: None,
    };

    let jwt_validator = Arc::new(
        auth::JwtValidator::new(auth_config)
            .await
            .expect("Failed to create JWT validator"),
    );

    // Test app state creation
    let app_state = api::AppState {
        pool: pool.clone(),
        jwt_validator: jwt_validator.clone(),
    };

    // Test router creation
    let _app: axum::Router = axum::Router::new()
        .merge(api::router(app_state.clone()))
        .layer(api::cors::cors_layer())
        .with_state(app_state);

    // If we got here, all main components initialized successfully
}

/// Test configuration with different environment variables
#[tokio::test]
async fn test_config_variations() {
    // Test with minimal config
    unsafe {
        std::env::set_var("DATABASE_URL", "postgresql://test@localhost/test");
        std::env::remove_var("PORT");
        std::env::remove_var("AUTH_ENABLED");
    }

    let config = config::Config::from_env().expect("Config should load with defaults");
    assert_eq!(config.port, 8000); // Default port
    assert_eq!(config.auth_enabled, false); // Default auth disabled

    // Test with auth enabled
    unsafe {
        std::env::set_var("AUTH_ENABLED", "true");
        std::env::set_var("JWKS_URL", "https://example.com/jwks");
    }

    let config = config::Config::from_env().expect("Config should load with auth");
    assert_eq!(config.auth_enabled, true);
    assert_eq!(
        config.jwks_url,
        Some("https://example.com/jwks".to_string())
    );
}

/// Test that server components handle errors appropriately
#[tokio::test]
async fn test_error_handling() {
    // Test with invalid database URL
    unsafe {
        std::env::set_var("DATABASE_URL", "invalid://url");
    }

    let config = config::Config::from_env().expect("Config should load");
    let pool_result = db::create_pool(&config.database_url).await;
    assert!(pool_result.is_err(), "Invalid database URL should fail");

    // Test with missing required environment variables
    unsafe {
        std::env::remove_var("DATABASE_URL");
    }

    let config_result = config::Config::from_env();
    assert!(config_result.is_err(), "Should fail without DATABASE_URL");
    assert!(
        config_result
            .unwrap_err()
            .to_string()
            .contains("DATABASE_URL")
    );
}

/// Test tracing initialization
#[tokio::test]
async fn test_tracing_initialization() {
    // Test with default filter
    unsafe {
        std::env::remove_var("RUST_LOG");
    }

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "blackledger=debug,tower_http=debug".into());

    let filter_string = format!("{:?}", filter);
    assert!(filter_string.contains("blackledger") || filter_string.contains("debug"));

    // Test with custom filter
    unsafe {
        std::env::set_var("RUST_LOG", "blackledger=info");
    }

    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().expect("Should parse RUST_LOG");

    let filter_string = format!("{:?}", filter);
    assert!(filter_string.contains("blackledger") || filter_string.contains("info"));
}

/// Test app state and router integration
#[tokio::test]
async fn test_app_state_router_integration() {
    unsafe {
        std::env::set_var(
            "DATABASE_URL",
            "postgresql://blackledger_test:test@localhost:5434/blackledger_test",
        );
        std::env::set_var("AUTH_ENABLED", "false");
    }

    let config = config::Config::from_env().expect("Failed to load config");
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

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

    let app_state = api::AppState {
        pool: pool.clone(),
        jwt_validator,
    };

    // Test that router can be created with app state
    let router = api::router(app_state.clone());

    // Test that CORS layer can be added
    let _app: axum::Router = axum::Router::new()
        .merge(router)
        .layer(api::cors::cors_layer())
        .with_state(app_state);
}

/// Test database connection and migration verification
#[tokio::test]
async fn test_database_setup() {
    unsafe {
        std::env::set_var(
            "DATABASE_URL",
            "postgresql://blackledger_test:test@localhost:5434/blackledger_test",
        );
    }

    let config = config::Config::from_env().expect("Failed to load config");
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Verify tables exist
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.tables 
         WHERE table_schema = 'public' 
         AND table_name IN ('ledger', 'account', 'transaction', 'entry', 'currency')",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query tables");

    assert!(result.0 >= 5, "Should have at least 5 core tables");

    // Test database constraints exist (immutability triggers)
    let triggers: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.triggers 
         WHERE trigger_schema = 'public' 
         AND (trigger_name LIKE '%no_update%' OR trigger_name LIKE '%no_delete%')",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query triggers");

    assert!(
        triggers.0 > 0,
        "Should have no_update/no_delete triggers for immutability"
    );
}
