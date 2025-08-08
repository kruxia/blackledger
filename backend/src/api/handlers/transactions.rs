use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};

use crate::services::posting::post_transactions_batch;
use crate::{
    api::{AppState, pagination::PaginatedResponse, search::TransactionSearchParams},
    auth::OptionalAuthUser,
    db::queries::transaction::{count_transactions, search_transactions},
    error::ApiResult,
    models::transaction::{CreateTransaction, Transaction},
};

pub async fn handle_create_transactions(
    State(state): State<AppState>,
    OptionalAuthUser(auth_user): OptionalAuthUser,
    Json(input): Json<Vec<CreateTransaction>>,
) -> ApiResult<(StatusCode, Json<Vec<Transaction>>)> {
    let user_id = auth_user.as_ref().map(|u| u.sub.as_str());
    let results = post_transactions_batch(&state.pool, &input, user_id).await?;

    let response: Vec<Transaction> = results
        .into_iter()
        .map(|(mut transaction, entries)| {
            transaction.entries = entries;
            transaction
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

    let pagination = params.base.to_pagination_params();
    let response = PaginatedResponse::new(transactions, &pagination, Some(total));
    Ok(Json(response))
}
