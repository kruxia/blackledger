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
        effective: None, // Will default to created timestamp
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
        effective: Some(Utc::now()), // Explicitly set effective date
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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
        effective: None,
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

#[sqlx::test]
async fn test_credit_normal_account_balance_calculation(pool: PgPool) {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"INSERT INTO ledger (name) VALUES ('Test Ledger') RETURNING *"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create USD currency
    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    // Create a credit-normal account (Revenue)
    let revenue_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Revenue', 'CR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create a debit-normal account (Cash)
    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, 'Cash', 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Post a transaction with revenue (credit-normal account receives credit)
    let tx1 = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Revenue transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(1000.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(1000.00)), // Credit to revenue
                account_version: None,
            },
        ],
    };

    post_transaction(&pool, &tx1, None).await.unwrap();

    // Post another revenue transaction
    let tx2 = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Another revenue transaction".to_string()),
        meta: None,
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
                credit: Some(dec!(500.00)), // Another credit to revenue
                account_version: None,
            },
        ],
    };

    post_transaction(&pool, &tx2, None).await.unwrap();

    // Query account balances
    use blackledger::api::search::{AccountSearchParams, SearchParams};
    use blackledger::db::queries::account::get_accounts_with_balances;

    let params = AccountSearchParams {
        id: None, // Get all accounts
        ledger_id: Some(ledger.id.to_string()),
        parent_id: None,
        version: None,
        number: None,
        name: None,
        normal: None,
        base: SearchParams::default(),
    };

    let accounts_with_balances = get_accounts_with_balances(&pool, &params).await.unwrap();

    // Find the revenue account balance
    let revenue_balance = accounts_with_balances
        .iter()
        .find(|ab| ab.account.id == revenue_account.id)
        .unwrap();

    // Find the cash account balance
    let cash_balance = accounts_with_balances
        .iter()
        .find(|ab| ab.account.id == cash_account.id)
        .unwrap();

    // Revenue (CR normal) should show positive 1500 (credits - debits = 1500 - 0)
    assert_eq!(
        revenue_balance.balances.get("USD"),
        Some(&dec!(1500.00)),
        "Revenue account (CR normal) should have positive balance of 1500"
    );

    // Cash (DR normal) should show positive 1500 (debits - credits = 1500 - 0)
    assert_eq!(
        cash_balance.balances.get("USD"),
        Some(&dec!(1500.00)),
        "Cash account (DR normal) should have positive balance of 1500"
    );
}

#[sqlx::test]
async fn test_concurrent_transaction_posting(pool: PgPool) {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    use uuid::Uuid;

    let ledger_name = format!("Concurrent Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Run multiple concurrent transactions
    let pool = Arc::new(pool);
    let mut tasks = JoinSet::new();

    for i in 0..5 {
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
                        account_version: None,
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

    // All transactions should succeed
    assert_eq!(results.len(), 5);
    for result in &results {
        assert!(result.is_ok());
    }

    // Verify final account versions are sequential
    let final_cash = sqlx::query_as::<_, Account>(r#"SELECT * FROM account WHERE id = $1"#)
        .bind(cash_account.id)
        .fetch_one(&*pool)
        .await
        .unwrap();

    assert!(final_cash.version.is_some());
}

#[sqlx::test]
async fn test_transaction_with_zero_amounts(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Zero Amount Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test transaction with zero amounts
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Zero amount transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(0)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(0)),
                account_version: None,
            },
        ],
    };

    // Zero amounts should fail validation
    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("must be positive") || msg.contains("zero"));
        }
        _ => panic!("Expected validation error for zero amounts"),
    }
}

