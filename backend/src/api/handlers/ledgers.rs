use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::{
    api::AppState,
    db::queries::ledger::{create_ledger, get_ledger_by_id, list_ledgers, update_ledger},
    error::ApiResult,
    models::ledger::{CreateLedger, Ledger, UpdateLedger},
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn handle_create_ledger(
    State(state): State<AppState>,
    Json(input): Json<CreateLedger>,
) -> ApiResult<(StatusCode, Json<Ledger>)> {
    let ledger = create_ledger(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(ledger)))
}

pub async fn handle_get_ledger(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Ledger>> {
    let ledger = get_ledger_by_id(&state.pool, id).await?;
    Ok(Json(ledger))
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
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<Vec<Ledger>>> {
    let ledgers = list_ledgers(&state.pool, query.limit, query.offset).await?;
    Ok(Json(ledgers))
}