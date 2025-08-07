//! Basic accounting example demonstrating core Blackledger functionality
//!
//! This example shows how to:
//! - Create a ledger and chart of accounts
//! - Post double-entry transactions
//! - Query account balances
//! - Handle multi-currency transactions
//!
//! Run with: `cargo run --example basic_accounting`

use anyhow::Result;
use blackledger::{
    db,
    db::queries::{
        account::{create_account, get_account_balances},
        currency::create_currency,
        ledger::create_ledger,
    },
    models::{
        account::{CreateAccount, NormalBalance},
        ledger::CreateLedger,
        transaction::{CreateEntry, CreateTransaction},
    },
    services::posting::post_transaction,
};
use chrono::Utc;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and logging
    tracing_subscriber::fmt::init();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::create_pool(&database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("🏦 Blackledger Basic Accounting Example\n");

    // Step 1: Create currencies
    println!("1️⃣ Creating currencies...");
    create_currency(&pool, "USD").await?;
    create_currency(&pool, "EUR").await?;
    println!("   ✅ Created USD and EUR currencies\n");

    // Step 2: Create a ledger
    println!("2️⃣ Creating ledger...");
    let ledger_input = CreateLedger {
        name: "Example Company Books".to_string(),
    };
    let ledger = create_ledger(&pool, &ledger_input).await?;
    println!(
        "   ✅ Created ledger: {} (ID: {})\n",
        ledger.name, ledger.id
    );

    // Step 3: Create chart of accounts
    println!("3️⃣ Creating chart of accounts...");

    // Asset accounts
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
    .await?;
    println!("   📊 Created Cash account ({})", cash_account.id);

    let ar_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Accounts Receivable".to_string(),
            number: Some(1200),
            normal: NormalBalance::Debit,
        },
    )
    .await?;
    println!("   📊 Created A/R account ({})", ar_account.id);

    // Liability accounts
    let ap_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Accounts Payable".to_string(),
            number: Some(2000),
            normal: NormalBalance::Credit,
        },
    )
    .await?;
    println!("   📊 Created A/P account ({})", ap_account.id);

    // Revenue account
    let revenue_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Sales Revenue".to_string(),
            number: Some(4000),
            normal: NormalBalance::Credit,
        },
    )
    .await?;
    println!("   📊 Created Revenue account ({})", revenue_account.id);

    // Expense account
    let expense_account = create_account(
        &pool,
        &CreateAccount {
            ledger_id: ledger.id,
            parent_id: None,
            name: "Operating Expenses".to_string(),
            number: Some(5000),
            normal: NormalBalance::Debit,
        },
    )
    .await?;
    println!("   📊 Created Expense account ({})\n", expense_account.id);

    // Step 4: Post transactions
    println!("4️⃣ Posting transactions...");

    // Transaction 1: Cash sale
    let sale_transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Cash sale of merchandise".to_string()),
        meta: Some(serde_json::json!({
            "invoice_number": "INV-001",
            "customer": "ABC Corp"
        })),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(500.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(500.00)),
                account_version: None,
            },
        ],
    };

    let (transaction1, _) =
        post_transaction(&pool, &sale_transaction, Some("example_user")).await?;
    println!(
        "   💰 Posted cash sale: ${} (Transaction {})",
        500.00, transaction1.id
    );

    // Transaction 2: Credit sale
    let credit_sale = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Credit sale to customer".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: ar_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(1000.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(1000.00)),
                account_version: None,
            },
        ],
    };

    let (transaction2, _) = post_transaction(&pool, &credit_sale, Some("example_user")).await?;
    println!(
        "   💳 Posted credit sale: ${} (Transaction {})",
        1000.00, transaction2.id
    );

    // Transaction 3: Pay expense
    let expense_payment = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("Paid office rent".to_string()),
        meta: None,
        entries: vec![
            CreateEntry {
                account_id: expense_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(200.00)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(200.00)),
                account_version: None,
            },
        ],
    };

    let (transaction3, _) = post_transaction(&pool, &expense_payment, Some("example_user")).await?;
    println!(
        "   🏢 Posted expense: ${} (Transaction {})",
        200.00, transaction3.id
    );

    // Transaction 4: Multi-currency transaction
    let forex_transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("EUR sale with USD cash".to_string()),
        meta: None,
        entries: vec![
            // Receive USD
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(1100.00)),
                credit: None,
                account_version: None,
            },
            // Record EUR revenue
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(1000.00)),
                account_version: None,
            },
            // Balance the USD side
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(1100.00)),
                account_version: None,
            },
            // Balance the EUR side (contra revenue for tracking)
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "EUR".to_string(),
                debit: Some(dec!(1000.00)),
                credit: None,
                account_version: None,
            },
        ],
    };

    let (transaction4, _) =
        post_transaction(&pool, &forex_transaction, Some("example_user")).await?;
    println!(
        "   💱 Posted multi-currency transaction (Transaction {})\n",
        transaction4.id
    );

    // Step 5: Query balances
    println!("5️⃣ Account Balances:");
    println!("   {}", "─".repeat(50));

    let balances = get_account_balances(&pool, ledger.id, None).await?;

    for balance in balances {
        let account_name = match balance.account_id {
            id if id == cash_account.id => "Cash",
            id if id == ar_account.id => "Accounts Receivable",
            id if id == ap_account.id => "Accounts Payable",
            id if id == revenue_account.id => "Sales Revenue",
            id if id == expense_account.id => "Operating Expenses",
            _ => "Unknown",
        };

        println!(
            "   {:<25} {:>10} {}",
            account_name,
            format!("{:.2}", balance.balance),
            balance.currency_code
        );
    }

    println!("\n✨ Example completed successfully!");

    Ok(())
}
