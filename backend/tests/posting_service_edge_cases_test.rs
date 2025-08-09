use blackledger::{
    error::ApiError,
    models::{
        account::Account,
        currency::Currency,
        ledger::Ledger,
        transaction::{CreateEntry, CreateTransaction},
    },
    services::posting::{
        get_transaction_with_entries, post_transaction, post_transactions_batch,
        reverse_transaction,
    },
};
use chrono::Utc;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a unique test ledger with accounts
async fn setup_unique_test_data(pool: &PgPool) -> (Ledger, Account, Account, Currency) {
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(pool)
            .await
            .unwrap();

    let currency = sqlx::query_as::<_, Currency>(
        r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING RETURNING *"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();

    let revenue_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'CR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Revenue {}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();

    (ledger, cash_account, revenue_account, currency)
}

// Test rollback scenarios
#[sqlx::test]
async fn test_transaction_rollback_on_entry_creation_failure(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Create a transaction with an invalid currency that will fail during entry creation
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Transaction that should rollback".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "INVALID_CURRENCY".to_string(), // This will fail validation
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    // Verify no transaction was created (rollback occurred)
    let transaction_count: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM transaction WHERE ledger_id = $1"#)
            .bind(ledger.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        transaction_count, 0,
        "Transaction should have been rolled back"
    );

    // Verify no entries were created
    let entry_count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM entry WHERE ledger_id = $1"#)
        .bind(ledger.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(entry_count, 0, "Entries should have been rolled back");

    // Verify account versions were not updated
    let cash_account_after = sqlx::query_as::<_, Account>(r#"SELECT * FROM account WHERE id = $1"#)
        .bind(cash_account.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(
        cash_account_after.version.is_none(),
        "Account version should not have been updated"
    );
}

#[sqlx::test]
async fn test_transaction_rollback_on_account_version_update_failure(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // First post a valid transaction to set account versions
    let initial_tx = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Initial transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(50.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(50.00)),
                account_version: None,
            },
        ],
    };

    let (_, entries) = post_transaction(&pool, &initial_tx, None).await.unwrap();
    let cash_entry_id = entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap()
        .id;

    // Now try to post with an outdated version (simulating concurrent modification)
    let conflicting_tx = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Conflicting transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(25.00)),
                credit: None,
                account_version: Some(cash_entry_id - 1), // Wrong version
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(25.00)),
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &conflicting_tx, None).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::OptimisticLockError => {}
        _ => panic!("Expected OptimisticLockError"),
    }

    // Verify only the initial transaction exists
    let transaction_count: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM transaction WHERE ledger_id = $1"#)
            .bind(ledger.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        transaction_count, 1,
        "Only initial transaction should exist"
    );
}

// Test database constraint violations
#[sqlx::test]
async fn test_transaction_with_nonexistent_ledger_fails(pool: PgPool) {
    // Create accounts but use non-existent ledger ID
    let nonexistent_ledger_id = 999999999;

    let input = CreateTransaction {
        ledger_id: nonexistent_ledger_id,
        effective: None,
        memo: Some("Transaction with invalid ledger".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: 1, // These won't matter since validation will fail
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

    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("does not exist") || msg.contains("not found"));
        }
        _ => panic!("Expected validation error for non-existent ledger"),
    }
}

