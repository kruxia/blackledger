use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::queries::account::{
    create_account, get_account_balances, get_account_by_id, list_accounts, update_account,
};
use crate::error::ApiResult;
use crate::models::account::{Account, AccountBalance, CreateAccount, UpdateAccount};

#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    pub ledger_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GetBalancesQuery {
    pub ledger_id: Uuid,
    pub account_ids: Option<Vec<Uuid>>,
}

pub async fn handle_create_account(
    State(pool): State<PgPool>,
    Json(input): Json<CreateAccount>,
) -> ApiResult<(StatusCode, Json<Account>)> {
    let account = create_account(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

pub async fn handle_get_account(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Account>> {
    let account = get_account_by_id(&pool, id).await?;
    Ok(Json(account))
}

pub async fn handle_update_account(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateAccount>,
) -> ApiResult<Json<Account>> {
    let account = update_account(&pool, id, &input).await?;
    Ok(Json(account))
}

pub async fn handle_list_accounts(
    State(pool): State<PgPool>,
    Query(query): Query<ListAccountsQuery>,
) -> ApiResult<Json<Vec<Account>>> {
    let accounts = list_accounts(
        &pool,
        query.ledger_id,
        query.parent_id,
        query.limit,
        query.offset,
    )
    .await?;
    Ok(Json(accounts))
}

pub async fn handle_get_balances(
    State(pool): State<PgPool>,
    Query(query): Query<GetBalancesQuery>,
) -> ApiResult<Json<Vec<AccountBalance>>> {
    let balances = get_account_balances(&pool, query.ledger_id, query.account_ids).await?;
    Ok(Json(balances))
}