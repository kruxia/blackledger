use axum::{
    Router,
    extract::{FromRef, State},
    middleware as axum_middleware,
    response::Json,
    routing::{get, patch},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::error::ApiResult;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_validator: Arc<crate::auth::JwtValidator>,
}

// Implement FromRef to allow extracting JwtValidator from AppState
impl FromRef<AppState> for Arc<crate::auth::JwtValidator> {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_validator.clone()
    }
}

pub mod cors;
mod handlers;
mod middleware;
pub mod pagination;
pub mod search;

pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        // Currency endpoints
        .route(
            "/currencies",
            get(handlers::currencies::handle_search_currencies)
                .post(handlers::currencies::handle_create_currencies),
        )
        // Ledger endpoints
        .route(
            "/ledgers",
            get(handlers::ledgers::handle_search_ledgers)
                .post(handlers::ledgers::handle_create_ledgers),
        )
        .route(
            "/ledgers/:id",
            patch(handlers::ledgers::handle_update_ledger),
        )
        // Account endpoints
        .route(
            "/accounts",
            get(handlers::accounts::handle_search_accounts)
                .post(handlers::accounts::handle_create_accounts),
        )
        .route(
            "/accounts/:id",
            patch(handlers::accounts::handle_update_account),
        )
        .route(
            "/accounts/balances",
            get(handlers::accounts::handle_get_accounts_with_balances),
        )
        // Transaction endpoints
        .route(
            "/transactions",
            get(handlers::transactions::handle_search_transactions)
                .post(handlers::transactions::handle_create_transactions),
        )
        .layer(TraceLayer::new_for_http())
        .layer(axum_middleware::from_fn_with_state(
            Arc::clone(&state.jwt_validator),
            middleware::auth::auth_middleware,
        ));

    // Only health check is public (no tracing for this route)
    let public_routes = Router::new().route("/", get(health_check));

    Router::new().merge(protected_routes).merge(public_routes)
}

async fn health_check(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    // Test database connectivity
    sqlx::query("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| crate::error::ApiError::Database(e))?;

    Ok(Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "auth_enabled": state.jwt_validator.config.enabled,
    })))
}