#[sqlx::test]
async fn test_reverse_transaction(pool: PgPool) {
    use blackledger::services::posting::reverse_transaction;
    use uuid::Uuid;

    let ledger_name = format!("Reversal Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Post original transaction
    let original = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Original transaction".to_string()),
        meta: Some(serde_json::json!({"original": true})),
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

    let (original_tx, _original_entries) = post_transaction(&pool, &original, Some("test_user"))
        .await
        .unwrap();

    // Reverse the transaction
    let (reversal_tx, reversal_entries) = reverse_transaction(
        &pool,
        original_tx.id,
        Some("Custom reversal memo".to_string()),
        Some("reversal_user"),
    )
    .await
    .unwrap();

    // Verify reversal metadata
    assert!(reversal_tx.meta.is_some());
    let meta = reversal_tx.meta.unwrap();
    assert_eq!(meta["reversed_transaction_id"], original_tx.id);
    assert_eq!(meta["reversal"], true);
    assert_eq!(meta["original_meta"]["original"], true);

    // Verify reversal memo
    assert_eq!(reversal_tx.memo, Some("Custom reversal memo".to_string()));

    // Verify reversal has same effective date as original
    assert_eq!(reversal_tx.effective, original_tx.effective);

    // Verify entries are reversed (debits and credits swapped)
    assert_eq!(reversal_entries.len(), 2);

    let cash_reversal = reversal_entries
        .iter()
        .find(|e| e.account_id == cash_account.id)
        .unwrap();
    assert_eq!(cash_reversal.debit, None);
    assert_eq!(cash_reversal.credit, Some(dec!(100.00)));

    let revenue_reversal = reversal_entries
        .iter()
        .find(|e| e.account_id == revenue_account.id)
        .unwrap();
    assert_eq!(revenue_reversal.debit, Some(dec!(100.00)));
    assert_eq!(revenue_reversal.credit, None);

    // Verify net effect is zero
    use blackledger::api::search::{AccountSearchParams, SearchParams};
    use blackledger::db::queries::account::get_accounts_with_balances;

    let params = AccountSearchParams {
        id: None,
        ledger_id: Some(ledger.id.to_string()),
        parent_id: None,
        version: None,
        number: None,
        name: None,
        normal: None,
        base: SearchParams::default(),
    };

    let accounts_with_balances = get_accounts_with_balances(&pool, &params).await.unwrap();

    for account_balance in accounts_with_balances {
        assert_eq!(
            account_balance.balances.get("USD"),
            Some(&dec!(0)),
            "Account {} should have zero balance after reversal",
            account_balance.account.name
        );
    }
}

#[sqlx::test]
async fn test_reverse_nonexistent_transaction(pool: PgPool) {
    use blackledger::services::posting::reverse_transaction;

    let result = reverse_transaction(
        &pool, 999999, // Non-existent transaction ID
        None, None,
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound(msg) => {
            assert!(msg.contains("Transaction 999999"));
        }
        _ => panic!("Expected NotFound error for non-existent transaction"),
    }
}

#[sqlx::test]
async fn test_post_transactions_batch(pool: PgPool) {
    use blackledger::services::posting::post_transactions_batch;
    use uuid::Uuid;

    let ledger_name = format!("Batch Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    let expense_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Expense {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create multiple transactions for batch posting
    let transactions = vec![
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Batch transaction 1".to_string()),
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
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Batch transaction 2".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: expense_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(50.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: cash_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(50.00)),
                    account_version: None,
                },
            ],
        },
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Batch transaction 3".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: cash_account.id,
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

    // Post batch
    let results = post_transactions_batch(&pool, &transactions, Some("batch_user"))
        .await
        .unwrap();

    // Verify all transactions were posted
    assert_eq!(results.len(), 3);

    for (i, (tx, entries)) in results.iter().enumerate() {
        assert_eq!(tx.memo, Some(format!("Batch transaction {}", i + 1)));
        assert_eq!(entries.len(), 2);

        // Verify audit metadata
        assert!(tx.meta.is_some());
        let meta = tx.meta.as_ref().unwrap();
        assert_eq!(meta["audit"]["posted_by"], "batch_user");
    }

    // Verify final balances
    use blackledger::api::search::{AccountSearchParams, SearchParams};
    use blackledger::db::queries::account::get_accounts_with_balances;

    let params = AccountSearchParams {
        id: None,
        ledger_id: Some(ledger.id.to_string()),
        parent_id: None,
        version: None,
        number: None,
        name: None,
        normal: None,
        base: SearchParams::default(),
    };

    let accounts_with_balances = get_accounts_with_balances(&pool, &params).await.unwrap();

    let cash_balance = accounts_with_balances
        .iter()
        .find(|ab| ab.account.id == cash_account.id)
        .unwrap();
    assert_eq!(cash_balance.balances.get("USD"), Some(&dec!(75.00))); // 100 - 50 + 25

    let revenue_balance = accounts_with_balances
        .iter()
        .find(|ab| ab.account.id == revenue_account.id)
        .unwrap();
    assert_eq!(revenue_balance.balances.get("USD"), Some(&dec!(125.00))); // 100 + 25

    let expense_balance = accounts_with_balances
        .iter()
        .find(|ab| ab.account.id == expense_account.id)
        .unwrap();
    assert_eq!(expense_balance.balances.get("USD"), Some(&dec!(50.00)));
}

