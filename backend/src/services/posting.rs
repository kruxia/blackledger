//! Transaction posting service
//!
//! Handles the complete transaction posting workflow including validation,
//! database persistence, account version updates, and audit logging.

use chrono::Utc;
use sqlx::PgPool;

use crate::error::{ApiError, ApiResult};
use crate::models::entry::Entry;
use crate::models::transaction::{CreateTransaction, Transaction};
use crate::services::validation::{validate_account_versions, validate_transaction};

/// Posts a validated transaction to the ledger
///
/// # Process
///
/// 1. Validates the transaction (balances, accounts, currencies)
/// 2. Begins database transaction
/// 3. Validates account versions with row locking
/// 4. Creates transaction record with audit metadata
/// 5. Creates entry records
/// 6. Updates account versions
/// 7. Commits database transaction
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `input` - Transaction to post
/// * `user_id` - Optional user ID for audit trail
///
/// # Returns
///
/// The posted transaction and its entries
///
/// # Errors
///
/// - `ApiError::Validation` - Invalid transaction structure
/// - `ApiError::OptimisticLockError` - Account version mismatch
/// - `ApiError::Database` - Database operation failed
pub async fn post_transaction(
    pool: &PgPool,
    input: &CreateTransaction,
    user_id: Option<&str>,
) -> ApiResult<(Transaction, Vec<Entry>)> {
    validate_transaction(pool, input).await?;

    let mut tx = pool.begin().await?;

    let result = post_transaction_in_tx(pool, &mut tx, input, user_id).await?;

    tx.commit().await?;

    Ok(result)
}

pub async fn get_transaction_with_entries(
    pool: &PgPool,
    transaction_id: i64,
) -> ApiResult<(Transaction, Vec<Entry>)> {
    let transaction =
        sqlx::query_as::<_, Transaction>(r#"SELECT * FROM transaction WHERE id = $1"#)
            .bind(transaction_id)
            .fetch_one(pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    ApiError::NotFound(format!("Transaction {} not found", transaction_id))
                }
                _ => ApiError::Database(e),
            })?;

    let entries = sqlx::query_as::<_, Entry>(
        r#"
        SELECT * FROM entry 
        WHERE transaction_id = $1
        ORDER BY id
        "#,
    )
    .bind(transaction_id)
    .fetch_all(pool)
    .await?;

    Ok((transaction, entries))
}

/// Creates a reversal transaction to offset a previous transaction
///
/// Reversals are used to correct errors since transactions are immutable.
/// The reversal swaps debits and credits from the original transaction.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `transaction_id` - ID of transaction to reverse
/// * `memo` - Optional memo for the reversal
/// * `user_id` - Optional user ID for audit trail
///
/// # Returns
///
/// The reversal transaction and its entries
pub async fn reverse_transaction(
    pool: &PgPool,
    transaction_id: i64,
    memo: Option<String>,
    user_id: Option<&str>,
) -> ApiResult<(Transaction, Vec<Entry>)> {
    let (original_transaction, original_entries) =
        get_transaction_with_entries(pool, transaction_id).await?;

    let mut reversed_entries = Vec::new();
    for entry in original_entries {
        reversed_entries.push(crate::models::transaction::CreateEntry {
            account_id: entry.account_id,
            currency: entry.currency,
            debit: entry.credit,
            credit: entry.debit,
            account_version: None,
        });
    }

    let reversal_memo =
        memo.unwrap_or_else(|| format!("Reversal of transaction {}", transaction_id));

    let mut meta = serde_json::json!({
        "reversed_transaction_id": transaction_id,
        "reversal": true,
    });

    if let Some(original_meta) = original_transaction.meta {
        meta["original_meta"] = original_meta;
    }

    let reversal_input = CreateTransaction {
        ledger_id: original_transaction.ledger_id,
        effective: Some(original_transaction.effective), // Use same effective date as original
        memo: Some(reversal_memo),
        meta: Some(meta),
        entries: reversed_entries,
    };

    post_transaction(pool, &reversal_input, user_id).await
}

/// Posts multiple transactions in a single atomic database transaction
///
/// All transactions must succeed or the entire batch will be rolled back.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `inputs` - Transactions to post
/// * `user_id` - Optional user ID for audit trail
///
/// # Returns
///
/// Vector of posted transactions and their entries
///
/// # Errors
///
/// - `ApiError::Validation` - Any transaction is invalid
/// - `ApiError::OptimisticLockError` - Account version mismatch in any transaction
/// - `ApiError::Database` - Database operation failed
pub async fn post_transactions_batch(
    pool: &PgPool,
    inputs: &[CreateTransaction],
    user_id: Option<&str>,
) -> ApiResult<Vec<(Transaction, Vec<Entry>)>> {
    // Validate all transactions first
    for input in inputs {
        validate_transaction(pool, input).await?;
    }

    // Start a single database transaction for all operations
    let mut tx = pool.begin().await?;
    let mut results = Vec::new();

    for input in inputs {
        // Post each transaction within the shared database transaction
        let (transaction, entries) = post_transaction_in_tx(pool, &mut tx, input, user_id).await?;
        results.push((transaction, entries));
    }

    // Commit all transactions together
    tx.commit().await?;

    Ok(results)
}

/// Internal helper to post a transaction within an existing database transaction
///
/// This is used by both `post_transaction` and `post_transactions_batch` to share
/// the core posting logic while allowing batch operations to maintain atomicity.
async fn post_transaction_in_tx(
    pool: &PgPool,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &CreateTransaction,
    user_id: Option<&str>,
) -> ApiResult<(Transaction, Vec<Entry>)> {
    // Validate account versions with row locking
    validate_account_versions(pool, &input.entries, tx).await?;

    let created = Utc::now();
    let mut meta = input.meta.clone();

    if let Some(uid) = user_id {
        let user_meta = serde_json::json!({
            "posted_by": uid,
            "posted_at": created.to_rfc3339(),
        });

        if let Some(ref mut existing_meta) = meta {
            if let serde_json::Value::Object(map) = existing_meta {
                map.insert("audit".to_string(), user_meta);
            }
        } else {
            meta = Some(serde_json::json!({
                "audit": user_meta
            }));
        }
    }

    // Use provided effective date or default to created timestamp
    let effective = input.effective.unwrap_or(created);

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transaction (ledger_id, created, effective, memo, meta)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.ledger_id)
    .bind(created)
    .bind(effective)
    .bind(&input.memo)
    .bind(&meta)
    .fetch_one(&mut **tx)
    .await?;

    let mut entries = Vec::new();

    for entry_input in &input.entries {
        let entry = sqlx::query_as::<_, Entry>(
            r#"
            INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(input.ledger_id)
        .bind(transaction.id)
        .bind(entry_input.account_id)
        .bind(&entry_input.currency)
        .bind(entry_input.debit)
        .bind(entry_input.credit)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE account
            SET version = $1
            WHERE id = $2
            "#,
        )
        .bind(entry.id)
        .bind(entry_input.account_id)
        .execute(&mut **tx)
        .await?;

        entries.push(entry);
    }

    Ok((transaction, entries))
}
