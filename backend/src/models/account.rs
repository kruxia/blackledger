use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// The normal balance side of an account (Debit or Credit)
///
/// This determines whether increases to the account are recorded
/// as debits or credits:
/// - Assets and Expenses normally have Debit balances
/// - Liabilities, Equity, and Revenue normally have Credit balances
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

/// An account in the chart of accounts
///
/// Accounts are organized hierarchically and track balances for specific
/// categories of assets, liabilities, equity, revenue, or expenses.
///
/// # Versioning
///
/// The `version` field contains the ID of the last entry posted to this account,
/// enabling optimistic locking for concurrent transaction posting.
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

/// Balance information for an account in a specific currency
///
/// Since accounts can have entries in multiple currencies,
/// balances are calculated per currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: i64,
    pub currency_code: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
}