use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

use crate::services::posting::post_transaction;
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

pub async fn handle_list_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionSearchParams>,
) -> ApiResult<Json<PaginatedResponse<Transaction>>> {
    let transactions = search_transactions(&state.pool, &params).await?;
    let total = count_transactions(&state.pool, &params).await?;

    let pagination = PaginationParams {
        page: params.common.page.unwrap_or(1),
        size: params.common.size.unwrap_or(20),
    };

    let response = PaginatedResponse::new(transactions, &pagination, Some(total));
    Ok(Json(response))
}
