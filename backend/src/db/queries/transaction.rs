use sqlx::{PgPool, QueryBuilder, Postgres, Row};

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

    // Use QueryBuilder for dynamic SQL generation
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, ledger_id, memo, meta, posted, effective FROM transaction WHERE 1=1"
    );

    // Handle ledger_id filter
    if let Some(ledger_id) = params.ledger_id {
        query_builder.push(" AND ledger_id = ");
        query_builder.push_bind(ledger_id);
    }

    // Handle account_id filter (requires join with entry table)
    if let Some(account_id) = params.account_id {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE account_id = ");
        query_builder.push_bind(account_id);
        query_builder.push(")");
    }

    // Handle description filter (regex pattern matching on memo field)
    if let Some(ref description) = params.description {
        query_builder.push(" AND memo ~* ");
        query_builder.push_bind(description);
    }

    // Handle currency_code filter (requires join with entry table)
    if let Some(ref currency_code) = params.currency_code {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE curr = ");
        query_builder.push_bind(currency_code);
        query_builder.push(")");
    }

    // Handle amount range filters (requires join with entry table)
    if params.from_amount.is_some() || params.to_amount.is_some() {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE ");
        
        if let Some(from_amount) = params.from_amount {
            query_builder.push("(debit >= ");
            query_builder.push_bind(from_amount);
            query_builder.push(" OR credit >= ");
            query_builder.push_bind(from_amount);
            query_builder.push(")");
            
            if params.to_amount.is_some() {
                query_builder.push(" AND ");
            }
        }
        
        if let Some(to_amount) = params.to_amount {
            query_builder.push("(debit <= ");
            query_builder.push_bind(to_amount);
            query_builder.push(" OR credit <= ");
            query_builder.push_bind(to_amount);
            query_builder.push(")");
        }
        
        query_builder.push(")");
    }

    // Add sorting based on SearchParams with whitelist validation
    const ALLOWED_COLUMNS: &[&str] = &["id", "ledger_id", "memo", "posted", "effective"];
    if let Some(order_clause) = params.base.parse_order_by(ALLOWED_COLUMNS) {
        query_builder.push(" ORDER BY ");
        query_builder.push(order_clause);
    } else {
        // Default ordering
        query_builder.push(" ORDER BY posted DESC");
    }

    // Add pagination with parameter binding
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute the query
    let query = query_builder.build();
    let rows = query.fetch_all(pool).await?;

    let mut transactions = Vec::new();
    for row in rows {
        let transaction = Transaction {
            id: row.try_get("id")?,
            ledger_id: row.try_get("ledger_id")?,
            memo: row.try_get("memo")?,
            meta: row.try_get("meta")?,
            posted: row.try_get("posted")?,
            effective: row.try_get("effective")?,
        };
        transactions.push(transaction);
    }

    Ok(transactions)
}

pub async fn count_transactions(
    pool: &PgPool,
    params: &crate::api::search::TransactionSearchParams,
) -> ApiResult<i64> {
    // Use QueryBuilder for dynamic SQL generation (matching search_transactions logic)
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) as count FROM transaction WHERE 1=1"
    );

    // Handle ledger_id filter
    if let Some(ledger_id) = params.ledger_id {
        query_builder.push(" AND ledger_id = ");
        query_builder.push_bind(ledger_id);
    }

    // Handle account_id filter (requires join with entry table)
    if let Some(account_id) = params.account_id {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE account_id = ");
        query_builder.push_bind(account_id);
        query_builder.push(")");
    }

    // Handle description filter (regex pattern matching on memo field)
    if let Some(ref description) = params.description {
        query_builder.push(" AND memo ~* ");
        query_builder.push_bind(description);
    }

    // Handle currency_code filter (requires join with entry table)
    if let Some(ref currency_code) = params.currency_code {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE curr = ");
        query_builder.push_bind(currency_code);
        query_builder.push(")");
    }

    // Handle amount range filters (requires join with entry table)
    if params.from_amount.is_some() || params.to_amount.is_some() {
        query_builder.push(" AND id IN (SELECT DISTINCT transaction_id FROM entry WHERE ");
        
        if let Some(from_amount) = params.from_amount {
            query_builder.push("(debit >= ");
            query_builder.push_bind(from_amount);
            query_builder.push(" OR credit >= ");
            query_builder.push_bind(from_amount);
            query_builder.push(")");
            
            if params.to_amount.is_some() {
                query_builder.push(" AND ");
            }
        }
        
        if let Some(to_amount) = params.to_amount {
            query_builder.push("(debit <= ");
            query_builder.push_bind(to_amount);
            query_builder.push(" OR credit <= ");
            query_builder.push_bind(to_amount);
            query_builder.push(")");
        }
        
        query_builder.push(")");
    }

    // Execute the query
    let query = query_builder.build();
    let row = query.fetch_one(pool).await?;
    let count: i64 = row.try_get("count")?;

    Ok(count)
}
