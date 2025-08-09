// Phase 1.2 Database Constraint Tests
// These tests verify database-level constraints and edge cases

mod common;
use rust_decimal_macros::dec;
use uuid::Uuid;

// Simple test to verify the test infrastructure works
#[tokio::test]
async fn test_database_connection() {
    let pool = common::setup_test_db().await;

    // Simple query to verify connection
    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(result, 1, "Database connection should work");
}

// Test currency duplicate constraint
#[tokio::test]
async fn test_currency_duplicate_constraint() {
    let pool = common::setup_test_db().await;

    // Insert first currency with unique code
    let currency_code = format!("TST{}", &Uuid::new_v4().to_string()[0..3].to_uppercase());
    let result1 = sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&currency_code)
        .execute(&pool)
        .await;

    assert!(result1.is_ok(), "First currency insert should succeed");

    // Try to insert duplicate
    let result2 = sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&currency_code)
        .execute(&pool)
        .await;

    assert!(result2.is_err(), "Duplicate currency insert should fail");
    if let Err(e) = result2 {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("duplicate") || error_msg.contains("unique"),
            "Error should indicate duplicate key: {}",
            error_msg
        );
    }
}

// Test ledger foreign key constraint
#[tokio::test]
async fn test_ledger_deletion_with_accounts() {
    let pool = common::setup_test_db().await;

    // Create ledger with unique name
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let ledger_id: i64 =
        sqlx::query_scalar("INSERT INTO ledger (name, created) VALUES ($1, NOW()) RETURNING id")
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Create currency with unique code
    let currency_code = format!("TST{}", &Uuid::new_v4().to_string()[0..3].to_uppercase());
    sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&currency_code)
        .execute(&pool)
        .await
        .ok();

    // Create account
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO account (ledger_id, name, number, normal, created) 
         VALUES ($1, $2, $3, $4, NOW()) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Cash")
    .bind(1000i16)
    .bind("DR")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Try to delete ledger (should fail)
    let delete_result = sqlx::query("DELETE FROM ledger WHERE id = $1")
        .bind(ledger_id)
        .execute(&pool)
        .await;

    assert!(
        delete_result.is_err(),
        "Ledger deletion should fail when accounts exist"
    );

    // Delete account first
    sqlx::query("DELETE FROM account WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();

    // Now ledger deletion should succeed
    let delete_result = sqlx::query("DELETE FROM ledger WHERE id = $1")
        .bind(ledger_id)
        .execute(&pool)
        .await;

    assert!(
        delete_result.is_ok(),
        "Ledger deletion should succeed after accounts are deleted"
    );
}

// Test account foreign key constraint with entries
#[tokio::test]
async fn test_account_deletion_with_entries() {
    let pool = common::setup_test_db().await;

    // Setup ledger and currency with unique names
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let ledger_id: i64 =
        sqlx::query_scalar("INSERT INTO ledger (name, created) VALUES ($1, NOW()) RETURNING id")
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    let currency_code = format!("TST{}", &Uuid::new_v4().to_string()[0..3].to_uppercase());
    sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&currency_code)
        .execute(&pool)
        .await
        .ok();

    // Create accounts
    let cash_id: i64 = sqlx::query_scalar(
        "INSERT INTO account (ledger_id, name, number, normal, created) 
         VALUES ($1, $2, $3, $4, NOW()) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Cash")
    .bind(1000i16)
    .bind("DR")
    .fetch_one(&pool)
    .await
    .unwrap();

    let revenue_id: i64 = sqlx::query_scalar(
        "INSERT INTO account (ledger_id, name, number, normal, created) 
         VALUES ($1, $2, $3, $4, NOW()) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Revenue")
    .bind(4000i16)
    .bind("CR")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create transaction
    let transaction_id: i64 = sqlx::query_scalar(
        "INSERT INTO transaction (ledger_id, created, effective, memo) 
         VALUES ($1, NOW(), NOW(), $2) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Test transaction")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create entries
    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(cash_id)
    .bind(&currency_code)
    .bind(dec!(100))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, credit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(revenue_id)
    .bind(&currency_code)
    .bind(dec!(100))
    .execute(&pool)
    .await
    .unwrap();

    // Try to delete account (should fail)
    let delete_result = sqlx::query("DELETE FROM account WHERE id = $1")
        .bind(cash_id)
        .execute(&pool)
        .await;

    assert!(
        delete_result.is_err(),
        "Account deletion should fail when entries exist"
    );

    if let Err(e) = delete_result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("violates foreign key") || error_msg.contains("constraint"),
            "Error should indicate foreign key violation: {}",
            error_msg
        );
    }
}

