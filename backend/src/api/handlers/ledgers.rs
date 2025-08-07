use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};

use crate::{
    api::{AppState, search::LedgerSearchParams},
    db::queries::ledger::{create_ledger, search_ledgers, update_ledger},
    error::ApiResult,
    models::ledger::{CreateLedger, Ledger, UpdateLedger},
};

pub async fn handle_create_ledger(
    State(state): State<AppState>,
    Json(input): Json<CreateLedger>,
) -> ApiResult<(StatusCode, Json<Ledger>)> {
    let ledger = create_ledger(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(ledger)))
}

pub async fn handle_update_ledger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateLedger>,
) -> ApiResult<Json<Ledger>> {
    let ledger = update_ledger(&state.pool, id, &input).await?;
    Ok(Json(ledger))
}

pub async fn handle_list_ledgers(
    State(state): State<AppState>,
    Query(params): Query<LedgerSearchParams>,
) -> ApiResult<Json<Vec<Ledger>>> {
    let ledgers = search_ledgers(&state.pool, &params).await?;
    Ok(Json(ledgers))
}
