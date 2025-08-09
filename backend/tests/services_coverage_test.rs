// Tests for services module to improve coverage

mod common;
use uuid;

use blackledger::{
    db::queries::{account::create_account, currency::create_currency, ledger::create_ledger},
    error::ApiError,
    models::{
        account::{CreateAccount, NormalBalance},
        ledger::CreateLedger,
        transaction::{CreateEntry, CreateTransaction},
    },
    services::{
        posting::post_transaction,
        validation::{validate_double_entry_balance, validate_entries, validate_transaction},
    },
};
use chrono::Utc;
use rust_decimal_macros::dec;
use serde_json::json;

#[tokio::test]
async fn test_post_transaction_with_metadata() {
    let pool = common::setup_test_db().await;

    // Setup test data
    let ledger = create_ledger(
        &pool,
        &CreateLedger {
            name: format!("Test Ledger {}", uuid::Uuid::new_v4()),
        },
    )
    .await
    .unwrap();

    create_currency(&pool, "USD").await.unwrap();

    let cash_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Cash".to_string(),
            number: Some(1000),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let revenue_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Revenue".to_string(),
            number: Some(4000),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    // Test transaction with metadata
    let transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(Utc::now()),
        memo: Some("Transaction with metadata".to_string()),
        meta: Some(json!({
            "invoice_number": "INV-001",
            "customer_id": 12345,
            "tags": ["sale", "retail"]
        })),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(500.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(500.00)),
                account_version: None,
            },
        ],
    };

    let (posted_transaction, entries) = post_transaction(&pool, &transaction, Some("test_user"))
        .await
        .unwrap();

    assert_eq!(
        posted_transaction.memo,
        Some("Transaction with metadata".to_string())
    );
    assert!(posted_transaction.meta.is_some());
    assert_eq!(
        posted_transaction.meta.as_ref().unwrap()["invoice_number"],
        "INV-001"
    );
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn test_post_multiple_transactions() {
    let pool = common::setup_test_db().await;

    // Setup test data
    let ledger = create_ledger(
        &pool,
        &CreateLedger {
            name: format!("Batch Test Ledger {}", uuid::Uuid::new_v4()),
        },
    )
    .await
    .unwrap();

    create_currency(&pool, "USD").await.unwrap();

    let cash_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Cash".to_string(),
            number: Some(1000),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let expense_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Expenses".to_string(),
            number: Some(5000),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Create batch of transactions
    let transactions = vec![
        CreateTransaction {
            ledger_id: ledger.id,
            effective: Some(Utc::now()),
            memo: Some("Batch transaction 1".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: expense_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(100.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: cash_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(100.00)),
                    account_version: None,
                },
            ],
        },
        CreateTransaction {
            ledger_id: ledger.id,
            effective: Some(Utc::now()),
            memo: Some("Batch transaction 2".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: expense_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(200.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: cash_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(200.00)),
                    account_version: None,
                },
            ],
        },
    ];

    // Post transactions sequentially
    let result1 = post_transaction(&pool, &transactions[0], Some("batch_user"))
        .await
        .unwrap();
    assert_eq!(
        result1.0.memo,
        Some("Batch transaction 1".to_string())
    );

    let result2 = post_transaction(&pool, &transactions[1], Some("batch_user"))
        .await
        .unwrap();
    assert_eq!(
        result2.0.memo,
        Some("Batch transaction 2".to_string())
    );

    // Verify both transactions were posted
    assert!(result1.0.id != result2.0.id);
    assert_eq!(result1.1.len(), 2);
    assert_eq!(result2.1.len(), 2);
}

