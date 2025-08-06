use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod config;
mod db;
mod error;
mod models;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "blackledger=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    
    // Create database pool
    let pool = db::create_pool(&config.database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    // Set up JWT validator if auth is enabled
    let auth_config = auth::AuthConfig {
        enabled: config.auth_enabled,
        jwks_url: config.jwks_url.clone(),
        audience: None,
        issuer: None,
    };
    
    let _jwt_validator = Arc::new(
        auth::JwtValidator::new(auth_config)
            .await
            .expect("Failed to create JWT validator")
    );

    // Build application
    let app = Router::new()
        .nest("/api", api::router())
        .layer(TraceLayer::new_for_http())
        .layer(api::cors::cors_layer())
        .with_state(pool);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}