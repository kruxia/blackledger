use blackledger::{
    models::{
        currency::Currency,
        ledger::Ledger,
        account::Account,
        transaction::{CreateTransaction, CreateEntry},
    },
    services::posting::post_transaction,
    services::validation::{
        validate_entries,
        validate_double_entry_balance,
    },
    error::ApiError,
};
use rust_decimal_macros::dec;
use chrono::Utc;
use sqlx::PgPool;

async fn setup_test_data(pool: &PgPool) -> (Ledger, Account, Account, Currency) {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"INSERT INTO ledger (name) VALUES ('Test Ledger') RETURNING *"#
    )
    .fetch_one(pool)
    .await
    .unwrap();
    
    let currency = sqlx::query_as::<_, Currency>(
        r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING RETURNING *"#
    )
    .fetch_one(pool)
    .await
    .unwrap();
    
    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Cash', 'DR') 
        RETURNING *
        "#
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();
    
    let revenue_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Revenue', 'CR') 
        RETURNING *
        "#
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();
    
    (ledger, cash_account, revenue_account, currency)
}

#[sqlx::test]
async fn test_valid_transaction_posting(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_test_data(&pool).await;
    
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Test transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };
    
    let (transaction, entries) = post_transaction(&pool, &input, Some("test_user")).await.unwrap();
    
    assert_eq!(transaction.ledger_id, ledger.id);
    assert_eq!(transaction.memo, Some("Test transaction".to_string()));
    assert_eq!(entries.len(), 2);
    
    let cash_entry = entries.iter().find(|e| e.account_id == cash_account.id).unwrap();
    assert_eq!(cash_entry.debit, Some(dec!(100.00)));
    assert_eq!(cash_entry.credit, None);
    
    let revenue_entry = entries.iter().find(|e| e.account_id == revenue_account.id).unwrap();
    assert_eq!(revenue_entry.debit, None);
    assert_eq!(revenue_entry.credit, Some(dec!(100.00)));
    
    let updated_cash = sqlx::query_as::<_, Account>(
        r#"SELECT * FROM account WHERE id = $1"#
    )
    .bind(cash_account.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert_eq!(updated_cash.version, Some(cash_entry.id));
}

#[sqlx::test]
async fn test_unbalanced_transaction_fails(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_test_data(&pool).await;
    
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Unbalanced transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(50.00)),
                account_version: None,
            },
        ],
    };
    
    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("does not balance"));
        }
        _ => panic!("Expected validation error for unbalanced transaction"),
    }
}

#[sqlx::test]
async fn test_multi_currency_transaction(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_test_data(&pool).await;
    
    sqlx::query("INSERT INTO currency (code) VALUES ('EUR') ON CONFLICT DO NOTHING")
        .execute(&pool)
        .await
        .unwrap();
    
    let forex_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Forex Gain/Loss', 'CR') 
        RETURNING *
        "#
    )
    .bind(ledger.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Multi-currency transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: forex_account.id,
                currency_code: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85.00)),
                account_version: None,
            },
        ],
    };
    
    let (_transaction, entries) = post_transaction(&pool, &input, None).await.unwrap();
    
    assert_eq!(entries.len(), 4);
    
    let usd_entries: Vec<_> = entries.iter().filter(|e| e.currency_code == "USD").collect();
    assert_eq!(usd_entries.len(), 2);
    
    let eur_entries: Vec<_> = entries.iter().filter(|e| e.currency_code == "EUR").collect();
    assert_eq!(eur_entries.len(), 2);
}

#[sqlx::test]
async fn test_optimistic_locking(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_test_data(&pool).await;
    
    let input1 = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("First transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(50.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(50.00)),
                account_version: None,
            },
        ],
    };
    
    let (_transaction1, entries1) = post_transaction(&pool, &input1, None).await.unwrap();
    let cash_entry1 = entries1.iter().find(|e| e.account_id == cash_account.id).unwrap();
    
    let input2_correct_version = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Second transaction with correct version".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(25.00)),
                credit: None,
                account_version: Some(cash_entry1.id),
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(25.00)),
                account_version: None,
            },
        ],
    };
    
    let result2 = post_transaction(&pool, &input2_correct_version, None).await;
    assert!(result2.is_ok());
    
    let input3_wrong_version = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Transaction with wrong version".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(30.00)),
                credit: None,
                account_version: Some(cash_entry1.id),
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(30.00)),
                account_version: None,
            },
        ],
    };
    
    let result3 = post_transaction(&pool, &input3_wrong_version, None).await;
    assert!(result3.is_err());
    
    match result3.unwrap_err() {
        ApiError::OptimisticLockError => {}
        _ => panic!("Expected optimistic lock error"),
    }
}

#[sqlx::test]
async fn test_invalid_currency_fails(pool: PgPool) {
    let (ledger, cash_account, revenue_account, _currency) = setup_test_data(&pool).await;
    
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Invalid currency transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "XXX".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "XXX".to_string(),
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
            assert!(msg.contains("Currency XXX does not exist"));
        }
        _ => panic!("Expected validation error for invalid currency"),
    }
}

#[sqlx::test]
async fn test_invalid_account_fails(pool: PgPool) {
    let (ledger, _cash_account, _revenue_account, _currency) = setup_test_data(&pool).await;
    
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Invalid account transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: 999999,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 999998,
                currency_code: "USD".to_string(),
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
            assert!(msg.contains("Account 999999 does not exist"));
        }
        _ => panic!("Expected validation error for invalid account"),
    }
}

#[test]
fn test_validate_entries_unit() {
    let valid_entries = vec![
        CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency_code: "USD".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
    ];
    
    assert!(validate_entries(&valid_entries).is_ok());
    
    let empty_entries: Vec<CreateEntry> = vec![];
    assert!(validate_entries(&empty_entries).is_err());
    
    let both_debit_credit = vec![
        CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: Some(dec!(100)),
            account_version: None,
        },
    ];
    assert!(validate_entries(&both_debit_credit).is_err());
    
    let negative_amount = vec![
        CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(-100)),
            credit: None,
            account_version: None,
        },
    ];
    assert!(validate_entries(&negative_amount).is_err());
}

#[test]
fn test_validate_double_entry_balance_unit() {
    let balanced_entries = vec![
        CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency_code: "USD".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
    ];
    
    assert!(validate_double_entry_balance(&balanced_entries).is_ok());
    
    let unbalanced_entries = vec![
        CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency_code: "USD".to_string(),
            debit: None,
            credit: Some(dec!(50)),
            account_version: None,
        },
    ];
    
    let result = validate_double_entry_balance(&unbalanced_entries);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not balance"));
}