#[tokio::test]
async fn test_validate_transaction_with_invalid_account() {
    let pool = common::setup_test_db().await;

    let ledger = create_ledger(
        &pool,
        &CreateLedger {
            name: format!("Validation Test Ledger {}", uuid::Uuid::new_v4()),
        },
    )
    .await
    .unwrap();

    create_currency(&pool, "USD").await.unwrap();

    // Transaction with non-existent account
    let transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(Utc::now()),
        memo: Some("Invalid account test".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: 999999, // Non-existent account
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 999998, // Non-existent account
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let result = validate_transaction(&pool, &transaction).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound(msg) => assert!(msg.contains("Account")),
        ApiError::Validation(msg) => assert!(msg.contains("Account") || msg.contains("account")),
        e => panic!("Expected NotFound or Validation error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_validate_transaction_with_invalid_currency() {
    let pool = common::setup_test_db().await;

    let ledger = create_ledger(
        &pool,
        &CreateLedger {
            name: format!("Currency Test Ledger {}", uuid::Uuid::new_v4()),
        },
    )
    .await
    .unwrap();

    let cash_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Cash".to_string(),
            number: Some(1000),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let revenue_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Revenue".to_string(),
            number: Some(4000),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    // Transaction with non-existent currency
    let transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(Utc::now()),
        memo: Some("Invalid currency test".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "XXX".to_string(), // Non-existent currency
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "XXX".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let result = validate_transaction(&pool, &transaction).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound(msg) => assert!(msg.contains("Currency")),
        ApiError::Validation(msg) => assert!(msg.contains("Currency") || msg.contains("currency")),
        e => panic!("Expected NotFound or Validation error for currency, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_validate_entries_edge_cases() {
    // Test with zero amounts
    let entries_with_zero = vec![
        CreateEntry {
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(0)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency: "USD".to_string(),
            debit: None,
            credit: Some(dec!(0)),
            account_version: None,
        },
    ];

    let result = validate_entries(&entries_with_zero);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => assert!(msg.contains("positive")),
        _ => panic!("Expected Validation error for zero amounts"),
    }

    // Test with negative amounts
    let entries_with_negative = vec![
        CreateEntry {
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(-100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency: "USD".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
    ];

    let result = validate_entries(&entries_with_negative);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => assert!(msg.contains("positive")),
        _ => panic!("Expected Validation error for negative amounts"),
    }

    // Test with both debit and credit
    let entries_with_both = vec![CreateEntry {
        account_id: 1,
        currency: "USD".to_string(),
        debit: Some(dec!(100)),
        credit: Some(dec!(100)),
        account_version: None,
    }];

    let result = validate_entries(&entries_with_both);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => assert!(msg.contains("both debit and credit")),
        _ => panic!("Expected Validation error for both debit and credit"),
    }

    // Test with neither debit nor credit
    let entries_with_neither = vec![CreateEntry {
        account_id: 1,
        currency: "USD".to_string(),
        debit: None,
        credit: None,
        account_version: None,
    }];

    let result = validate_entries(&entries_with_neither);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => assert!(msg.contains("either debit or credit")),
        _ => panic!("Expected Validation error for neither debit nor credit"),
    }
}

#[tokio::test]
async fn test_validate_double_entry_balance_complex() {
    // Test with multiple currencies that each balance
    let multi_currency_balanced = vec![
        CreateEntry {
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency: "USD".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
        CreateEntry {
            account_id: 3,
            currency: "EUR".to_string(),
            debit: Some(dec!(85)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 4,
            currency: "EUR".to_string(),
            debit: None,
            credit: Some(dec!(85)),
            account_version: None,
        },
    ];

    assert!(validate_double_entry_balance(&multi_currency_balanced).is_ok());

    // Test with multiple currencies where one doesn't balance
    let multi_currency_unbalanced = vec![
        CreateEntry {
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency: "USD".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
        CreateEntry {
            account_id: 3,
            currency: "EUR".to_string(),
            debit: Some(dec!(85)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 4,
            currency: "EUR".to_string(),
            debit: None,
            credit: Some(dec!(80)), // Unbalanced EUR
            account_version: None,
        },
    ];

    let result = validate_double_entry_balance(&multi_currency_unbalanced);
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("does not balance"));
        }
        _ => panic!("Expected Validation error for unbalanced transaction"),
    }
}