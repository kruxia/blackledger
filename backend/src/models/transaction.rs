use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub ledger_id: i64,
    pub posted: DateTime<Utc>,
    pub effective: DateTime<Utc>,
    pub memo: Option<String>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransaction {
    pub ledger_id: i64,
    pub effective: DateTime<Utc>,
    pub memo: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub entries: Vec<CreateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    pub account_id: i64,
    pub currency_code: String,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub debit: Option<rust_decimal::Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub credit: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub account_version: Option<i64>,
}