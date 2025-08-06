use sqlx::PgPool;
use chrono::{DateTime, Utc};

use crate::error::ApiResult;
use crate::models::entry::Entry;

pub async fn get_entries_by_transaction(pool: &PgPool, transaction_id: i64) -> ApiResult<Vec<Entry>> {
    let entries = sqlx::query_as::<_, Entry>(
        r#"
        SELECT * FROM entry 
        WHERE transaction_id = $1 
        ORDER BY created
        "#
    )
    .bind(transaction_id)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

pub async fn get_entries_by_account(
    pool: &PgPool,
    account_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Entry>> {
    let entries = sqlx::query_as::<_, Entry>(
        r#"
        SELECT * FROM entry 
        WHERE account_id = $1 
        ORDER BY created DESC
        LIMIT $2
        OFFSET $3
        "#
    )
    .bind(account_id)
    .bind(limit.unwrap_or(100))
    .bind(offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

pub async fn list_entries(
    pool: &PgPool,
    ledger_id: Option<i64>,
    account_id: Option<i64>,
    transaction_id: Option<i64>,
    _currency_code: Option<String>,
    _from_date: Option<DateTime<Utc>>,
    _to_date: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Entry>> {
    // Simplified implementation - handle the most common cases
    if let Some(tid) = transaction_id {
        return get_entries_by_transaction(pool, tid).await;
    }
    
    if let Some(aid) = account_id {
        return get_entries_by_account(pool, aid, limit, offset).await;
    }
    
    // For ledger_id filtering, we need to join with transaction table
    if let Some(lid) = ledger_id {
        let entries = sqlx::query_as::<_, Entry>(
            r#"
            SELECT e.* FROM entry e
            INNER JOIN transaction t ON e.transaction_id = t.id
            WHERE t.ledger_id = $1
            ORDER BY e.created DESC
            LIMIT $2
            OFFSET $3
            "#
        )
        .bind(lid)
        .bind(limit.unwrap_or(100))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await?;
        
        return Ok(entries);
    }
    
    // Default case - return all entries with pagination
    let entries = sqlx::query_as::<_, Entry>(
        r#"
        SELECT * FROM entry
        ORDER BY created DESC
        LIMIT $1
        OFFSET $2
        "#
    )
    .bind(limit.unwrap_or(100))
    .bind(offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;

    Ok(entries)
}