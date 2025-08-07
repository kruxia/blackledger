use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_with::{DisplayFromStr, serde_as};

static ORDERBY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-?\w+(,-?\w+)*$").expect("Invalid orderby regex pattern"));

// Regex pattern for name filters - allows word chars, hyphens, dots, spaces, and regex special chars (^, $, *, ?)
// Also allows commas for multiple patterns
static NAME_FILTER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[\^\$\*\?\w\-\. ]+(,[\^\$\*\?\w\-\. ]+)*$")
        .expect("Invalid name filter regex pattern")
});

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

fn deserialize_name_filter<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_str: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref s) = opt_str {
        // Validate that the name filter matches our allowed pattern
        // This prevents SQL injection by ensuring only safe characters are used
        if !NAME_FILTER_REGEX.is_match(s) {
            return Err(serde::de::Error::custom(format!(
                "Invalid name filter format: '{}'. Must contain only letters, numbers, spaces, hyphens, dots, and regex metacharacters (^, $, *, ?). Multiple patterns can be separated by commas.",
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
pub struct LedgerSearchParams {
    /// Comma-delimited list of ledger IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub id: Option<String>,
    /// Comma-delimited list of name regex patterns (validated for safety)
    #[serde(default, deserialize_with = "deserialize_name_filter")]
    pub name: Option<String>,
    #[serde(flatten)]
    pub base: SearchParams,
}

fn deserialize_id_list<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_str: Option<String> = Option::deserialize(deserializer)?;

    if let Some(ref s) = opt_str {
        // Validate that it's a comma-delimited list of numbers
        let id_pattern = Regex::new(r"^[0-9]+(,[0-9]+)*$").unwrap();
        if !id_pattern.is_match(s) {
            return Err(serde::de::Error::custom(format!(
                "Invalid id format: '{}'. Must be comma-delimited list of numbers (e.g., '1,2,3')",
                s
            )));
        }
    }

    Ok(opt_str)
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountSearchParams {
    /// Comma-delimited list of account IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub id: Option<String>,
    /// Comma-delimited list of ledger IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list", alias = "ledger")]
    pub ledger_id: Option<String>,
    /// Comma-delimited list of parent IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list", alias = "parent")]
    pub parent_id: Option<String>,
    /// Comma-delimited list of version IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub version: Option<String>,
    /// Comma-delimited list of account numbers (e.g., "100,200,300")
    pub number: Option<String>,
    /// Comma-delimited list of name patterns (regex patterns, validated for safety)
    #[serde(default, deserialize_with = "deserialize_name_filter")]
    pub name: Option<String>,
    /// Normal balance type: DR/CR (case-insensitive, also accepts debit/credit)
    pub normal: Option<String>,
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
