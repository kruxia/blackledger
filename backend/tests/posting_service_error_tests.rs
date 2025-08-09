use blackledger::{
    error::ApiError,
    models::{
        account::Account,
        ledger::Ledger,
        transaction::{CreateEntry, CreateTransaction},
    },
    services::posting::{get_transaction_with_entries, post_transaction},
};
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn test_get_transaction_with_entries_not_found(pool: PgPool) {
    // Test line 68-70: NotFound error path
    let result = get_transaction_with_entries(&pool, 999999999).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound(msg) => {
            assert!(msg.contains("Transaction 999999999"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[sqlx::test]
async fn test_post_transaction_with_database_constraint_violation(pool: PgPool) {
    // This tests database error handling paths

    // Try to post a transaction with a non-existent ledger (will fail on FK constraint)
    let input = CreateTransaction {
        ledger_id: 999999999, // Non-existent ledger
        effective: None,
        memo: Some("This should fail".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: 1,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 2,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    // This should trigger validation error when checking ledger exists
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("Ledger") || msg.contains("does not exist"));
        }
        _ => panic!("Expected Validation error for non-existent ledger"),
    }
}

#[sqlx::test]
async fn test_transaction_commit_after_successful_post(pool: PgPool) {
    // This test ensures line 52 (tx.commit()) is covered
    let ledger_name = format!("Commit Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    let account1 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Account1 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let account2 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'CR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Account2 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Test commit".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: account1.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    // Post transaction and verify it commits successfully
    let (transaction, entries) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    // Verify transaction was committed by fetching it again
    let (fetched_tx, fetched_entries) = get_transaction_with_entries(&pool, transaction.id)
        .await
        .unwrap();

    assert_eq!(fetched_tx.id, transaction.id);
    assert_eq!(fetched_entries.len(), entries.len());

    // Verify the commit persisted the data
    let count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM transaction WHERE id = $1"#)
        .bind(transaction.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count, 1, "Transaction should be committed to database");
}

#[sqlx::test]
async fn test_entry_creation_covers_await_points(pool: PgPool) {
    // This test specifically targets lines 239 and 270 (await points in entry creation)
    let ledger_name = format!("Entry Await Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    // Create multiple accounts to test the loop
    let mut accounts = Vec::new();
    for i in 0..5 {
        let account = sqlx::query_as::<_, Account>(
            r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, $3) RETURNING *"#,
        )
        .bind(ledger.id)
        .bind(format!("Account {} {}", i, Uuid::new_v4()))
        .bind(if i % 2 == 0 { "DR" } else { "CR" })
        .fetch_one(&pool)
        .await
        .unwrap();
        accounts.push(account);
    }

    // Create a transaction with many entries to ensure loop coverage
    let mut entries = Vec::new();

    // Add entries that balance
    for i in 0..10 {
        entries.push(CreateEntry {
            account_id: accounts[i % 5].id,
            currency: "USD".to_string(),
            debit: if i % 2 == 0 { Some(dec!(10.00)) } else { None },
            credit: if i % 2 == 0 { None } else { Some(dec!(10.00)) },
            account_version: None,
        });
    }

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Multi-entry transaction".to_string()),
        meta: Some(serde_json::json!({
            "test": "entry_await_coverage"
        })),
        entries,
    };

    let (transaction, created_entries) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    assert_eq!(created_entries.len(), 10);
    assert!(transaction.meta.is_some());

    // Verify all entries were created and account versions updated
    for entry in &created_entries {
        let account = sqlx::query_as::<_, Account>(r#"SELECT * FROM account WHERE id = $1"#)
            .bind(entry.account_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        // Account version should be updated to the entry ID
        assert!(account.version.is_some());
    }
}

#[sqlx::test]
async fn test_fetch_all_entries_coverage(pool: PgPool) {
    // This test targets line 82 (fetch_all await point)
    let ledger_name = format!("Fetch All Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    let account1 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Account1 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let account2 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'CR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Account2 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Post a transaction first
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Transaction for fetch_all test".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: account1.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, _) = post_transaction(&pool, &input, None).await.unwrap();

    // Now fetch the transaction with entries to cover line 82
    let (fetched_tx, fetched_entries) = get_transaction_with_entries(&pool, transaction.id)
        .await
        .unwrap();

    assert_eq!(fetched_tx.id, transaction.id);
    assert_eq!(fetched_entries.len(), 2);

    // Verify entries are ordered by ID
    assert!(fetched_entries[0].id < fetched_entries[1].id);
}
