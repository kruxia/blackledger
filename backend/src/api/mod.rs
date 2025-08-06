use axum::{
    extract::State,
    middleware as axum_middleware,
    response::Json,
    routing::{get, patch, post},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;

use crate::error::ApiResult;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_validator: Arc<crate::auth::JwtValidator>,
}

pub mod cors;
mod handlers;
mod middleware;
pub mod pagination;
pub mod search;


pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        // Currency endpoints (protected)
        .route("/currencies", post(handlers::currencies::handle_create_currency))
        // Ledger endpoints (protected)
        .route("/ledgers", post(handlers::ledgers::handle_create_ledger))
        .route("/ledgers/:id", patch(handlers::ledgers::handle_update_ledger))
        // Account endpoints (protected)
        .route("/accounts", post(handlers::accounts::handle_create_account))
        .route("/accounts/:id", patch(handlers::accounts::handle_update_account))
        // Transaction endpoints (protected)
        .route("/transactions", post(handlers::transactions::handle_create_transaction))
        .layer(axum_middleware::from_fn_with_state(
            Arc::clone(&state.jwt_validator),
            middleware::auth::auth_middleware,
        ));
    
    let public_routes = Router::new()
        .route("/", get(health_check))
        // Currency endpoints (public)
        .route("/currencies", get(handlers::currencies::handle_list_currencies))
        // Ledger endpoints (public)
        .route("/ledgers", get(handlers::ledgers::handle_list_ledgers))
        .route("/ledgers/:id", get(handlers::ledgers::handle_get_ledger))
        // Account endpoints (public)
        .route("/accounts", get(handlers::accounts::handle_list_accounts))
        .route("/accounts/balances", get(handlers::accounts::handle_get_balances))
        .route("/accounts/:id", get(handlers::accounts::handle_get_account))
        // Transaction endpoints (public)
        .route("/transactions", get(handlers::transactions::handle_list_transactions))
        .route("/transactions/:id", get(handlers::transactions::handle_get_transaction))
        // Entry endpoints (public)
        .route("/entries", get(handlers::transactions::handle_list_entries));
    
    Router::new()
        .merge(protected_routes)
        .merge(public_routes)
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

