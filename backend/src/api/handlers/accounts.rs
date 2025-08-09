use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};

use crate::{
    api::{AppState, pagination::PaginatedResponse, search::AccountSearchParams},
    db::queries::account::{
        count_accounts, create_accounts_batch, get_accounts_with_balances, search_accounts,
        update_account,
    },
    error::ApiResult,
    models::account::{Account, AccountBalances, CreateAccount, UpdateAccount},
};

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
    // Run search and count queries concurrently
    let (accounts_result, count_result) = tokio::join!(
        search_accounts(&state.pool, &params),
        count_accounts(&state.pool, &params)
    );

    let accounts = accounts_result?;
    let total = count_result?;

    let pagination = params.base.to_pagination_params();
    let response = PaginatedResponse::new(accounts, &pagination, Some(total));
    Ok(Json(response))
}

pub async fn handle_get_accounts_with_balances(
    State(state): State<AppState>,
    Query(params): Query<AccountSearchParams>,
) -> ApiResult<Json<PaginatedResponse<AccountBalances>>> {
    // Run search and count queries concurrently
    let (accounts_result, count_result) = tokio::join!(
        get_accounts_with_balances(&state.pool, &params),
        count_accounts(&state.pool, &params)
    );

    let accounts_with_balances = accounts_result?;
    let total = count_result?;

    let pagination = params.base.to_pagination_params();
    let response = PaginatedResponse::new(accounts_with_balances, &pagination, Some(total));
    Ok(Json(response))
}
