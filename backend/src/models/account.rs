use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    pub id: i64,
    pub ledger_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub number: Option<i16>,
    pub created: DateTime<Utc>,
    pub normal: NormalBalance,
    pub version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub ledger_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub number: Option<i16>,
    pub normal: NormalBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccount {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: i64,
    pub currency_code: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
}