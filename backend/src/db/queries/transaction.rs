use sqlx::PgPool;

use crate::error::{ApiError, ApiResult};
use crate::models::transaction::Transaction;
use crate::services::posting;

// Note: This function is deprecated in favor of services::posting::post_transaction
// which includes proper validation and user context
pub async fn create_transaction(
    pool: &PgPool,
    input: &crate::models::transaction::CreateTransaction,
) -> ApiResult<Transaction> {
    // Use the posting service instead
    let (transaction, _entries) = posting::post_transaction(pool, input, None).await?;
    Ok(transaction)
}

pub async fn get_transaction_by_id(pool: &PgPool, id: i64) -> ApiResult<Transaction> {
    let record = sqlx::query!(
        r#"SELECT id, ledger_id, posted, effective, memo, meta FROM transaction WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Transaction {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    Ok(Transaction {
        id: record.id,
        ledger_id: record.ledger_id,
        posted: record.posted,
        effective: record.effective,
        memo: record.memo,
        meta: record.meta,
    })
}

pub async fn list_transactions(
    pool: &PgPool,
    ledger_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Transaction>> {
    let transactions = if let Some(lid) = ledger_id {
        let records = sqlx::query!(
            r#"
            SELECT id, ledger_id, posted, effective, memo, meta FROM transaction
            WHERE ledger_id = $1
            ORDER BY posted DESC
            LIMIT $2
            OFFSET $3
            "#,
            lid,
            limit.unwrap_or(100),
            offset.unwrap_or(0)
        )
        .fetch_all(pool)
        .await?;

        records
            .into_iter()
            .map(|r| Transaction {
                id: r.id,
                ledger_id: r.ledger_id,
                posted: r.posted,
                effective: r.effective,
                memo: r.memo,
                meta: r.meta,
            })
            .collect()
    } else {
        let records = sqlx::query!(
            r#"
            SELECT id, ledger_id, posted, effective, memo, meta FROM transaction
            ORDER BY posted DESC
            LIMIT $1
            OFFSET $2
            "#,
            limit.unwrap_or(100),
            offset.unwrap_or(0)
        )
        .fetch_all(pool)
        .await?;

        records
            .into_iter()
            .map(|r| Transaction {
                id: r.id,
                ledger_id: r.ledger_id,
                posted: r.posted,
                effective: r.effective,
                memo: r.memo,
                meta: r.meta,
            })
            .collect()
    };

    Ok(transactions)
}

pub async fn search_transactions(
    pool: &PgPool,
    params: &crate::api::search::TransactionSearchParams,
) -> ApiResult<Vec<Transaction>> {
    let limit = params.base.get_limit() as i64;
    let offset = params.base.get_offset() as i64;

    // For now, using simplified search based on ledger_id
    // In production, you'd build a dynamic query with all search parameters
    list_transactions(pool, params.ledger_id, Some(limit), Some(offset)).await
}

pub async fn count_transactions(
    pool: &PgPool,
    params: &crate::api::search::TransactionSearchParams,
) -> ApiResult<i64> {
    let count = if let Some(ledger_id) = params.ledger_id {
        let record = sqlx::query!(
            "SELECT COUNT(*) as count FROM transaction WHERE ledger_id = $1",
            ledger_id
        )
        .fetch_one(pool)
        .await?;
        record.count.unwrap_or(0)
    } else {
        let record = sqlx::query!("SELECT COUNT(*) as count FROM transaction")
            .fetch_one(pool)
            .await?;
        record.count.unwrap_or(0)
    };

    Ok(count)
}
