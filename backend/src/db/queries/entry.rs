use sqlx::PgPool;
use std::collections::HashMap;

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

/// Fetch entries for multiple transactions efficiently in a single query
pub async fn get_entries_for_transactions(
    pool: &PgPool,
    transaction_ids: &[i64],
) -> ApiResult<HashMap<i64, Vec<Entry>>> {
    if transaction_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let records = sqlx::query!(
        r#"
        SELECT id, ledger_id, transaction_id, account_id, curr, debit, credit 
        FROM entry 
        WHERE transaction_id = ANY($1)
        ORDER BY transaction_id, id
        "#,
        transaction_ids
    )
    .fetch_all(pool)
    .await?;

    let mut entries_map: HashMap<i64, Vec<Entry>> = HashMap::new();

    for r in records {
        let entry = Entry {
            id: r.id,
            ledger_id: r.ledger_id,
            transaction_id: r.transaction_id,
            account_id: r.account_id,
            currency_code: r.curr,
            debit: r.debit,
            credit: r.credit,
        };

        entries_map
            .entry(r.transaction_id)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    Ok(entries_map)
}
