use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_with::{DisplayFromStr, serde_as};

static ORDERBY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-?\w+(,-?\w+)*$").expect("Invalid orderby regex pattern"));

// Regex pattern for name filters - allows word chars, hyphens, dots, spaces, and certain regex metacharacters
// Pattern structure: optional ^, then main pattern chars, then optional $ at the end
// Also allows commas for multiple patterns
static NAME_FILTER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\^?[\*\?\w\-\. ]+\$?)(,\^?[\*\?\w\-\. ]+\$?)*$")
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
        // The regex allows: word chars (\w), hyphens, dots, spaces, and regex metacharacters (^, $, *, ?)
        // This combined with parameterized queries provides SQL injection protection
        if !NAME_FILTER_REGEX.is_match(s) {
            return Err(serde::de::Error::custom(format!(
                "Invalid name filter format: '{}'. Patterns must contain only letters, numbers, spaces, hyphens, dots, *, and ?. Can optionally start with ^ and/or end with $. Multiple patterns can be separated by commas.",
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
    /// The orderby field is pre-validated against SQL injection and checked against a whitelist
    pub fn parse_order_by(&self, allowed_columns: &[&str]) -> Option<String> {
        self.orderby.as_ref().and_then(|orderby| {
            let parts: Vec<String> = orderby
                .split(',')
                .filter_map(|field| {
                    let field = field.trim();
                    let (column, is_desc) = if field.starts_with('-') {
                        (&field[1..], true)
                    } else {
                        (field, false)
                    };

                    // Validate against whitelist
                    if allowed_columns.contains(&column) {
                        Some(format!(
                            "{} {}",
                            column,
                            if is_desc { "DESC" } else { "ASC" }
                        ))
                    } else {
                        // Invalid column name, skip it
                        None
                    }
                })
                .collect();

            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
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
    /// Comma-delimited list of transaction IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub tx: Option<String>,
    /// Comma-delimited list of ledger IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list", alias = "ledger")]
    pub ledger_id: Option<String>,
    /// Comma-delimited list of account IDs (e.g., "1,2,3")
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub acct: Option<String>,
    /// Currency code patterns (comma-delimited regex patterns)
    pub currency: Option<String>,
    /// Memo patterns (regex patterns)
    pub memo: Option<String>,
    #[serde(flatten)]
    pub base: SearchParams,
}
