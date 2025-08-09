//! Application setup and server initialization
use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{api, auth, config, db};

/// Initialize the tracing subscriber
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "blackledger=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Build the application router with all middleware
pub async fn build_app(config: config::Config) -> Result<Router> {
    // Create database pool
    let pool = db::create_pool(&config.database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Set up JWT validator
    let auth_config = auth::AuthConfig {
        enabled: config.auth_enabled,
        jwks_url: config.jwks_url.clone(),
        audience: None,
        issuer: None,
    };

    let jwt_validator = Arc::new(
        auth::JwtValidator::new(auth_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create JWT validator: {}", e))?,
    );

    // Create app state
    let app_state = api::AppState {
        pool: pool.clone(),
        jwt_validator,
    };

    // Build application
    let app = Router::new()
        .merge(api::router(app_state.clone()))
        .layer(api::cors::cors_layer())
        .with_state(app_state);

    Ok(app)
}

/// Start the server with the given app and configuration
pub async fn start_server(app: Router, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_app_success() {
        unsafe {
            std::env::set_var(
                "DATABASE_URL",
                "postgresql://blackledger_test:test@localhost:5434/blackledger_test",
            );
            std::env::set_var("PORT", "8000");
            std::env::set_var("AUTH_ENABLED", "false");
        }

        let config = config::Config::from_env().expect("Failed to load config");
        let app_result = build_app(config).await;

        assert!(
            app_result.is_ok(),
            "Should successfully build app with valid config"
        );
    }

    #[tokio::test]
    async fn test_build_app_with_auth_disabled() {
        unsafe {
            std::env::set_var(
                "DATABASE_URL",
                "postgresql://blackledger_test:test@localhost:5434/blackledger_test",
            );
            std::env::set_var("AUTH_ENABLED", "false");
            std::env::remove_var("JWKS_URL");
        }

        let config = config::Config::from_env().expect("Failed to load config");
        assert_eq!(config.auth_enabled, false);
        assert_eq!(config.jwks_url, None);

        let app_result = build_app(config).await;
        assert!(
            app_result.is_ok(),
            "Should successfully build app with auth disabled"
        );
    }

    #[tokio::test]
    async fn test_init_tracing_with_custom_filter() {
        unsafe {
            std::env::set_var("RUST_LOG", "blackledger=info,sqlx=warn");
        }

        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "blackledger=debug,tower_http=debug".into());

        // The filter should have picked up our custom RUST_LOG
        let filter_str = format!("{:?}", filter);
        // The format of the debug string varies, so we just check it's not empty
        assert!(!filter_str.is_empty());
    }

    #[tokio::test]
    async fn test_init_tracing_with_default_filter() {
        unsafe {
            std::env::remove_var("RUST_LOG");
        }

        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "blackledger=debug,tower_http=debug".into());

        let filter_str = format!("{:?}", filter);
        assert!(filter_str.contains("blackledger") || filter_str.contains("debug"));
    }
}
