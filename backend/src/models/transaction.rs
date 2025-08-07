use crate::models::entry::Entry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// An immutable accounting transaction containing multiple balanced entries
///
/// Transactions are the core of the double-entry accounting system. Each transaction
/// must contain at least two entries that balance (sum of debits = sum of credits)
/// for each currency involved.
///
/// # Immutability
///
/// Once posted, transactions cannot be modified or deleted. Corrections must be
/// made through new offsetting transactions (reversals).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub ledger_id: i64,
    pub created: DateTime<Utc>,
    pub effective: DateTime<Utc>,
    pub memo: Option<String>,
    pub meta: Option<serde_json::Value>,
    #[sqlx(skip)]
    #[serde(default)]
    pub entries: Vec<Entry>,
}

/// Input for creating a new transaction
///
/// # Example
///
/// ```
/// use blackledger::models::transaction::{CreateTransaction, CreateEntry};
/// use chrono::Utc;
/// use rust_decimal_macros::dec;
///
/// let transaction = CreateTransaction {
///     ledger_id: 1,
///     effective: Utc::now(),
///     memo: Some("Sale of goods".to_string()),
///     meta: None,
///     entries: vec![
///         CreateEntry {
///             account_id: 1001,  // Cash account
///             currency_code: "USD".to_string(),
///             debit: Some(dec!(100.00)),
///             credit: None,
///             account_version: None,
///         },
///         CreateEntry {
///             account_id: 4001,  // Revenue account
///             currency_code: "USD".to_string(),
///             debit: None,
///             credit: Some(dec!(100.00)),
///             account_version: None,
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransaction {
    pub ledger_id: i64,
    pub effective: DateTime<Utc>,
    pub memo: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub entries: Vec<CreateEntry>,
}

/// Input for creating an entry within a transaction
///
/// Each entry must have either a debit OR credit amount (not both).
/// The account_version field enables optimistic locking to prevent
/// concurrent modifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntry {
    #[serde(rename = "acct")]
    pub account_id: i64,
    #[serde(rename = "curr")]
    pub currency_code: String,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub debit: Option<rust_decimal::Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub credit: Option<rust_decimal::Decimal>,
    #[serde(default, rename = "version")]
    pub account_version: Option<i64>,
}
