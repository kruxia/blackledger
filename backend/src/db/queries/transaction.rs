use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::error::{ApiError, ApiResult};
use crate::models::transaction::{Transaction, CreateTransaction};

pub async fn create_transaction(
    pool: &PgPool,
    input: &CreateTransaction,
) -> ApiResult<Transaction> {
    // Validate that the transaction balances
    validate_transaction_balance(&input.entries)?;

    // Start a database transaction
    let mut tx = pool.begin().await?;

    // Create the transaction
    let transaction_id = Uuid::new_v4();
    let posted = Utc::now();
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transaction (id, ledger_id, posted, effective, description, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(transaction_id)
    .bind(input.ledger_id)
    .bind(posted)
    .bind(input.effective)
    .bind(&input.description)
    .bind(&input.metadata)
    .fetch_one(&mut *tx)
    .await?;

    // Create entries and update account versions
    for entry in &input.entries {
        // Validate account version if provided
        if let Some(expected_version) = entry.account_version {
            let row = sqlx::query(
                r#"SELECT latest_entry_id FROM account WHERE id = $1"#
            )
            .bind(entry.account_id)
            .fetch_one(&mut *tx)
            .await?;
            
            let current_version: Option<Uuid> = row.get("latest_entry_id");

            if current_version != Some(expected_version) {
                return Err(ApiError::OptimisticLockError);
            }
        }

        // Create the entry
        let entry_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO entry (id, transaction_id, account_id, currency_code, dr, cr, description, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(entry_id)
        .bind(transaction_id)
        .bind(entry.account_id)
        .bind(&entry.currency_code)
        .bind(entry.dr)
        .bind(entry.cr)
        .bind(&entry.description)
        .bind(&entry.metadata)
        .execute(&mut *tx)
        .await?;

        // Update account's latest_entry_id
        sqlx::query(
            r#"
            UPDATE account
            SET latest_entry_id = $1,
                updated = CURRENT_TIMESTAMP
            WHERE id = $2
            "#
        )
        .bind(entry_id)
        .bind(entry.account_id)
        .execute(&mut *tx)
        .await?;
    }

    // Commit the transaction
    tx.commit().await?;

    Ok(transaction)
}

pub async fn get_transaction_by_id(pool: &PgPool, id: Uuid) -> ApiResult<Transaction> {
    let transaction = sqlx::query_as::<_, Transaction>(
        r#"SELECT * FROM transaction WHERE id = $1"#
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Transaction {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    Ok(transaction)
}

pub async fn list_transactions(
    pool: &PgPool,
    ledger_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Transaction>> {
    let transactions = if let Some(lid) = ledger_id {
        sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transaction
            WHERE ledger_id = $1
            ORDER BY posted DESC
            LIMIT $2
            OFFSET $3
            "#
        )
        .bind(lid)
        .bind(limit.unwrap_or(100))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transaction
            ORDER BY posted DESC
            LIMIT $1
            OFFSET $2
            "#
        )
        .bind(limit.unwrap_or(100))
        .bind(offset.unwrap_or(0))
        .fetch_all(pool)
        .await?
    };

    Ok(transactions)
}

fn validate_transaction_balance(entries: &[crate::models::transaction::CreateEntry]) -> ApiResult<()> {
    // Group entries by currency
    let mut balances: HashMap<String, Decimal> = HashMap::new();

    for entry in entries {
        let balance = balances.entry(entry.currency_code.clone()).or_insert(Decimal::ZERO);
        
        if let Some(dr) = entry.dr {
            *balance += dr;
        }
        
        if let Some(cr) = entry.cr {
            *balance -= cr;
        }
    }

    // Check that each currency balances to zero
    for (currency, balance) in balances {
        if balance != Decimal::ZERO {
            return Err(ApiError::Validation(
                format!("Transaction does not balance for currency {}: {}", currency, balance)
            ));
        }
    }

    Ok(())
}