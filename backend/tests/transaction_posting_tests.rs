use blackledger::{
    error::ApiError,
    models::{
        account::Account,
        currency::Currency,
        ledger::Ledger,
        transaction::{CreateEntry, CreateTransaction},
    },
    services::posting::post_transaction,
    services::validation::{validate_double_entry_balance, validate_entries},
};
use chrono::Utc;
use rust_decimal_macros::dec;
use sqlx::PgPool;

async fn setup_test_data(pool: &PgPool) -> (Ledger, Account, Account, Currency) {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"INSERT INTO ledger (name) VALUES ('Test Ledger') RETURNING *"#,
    )
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
        VALUES ($1, 'Cash', 'DR') 
        RETURNING *
        "#,
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
        "#,
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();

    (ledger, cash_account, revenue_account, currency)
}

async fn setup_multi_currency_test_data(
    pool: &PgPool,
) -> (Ledger, Account, Account, Account, Currency, Currency) {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"INSERT INTO ledger (name) VALUES ('Multi-Currency Ledger') RETURNING *"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let usd = sqlx::query_as::<_, Currency>(
        r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING RETURNING *"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let eur = sqlx::query_as::<_, Currency>(
        r#"INSERT INTO currency (code) VALUES ('EUR') ON CONFLICT DO NOTHING RETURNING *"#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let cash_usd = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Cash USD', 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();

    let cash_eur = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Cash EUR', 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();

    let revenue = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Revenue Multi-Currency', 'CR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .fetch_one(pool)
    .await
    .unwrap();

    (ledger, cash_usd, cash_eur, revenue, usd, eur)
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

    let (transaction, entries) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    assert_eq!(transaction.ledger_id, ledger.id);
    assert_eq!(transaction.memo, Some("Test transaction".to_string()));
    assert_eq!(entries.len(), 2);

    let cash_entry = entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();
    assert_eq!(cash_entry.debit, Some(dec!(100.00)));
    assert_eq!(cash_entry.credit, None);

    let revenue_entry = entries
        .iter()
        .find(|e| e.account_id == revenue_account.id)
        .unwrap();
    assert_eq!(revenue_entry.debit, None);
    assert_eq!(revenue_entry.credit, Some(dec!(100.00)));

    let updated_cash = sqlx::query_as::<_, Account>(r#"SELECT * FROM account WHERE id = $1"#)
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
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
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
        "#,
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
            CreateEntry {
                account_id: cash_account.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: forex_account.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85.00)),
                account_version: None,
            },
        ],
    };

    let (_transaction, entries) = post_transaction(&pool, &input, None).await.unwrap();

    assert_eq!(entries.len(), 4);

    let usd_entries: Vec<_> = entries.iter().filter(|e| e.currency == "USD").collect();
    assert_eq!(usd_entries.len(), 2);

    let eur_entries: Vec<_> = entries.iter().filter(|e| e.currency == "EUR").collect();
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

    let (_transaction1, entries1) = post_transaction(&pool, &input1, None).await.unwrap();
    let cash_entry1 = entries1
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();

    let input2_correct_version = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Second transaction with correct version".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(25.00)),
                credit: None,
                account_version: Some(cash_entry1.id),
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
                currency: "USD".to_string(),
                debit: Some(dec!(30.00)),
                credit: None,
                account_version: Some(cash_entry1.id),
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
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
                currency: "XXX".to_string(),
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
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 999998,
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
            assert!(
                msg.contains("Account 999999 does not exist")
                    || msg.contains("Account 999998 does not exist")
            );
        }
        _ => panic!("Expected validation error for invalid account"),
    }
}

#[test]
fn test_validate_entries_unit() {
    let valid_entries = vec![
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
    ];

    assert!(validate_entries(&valid_entries).is_ok());

    let empty_entries: Vec<CreateEntry> = vec![];
    assert!(validate_entries(&empty_entries).is_err());

    let both_debit_credit = vec![CreateEntry {
        account_id: 1,
        currency: "USD".to_string(),
        debit: Some(dec!(100)),
        credit: Some(dec!(100)),
        account_version: None,
    }];
    assert!(validate_entries(&both_debit_credit).is_err());

    let negative_amount = vec![CreateEntry {
        account_id: 1,
        currency: "USD".to_string(),
        debit: Some(dec!(-100)),
        credit: None,
        account_version: None,
    }];
    assert!(validate_entries(&negative_amount).is_err());
}

#[test]
fn test_validate_double_entry_balance_unit() {
    let balanced_entries = vec![
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
    ];

    assert!(validate_double_entry_balance(&balanced_entries).is_ok());

    let unbalanced_entries = vec![
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
            credit: Some(dec!(50)),
            account_version: None,
        },
    ];

    let result = validate_double_entry_balance(&unbalanced_entries);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not balance"));
}