#[sqlx::test]
async fn test_batch_with_invalid_transaction_fails(pool: PgPool) {
    use blackledger::services::posting::post_transactions_batch;
    use uuid::Uuid;

    let ledger_name = format!("Batch Fail Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Mix of valid and invalid transactions
    let transactions = vec![
        // Valid transaction
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Valid transaction".to_string()),
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
        // Invalid transaction (unbalanced)
        CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some("Invalid transaction".to_string()),
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
                    credit: Some(dec!(75.00)), // Unbalanced!
                    account_version: None,
                },
            ],
        },
    ];

    // Batch should fail entirely
    let result = post_transactions_batch(&pool, &transactions, None).await;
    assert!(result.is_err());

    // Verify no transactions were posted (atomicity)
    let count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM transaction WHERE ledger_id = $1"#)
        .bind(ledger.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        count, 0,
        "No transactions should be posted when batch fails"
    );
}

#[sqlx::test]
async fn test_transaction_metadata_and_audit_trail(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Metadata Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test with custom metadata
    let custom_meta = serde_json::json!({
        "invoice_id": "INV-2024-001",
        "customer": "Acme Corp",
        "payment_method": "wire_transfer",
        "tags": ["revenue", "Q1-2024"],
    });

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(Utc::now()),
        memo: Some("Payment for invoice INV-2024-001".to_string()),
        meta: Some(custom_meta.clone()),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(1500.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(1500.00)),
                account_version: None,
            },
        ],
    };

    // Post with user ID for audit
    let (transaction, _entries) = post_transaction(&pool, &input, Some("john.doe@example.com"))
        .await
        .unwrap();

    // Verify metadata preservation and audit trail
    assert!(transaction.meta.is_some());
    let meta = transaction.meta.unwrap();

    // Check custom metadata is preserved
    assert_eq!(meta["invoice_id"], "INV-2024-001");
    assert_eq!(meta["customer"], "Acme Corp");
    assert_eq!(meta["payment_method"], "wire_transfer");
    assert_eq!(meta["tags"][0], "revenue");
    assert_eq!(meta["tags"][1], "Q1-2024");

    // Check audit metadata was added
    assert!(meta["audit"].is_object());
    assert_eq!(meta["audit"]["posted_by"], "john.doe@example.com");
    assert!(meta["audit"]["posted_at"].is_string());

    // Verify posted_at is a valid timestamp
    let posted_at = meta["audit"]["posted_at"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(posted_at).is_ok());
}

#[sqlx::test]
async fn test_transaction_without_user_id_no_audit(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("No Audit Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Transaction without user".to_string()),
        meta: None, // No initial metadata
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

    // Post without user ID
    let (transaction, _entries) = post_transaction(&pool, &input, None).await.unwrap();

    // Should have no metadata when posted without user ID and no initial metadata
    assert!(transaction.meta.is_none());
}

