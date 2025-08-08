use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};

use crate::{
    api::{AppState, pagination::PaginatedResponse, search::LedgerSearchParams},
    db::queries::ledger::{count_ledgers, create_ledgers_batch, search_ledgers, update_ledger},
    error::ApiResult,
    models::ledger::{CreateLedger, Ledger, UpdateLedger},
};

pub async fn handle_create_ledgers(
    State(state): State<AppState>,
    Json(input): Json<Vec<CreateLedger>>,
) -> ApiResult<(StatusCode, Json<Vec<Ledger>>)> {
    let ledgers = create_ledgers_batch(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(ledgers)))
}

pub async fn handle_update_ledger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateLedger>,
) -> ApiResult<Json<Ledger>> {
    let ledger = update_ledger(&state.pool, id, &input).await?;
    Ok(Json(ledger))
}

pub async fn handle_search_ledgers(
    State(state): State<AppState>,
    Query(params): Query<LedgerSearchParams>,
) -> ApiResult<Json<PaginatedResponse<Ledger>>> {
    // Run search and count queries concurrently
    let (ledgers_result, count_result) = tokio::join!(
        search_ledgers(&state.pool, &params),
        count_ledgers(&state.pool, &params)
    );

    let ledgers = ledgers_result?;
    let total = count_result?;

    let pagination = params.base.to_pagination_params();
    let response = PaginatedResponse::new(ledgers, &pagination, Some(total));
    Ok(Json(response))
}
