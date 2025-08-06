use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub ledger_id: Uuid,
    pub posted: DateTime<Utc>,
    pub effective: DateTime<Utc>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransaction {
    pub ledger_id: Uuid,
    pub effective: DateTime<Utc>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub entries: Vec<CreateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    pub account_id: Uuid,
    pub currency_code: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub dr: Option<rust_decimal::Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub cr: Option<rust_decimal::Decimal>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub account_version: Option<Uuid>,
}