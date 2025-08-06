use axum::{
    extract::State,
    response::Json,
    routing::{get, patch, post},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::error::ApiResult;

pub mod cors;
mod handlers;
mod middleware;
mod pagination;
mod search;

pub use pagination::{PaginatedResponse, PaginationParams};
pub use search::{SearchParams, AccountSearchParams, TransactionSearchParams, EntrySearchParams};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/", get(health_check))
        // Currency endpoints
        .route("/currencies", get(handlers::currencies::handle_list_currencies))
        .route("/currencies", post(handlers::currencies::handle_create_currency))
        // Ledger endpoints
        .route("/ledgers", get(handlers::ledgers::handle_list_ledgers))
        .route("/ledgers", post(handlers::ledgers::handle_create_ledger))
        .route("/ledgers/:id", get(handlers::ledgers::handle_get_ledger))
        .route("/ledgers/:id", patch(handlers::ledgers::handle_update_ledger))
        // Account endpoints
        .route("/accounts", get(handlers::accounts::handle_list_accounts))
        .route("/accounts", post(handlers::accounts::handle_create_account))
        .route("/accounts/balances", get(handlers::accounts::handle_get_balances))
        .route("/accounts/:id", get(handlers::accounts::handle_get_account))
        .route("/accounts/:id", patch(handlers::accounts::handle_update_account))
        // Transaction endpoints
        .route("/transactions", get(handlers::transactions::handle_list_transactions))
        .route("/transactions", post(handlers::transactions::handle_create_transaction))
        .route("/transactions/:id", get(handlers::transactions::handle_get_transaction))
        // Entry endpoints
        .route("/entries", get(handlers::transactions::handle_list_entries))
}

async fn health_check(State(pool): State<PgPool>) -> ApiResult<Json<Value>> {
    // Test database connectivity
    sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await
        .map_err(|e| crate::error::ApiError::Database(e))?;

    Ok(Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

