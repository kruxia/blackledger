use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Currency {
    pub code: String,
    pub created: DateTime<Utc>,
}

static CURRENCY_CODE_REGEX: OnceLock<Regex> = OnceLock::new();

impl Currency {
    pub fn is_valid_code(code: &str) -> bool {
        let regex = CURRENCY_CODE_REGEX.get_or_init(|| {
            Regex::new(r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$").expect("Invalid regex pattern")
        });
        regex.is_match(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_currency_code() {
        // Standard 3-letter codes
        assert!(Currency::is_valid_code("USD"));
        assert!(Currency::is_valid_code("EUR"));
        assert!(Currency::is_valid_code("GBP"));
        assert!(Currency::is_valid_code("CAD"));

        // With numbers (like the Python examples)
        assert!(Currency::is_valid_code("USD2"));
        assert!(Currency::is_valid_code("A1"));

        // Stock symbols and other codes
        assert!(Currency::is_valid_code("GOOG"));
        assert!(Currency::is_valid_code("AAPL"));

        // With allowed special characters
        assert!(Currency::is_valid_code("US-D"));
        assert!(Currency::is_valid_code("US_D"));
        assert!(Currency::is_valid_code("US.D"));
        assert!(Currency::is_valid_code("A-B_C.D2"));
    }

    #[test]
    fn test_invalid_currency_code() {
        // Single character
        assert!(!Currency::is_valid_code("U"));

        // Lowercase letters
        assert!(!Currency::is_valid_code("usd"));
        assert!(!Currency::is_valid_code("UsD"));

        // Starting with number or special char
        assert!(!Currency::is_valid_code("1USD"));
        assert!(!Currency::is_valid_code("-USD"));
        assert!(!Currency::is_valid_code(".USD"));
        assert!(!Currency::is_valid_code("_USD"));

        // Ending with special char
        assert!(!Currency::is_valid_code("USD-"));
        assert!(!Currency::is_valid_code("USD."));
        assert!(!Currency::is_valid_code("USD_"));

        // Empty string
        assert!(!Currency::is_valid_code(""));

        // Invalid characters
        assert!(!Currency::is_valid_code("US$"));
        assert!(!Currency::is_valid_code("US@D"));
        assert!(!Currency::is_valid_code("US!"));
    }
}
