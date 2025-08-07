use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    #[serde(rename = "_limit")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<i32>,
    #[serde(rename = "_offset")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<i32>,
    #[serde(rename = "_orderby")]
    pub orderby: Option<String>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            limit: Some(100),
            offset: None,
            orderby: None,
        }
    }
}

impl SearchParams {
    /// Parse the orderby field into SQL ORDER BY clause components
    /// e.g., "name,-created" becomes "name ASC, created DESC"
    pub fn parse_order_by(&self) -> Option<String> {
        self.orderby.as_ref().map(|orderby| {
            orderby
                .split(',')
                .map(|field| {
                    let field = field.trim();
                    if field.starts_with('-') {
                        format!("{} DESC", &field[1..])
                    } else {
                        format!("{} ASC", field)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
    }
    
    /// Get the limit with a maximum of 100
    pub fn get_limit(&self) -> i32 {
        self.limit.unwrap_or(100).min(100)
    }
    
    /// Get the offset, defaulting to 0
    pub fn get_offset(&self) -> i32 {
        self.offset.unwrap_or(0)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrencySearchParams {
    pub code: Option<String>,
    #[serde(flatten)]
    pub base: SearchParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountSearchParams {
    pub ledger_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub number: Option<String>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub base: SearchParams,
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
    pub base: SearchParams,
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
    pub base: SearchParams,
}
