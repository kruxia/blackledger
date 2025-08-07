use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

use crate::services::posting::post_transaction;
use crate::{
    api::{
        pagination::{PaginatedResponse, PaginationParams},
        search::{EntrySearchParams, TransactionSearchParams},
        AppState,
    },
    auth::OptionalAuthUser,
    db::queries::entry::{count_entries, get_entries_by_transaction, search_entries},
    db::queries::transaction::{count_transactions, get_transaction_by_id, search_transactions},
    error::ApiResult,
    models::entry::Entry,
    models::transaction::{CreateTransaction, Transaction},
};

#[derive(Debug, Serialize)]
pub struct TransactionWithEntries {
    #[serde(flatten)]
    pub transaction: Transaction,
    pub entries: Vec<Entry>,
}

pub async fn handle_create_transaction(
    State(state): State<AppState>,
    OptionalAuthUser(auth_user): OptionalAuthUser,
    Json(input): Json<CreateTransaction>,
) -> ApiResult<(StatusCode, Json<TransactionWithEntries>)> {
    let user_id = auth_user.as_ref().map(|u| u.sub.as_str());
    let (transaction, entries) = post_transaction(&state.pool, &input, user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(TransactionWithEntries {
            transaction,
            entries,
        }),
    ))
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
    Query(params): Query<TransactionSearchParams>,
) -> ApiResult<Json<PaginatedResponse<Transaction>>> {
    let transactions = search_transactions(&state.pool, &params).await?;
    let total = count_transactions(&state.pool, &params).await?;

    let pagination = PaginationParams {
        page: params.common.page.unwrap_or(1),
        page_size: params.common.page_size.unwrap_or(20),
    };

    let response = PaginatedResponse::new(transactions, &pagination, Some(total));
    Ok(Json(response))
}

pub async fn handle_list_entries(
    State(state): State<AppState>,
    Query(params): Query<EntrySearchParams>,
) -> ApiResult<Json<PaginatedResponse<Entry>>> {
    let entries = search_entries(&state.pool, &params).await?;
    let total = count_entries(&state.pool, &params).await?;

    let pagination = PaginationParams {
        page: params.common.page.unwrap_or(1),
        page_size: params.common.page_size.unwrap_or(20),
    };

    let response = PaginatedResponse::new(entries, &pagination, Some(total));
    Ok(Json(response))
}
