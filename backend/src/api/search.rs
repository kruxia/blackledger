use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountSearchParams {
    pub ledger_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub number: Option<String>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub common: SearchParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionSearchParams {
    pub ledger_id: Option<i64>,
    pub account_id: Option<i64>,
    pub description: Option<String>,
    pub from_amount: Option<rust_decimal::Decimal>,
    pub to_amount: Option<rust_decimal::Decimal>,
    pub currency_code: Option<String>,
    #[serde(flatten)]
    pub common: SearchParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntrySearchParams {
    pub ledger_id: Option<i64>,
    pub account_id: Option<i64>,
    pub transaction_id: Option<i64>,
    pub currency_code: Option<String>,
    pub from_amount: Option<rust_decimal::Decimal>,
    pub to_amount: Option<rust_decimal::Decimal>,
    #[serde(flatten)]
    pub common: SearchParams,
}