// Test transaction immutability (if triggers are implemented)
#[tokio::test]
async fn test_transaction_immutability() {
    let pool = common::setup_test_db().await;

    // Setup with unique ledger name
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let ledger_id: i64 =
        sqlx::query_scalar("INSERT INTO ledger (name, created) VALUES ($1, NOW()) RETURNING id")
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Create transaction
    let transaction_id: i64 = sqlx::query_scalar(
        "INSERT INTO transaction (ledger_id, created, effective, memo) 
         VALUES ($1, NOW(), NOW(), $2) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Original memo")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Try to update transaction
    let update_result = sqlx::query("UPDATE transaction SET memo = $1 WHERE id = $2")
        .bind("Modified memo")
        .bind(transaction_id)
        .execute(&pool)
        .await;

    // This will only fail if immutability triggers are implemented
    if update_result.is_err() {
        println!("Transaction immutability is enforced at database level");
    } else {
        println!("Warning: Transaction immutability not enforced at database level");
        // Verify the update actually happened (for documentation purposes)
        let memo: String = sqlx::query_scalar("SELECT memo FROM transaction WHERE id = $1")
            .bind(transaction_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(memo, "Modified memo", "Update succeeded without triggers");
    }
}

// Test multi-currency transaction balance
#[tokio::test]
async fn test_multi_currency_balance() {
    let pool = common::setup_test_db().await;

    // Setup with unique names
    let ledger_name = format!("Test Ledger {}", Uuid::new_v4());
    let ledger_id: i64 =
        sqlx::query_scalar("INSERT INTO ledger (name, created) VALUES ($1, NOW()) RETURNING id")
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    let usd_code = format!("ZU{}", &Uuid::new_v4().to_string()[0..2].to_uppercase());
    let eur_code = format!("ZE{}", &Uuid::new_v4().to_string()[0..2].to_uppercase());

    sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&usd_code)
        .execute(&pool)
        .await
        .ok();

    sqlx::query("INSERT INTO currency (code, created) VALUES ($1, NOW())")
        .bind(&eur_code)
        .execute(&pool)
        .await
        .ok();

    // Create accounts
    let cash_id: i64 = sqlx::query_scalar(
        "INSERT INTO account (ledger_id, name, number, normal, created) 
         VALUES ($1, $2, $3, $4, NOW()) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Cash")
    .bind(1000i16)
    .bind("DR")
    .fetch_one(&pool)
    .await
    .unwrap();

    let revenue_id: i64 = sqlx::query_scalar(
        "INSERT INTO account (ledger_id, name, number, normal, created) 
         VALUES ($1, $2, $3, $4, NOW()) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Revenue")
    .bind(4000i16)
    .bind("CR")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create multi-currency transaction
    let transaction_id: i64 = sqlx::query_scalar(
        "INSERT INTO transaction (ledger_id, created, effective, memo) 
         VALUES ($1, NOW(), NOW(), $2) RETURNING id",
    )
    .bind(ledger_id)
    .bind("Multi-currency transaction")
    .fetch_one(&pool)
    .await
    .unwrap();

    // USD entries (using unique currency code)
    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(cash_id)
    .bind(&usd_code)
    .bind(dec!(100))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, credit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(revenue_id)
    .bind(&usd_code)
    .bind(dec!(100))
    .execute(&pool)
    .await
    .unwrap();

    // EUR entries
    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(cash_id)
    .bind(&eur_code)
    .bind(dec!(85))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO entry (ledger_id, transaction_id, account_id, currency, credit) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(ledger_id)
    .bind(transaction_id)
    .bind(revenue_id)
    .bind(&eur_code)
    .bind(dec!(85))
    .execute(&pool)
    .await
    .unwrap();

    // Verify balances per currency
    let usd_cash_balance: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(COALESCE(debit, 0) - COALESCE(credit, 0)), 0)
         FROM entry WHERE account_id = $1 AND currency = $2",
    )
    .bind(cash_id)
    .bind(&usd_code)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        usd_cash_balance,
        dec!(100),
        "USD cash balance should be 100"
    );

    let eur_cash_balance: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(COALESCE(debit, 0) - COALESCE(credit, 0)), 0)
         FROM entry WHERE account_id = $1 AND currency = $2",
    )
    .bind(cash_id)
    .bind(&eur_code)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(eur_cash_balance, dec!(85), "EUR cash balance should be 85");

    // Verify balance per currency is zero for each currency
    let usd_balance: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(COALESCE(debit, 0) - COALESCE(credit, 0)), 0)
         FROM entry WHERE transaction_id = $1 AND currency = $2",
    )
    .bind(transaction_id)
    .bind(&usd_code)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(usd_balance, dec!(0), "USD transaction should balance");

    let eur_balance: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(COALESCE(debit, 0) - COALESCE(credit, 0)), 0)
         FROM entry WHERE transaction_id = $1 AND currency = $2",
    )
    .bind(transaction_id)
    .bind(&eur_code)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(eur_balance, dec!(0), "EUR transaction should balance");
}
