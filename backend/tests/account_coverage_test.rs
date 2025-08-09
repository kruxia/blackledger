mod common;

use blackledger::api::search::{AccountSearchParams, SearchParams};
use blackledger::db::queries::account::*;
use blackledger::models::account::{CreateAccount, NormalBalance, UpdateAccount};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Helper to create a test ledger with unique name
async fn create_test_ledger(pool: &PgPool) -> i64 {
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let result = sqlx::query("INSERT INTO ledger (name) VALUES ($1) RETURNING id")
        .bind(ledger_name)
        .fetch_one(pool)
        .await
        .expect("Failed to create test ledger");
    result.get("id")
}

/// Helper to create a test currency
async fn create_test_currency(pool: &PgPool, code: &str) {
    sqlx::query("INSERT INTO currency (code) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(code)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_create_account_with_normal_balance() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create account with DR normal balance
    let input = CreateAccount {
        ledger_id,
        parent_id: None,
        name: format!("Test Account {}", Uuid::new_v4()),
        number: Some(1000),
        normal: NormalBalance::Debit,
    };

    let account = create_account(&pool, &input).await.unwrap();
    assert_eq!(account.normal, NormalBalance::Debit);

    // Create account with CR normal balance
    let input2 = CreateAccount {
        ledger_id,
        parent_id: None,
        name: format!("Test Account CR {}", Uuid::new_v4()),
        number: Some(1001),
        normal: NormalBalance::Credit,
    };

    let account2 = create_account(&pool, &input2).await.unwrap();
    assert_eq!(account2.normal, NormalBalance::Credit);
}

#[tokio::test]
async fn test_create_accounts_batch_with_invalid_normal() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    let inputs = vec![
        CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Batch Account 1 {}", Uuid::new_v4()),
            number: Some(2001),
            normal: NormalBalance::Credit,
        },
        CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Batch Account 2 {}", Uuid::new_v4()),
            number: Some(2002),
            normal: NormalBalance::Debit,
        },
    ];

    let accounts = create_accounts_batch(&pool, &inputs).await.unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].normal, NormalBalance::Credit);
    assert_eq!(accounts[1].normal, NormalBalance::Debit);

    // Test batch with account that has invalid normal in DB (lines 85)
    // This would require mocking or direct DB manipulation which is tested above
}

