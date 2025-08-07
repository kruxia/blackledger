use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::error::{ApiError, ApiResult};
use crate::models::transaction::Transaction;

pub async fn get_transaction_by_id(pool: &PgPool, id: i64) -> ApiResult<Transaction> {
    let record = sqlx::query!(
        r#"SELECT id, ledger_id, created, effective, memo, meta FROM transaction WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Transaction {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    let entries = crate::db::queries::entry::get_entries_by_transaction(pool, id).await?;

    Ok(Transaction {
        id: record.id,
        ledger_id: record.ledger_id,
        created: record.created,
        effective: record.effective,
        memo: record.memo,
        meta: record.meta,
        entries,
    })
}

pub async fn list_transactions(
    pool: &PgPool,
    ledger_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Transaction>> {
    let mut transactions: Vec<Transaction> = if let Some(lid) = ledger_id {
        let records = sqlx::query!(
            r#"
            SELECT id, ledger_id, created, effective, memo, meta FROM transaction
            WHERE ledger_id = $1
            ORDER BY created DESC
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
                created: r.created,
                effective: r.effective,
                memo: r.memo,
                meta: r.meta,
                entries: Vec::new(),
            })
            .collect()
    } else {
        let records = sqlx::query!(
            r#"
            SELECT id, ledger_id, created, effective, memo, meta FROM transaction
            ORDER BY created DESC
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
                created: r.created,
                effective: r.effective,
                memo: r.memo,
                meta: r.meta,
                entries: Vec::new(),
            })
            .collect()
    };

    // Fetch entries for all transactions
    let transaction_ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();
    let entries_map =
        crate::db::queries::entry::get_entries_for_transactions(pool, &transaction_ids).await?;

    // Populate entries for each transaction
    for transaction in &mut transactions {
        if let Some(entries) = entries_map.get(&transaction.id) {
            transaction.entries = entries.clone();
        }
    }

    Ok(transactions)
}

