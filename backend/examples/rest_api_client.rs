//! REST API client example demonstrating HTTP interactions with Blackledger
//!
//! This example shows how to interact with the Blackledger REST API using
//! an HTTP client. It demonstrates:
//! - Authentication with JWT tokens
//! - Creating ledgers and accounts
//! - Posting transactions
//! - Querying with pagination and search
//!
//! Run the server first: `cargo run`
//! Then run this example: `cargo run --example rest_api_client`

use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct Ledger {
    id: i64,
    name: String,
    created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Account {
    id: i64,
    ledger_id: i64,
    name: String,
    number: Option<i16>,
    normal: String,
}

#[derive(Debug, Serialize)]
struct CreateTransaction {
    ledger_id: i64,
    effective: DateTime<Utc>,
    memo: Option<String>,
    entries: Vec<CreateEntry>,
}

#[derive(Debug, Serialize)]
struct CreateEntry {
    account_id: i64,
    currency_code: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    debit: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    credit: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

#[derive(Debug, Deserialize)]
struct PaginationMeta {
    page: u32,
    page_size: u32,
    total: Option<i64>,
    has_more: bool,
}

struct ApiClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
}

impl ApiClient {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            auth_token: None,
        }
    }

    fn with_auth(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    async fn create_ledger(&self, name: &str) -> Result<Ledger> {
        let mut req = self
            .client
            .post(format!("{}/api/ledgers", self.base_url))
            .json(&json!({ "name": name }));

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        if response.status() != StatusCode::CREATED {
            anyhow::bail!("Failed to create ledger: {}", response.status());
        }

        Ok(response.json().await?)
    }

    async fn create_account(&self, account: &serde_json::Value) -> Result<Account> {
        let mut req = self
            .client
            .post(format!("{}/api/accounts", self.base_url))
            .json(account);

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        if response.status() != StatusCode::CREATED {
            anyhow::bail!("Failed to create account: {}", response.status());
        }

        Ok(response.json().await?)
    }

    async fn post_transaction(&self, transaction: &CreateTransaction) -> Result<serde_json::Value> {
        let mut req = self
            .client
            .post(format!("{}/api/transactions", self.base_url))
            .json(transaction);

        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        if response.status() != StatusCode::CREATED {
            let error = response.text().await?;
            anyhow::bail!("Failed to post transaction: {}", error);
        }

        Ok(response.json().await?)
    }

    async fn list_accounts(&self, ledger_id: i64, page: u32) -> Result<PaginatedResponse<Account>> {
        let response = self
            .client
            .get(format!("{}/api/accounts", self.base_url))
            .query(&[
                ("ledger_id", ledger_id.to_string()),
                ("page", page.to_string()),
                ("page_size", "10".to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to list accounts: {}", response.status());
        }

        Ok(response.json().await?)
    }

    async fn get_balances(&self, ledger_id: i64) -> Result<Vec<serde_json::Value>> {
        let response = self
            .client
            .get(format!("{}/api/accounts/balances", self.base_url))
            .query(&[("ledger_id", ledger_id.to_string())])
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to get balances: {}", response.status());
        }

        Ok(response.json().await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌐 Blackledger REST API Client Example\n");

    // Configure API client
    let base_url =
        std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // In production, obtain this from your auth provider
    let auth_token = std::env::var("AUTH_TOKEN").ok();

    let mut client = ApiClient::new(base_url.clone());
    if let Some(token) = auth_token {
        println!("🔐 Using authentication token");
        client = client.with_auth(token);
    } else {
        println!("⚠️  No auth token provided - some operations may fail");
    }

    println!("📡 Connecting to: {}\n", base_url);

    // Step 1: Create a ledger
    println!("1️⃣ Creating ledger...");
    let ledger = client.create_ledger("API Example Ledger").await?;
    println!(
        "   ✅ Created ledger: {} (ID: {})\n",
        ledger.name, ledger.id
    );

    // Step 2: Create accounts
    println!("2️⃣ Creating accounts...");

    let cash_account = client
        .create_account(&json!({
            "ledger_id": ledger.id,
            "name": "Cash",
            "number": 1000,
            "normal": "DR"
        }))
        .await?;
    println!("   📊 Created Cash account (ID: {})", cash_account.id);

    let revenue_account = client
        .create_account(&json!({
            "ledger_id": ledger.id,
            "name": "Revenue",
            "number": 4000,
            "normal": "CR"
        }))
        .await?;
    println!(
        "   📊 Created Revenue account (ID: {})\n",
        revenue_account.id
    );

    // Step 3: Post a transaction
    println!("3️⃣ Posting transaction...");

    use rust_decimal_macros::dec;
    let transaction = CreateTransaction {
        ledger_id: ledger.id,
        effective: Utc::now(),
        memo: Some("API test transaction".to_string()),
        entries: vec![
            CreateEntry {
                account_id: cash_account.id,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100.00)),
                credit: None,
            },
            CreateEntry {
                account_id: revenue_account.id,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100.00)),
            },
        ],
    };

    let posted = client.post_transaction(&transaction).await?;
    println!(
        "   ✅ Posted transaction: {}\n",
        posted["transaction"]["id"].as_i64().unwrap()
    );

    // Step 4: Query with pagination
    println!("4️⃣ Querying accounts with pagination...");
    let page1 = client.list_accounts(ledger.id, 1).await?;
    println!("   📄 Page 1: {} accounts", page1.data.len());
    println!("   📊 Total accounts: {:?}", page1.pagination.total);
    println!("   ➡️  Has more: {}\n", page1.pagination.has_more);

    // Step 5: Get balances
    println!("5️⃣ Getting account balances...");
    let balances = client.get_balances(ledger.id).await?;

    for balance in balances {
        println!(
            "   Account {}: {} {}",
            balance["account_id"], balance["balance"], balance["currency_code"]
        );
    }

    println!("\n✨ API example completed successfully!");
    println!("   View the API docs at: {}/docs", base_url);

    Ok(())
}
