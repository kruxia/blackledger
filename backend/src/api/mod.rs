use axum::{
    Router,
    extract::State,
    middleware as axum_middleware,
    response::Json,
    routing::{get, patch},
};
use serde_json::{Value, json};
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
        // Currency endpoints
        .route(
            "/currencies",
            get(handlers::currencies::handle_list_currencies)
                .post(handlers::currencies::handle_create_currency),
        )
        // Ledger endpoints
        .route(
            "/ledgers",
            get(handlers::ledgers::handle_list_ledgers)
                .post(handlers::ledgers::handle_create_ledger),
        )
        .route(
            "/ledgers/:id",
            patch(handlers::ledgers::handle_update_ledger),
        )
        // Account endpoints
        .route(
            "/accounts",
            get(handlers::accounts::handle_list_accounts)
                .post(handlers::accounts::handle_create_account),
        )
        .route(
            "/accounts/:id",
            patch(handlers::accounts::handle_update_account),
        )
        .route(
            "/accounts/balances",
            get(handlers::accounts::handle_get_balances),
        )
        // Transaction endpoints
        .route(
            "/transactions",
            get(handlers::transactions::handle_list_transactions)
                .post(handlers::transactions::handle_create_transaction),
        )
        .layer(axum_middleware::from_fn_with_state(
            Arc::clone(&state.jwt_validator),
            middleware::auth::auth_middleware,
        ));

    // Only health check is public
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