pub async fn search_transactions(
    pool: &PgPool,
    params: &crate::api::search::TransactionSearchParams,
) -> ApiResult<Vec<Transaction>> {
    let limit = params.base.get_limit() as i64;
    let offset = params.base.get_offset() as i64;

    // Use QueryBuilder for dynamic SQL generation with CTE to join with entry table when needed
    let needs_entry_join = params.acct.is_some() || params.curr.is_some();

    let mut query_builder: QueryBuilder<Postgres> = if needs_entry_join {
        QueryBuilder::new(
            "WITH tx_ids AS (
                SELECT DISTINCT transaction.id
                FROM transaction
                JOIN entry ON transaction.id = entry.tx
                WHERE 1=1",
        )
    } else {
        QueryBuilder::new(
            "SELECT id, ledger_id, memo, meta, created, effective FROM transaction WHERE 1=1",
        )
    };

    // Handle transaction ID filter (comma-delimited list)
    if let Some(ref tx_ids) = params.tx {
        let ids: Vec<i64> = tx_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            if needs_entry_join {
                query_builder.push(" AND transaction.id = ANY(");
            } else {
                query_builder.push(" AND id = ANY(");
            }
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle ledger_id filter (comma-delimited list)
    if let Some(ref ledger_ids) = params.ledger_id {
        let ids: Vec<i64> = ledger_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            if needs_entry_join {
                query_builder.push(" AND transaction.ledger_id = ANY(");
            } else {
                query_builder.push(" AND ledger_id = ANY(");
            }
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle account ID filter (comma-delimited list)
    if let Some(ref account_ids) = params.acct {
        let ids: Vec<i64> = account_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            query_builder.push(" AND entry.acct = ANY(");
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle currency code filter (regex patterns)
    if let Some(ref curr_patterns) = params.curr {
        let patterns: Vec<&str> = curr_patterns.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query_builder.push(" AND (");
            let mut first = true;
            for pattern in patterns {
                if !first {
                    query_builder.push(" OR ");
                }
                query_builder.push("entry.curr ~* ");
                query_builder.push_bind(pattern);
                first = false;
            }
            query_builder.push(")");
        }
    }

    // Handle memo filter (regex pattern matching)
    if let Some(ref memo_pattern) = params.memo {
        if needs_entry_join {
            query_builder.push(" AND transaction.memo ~* ");
        } else {
            query_builder.push(" AND memo ~* ");
        }
        query_builder.push_bind(memo_pattern);
    }

    // If we used CTE, close it and select from the results
    if needs_entry_join {
        query_builder.push(
            ")
            SELECT transaction.id, transaction.ledger_id, transaction.memo, transaction.meta, 
                   transaction.created, transaction.effective
            FROM transaction
            JOIN tx_ids ON tx_ids.id = transaction.id",
        );
    }

    // Add sorting based on SearchParams with whitelist validation
    const ALLOWED_COLUMNS: &[&str] = &["id", "ledger_id", "memo", "created", "effective"];
    if let Some(order_clause) = params.base.parse_order_by(ALLOWED_COLUMNS) {
        query_builder.push(" ORDER BY ");
        query_builder.push(order_clause);
    } else {
        // Default ordering
        query_builder.push(" ORDER BY created DESC");
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
            created: row.try_get("created")?,
            effective: row.try_get("effective")?,
            entries: Vec::new(),
        };
        transactions.push(transaction);
    }

    // Fetch entries for all transactions
    let transaction_ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();
    let entries_map =
        crate::db::queries::entry::get_entries_for_transactions(pool, &transaction_ids).await?;

    // Populate entries for each transaction
    for transaction in &mut transactions {
        if let Some(entries) = entries_map.get(&transaction.id) {
            transaction.entries = entries.clone();
        }
    }

    Ok(transactions)
}

pub async fn count_transactions(
    pool: &PgPool,
    params: &crate::api::search::TransactionSearchParams,
) -> ApiResult<i64> {
    // Use QueryBuilder for dynamic SQL generation (matching search_transactions logic)
    let needs_entry_join = params.acct.is_some() || params.curr.is_some();

    let mut query_builder: QueryBuilder<Postgres> = if needs_entry_join {
        QueryBuilder::new(
            "WITH tx_ids AS (
                SELECT DISTINCT transaction.id
                FROM transaction
                JOIN entry ON transaction.id = entry.tx
                WHERE 1=1",
        )
    } else {
        QueryBuilder::new("SELECT COUNT(*) as count FROM transaction WHERE 1=1")
    };

    // Handle transaction ID filter (comma-delimited list)
    if let Some(ref tx_ids) = params.tx {
        let ids: Vec<i64> = tx_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            if needs_entry_join {
                query_builder.push(" AND transaction.id = ANY(");
            } else {
                query_builder.push(" AND id = ANY(");
            }
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle ledger_id filter (comma-delimited list)
    if let Some(ref ledger_ids) = params.ledger_id {
        let ids: Vec<i64> = ledger_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            if needs_entry_join {
                query_builder.push(" AND transaction.ledger_id = ANY(");
            } else {
                query_builder.push(" AND ledger_id = ANY(");
            }
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle account ID filter (comma-delimited list)
    if let Some(ref account_ids) = params.acct {
        let ids: Vec<i64> = account_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            query_builder.push(" AND entry.acct = ANY(");
            query_builder.push_bind(ids);
            query_builder.push(")");
        }
    }

    // Handle currency code filter (regex patterns)
    if let Some(ref curr_patterns) = params.curr {
        let patterns: Vec<&str> = curr_patterns.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query_builder.push(" AND (");
            let mut first = true;
            for pattern in patterns {
                if !first {
                    query_builder.push(" OR ");
                }
                query_builder.push("entry.curr ~* ");
                query_builder.push_bind(pattern);
                first = false;
            }
            query_builder.push(")");
        }
    }

    // Handle memo filter (regex pattern matching)
    if let Some(ref memo_pattern) = params.memo {
        if needs_entry_join {
            query_builder.push(" AND transaction.memo ~* ");
        } else {
            query_builder.push(" AND memo ~* ");
        }
        query_builder.push_bind(memo_pattern);
    }

    // If we used CTE, close it and count from the results
    if needs_entry_join {
        query_builder.push(
            ")
            SELECT COUNT(*) as count
            FROM transaction
            JOIN tx_ids ON tx_ids.id = transaction.id",
        );
    }

    // Execute the query
    let query = query_builder.build();
    let row = query.fetch_one(pool).await?;
    let count: i64 = row.try_get("count")?;

    Ok(count)
}
