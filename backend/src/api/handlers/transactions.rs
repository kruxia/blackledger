use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::AppState,
    db::queries::transaction::{create_transaction, get_transaction_by_id, list_transactions},
    db::queries::entry::{get_entries_by_transaction, list_entries},
    error::ApiResult,
    models::transaction::{CreateTransaction, Transaction},
    models::entry::Entry,
};

#[derive(Debug, Deserialize)]
pub struct ListTransactionsQuery {
    pub ledger_id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListEntriesQuery {
    pub ledger_id: Option<i64>,
    pub account_id: Option<i64>,
    pub transaction_id: Option<i64>,
    pub currency_code: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TransactionWithEntries {
    #[serde(flatten)]
    pub transaction: Transaction,
    pub entries: Vec<Entry>,
}

pub async fn handle_create_transaction(
    State(state): State<AppState>,
    Json(input): Json<CreateTransaction>,
) -> ApiResult<(StatusCode, Json<Transaction>)> {
    let transaction = create_transaction(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(transaction)))
}

pub async fn handle_get_transaction(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<TransactionWithEntries>> {
    let transaction = get_transaction_by_id(&state.pool, id).await?;
    let entries = get_entries_by_transaction(&state.pool, id).await?;
    
    Ok(Json(TransactionWithEntries {
        transaction,
        entries,
    }))
}

pub async fn handle_list_transactions(
    State(state): State<AppState>,
    Query(query): Query<ListTransactionsQuery>,
) -> ApiResult<Json<Vec<Transaction>>> {
    let transactions = list_transactions(&state.pool, query.ledger_id, query.limit, query.offset).await?;
    Ok(Json(transactions))
}

pub async fn handle_list_entries(
    State(state): State<AppState>,
    Query(query): Query<ListEntriesQuery>,
) -> ApiResult<Json<Vec<Entry>>> {
    let entries = list_entries(
        &state.pool,
        query.ledger_id,
        query.account_id,
        query.transaction_id,
        query.currency_code,
        None,
        None,
        query.limit,
        query.offset,
    )
    .await?;
    Ok(Json(entries))
}