#[sqlx::test]
async fn test_multi_currency_transaction_valid(pool: PgPool) {
    let (ledger, cash_usd, cash_eur, revenue, _usd, _eur) =
        setup_multi_currency_test_data(&pool).await;

    // Valid multi-currency transaction: Each currency balances independently
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Multi-currency sale".to_string()),
        meta: None,
        entries: vec![
            // USD entries
            CreateEntry {
                account_id: cash_usd.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
            // EUR entries
            CreateEntry {
                account_id: cash_eur.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, entries) = post_transaction(&pool, &input, Some("test_user"))
        .await
        .unwrap();

    assert_eq!(transaction.ledger_id, ledger.id);
    assert_eq!(entries.len(), 4);

    // Verify USD entries balance
    let usd_entries: Vec<_> = entries.iter().filter(|e| e.currency == "USD").collect();
    assert_eq!(usd_entries.len(), 2);
    let usd_debits: rust_decimal::Decimal = usd_entries.iter().filter_map(|e| e.debit).sum();
    let usd_credits: rust_decimal::Decimal = usd_entries.iter().filter_map(|e| e.credit).sum();
    assert_eq!(usd_debits, usd_credits);

    // Verify EUR entries balance
    let eur_entries: Vec<_> = entries.iter().filter(|e| e.currency == "EUR").collect();
    assert_eq!(eur_entries.len(), 2);
    let eur_debits: rust_decimal::Decimal = eur_entries.iter().filter_map(|e| e.debit).sum();
    let eur_credits: rust_decimal::Decimal = eur_entries.iter().filter_map(|e| e.credit).sum();
    assert_eq!(eur_debits, eur_credits);
}

#[sqlx::test]
async fn test_multi_currency_transaction_unbalanced_fails(pool: PgPool) {
    let (ledger, cash_usd, cash_eur, revenue, _usd, _eur) =
        setup_multi_currency_test_data(&pool).await;

    // Invalid: USD balances but EUR doesn't
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Unbalanced multi-currency".to_string()),
        meta: None,
        entries: vec![
            // USD entries (balanced)
            CreateEntry {
                account_id: cash_usd.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
            // EUR entries (unbalanced!)
            CreateEntry {
                account_id: cash_eur.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(80.00)), // Wrong amount!
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("EUR"));
            assert!(msg.contains("does not balance"));
        }
        _ => panic!("Expected validation error for unbalanced EUR"),
    }
}

#[test]
fn test_validate_multi_currency_balance_unit() {
    // Test that each currency must balance independently
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

    // Test mixed currencies cannot offset each other
    let mixed_unbalanced = vec![
        CreateEntry {
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        },
        CreateEntry {
            account_id: 2,
            currency: "EUR".to_string(),
            debit: None,
            credit: Some(dec!(100)),
            account_version: None,
        },
    ];

    let result = validate_double_entry_balance(&mixed_unbalanced);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("does not balance"));
    // Should mention the specific currencies that don't balance
    assert!(error_msg.contains("USD") || error_msg.contains("EUR"));
}

#[sqlx::test]
async fn test_account_with_multiple_currency_balances(pool: PgPool) {
    let (ledger, _cash_usd, _cash_eur, revenue, _usd, _eur) =
        setup_multi_currency_test_data(&pool).await;

    // Post transactions in different currencies to the same account
    let tx1 = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("USD revenue".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: revenue.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
        ],
    };

    let tx2 = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("EUR revenue".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: revenue.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85.00)),
                account_version: None,
            },
            CreateEntry {
                account_id: revenue.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
        ],
    };

    // Post both transactions
    post_transaction(&pool, &tx1, None).await.unwrap();
    post_transaction(&pool, &tx2, None).await.unwrap();

    // Query account balances
    use blackledger::api::search::{AccountSearchParams, SearchParams};
    use blackledger::db::queries::account::get_accounts_with_balances;

    let params = AccountSearchParams {
        id: Some(revenue.id.to_string()),
        ledger_id: Some(ledger.id.to_string()),
        parent_id: None,
        version: None,
        number: None,
        name: None,
        normal: None,
        base: SearchParams::default(),
    };

    let accounts_with_balances = get_accounts_with_balances(&pool, &params).await.unwrap();

    assert_eq!(accounts_with_balances.len(), 1);
    let revenue_balances = &accounts_with_balances[0];
    assert_eq!(revenue_balances.account.id, revenue.id);

    // Revenue account should have balances in both currencies
    // Since we debited and credited the same amounts, balances should be zero
    assert_eq!(revenue_balances.balances.len(), 2);
    assert_eq!(revenue_balances.balances.get("USD"), Some(&dec!(0)));
    assert_eq!(revenue_balances.balances.get("EUR"), Some(&dec!(0)));
}
