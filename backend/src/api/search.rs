use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_with::{DisplayFromStr, serde_as};

static ORDERBY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-?\w+(,-?\w+)*$").expect("Invalid orderby regex pattern"));

fn deserialize_orderby<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_str: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref s) = opt_str {
        if !ORDERBY_REGEX.is_match(s) {
            return Err(serde::de::Error::custom(format!(
                "Invalid orderby format: '{}'. Must match pattern: ^-?\\w+(,-?\\w+)*$",
                s
            )));
        }
    }

    Ok(opt_str)
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    #[serde(rename = "_limit")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<i32>,
    #[serde(rename = "_offset")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<i32>,
    #[serde(
        rename = "_orderby",
        default,
        deserialize_with = "deserialize_orderby",
        skip_serializing_if = "Option::is_none"
    )]
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
    /// The orderby field is pre-validated against SQL injection
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