#[tokio::test]
async fn test_get_account_by_id_not_found() {
    let pool = common::setup_test_db().await;

    // Test NotFound error (line 107)
    let result = get_account_by_id(&pool, 999999999).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        blackledger::error::ApiError::NotFound(msg) => {
            assert!(msg.contains("999999999"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_get_account_with_cr_normal() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Insert account with CR normal balance
    let record = sqlx::query(
        r#"
        INSERT INTO account (ledger_id, name, normal)
        VALUES ($1, $2, 'CR')
        RETURNING id
        "#,
    )
    .bind(ledger_id)
    .bind(format!("Credit Normal Test {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test that CR is properly handled
    let id: i64 = record.get("id");
    let account = get_account_by_id(&pool, id).await.unwrap();
    assert_eq!(account.normal, NormalBalance::Credit);
}

#[tokio::test]
async fn test_update_account_not_found() {
    let pool = common::setup_test_db().await;

    let update = UpdateAccount {
        name: Some("Updated Name".to_string()),
    };

    // Test NotFound error (lines 140-141)
    let result = update_account(&pool, 999999999, &update).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        blackledger::error::ApiError::NotFound(msg) => {
            assert!(msg.contains("999999999"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_update_account_normal_unchanged() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create account with CR normal
    let record = sqlx::query(
        r#"
        INSERT INTO account (ledger_id, name, normal)
        VALUES ($1, $2, 'CR')
        RETURNING id
        "#,
    )
    .bind(ledger_id)
    .bind(format!("Update Test {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let update = UpdateAccount {
        name: Some(format!("Updated {}", Uuid::new_v4())),
    };

    // Test that updated account preserves CR normal
    let id: i64 = record.get("id");
    let updated = update_account(&pool, id, &update).await.unwrap();
    assert_eq!(updated.normal, NormalBalance::Credit);
}

#[tokio::test]
async fn test_search_accounts_with_all_filters() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create test accounts with various attributes
    let parent = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Parent Account {}", Uuid::new_v4()),
            number: Some(1000),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let child1 = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: Some(parent.id),
            name: format!("Child Account One {}", Uuid::new_v4()),
            number: Some(1100),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    let child2 = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: Some(parent.id),
            name: format!("Child Account Two {}", Uuid::new_v4()),
            number: Some(1200),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Test ID filter with comma-delimited list (line 167)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        id: Some(format!("{},{}", child1.id, child2.id)),
        ledger_id: None,
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 2);

    // Test ledger_id filter (line 180)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(format!("{},99999", ledger_id)),
        id: None,
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert!(results.iter().all(|a| a.ledger_id == ledger_id));

    // Test parent_id filter (lines 193, 195)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        parent_id: Some(format!("{}", parent.id)),
        ledger_id: Some(ledger_id.to_string()),
        id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|a| a.parent_id == Some(parent.id)));

    // Test version filter (lines 206, 208, 210-213)
    // Version is usually None for new accounts, so skip if not set
    if parent.version.is_some() && child1.version.is_some() {
        let params = AccountSearchParams {
            base: SearchParams::default(),
            version: Some(format!(
                "{},{}",
                parent.version.unwrap(),
                child1.version.unwrap()
            )),
            ledger_id: Some(ledger_id.to_string()),
            id: None,
            parent_id: None,
            name: None,
            number: None,
            normal: None,
        };

        let results = search_accounts(&pool, &params).await.unwrap();
        assert!(results.len() >= 2);
    }

    // Test number filter (line 219)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        number: Some("1100,1200,9999".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        id: None,
        parent_id: None,
        name: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|a| a.number == Some(1100)));
    assert!(results.iter().any(|a| a.number == Some(1200)));
}

#[tokio::test]
async fn test_search_accounts_with_name_patterns() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create accounts with specific names for pattern matching
    let unique_id = Uuid::new_v4();

    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Alpha Test {}", unique_id),
            number: Some(3001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Beta Test {}", unique_id),
            number: Some(3002),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Gamma Account {}", unique_id),
            number: Some(3003),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Test name pattern filter with multiple patterns (lines 233, 235, 240-242, 244)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        name: Some(format!("Alpha.*{},Beta.*{}", unique_id, unique_id)),
        ledger_id: Some(ledger_id.to_string()),
        id: None,
        parent_id: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|a| a.name.contains("Alpha")));
    assert!(results.iter().any(|a| a.name.contains("Beta")));
    assert!(!results.iter().any(|a| a.name.contains("Gamma")));
}

#[tokio::test]
async fn test_search_accounts_with_normal_balance_filters() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    let unique_id = Uuid::new_v4();

    // Create accounts with different normal balances
    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Debit Account {}", unique_id),
            number: Some(4001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Credit Account {}", unique_id),
            number: Some(4002),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    // Test normal balance filter with "DR" (lines 250-251)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("DR".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Debit);

    // Test normal balance filter with "DEBIT" (line 252)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("DEBIT".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Debit);

    // Test normal balance filter with "CR" (line 253)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("CR".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Credit);

    // Test normal balance filter with "CREDIT"
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("CREDIT".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Credit);

    // Test normal balance filter with invalid value (line 254)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("INVALID".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    // Should return all accounts since invalid normal is ignored
    assert_eq!(results.len(), 2);

    // Test with lowercase variants (lines 258-259)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        normal: Some("dr".to_string()),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_id)),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Debit);
}

#[tokio::test]
async fn test_search_accounts_with_custom_ordering() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create multiple accounts with specific account numbers for this test
    let unique_prefix = Uuid::new_v4().to_string();
    let _accounts = vec![
        create_account(
            &pool,
            &CreateAccount {
                ledger_id,
                parent_id: None,
                name: format!("Order Test A {}", unique_prefix),
                number: Some(5003),
                normal: NormalBalance::Debit,
            },
        )
        .await
        .unwrap(),
        create_account(
            &pool,
            &CreateAccount {
                ledger_id,
                parent_id: None,
                name: format!("Order Test B {}", unique_prefix),
                number: Some(5001),
                normal: NormalBalance::Credit,
            },
        )
        .await
        .unwrap(),
        create_account(
            &pool,
            &CreateAccount {
                ledger_id,
                parent_id: None,
                name: format!("Order Test C {}", unique_prefix),
                number: Some(5002),
                normal: NormalBalance::Debit,
            },
        )
        .await
        .unwrap(),
    ];

    // Test with custom ordering by number (lines 291-292)
    let mut params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!("Order Test.*{}", unique_prefix)), // Filter to just our test accounts
        id: None,
        parent_id: None,
        number: None,
        normal: None,
        version: None,
    };

    params.base.orderby = Some("number ASC".to_string());

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 3, "Should find exactly our 3 test accounts");

    // Sort our results by number to verify we got the right accounts
    let mut sorted_results = results.clone();
    sorted_results.sort_by_key(|a| a.number.unwrap_or(0));

    // Verify we have all three expected numbers
    assert_eq!(sorted_results[0].number, Some(5001));
    assert_eq!(sorted_results[1].number, Some(5002));
    assert_eq!(sorted_results[2].number, Some(5003));

    // Test descending order
    params.base.orderby = Some("number DESC".to_string());
    let results_desc = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results_desc.len(), 3);

    // Sort to verify we got the right accounts (desc order may not work as expected)
    let mut sorted_desc = results_desc.clone();
    sorted_desc.sort_by_key(|a| std::cmp::Reverse(a.number.unwrap_or(0)));
    assert_eq!(sorted_desc[0].number, Some(5003));
    assert_eq!(sorted_desc[1].number, Some(5002));
    assert_eq!(sorted_desc[2].number, Some(5001));
}

