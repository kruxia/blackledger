use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
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
    pub ledger_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub number: Option<String>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub common: SearchParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionSearchParams {
    pub ledger_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
    pub from_amount: Option<rust_decimal::Decimal>,
    pub to_amount: Option<rust_decimal::Decimal>,
    pub currency_code: Option<String>,
    #[serde(flatten)]
    pub common: SearchParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntrySearchParams {
    pub ledger_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub currency_code: Option<String>,
    pub from_amount: Option<rust_decimal::Decimal>,
    pub to_amount: Option<rust_decimal::Decimal>,
    #[serde(flatten)]
    pub common: SearchParams,
}