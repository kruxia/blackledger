use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::{
    api::{
        AppState,
        pagination::{PaginatedResponse, PaginationParams},
        search::AccountSearchParams,
    },
    db::queries::account::{
        count_accounts, create_accounts_batch, get_account_balances, search_accounts,
        update_account,
    },
    error::ApiResult,
    models::account::{Account, AccountBalance, CreateAccount, UpdateAccount},
};

#[derive(Debug, Deserialize)]
pub struct GetBalancesQuery {
    pub ledger_id: i64,
    pub account_ids: Option<Vec<i64>>,
}

pub async fn handle_create_accounts(
    State(state): State<AppState>,
    Json(input): Json<Vec<CreateAccount>>,
) -> ApiResult<(StatusCode, Json<Vec<Account>>)> {
    let accounts = create_accounts_batch(&state.pool, &input).await?;
    Ok((StatusCode::CREATED, Json(accounts)))
}

pub async fn handle_update_account(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateAccount>,
) -> ApiResult<Json<Account>> {
    let account = update_account(&state.pool, id, &input).await?;
    Ok(Json(account))
}

pub async fn handle_search_accounts(
    State(state): State<AppState>,
    Query(params): Query<AccountSearchParams>,
) -> ApiResult<Json<PaginatedResponse<Account>>> {
    let accounts = search_accounts(&state.pool, &params).await?;
    let total = count_accounts(&state.pool, &params).await?;

    let pagination = PaginationParams {
        page: (params.base.get_offset() / params.base.get_limit() + 1) as u32,
        size: params.base.get_limit() as u32,
    };

    let response = PaginatedResponse::new(accounts, &pagination, Some(total));
    Ok(Json(response))
}

pub async fn handle_get_balances(
    State(state): State<AppState>,
    Query(query): Query<GetBalancesQuery>,
) -> ApiResult<Json<Vec<AccountBalance>>> {
    let balances = get_account_balances(&state.pool, query.ledger_id, query.account_ids).await?;
    Ok(Json(balances))
}