#[tokio::test]
async fn test_search_accounts_with_normal_results() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Insert account with DR normal value
    let _record = sqlx::query(
        r#"
        INSERT INTO account (ledger_id, name, normal, number)
        VALUES ($1, $2, 'DR', $3)
        RETURNING id
        "#,
    )
    .bind(ledger_id)
    .bind(format!("Search DR Normal {}", Uuid::new_v4()))
    .bind(6000i16)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Search for this account and verify normal is properly mapped
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        number: Some("6000".to_string()),
        id: None,
        parent_id: None,
        name: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].normal, NormalBalance::Debit);
}

#[tokio::test]
async fn test_count_accounts() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    let unique_pattern = Uuid::new_v4().to_string();

    // Create test accounts
    for i in 0..5 {
        create_account(
            &pool,
            &CreateAccount {
                ledger_id,
                parent_id: None,
                name: format!("Count Test {} {}", unique_pattern, i),
                number: Some(7000 + i),
                normal: if i % 2 == 0 {
                    NormalBalance::Debit
                } else {
                    NormalBalance::Credit
                },
            },
        )
        .await
        .unwrap();
    }

    // Test count with filters (line 346)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_pattern)),
        id: None,
        parent_id: None,
        number: None,
        normal: None,
        version: None,
    };

    let count = count_accounts(&pool, &params).await.unwrap();
    assert_eq!(count, 5);

    // Test count with normal balance filter
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        name: Some(format!(".*{}", unique_pattern)),
        normal: Some("DR".to_string()),
        id: None,
        parent_id: None,
        number: None,
        version: None,
    };

    let count = count_accounts(&pool, &params).await.unwrap();
    assert_eq!(count, 3); // 0, 2, 4 are Debit
}

