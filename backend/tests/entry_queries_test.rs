use blackledger::{
    db::queries::entry::{get_entries_by_account, get_entries_for_transactions},
    models::{account::Account, entry::Entry, ledger::Ledger, transaction::Transaction},
};
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

async fn setup_test_data_with_entries(
    pool: &PgPool,
) -> (Ledger, Account, Account, Vec<Transaction>, Vec<Entry>) {
    // Create unique ledger
    let ledger_name = format!("Entry Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(pool)
            .await
            .unwrap();

    // Ensure USD currency exists
    sqlx::query(r#"INSERT INTO currency (code) VALUES ('USD') ON CONFLICT DO NOTHING"#)
        .execute(pool)
        .await
        .unwrap();

    // Create two accounts
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

    // Create multiple transactions with entries
    let mut transactions = Vec::new();
    let mut entries = Vec::new();

    for i in 0..3 {
        let transaction = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transaction (ledger_id, effective, memo)
            VALUES ($1, NOW(), $2)
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(format!("Test transaction {}", i))
        .fetch_one(pool)
        .await
        .unwrap();

        // Create debit entry
        let debit_entry = sqlx::query_as::<_, Entry>(
            r#"
            INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(transaction.id)
        .bind(cash_account.id)
        .bind("USD")
        .bind(Some(dec!(100.00) * rust_decimal::Decimal::from(i + 1)))
        .bind(None::<rust_decimal::Decimal>)
        .fetch_one(pool)
        .await
        .unwrap();

        // Create credit entry
        let credit_entry = sqlx::query_as::<_, Entry>(
            r#"
            INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(transaction.id)
        .bind(revenue_account.id)
        .bind("USD")
        .bind(None::<rust_decimal::Decimal>)
        .bind(Some(dec!(100.00) * rust_decimal::Decimal::from(i + 1)))
        .fetch_one(pool)
        .await
        .unwrap();

        transactions.push(transaction);
        entries.push(debit_entry);
        entries.push(credit_entry);
    }

    (ledger, cash_account, revenue_account, transactions, entries)
}

#[sqlx::test]
async fn test_get_entries_by_account_basic(pool: PgPool) {
    let (_, cash_account, revenue_account, _, _) = setup_test_data_with_entries(&pool).await;

    // Get entries for cash account
    let cash_entries = get_entries_by_account(&pool, cash_account.id, None, None)
        .await
        .unwrap();

    assert_eq!(
        cash_entries.len(),
        3,
        "Should have 3 entries for cash account"
    );

    // Verify all entries belong to cash account
    for entry in &cash_entries {
        assert_eq!(entry.account_id, cash_account.id);
        assert!(entry.debit.is_some(), "Cash entries should have debit");
        assert!(
            entry.credit.is_none(),
            "Cash entries should not have credit"
        );
    }

    // Get entries for revenue account
    let revenue_entries = get_entries_by_account(&pool, revenue_account.id, None, None)
        .await
        .unwrap();

    assert_eq!(
        revenue_entries.len(),
        3,
        "Should have 3 entries for revenue account"
    );

    // Verify all entries belong to revenue account
    for entry in &revenue_entries {
        assert_eq!(entry.account_id, revenue_account.id);
        assert!(entry.credit.is_some(), "Revenue entries should have credit");
        assert!(
            entry.debit.is_none(),
            "Revenue entries should not have debit"
        );
    }
}

#[sqlx::test]
async fn test_get_entries_by_account_with_pagination(pool: PgPool) {
    let (_, cash_account, _, _, _) = setup_test_data_with_entries(&pool).await;

    // Test with limit
    let limited_entries = get_entries_by_account(&pool, cash_account.id, Some(2), None)
        .await
        .unwrap();

    assert_eq!(
        limited_entries.len(),
        2,
        "Should return only 2 entries with limit=2"
    );

    // Test with offset
    let offset_entries = get_entries_by_account(&pool, cash_account.id, None, Some(1))
        .await
        .unwrap();

    assert_eq!(
        offset_entries.len(),
        2,
        "Should return 2 entries with offset=1 (skipping 1)"
    );

    // Test with both limit and offset
    let paginated_entries = get_entries_by_account(&pool, cash_account.id, Some(1), Some(1))
        .await
        .unwrap();

    assert_eq!(
        paginated_entries.len(),
        1,
        "Should return 1 entry with limit=1 and offset=1"
    );
}

#[sqlx::test]
async fn test_get_entries_by_account_empty_result(pool: PgPool) {
    let ledger_name = format!("Empty Test Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Create account with no entries
    let empty_account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Empty Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let entries = get_entries_by_account(&pool, empty_account.id, None, None)
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        0,
        "Should return empty vector for account with no entries"
    );
}

#[sqlx::test]
async fn test_get_entries_by_account_nonexistent(pool: PgPool) {
    let nonexistent_account_id = 999999999;

    let entries = get_entries_by_account(&pool, nonexistent_account_id, None, None)
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        0,
        "Should return empty vector for non-existent account"
    );
}

#[sqlx::test]
async fn test_get_entries_by_account_ordering(pool: PgPool) {
    let (_, cash_account, _, _, _) = setup_test_data_with_entries(&pool).await;

    let entries = get_entries_by_account(&pool, cash_account.id, None, None)
        .await
        .unwrap();

    // Verify entries are ordered by ID descending
    for i in 0..entries.len() - 1 {
        assert!(
            entries[i].id > entries[i + 1].id,
            "Entries should be ordered by ID descending"
        );
    }
}

#[sqlx::test]
async fn test_get_entries_for_transactions_basic(pool: PgPool) {
    let (_, _, _, transactions, _all_entries) = setup_test_data_with_entries(&pool).await;

    let transaction_ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();

    let entries_map = get_entries_for_transactions(&pool, &transaction_ids)
        .await
        .unwrap();

    // Should have entries for all transactions
    assert_eq!(
        entries_map.len(),
        3,
        "Should have entries for 3 transactions"
    );

    // Each transaction should have 2 entries
    for (tx_id, entries) in &entries_map {
        assert_eq!(entries.len(), 2, "Each transaction should have 2 entries");

        // Verify all entries belong to the correct transaction
        for entry in entries {
            assert_eq!(entry.transaction_id, *tx_id);
        }
    }

    // Verify total number of entries
    let total_entries: usize = entries_map.values().map(|v| v.len()).sum();
    assert_eq!(total_entries, 6, "Should have 6 total entries");
}

#[sqlx::test]
async fn test_get_entries_for_transactions_empty_input(pool: PgPool) {
    let empty_ids: Vec<i64> = vec![];

    let entries_map = get_entries_for_transactions(&pool, &empty_ids)
        .await
        .unwrap();

    assert_eq!(
        entries_map.len(),
        0,
        "Should return empty HashMap for empty input"
    );
}

#[sqlx::test]
async fn test_get_entries_for_transactions_nonexistent(pool: PgPool) {
    let nonexistent_ids = vec![999999999, 999999998];

    let entries_map = get_entries_for_transactions(&pool, &nonexistent_ids)
        .await
        .unwrap();

    assert_eq!(
        entries_map.len(),
        0,
        "Should return empty HashMap for non-existent transactions"
    );
}

#[sqlx::test]
async fn test_get_entries_for_transactions_mixed(pool: PgPool) {
    let (_, _, _, transactions, _) = setup_test_data_with_entries(&pool).await;

    // Mix of existing and non-existent transaction IDs
    let mixed_ids = vec![
        transactions[0].id,
        999999999, // non-existent
        transactions[1].id,
    ];

    let entries_map = get_entries_for_transactions(&pool, &mixed_ids)
        .await
        .unwrap();

    // Should only have entries for existing transactions
    assert_eq!(
        entries_map.len(),
        2,
        "Should have entries for 2 existing transactions"
    );
    assert!(entries_map.contains_key(&transactions[0].id));
    assert!(entries_map.contains_key(&transactions[1].id));
    assert!(!entries_map.contains_key(&999999999));
}

#[sqlx::test]
async fn test_get_entries_for_transactions_ordering(pool: PgPool) {
    let (_, _, _, transactions, _) = setup_test_data_with_entries(&pool).await;

    let transaction_ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();

    let entries_map = get_entries_for_transactions(&pool, &transaction_ids)
        .await
        .unwrap();

    // Verify entries within each transaction are ordered by ID
    for (_, entries) in &entries_map {
        for i in 0..entries.len() - 1 {
            assert!(
                entries[i].id < entries[i + 1].id,
                "Entries within transaction should be ordered by ID ascending"
            );
        }
    }
}

#[sqlx::test]
async fn test_get_entries_for_single_transaction(pool: PgPool) {
    let (_, _, _, transactions, _) = setup_test_data_with_entries(&pool).await;

    // Get entries for just one transaction
    let single_id = vec![transactions[0].id];

    let entries_map = get_entries_for_transactions(&pool, &single_id)
        .await
        .unwrap();

    assert_eq!(
        entries_map.len(),
        1,
        "Should have entries for 1 transaction"
    );
    assert!(entries_map.contains_key(&transactions[0].id));
    assert_eq!(
        entries_map[&transactions[0].id].len(),
        2,
        "Transaction should have 2 entries"
    );
}

#[sqlx::test]
async fn test_get_entries_with_multiple_currencies(pool: PgPool) {
    let ledger_name = format!("Multi-Currency Ledger {}", Uuid::new_v4());
    let ledger =
        sqlx::query_as::<_, Ledger>(r#"INSERT INTO ledger (name) VALUES ($1) RETURNING *"#)
            .bind(&ledger_name)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Ensure multiple currencies exist
    sqlx::query(
        r#"INSERT INTO currency (code) VALUES ('USD'), ('EUR'), ('GBP') ON CONFLICT DO NOTHING"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let account = sqlx::query_as::<_, Account>(
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Multi-Currency Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transaction (ledger_id, effective, memo)
        VALUES ($1, NOW(), $2)
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind("Multi-currency transaction")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create entries in different currencies
    let currencies = vec!["USD", "EUR", "GBP"];
    for (i, currency) in currencies.iter().enumerate() {
        sqlx::query_as::<_, Entry>(
            r#"
            INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(transaction.id)
        .bind(account.id)
        .bind(currency)
        .bind(Some(
            dec!(100.00) * rust_decimal::Decimal::from(i as i64 + 1),
        ))
        .bind(None::<rust_decimal::Decimal>)
        .fetch_one(&pool)
        .await
        .unwrap();
    }

    // Get all entries for the account
    let entries = get_entries_by_account(&pool, account.id, None, None)
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        3,
        "Should have 3 entries in different currencies"
    );

    // Verify we have all currencies
    let entry_currencies: Vec<String> = entries.iter().map(|e| e.currency.clone()).collect();
    assert!(entry_currencies.contains(&"USD".to_string()));
    assert!(entry_currencies.contains(&"EUR".to_string()));
    assert!(entry_currencies.contains(&"GBP".to_string()));
}

#[sqlx::test]
async fn test_large_pagination_parameters(pool: PgPool) {
    let (_, cash_account, _, _, _) = setup_test_data_with_entries(&pool).await;

    // Test with very large limit (should return all available)
    let entries = get_entries_by_account(&pool, cash_account.id, Some(1000000), None)
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        3,
        "Should return all 3 entries even with large limit"
    );

    // Test with very large offset (should return empty)
    let entries = get_entries_by_account(&pool, cash_account.id, None, Some(1000000))
        .await
        .unwrap();

    assert_eq!(
        entries.len(),
        0,
        "Should return no entries with very large offset"
    );
}

#[sqlx::test]
async fn test_get_entries_for_large_transaction_set(pool: PgPool) {
    let ledger_name = format!("Large Set Ledger {}", Uuid::new_v4());
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
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Test Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create many transactions
    let mut transaction_ids = Vec::new();
    for i in 0..20 {
        let transaction = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transaction (ledger_id, effective, memo)
            VALUES ($1, NOW(), $2)
            RETURNING *
            "#,
        )
        .bind(ledger.id)
        .bind(format!("Transaction {}", i))
        .fetch_one(&pool)
        .await
        .unwrap();

        // Create one entry per transaction for simplicity
        sqlx::query(
            r#"
            INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(ledger.id)
        .bind(transaction.id)
        .bind(account.id)
        .bind("USD")
        .bind(Some(dec!(10.00)))
        .bind(None::<rust_decimal::Decimal>)
        .execute(&pool)
        .await
        .unwrap();

        transaction_ids.push(transaction.id);
    }

    // Get entries for all transactions
    let entries_map = get_entries_for_transactions(&pool, &transaction_ids)
        .await
        .unwrap();

    assert_eq!(
        entries_map.len(),
        20,
        "Should have entries for all 20 transactions"
    );

    // Verify each transaction has its entry
    for tx_id in &transaction_ids {
        assert!(
            entries_map.contains_key(tx_id),
            "Should have entry for transaction {}",
            tx_id
        );
        assert_eq!(
            entries_map[tx_id].len(),
            1,
            "Each transaction should have 1 entry"
        );
    }
}

#[sqlx::test]
async fn test_entries_with_null_amounts(pool: PgPool) {
    let ledger_name = format!("Null Amount Ledger {}", Uuid::new_v4());
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
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Test Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transaction (ledger_id, effective, memo)
        VALUES ($1, NOW(), $2)
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind("Test transaction")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Create entry with debit only (credit is null)
    let debit_entry = sqlx::query_as::<_, Entry>(
        r#"
        INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(transaction.id)
    .bind(account.id)
    .bind("USD")
    .bind(Some(dec!(100.00)))
    .bind(None::<rust_decimal::Decimal>)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify the entry has correct null/non-null values
    assert!(debit_entry.debit.is_some(), "Debit should be set");
    assert!(debit_entry.credit.is_none(), "Credit should be null");
    assert_eq!(debit_entry.debit.unwrap(), dec!(100.00));

    // Retrieve and verify
    let entries = get_entries_by_account(&pool, account.id, None, None)
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].debit, Some(dec!(100.00)));
    assert_eq!(entries[0].credit, None);
}

#[sqlx::test]
async fn test_get_entries_with_decimal_precision(pool: PgPool) {
    let ledger_name = format!("Precision Test Ledger {}", Uuid::new_v4());
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
        r#"
        INSERT INTO account (ledger_id, name, normal) 
        VALUES ($1, $2, 'DR') 
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind(format!("Precision Account {}", Uuid::new_v4()))
    .fetch_one(&pool)
    .await
    .unwrap();

    let transaction = sqlx::query_as::<_, Transaction>(
        r#"
        INSERT INTO transaction (ledger_id, effective, memo)
        VALUES ($1, NOW(), $2)
        RETURNING *
        "#,
    )
    .bind(ledger.id)
    .bind("Precision test")
    .fetch_one(&pool)
    .await
    .unwrap();

    // Test with high precision decimals
    let precise_amount = dec!(123.456789);

    sqlx::query(
        r#"
        INSERT INTO entry (ledger_id, transaction_id, account_id, currency, debit, credit)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(ledger.id)
    .bind(transaction.id)
    .bind(account.id)
    .bind("USD")
    .bind(Some(precise_amount))
    .bind(None::<rust_decimal::Decimal>)
    .execute(&pool)
    .await
    .unwrap();

    // Retrieve and verify precision is maintained
    let entries = get_entries_by_account(&pool, account.id, None, None)
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].debit,
        Some(precise_amount),
        "Decimal precision should be preserved"
    );
}