#[sqlx::test]
async fn test_effective_date_handling(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Effective Date Test Ledger {}", Uuid::new_v4());
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

    let cash_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Cash {}", Uuid::new_v4()))
    .fetch_one(&pool)
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
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test with specific effective date
    let specific_date = Utc::now() - chrono::Duration::days(30);

    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: Some(specific_date),
        memo: Some("Backdated transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(250.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(250.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, _entries) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify effective date was set correctly
    assert_eq!(transaction.effective, specific_date);

    // Created date should be current (not backdated)
    assert!(transaction.created > specific_date);
    assert!(transaction.created <= Utc::now());
}

#[sqlx::test]
async fn test_multi_currency_complex_scenarios(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Complex Multi-Currency Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Setup multiple currencies
    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD'), ('EUR'), ('GBP'), ('JPY') ON CONFLICT DO NOTHING"#)
        .execute(&pool)
        .await
        .unwrap();

    // Create accounts
    let bank_usd = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Bank USD {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let bank_eur = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Bank EUR {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let bank_gbp = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Bank GBP {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let forex_gain_loss = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'CR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Forex Gain/Loss {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test complex multi-currency transaction with 3+ currencies
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Complex forex transaction".to_string()),
        meta: Some(serde_json::json!({
            "exchange_rates": {
                "USD_EUR": 0.85,
                "USD_GBP": 0.75
            }
        })),
        entries: vec![
            // USD leg
            CreateEntry {
                account_id: bank_usd.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(1000.00)),
                account_version: None,
            },
            CreateEntry {
                account_id: forex_gain_loss.id,
                currency: "USD".to_string(),
                debit: Some(dec!(1000.00)),
                credit: None,
                account_version: None,
            },
            // EUR leg
            CreateEntry {
                account_id: bank_eur.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(850.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: forex_gain_loss.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(850.00)),
                account_version: None,
            },
            // GBP leg
            CreateEntry {
                account_id: bank_gbp.id,
                currency: "GBP".to_string(),
                debit: Some(dec!(750.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: forex_gain_loss.id,
                currency: "GBP".to_string(),
                debit: None,
                credit: Some(dec!(750.00)),
                account_version: None,
            },
        ],
    };

    let (transaction, entries) = post_transaction(&pool, &input, Some("forex_trader"))
        .await
        .unwrap();

    // Verify all currencies balance independently
    assert_eq!(entries.len(), 6);

    let usd_entries: Vec<_> = entries.iter().filter(|e| e.currency == "USD").collect();
    let usd_debits: rust_decimal::Decimal = usd_entries.iter().filter_map(|e| e.debit).sum();
    let usd_credits: rust_decimal::Decimal = usd_entries.iter().filter_map(|e| e.credit).sum();
    assert_eq!(usd_debits, usd_credits);

    let eur_entries: Vec<_> = entries.iter().filter(|e| e.currency == "EUR").collect();
    let eur_debits: rust_decimal::Decimal = eur_entries.iter().filter_map(|e| e.debit).sum();
    let eur_credits: rust_decimal::Decimal = eur_entries.iter().filter_map(|e| e.credit).sum();
    assert_eq!(eur_debits, eur_credits);

    let gbp_entries: Vec<_> = entries.iter().filter(|e| e.currency == "GBP").collect();
    let gbp_debits: rust_decimal::Decimal = gbp_entries.iter().filter_map(|e| e.debit).sum();
    let gbp_credits: rust_decimal::Decimal = gbp_entries.iter().filter_map(|e| e.credit).sum();
    assert_eq!(gbp_debits, gbp_credits);

    // Verify metadata preserved
    assert!(transaction.meta.is_some());
    let meta = transaction.meta.unwrap();
    assert_eq!(meta["exchange_rates"]["USD_EUR"], 0.85);
    assert_eq!(meta["audit"]["posted_by"], "forex_trader");
}

#[sqlx::test]
async fn test_partial_currency_unbalanced_fails(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Partial Balance Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(
        r#"INSERT INTO currency (code) VALUES ('USD'), ('EUR'), ('GBP') ON CONFLICT DO NOTHING"#,
    )
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

    // Test where USD and EUR balance, but GBP doesn't
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Partially unbalanced".to_string()),
        meta: None,
        entries: vec![
            // USD balances
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
            // EUR balances
            CreateEntry {
                account_id: account1.id,
                currency: "EUR".to_string(),
                debit: Some(dec!(85.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85.00)),
                account_version: None,
            },
            // GBP does NOT balance
            CreateEntry {
                account_id: account1.id,
                currency: "GBP".to_string(),
                debit: Some(dec!(75.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "GBP".to_string(),
                debit: None,
                credit: Some(dec!(70.00)), // Wrong amount!
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &input, None).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("GBP"));
            assert!(msg.contains("does not balance"));
            // Should show the imbalance
            assert!(msg.contains("5") || msg.contains("75") || msg.contains("70"));
        }
        _ => panic!("Expected validation error for unbalanced GBP"),
    }
}

#[sqlx::test]
async fn test_decimal_precision_edge_cases(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Precision Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD'), ('BTC') ON CONFLICT DO NOTHING"#)
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

    // Test with maximum precision (28 decimal places for rust_decimal)
    let input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("High precision transaction".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: account1.id,
                currency: "BTC".to_string(),
                debit: Some(dec!(0.123456789012345678)), // High precision
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "BTC".to_string(),
                debit: None,
                credit: Some(dec!(0.123456789012345678)),
                account_version: None,
            },
            // Test with very small amounts
            CreateEntry {
                account_id: account1.id,
                currency: "USD".to_string(),
                debit: Some(dec!(0.01)), // Penny
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: account2.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(0.01)),
                account_version: None,
            },
        ],
    };

    let (_transaction, entries) = post_transaction(&pool, &input, None).await.unwrap();

    // Verify precision is preserved
    let btc_entry = entries
        .iter()
        .find(|e| e.currency == "BTC" && e.debit.is_some())
        .unwrap();
    assert_eq!(btc_entry.debit, Some(dec!(0.123456789012345678)));

    let usd_entry = entries
        .iter()
        .find(|e| e.currency == "USD" && e.debit.is_some())
        .unwrap();
    assert_eq!(usd_entry.debit, Some(dec!(0.01)));
}

#[sqlx::test]
async fn test_concurrent_version_conflict_handling(pool: PgPool) {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    use uuid::Uuid;

    let ledger_name = format!("Version Conflict Test Ledger {}", Uuid::new_v4());
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

    let shared_account = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Shared Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let counterparty = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'CR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Counterparty {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Post initial transaction to set version
    let initial = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Initial".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: shared_account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: counterparty.id,
                currency: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
                account_version: None,
            },
        ],
    };

    let (_initial_tx, initial_entries) = post_transaction(&pool, &initial, None).await.unwrap();
    let initial_version = initial_entries
        .iter()
        .find(|e| e.account_id == shared_account.id)
        .unwrap()
        .id;

    // Try to post multiple transactions concurrently with same version
    let pool = Arc::new(pool);
    let mut tasks = JoinSet::new();

    for i in 0..3 {
        let pool_clone = Arc::clone(&pool);
        let shared_id = shared_account.id;
        let counter_id = counterparty.id;
        let ledger_id = ledger.id;
        let version = initial_version;

        tasks.spawn(async move {
            let input = CreateTransaction {
                ledger_id,
                effective: None,
                memo: Some(format!("Concurrent {}", i)),
                meta: None,
                entries: vec![
                    CreateEntry {
                        account_id: shared_id,
                        currency: "USD".to_string(),
                        debit: Some(dec!(10.00)),
                        credit: None,
                        account_version: Some(version), // All using same version!
                    },
                    CreateEntry {
                        account_id: counter_id,
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

    assert_eq!(successes, 1, "Exactly one transaction should succeed");
    assert_eq!(
        failures, 2,
        "Two transactions should fail with version conflict"
    );

    // Verify the errors are optimistic lock errors
    for result in results.iter().filter(|r| r.is_err()) {
        match result.as_ref().unwrap_err() {
            ApiError::OptimisticLockError => {}
            e => panic!("Expected OptimisticLockError, got {:?}", e),
        }
    }
}

#[sqlx::test]
async fn test_large_batch_transaction_posting(pool: PgPool) {
    use blackledger::services::posting::post_transactions_batch;
    use uuid::Uuid;

    let ledger_name = format!("Large Batch Test Ledger {}", Uuid::new_v4());
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

    // Create many accounts
    let mut accounts = Vec::new();
    for i in 0..20 {
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

    // Create a large batch of transactions
    let mut transactions = Vec::new();
    for i in 0..50 {
        let from_account = &accounts[i % 10];
        let to_account = &accounts[10 + (i % 10)];

        // Always use debit/credit pairs that balance
        transactions.push(CreateTransaction {
            ledger_id: ledger.id,
            effective: None,
            memo: Some(format!("Batch transaction {}", i)),
            meta: Some(serde_json::json!({
                "batch_id": "large_batch_001",
                "sequence": i
            })),
            entries: vec![
                CreateEntry {
                    account_id: from_account.id,
                    currency: "USD".to_string(),
                    debit: Some(dec!(10.00)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: to_account.id,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(10.00)),
                    account_version: None,
                },
            ],
        });
    }

    // Post the large batch
    let start = std::time::Instant::now();
    let results = post_transactions_batch(&pool, &transactions, Some("batch_processor"))
        .await
        .unwrap();
    let duration = start.elapsed();

    // Verify all succeeded
    assert_eq!(results.len(), 50);

    // Verify performance (should complete in reasonable time)
    assert!(
        duration.as_secs() < 30,
        "Large batch should complete within 30 seconds, took {:?}",
        duration
    );

    // Verify metadata preserved
    for (i, (tx, _entries)) in results.iter().enumerate() {
        assert!(tx.meta.is_some());
        let meta = tx.meta.as_ref().unwrap();
        assert_eq!(meta["batch_id"], "large_batch_001");
        assert_eq!(meta["sequence"], i);
        assert_eq!(meta["audit"]["posted_by"], "batch_processor");
    }
}

#[sqlx::test]
async fn test_empty_and_invalid_entry_scenarios(pool: PgPool) {
    use uuid::Uuid;

    let ledger_name = format!("Invalid Entry Test Ledger {}", Uuid::new_v4());
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

    let account = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger.id)
    .bind(format!("Test Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test empty entries
    let empty_input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Empty transaction".to_string()),
        meta: None,
        entries: vec![],
    };

    let result = post_transaction(&pool, &empty_input, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("at least") || msg.contains("empty"));
        }
        _ => panic!("Expected validation error for empty entries"),
    }

    // Test single entry (can't balance)
    let single_input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Single entry".to_string()),
        meta: None,
        entries: vec![CreateEntry {
            account_id: account.id,
            currency: "USD".to_string(),
            debit: Some(dec!(100.00)),
            credit: None,
            account_version: None,
        }],
    };

    let result = post_transaction(&pool, &single_input, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("balance") || msg.contains("at least 2"));
        }
        _ => panic!("Expected validation error for single entry"),
    }

    // Test both debit and credit on same entry
    let both_input = CreateTransaction {
        ledger_id: ledger.id,
        effective: None,
        memo: Some("Both debit and credit".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: account.id,
                currency: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: Some(dec!(100.00)), // Both set!
                account_version: None,
            },
            CreateEntry {
                account_id: account.id,
                currency: "USD".to_string(),
                debit: None,
                credit: None, // Neither set!
                account_version: None,
            },
        ],
    };

    let result = post_transaction(&pool, &both_input, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Validation(msg) => {
            assert!(msg.contains("both") || msg.contains("exactly one"));
        }
        _ => panic!("Expected validation error for both debit and credit"),
    }
}

#[sqlx::test]
async fn test_account_ledger_mismatch(pool: PgPool) {
    use uuid::Uuid;

    // Create two separate ledgers
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

    // Create accounts in different ledgers
    let account_ledger1 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'DR') RETURNING *"#,
    )
    .bind(ledger1.id)
    .bind(format!("Account in Ledger1 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let account_ledger2 = sqlx::query_as::<_, Account>(
        r#"INSERT INTO account (ledger_id, name, normal) VALUES ($1, $2, 'CR') RETURNING *"#,
    )
    .bind(ledger2.id)
    .bind(format!("Account in Ledger2 {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Try to post transaction with accounts from different ledgers
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
            assert!(
                msg.contains("ledger") || msg.contains("does not exist"),
                "Error should mention ledger mismatch: {}",
                msg
            );
        }
        _ => panic!("Expected validation error for ledger mismatch"),
    }
}