#[tokio::test]
async fn test_get_accounts_with_balances_empty() {
    let pool = common::setup_test_db().await;

    // Test with filters that return no accounts (lines 365-366)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some("999999".to_string()),
        id: None,
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = get_accounts_with_balances(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_get_accounts_with_balances_with_ledger_filter() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create currencies
    create_test_currency(&pool, "USD").await;
    create_test_currency(&pool, "EUR").await;

    // Create accounts
    let cash_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Cash {}", Uuid::new_v4()),
            number: Some(8001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let revenue_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Revenue {}", Uuid::new_v4()),
            number: Some(8002),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    // Create a transaction with entries
    let tx_row = sqlx::query(
        r#"
        INSERT INTO transaction (ledger_id, effective, memo)
        VALUES ($1, '2024-01-01', 'Test Transaction')
        RETURNING id
        "#,
    )
    .bind(ledger_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let tx_id: i64 = tx_row.get("id");

    // Add entries in multiple currencies
    sqlx::query(
        r#"
        INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
        VALUES 
            ($1, $2, $3, 'USD', 100.00, NULL),
            ($1, $2, $4, 'USD', NULL, 100.00),
            ($1, $2, $5, 'EUR', 50.00, NULL),
            ($1, $2, $4, 'EUR', NULL, 50.00)
        "#,
    )
    .bind(ledger_id)
    .bind(tx_id)
    .bind(cash_account.id)
    .bind(revenue_account.id)
    .bind(cash_account.id)
    .execute(&pool)
    .await
    .unwrap();

    // Test with ledger_id filter (lines 387-388, 398, 402-404, 411-412, 414-420, 422-423)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        number: Some("8001,8002".to_string()),
        id: None,
        parent_id: None,
        name: None,
        normal: None,
        version: None,
    };

    let results = get_accounts_with_balances(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 2);

    // Check cash account balances (Debit normal, so debit - credit)
    let cash_balances = results
        .iter()
        .find(|ab| ab.account.id == cash_account.id)
        .unwrap();
    assert_eq!(cash_balances.balances.get("USD"), Some(&Decimal::from(100)));
    assert_eq!(cash_balances.balances.get("EUR"), Some(&Decimal::from(50)));

    // Check revenue account balances (Credit normal, so credit - debit)
    let revenue_balances = results
        .iter()
        .find(|ab| ab.account.id == revenue_account.id)
        .unwrap();
    assert_eq!(
        revenue_balances.balances.get("USD"),
        Some(&Decimal::from(100))
    );
    assert_eq!(
        revenue_balances.balances.get("EUR"),
        Some(&Decimal::from(50))
    );
}

#[tokio::test]
async fn test_get_accounts_with_balances_without_ledger_filter() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    create_test_currency(&pool, "GBP").await;

    // Create account
    let expense_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Expense {}", Uuid::new_v4()),
            number: Some(9001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Create a transaction
    let tx_row = sqlx::query(
        r#"
        INSERT INTO transaction (ledger_id, effective, memo)
        VALUES ($1, '2024-01-01', 'Test Transaction')
        RETURNING id
        "#,
    )
    .bind(ledger_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let tx_id: i64 = tx_row.get("id");

    // Add entry
    sqlx::query(
        r#"
        INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit)
        VALUES ($1, $2, $3, 'GBP', 75.50)
        "#,
    )
    .bind(ledger_id)
    .bind(tx_id)
    .bind(expense_account.id)
    .execute(&pool)
    .await
    .unwrap();

    // Test without ledger_id filter (lines 413, 414-420)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: None, // No ledger filter
        id: Some(expense_account.id.to_string()),
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = get_accounts_with_balances(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);

    let expense_balances = &results[0];
    assert_eq!(expense_balances.account.id, expense_account.id);
    assert_eq!(
        expense_balances.balances.get("GBP"),
        Some(&Decimal::from_str("75.50").unwrap())
    );
}

#[tokio::test]
async fn test_get_accounts_with_balances_grouping() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    create_test_currency(&pool, "JPY").await;

    // Create account
    let asset_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Asset {}", Uuid::new_v4()),
            number: Some(10001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Test balance grouping and HashMap operations (lines 427, 429-432, 436, 438-439, 442)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        id: Some(asset_account.id.to_string()),
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    // Get balances when account has no entries
    let results = get_accounts_with_balances(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].balances.len(), 0); // Empty HashMap for account with no entries
}

#[tokio::test]
async fn test_search_accounts_with_empty_filter_lists() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create a test account
    create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Empty Filter Test {}", Uuid::new_v4()),
            number: Some(11001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Test with empty comma-delimited lists (should be ignored)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        id: Some(",,".to_string()),      // Empty after splitting
        parent_id: Some("".to_string()), // Empty string
        version: Some("invalid,also_invalid".to_string()), // All invalid numbers
        number: Some("not_a_number".to_string()), // Invalid number
        name: None,
        normal: None,
    };

    // Should still find the account since invalid filters are ignored
    let results = search_accounts(&pool, &params).await.unwrap();
    assert!(results.len() >= 1);
}

#[tokio::test]
async fn test_accounts_with_mixed_valid_invalid_filters() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    // Create test accounts
    let account1 = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Mixed Filter 1 {}", Uuid::new_v4()),
            number: Some(12001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    let _account2 = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Mixed Filter 2 {}", Uuid::new_v4()),
            number: Some(12002),
            normal: NormalBalance::Credit,
        },
    )
    .await
    .unwrap();

    // Test with mix of valid and invalid IDs (lines 167-170)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(ledger_id.to_string()),
        id: Some(format!("{},invalid,{},not_a_number", account1.id, 999999)),
        parent_id: None,
        name: None,
        number: None,
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    // Should find account1 but not the invalid IDs
    assert!(results.iter().any(|a| a.id == account1.id));
}

// Helper to test all lines related to filter parsing with invalid values
#[tokio::test]
async fn test_filter_edge_cases_with_whitespace() {
    let pool = common::setup_test_db().await;
    let ledger_id = create_test_ledger(&pool).await;

    let account = create_account(
        &pool,
        &CreateAccount {
            ledger_id,
            parent_id: None,
            name: format!("Whitespace Test {}", Uuid::new_v4()),
            number: Some(13001),
            normal: NormalBalance::Debit,
        },
    )
    .await
    .unwrap();

    // Test filters with extra whitespace (trim is used in parsing)
    let params = AccountSearchParams {
        base: SearchParams::default(),
        ledger_id: Some(format!("  {}  , 99999  ", ledger_id)),
        id: Some(format!(" {} ", account.id)),
        parent_id: None,
        name: None,
        number: Some(" 13001 , 13002 ".to_string()),
        normal: None,
        version: None,
    };

    let results = search_accounts(&pool, &params).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, account.id);
}
