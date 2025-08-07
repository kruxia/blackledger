use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::error::ApiResult;
use crate::models::entry::Entry;

pub async fn get_entries_by_transaction(
    pool: &PgPool,
    transaction_id: i64,
) -> ApiResult<Vec<Entry>> {
    let records = sqlx::query!(
        r#"
        SELECT id, ledger_id, transaction_id, account_id, curr, debit, credit 
        FROM entry 
        WHERE transaction_id = $1 
        ORDER BY id
        "#,
        transaction_id
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            ledger_id: r.ledger_id,
            transaction_id: r.transaction_id,
            account_id: r.account_id,
            currency_code: r.curr,
            debit: r.debit,
            credit: r.credit,
        })
        .collect())
}

pub async fn get_entries_by_account(
    pool: &PgPool,
    account_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Entry>> {
    let records = sqlx::query!(
        r#"
        SELECT id, ledger_id, transaction_id, account_id, curr, debit, credit 
        FROM entry 
        WHERE account_id = $1 
        ORDER BY id DESC
        LIMIT $2
        OFFSET $3
        "#,
        account_id,
        limit.unwrap_or(100),
        offset.unwrap_or(0)
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            ledger_id: r.ledger_id,
            transaction_id: r.transaction_id,
            account_id: r.account_id,
            currency_code: r.curr,
            debit: r.debit,
            credit: r.credit,
        })
        .collect())
}

pub async fn list_entries(
    pool: &PgPool,
    ledger_id: Option<i64>,
    account_id: Option<i64>,
    transaction_id: Option<i64>,
    _currency_code: Option<String>,
    _from_amount: Option<Decimal>,
    _to_amount: Option<Decimal>,
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
        let records = sqlx::query!(
            r#"
            SELECT e.id, e.ledger_id, e.transaction_id, e.account_id, e.curr, e.debit, e.credit 
            FROM entry e
            INNER JOIN transaction t ON e.transaction_id = t.id
            WHERE t.ledger_id = $1
            ORDER BY e.id DESC
            LIMIT $2
            OFFSET $3
            "#,
            lid,
            limit.unwrap_or(100),
            offset.unwrap_or(0)
        )
        .fetch_all(pool)
        .await?;

        return Ok(records
            .into_iter()
            .map(|r| Entry {
                id: r.id,
                ledger_id: r.ledger_id,
                transaction_id: r.transaction_id,
                account_id: r.account_id,
                currency_code: r.curr,
                debit: r.debit,
                credit: r.credit,
            })
            .collect());
    }

    // Default case - return all entries with pagination
    let records = sqlx::query!(
        r#"
        SELECT id, ledger_id, transaction_id, account_id, curr, debit, credit 
        FROM entry
        ORDER BY id DESC
        LIMIT $1
        OFFSET $2
        "#,
        limit.unwrap_or(100),
        offset.unwrap_or(0)
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            ledger_id: r.ledger_id,
            transaction_id: r.transaction_id,
            account_id: r.account_id,
            currency_code: r.curr,
            debit: r.debit,
            credit: r.credit,
        })
        .collect())
}

pub async fn search_entries(
    pool: &PgPool,
    params: &crate::api::search::EntrySearchParams,
) -> ApiResult<Vec<Entry>> {
    let page = params.common.page.unwrap_or(1) as i64;
    let page_size = params.common.page_size.unwrap_or(20) as i64;
    let offset = (page - 1) * page_size;

    list_entries(
        pool,
        params.ledger_id,
        params.account_id,
        params.transaction_id,
        params.currency_code.clone(),
        params.from_amount,
        params.to_amount,
        Some(page_size),
        Some(offset),
    )
    .await
}

pub async fn count_entries(
    pool: &PgPool,
    params: &crate::api::search::EntrySearchParams,
) -> ApiResult<i64> {
    // Build count query based on parameters
    let count = if let Some(ledger_id) = params.ledger_id {
        let record = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM entry e
            INNER JOIN transaction t ON e.transaction_id = t.id
            WHERE t.ledger_id = $1
            "#,
            ledger_id
        )
        .fetch_one(pool)
        .await?;
        record.count.unwrap_or(0)
    } else if let Some(account_id) = params.account_id {
        let record = sqlx::query!(
            "SELECT COUNT(*) as count FROM entry WHERE account_id = $1",
            account_id
        )
        .fetch_one(pool)
        .await?;
        record.count.unwrap_or(0)
    } else if let Some(transaction_id) = params.transaction_id {
        let record = sqlx::query!(
            "SELECT COUNT(*) as count FROM entry WHERE transaction_id = $1",
            transaction_id
        )
        .fetch_one(pool)
        .await?;
        record.count.unwrap_or(0)
    } else {
        let record = sqlx::query!("SELECT COUNT(*) as count FROM entry")
            .fetch_one(pool)
            .await?;
        record.count.unwrap_or(0)
    };

    Ok(count)
}