#[sqlx::test]
async fn test_transaction_with_mismatched_ledger_and_account_fails(pool: PgPool) {
    // Create two separate ledgers with accounts
    let ledger1_name = format!("Ledger 1 {}", Uuid::new_v4());
    let ledger1 =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger1_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    let ledger2_name = format!("Ledger 2 {}", Uuid::new_v4());
    let ledger2 =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger2_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    let account_ledger1 = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger1.id)
    .bind(format!("Account L1 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let account_ledger2 = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'CR') 
        RETURNING *
        "#,
    )
    .bind(ledger2.id)
    .bind(format!("Account L2 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Try to post transaction to ledger1 using account from ledger2
    let input = CreateTransaction {
        ledger_id: ledger1.id,
        effective: None,
        memo: Some("Cross-ledger transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: account_ledger1.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account_ledger2.id, // Wrong ledger!
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("does not belong to ledger") || msg.contains("not in ledger"));
        }
        _ => panic!("Expected validation error for cross-ledger transaction"),
    }
}

// Test error conversion paths
#[sqlx::test]
async fn test_get_transaction_with_entries_not_found(pool: PgPool) {
    let nonexistent_id = 999999999;
    let result = get_transaction_with_entries(&pool, nonexistent_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound(msg) => {
            assert!(msg.contains(&nonexistent_id.to_string()));
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[sqlx::test]
async fn test_reverse_already_reversed_transaction(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Post original transaction
    let original = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Original transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (original_tx, _) = post_transaction(&pool, &original, None).await.unwrap();

    // Reverse it once
    let (reversal_tx, _) = reverse_transaction(
        &pool,
        original_tx.id,
        Some("First reversal".to_string()),
        Some("user1"),
    )
    .await
    .unwrap();

    // Try to reverse the reversal (this should work but create another reversal)
    let (double_reversal_tx, double_reversal_entries) = reverse_transaction(
        &pool,
        reversal_tx.id,
        Some("Reversing the reversal".to_string()),
        Some("user2"),
    )
    .await
    .unwrap();

    // Verify the double reversal has the correct metadata
    assert!(double_reversal_tx.meta.is_some());
    let meta = double_reversal_tx.meta.unwrap();
    assert_eq!(meta["reversed_transaction_id"], reversal_tx.id);
    assert_eq!(meta["reversal"], true);

    // The double reversal should have the same amounts as the original
    assert_eq!(double_reversal_entries.len(), 2);
    let cash_entry = double_reversal_entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();
    assert_eq!(cash_entry.debit, Some(dec!(100.00))); // Same as original
    assert_eq!(cash_entry.credit, None);
}

// Test concurrent posting scenarios
#[sqlx::test]
async fn test_concurrent_posting_with_version_conflicts(pool: PgPool) {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // First transaction to establish initial versions
    let initial = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Initial setup".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (_, initial_entries) = post_transaction(&pool, &initial, None).await.unwrap();
    let initial_cash_version = initial_entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap()
        .id;

    // Now try concurrent transactions with the same version
    let pool = Arc::new(pool);
    let mut tasks = JoinSet::new();

    for i in 0..3 {
        let pool_clone = Arc::clone(&pool);
        let cash_id = cash_account.id;
        let revenue_id = revenue_account.id;
        let ledger_id = ledger.id;

        tasks.spawn(async move {
            let input = CreateTransaction {
                ledger_id,
                effective: None,
                memo: Some(format!("Concurrent transaction {}", i)),
                meta: None,
                entries: vec![
                    CreateEntry {
                        account_id: cash_id,
                        currency: "USD".to_string(),
                        debit: Some(dec!(10.00)),
                        credit: None,
                        account_version: Some(initial_cash_version), // All use same version
                    },
                    CreateEntry {
                        account_id: revenue_id,
                        currency: "USD".to_string(),
                        debit: None,
                        credit: Some(dec!(10.00)),
                        account_version: None,
                    },
                ],
            };

            post_transaction(&pool_clone, &input, None).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap());
    }

    // Only one should succeed, others should fail with optimistic lock error
    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = results.iter().filter(|r| r.is_err()).count();

    assert_eq!(successes, 1, "Only one transaction should succeed");
    assert_eq!(
        failures, 2,
        "Two transactions should fail with version conflict"
    );

    // Verify the failures are optimistic lock errors
    for result in results.iter().filter(|r| r.is_err()) {
        match result.as_ref().unwrap_err() {
            ApiError::OptimisticLockError => {}
            _ => panic!("Expected OptimisticLockError for version conflict"),
        }
    }
}

// Test edge cases with metadata
#[sqlx::test]
async fn test_transaction_with_empty_metadata_handling(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Test with empty object metadata
    let empty_meta = serde_json::json!({});

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Transaction with empty metadata".to_string()),
        meta: Some(empty_meta),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    // Post with user ID to test audit metadata injection
    let (transaction, _) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    // Verify metadata was properly augmented with audit info
    assert!(transaction.meta.is_some());
    let meta = transaction.meta.unwrap();
    assert!(meta["audit"].is_object());
    assert_eq!(meta["audit"]["posted_by"], "test_user");
}

#[sqlx::test]
async fn test_transaction_with_complex_metadata_preservation(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Test with complex nested metadata
    let complex_meta = serde_json::json!({
        "invoice": {
            "id": "INV-123",
            "items": [
                {"sku": "ABC", "qty": 2},
                {"sku": "XYZ", "qty": 1}
            ]
        },
        "tags": ["urgent", "Q4-2024"],
        "approved_by": ["manager1", "manager2"],
        "special_chars": "Test with 'quotes' and \"double quotes\"",
        "unicode": "Test with émojis 🎉 and special chars ñ"
    });

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Transaction with complex metadata".to_string()),
        meta: Some(complex_meta.clone()),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, _) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    // Verify all original metadata is preserved
    assert!(transaction.meta.is_some());
    let meta = transaction.meta.unwrap();

    assert_eq!(meta["invoice"]["id"], "INV-123");
    assert_eq!(meta["invoice"]["items"][0]["sku"], "ABC");
    assert_eq!(meta["tags"][0], "urgent");
    assert_eq!(meta["approved_by"][1], "manager2");
    assert_eq!(
        meta["special_chars"],
        "Test with 'quotes' and \"double quotes\""
    );
    assert_eq!(meta["unicode"], "Test with émojis 🎉 and special chars ñ");

    // And audit was added
    assert!(meta["audit"].is_object());
    assert_eq!(meta["audit"]["posted_by"], "test_user");
}

// Test batch posting edge cases
#[sqlx::test]
async fn test_empty_batch_posting(pool: PgPool) {
    let empty_batch: Vec<CreateTransaction> = vec![];
    let result = post_transactions_batch(&pool, &empty_batch, None).await;

    // Empty batch should succeed but return empty results
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[sqlx::test]
async fn test_batch_partial_failure_rollback(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    let transactions = vec![
        // First valid transaction
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Valid transaction 1".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: cash_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(100.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: revenue_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(100.00)),
                    account_version: None,
                },
            ],
        },
        // Second valid transaction
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Valid transaction 2".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: cash_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(50.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: revenue_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(50.00)),
                    account_version: None,
                },
            ],
        },
        // Third invalid transaction (will cause batch to fail)
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Invalid transaction".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: 999999999, // Non-existent account
                    currency: "USD".to_string(),
                    debit: Some(dec!(25.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: revenue_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(25.00)),
                    account_version: None,
                },
            ],
        },
    ];

    let result = post_transactions_batch(&pool, &transactions, None).await;
    assert!(result.is_err());

    // Verify no transactions were posted (all rolled back)
    let tx_count: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM transaction WHERE ledger_id = $1"#)
            .bind(ledger.id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        tx_count, 0,
        "All transactions should be rolled back on batch failure"
    );

    // Verify account versions weren't updated
    let cash_after = sqlx::query_as::<_, Account>(r#"SELECT * FROM account WHERE id = $1"#)
        .bind(cash_account.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(
        cash_after.version.is_none(),
        "Account versions should not be updated on rollback"
    );
}

// Test transaction with various decimal edge cases
#[sqlx::test]
async fn test_transaction_with_max_decimal_precision(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Test with maximum precision decimals
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("High precision transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(123456789.123456789)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(123456789.123456789)),
                account_version: None,
            },
        ],
    };

    let (_, entries) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify precision is preserved
    let cash_entry = entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();
    assert_eq!(cash_entry.debit, Some(dec!(123456789.123456789)));
}

#[sqlx::test]
async fn test_transaction_with_very_small_amounts(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    // Test with very small amounts
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Tiny amount transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(0.000000001)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(0.000000001)),
                account_version: None,
            },
        ],
    };

    let (_, entries) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify small amounts are handled correctly
    let cash_entry = entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();
    assert_eq!(cash_entry.debit, Some(dec!(0.000000001)));
}

// Test effective date edge cases
#[sqlx::test]
async fn test_transaction_with_future_effective_date(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    let future_date = Utc::now() + chrono::Duration::days(365);

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(future_date),
        memo: Some("Future dated transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, _) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify future date is accepted
    assert_eq!(transaction.effective, future_date);
    assert!(transaction.created < transaction.effective);
}

#[sqlx::test]
async fn test_transaction_with_very_old_effective_date(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_unique_test_data(&pool).await;

    let old_date = Utc::now() - chrono::Duration::days(3650); // 10 years ago

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(old_date),
        memo: Some("Historical transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, _) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify old date is accepted
    assert_eq!(transaction.effective, old_date);
    assert!(transaction.created > transaction.effective);
}
