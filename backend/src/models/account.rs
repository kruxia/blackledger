use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum NormalBalance {
    #[serde(rename = "DR")]
    #[sqlx(rename = "DR")]
    Debit,
    #[serde(rename = "CR")]
    #[sqlx(rename = "CR")]
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub ledger_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub number: String,
    pub name: String,
    pub normal_balance: NormalBalance,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub latest_entry_id: Option<Uuid>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub ledger_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub number: String,
    pub name: String,
    pub normal_balance: NormalBalance,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccount {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: Uuid,
    pub currency_code: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
}