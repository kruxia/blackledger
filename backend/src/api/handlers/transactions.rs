use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

use crate::services::posting::post_transactions_batch;
use crate::{
    api::{
        AppState,
        pagination::{PaginatedResponse, PaginationParams},
        search::TransactionSearchParams,
    },
    auth::OptionalAuthUser,
    db::queries::transaction::{count_transactions, search_transactions},
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

pub async fn handle_create_transactions(
    State(state): State<AppState>,
    OptionalAuthUser(auth_user): OptionalAuthUser,
    Json(input): Json<Vec<CreateTransaction>>,
) -> ApiResult<(StatusCode, Json<Vec<TransactionWithEntries>>)> {
    let user_id = auth_user.as_ref().map(|u| u.sub.as_str());
    let results = post_transactions_batch(&state.pool, &input, user_id).await?;

    let response: Vec<TransactionWithEntries> = results
        .into_iter()
        .map(|(transaction, entries)| TransactionWithEntries {
            transaction,
            entries,
        })
        .collect();

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn handle_search_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionSearchParams>,
) -> ApiResult<Json<PaginatedResponse<Transaction>>> {
    let transactions = search_transactions(&state.pool, &params).await?;
    let total = count_transactions(&state.pool, &params).await?;

    let pagination = PaginationParams {
        page: (params.base.get_offset() / params.base.get_limit() + 1) as u32,
        size: params.base.get_limit() as u32,
    };

    let response = PaginatedResponse::new(transactions, &pagination, Some(total));
    Ok(Json(response))
}
