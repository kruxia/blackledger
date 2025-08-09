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
            Regex::new(r"^[A-Z][A-Z0-9\.\-_]*[A-Z0-9]$").expect("Invalid currency code")
        });
        regex.is_match(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_valid_currency_code() {
        // Standard 3-letter codes
        assert!(Currency::is_valid_code("USD"));
        assert!(Currency::is_valid_code("EUR"));
        assert!(Currency::is_valid_code("GBP"));
        assert!(Currency::is_valid_code("CAD"));
        assert!(Currency::is_valid_code("JPY"));
        assert!(Currency::is_valid_code("CHF"));
        assert!(Currency::is_valid_code("AUD"));
        assert!(Currency::is_valid_code("NZD"));

        // With numbers (like the Python examples)
        assert!(Currency::is_valid_code("USD2"));
        assert!(Currency::is_valid_code("A1"));
        assert!(Currency::is_valid_code("XX99"));

        // Stock symbols and other codes
        assert!(Currency::is_valid_code("GOOG"));
        assert!(Currency::is_valid_code("AAPL"));
        assert!(Currency::is_valid_code("MSFT"));

        // With allowed special characters
        assert!(Currency::is_valid_code("US-D"));
        assert!(Currency::is_valid_code("US_D"));
        assert!(Currency::is_valid_code("US.D"));
        assert!(Currency::is_valid_code("A-B_C.D2"));

        // Edge cases - minimum valid length (2 chars)
        assert!(Currency::is_valid_code("AA"));
        assert!(Currency::is_valid_code("A1"));
        assert!(Currency::is_valid_code("Z9"));

        // Long currency codes
        assert!(Currency::is_valid_code("CRYPTOCURRENCY"));
        assert!(Currency::is_valid_code("VERY_LONG_CURRENCY_CODE_123"));
    }

    #[test]
    fn test_invalid_currency_code() {
        // Single character
        assert!(!Currency::is_valid_code("U"));
        assert!(!Currency::is_valid_code("A"));
        assert!(!Currency::is_valid_code("Z"));
        assert!(!Currency::is_valid_code("1"));

        // Lowercase letters
        assert!(!Currency::is_valid_code("usd"));
        assert!(!Currency::is_valid_code("UsD"));
        assert!(!Currency::is_valid_code("Usd"));
        assert!(!Currency::is_valid_code("uSD"));
        assert!(!Currency::is_valid_code("usD"));

        // Starting with number or special char
        assert!(!Currency::is_valid_code("1USD"));
        assert!(!Currency::is_valid_code("-USD"));
        assert!(!Currency::is_valid_code(".USD"));
        assert!(!Currency::is_valid_code("_USD"));
        assert!(!Currency::is_valid_code("9ABC"));

        // Ending with special char
        assert!(!Currency::is_valid_code("USD-"));
        assert!(!Currency::is_valid_code("USD."));
        assert!(!Currency::is_valid_code("USD_"));
        assert!(!Currency::is_valid_code("ABC-"));

        // Empty string
        assert!(!Currency::is_valid_code(""));

        // Invalid characters
        assert!(!Currency::is_valid_code("US$"));
        assert!(!Currency::is_valid_code("US@D"));
        assert!(!Currency::is_valid_code("US!"));
        assert!(!Currency::is_valid_code("US#D"));
        assert!(!Currency::is_valid_code("US%D"));
        assert!(!Currency::is_valid_code("US&D"));
        assert!(!Currency::is_valid_code("US*D"));
        assert!(!Currency::is_valid_code("US(D"));
        assert!(!Currency::is_valid_code("US)D"));
        assert!(!Currency::is_valid_code("US[D"));
        assert!(!Currency::is_valid_code("US]D"));
        assert!(!Currency::is_valid_code("US{D"));
        assert!(!Currency::is_valid_code("US}D"));
        assert!(!Currency::is_valid_code("US/D"));
        assert!(!Currency::is_valid_code("US\\D"));
        assert!(!Currency::is_valid_code("US|D"));
        assert!(!Currency::is_valid_code("US D")); // Space
        assert!(!Currency::is_valid_code("US\tD")); // Tab
        assert!(!Currency::is_valid_code("US\nD")); // Newline

        // Unicode characters
        assert!(!Currency::is_valid_code("US€"));
        assert!(!Currency::is_valid_code("US£"));
        assert!(!Currency::is_valid_code("US¥"));
        assert!(!Currency::is_valid_code("USД"));
        assert!(!Currency::is_valid_code("US中"));
        assert!(!Currency::is_valid_code("US한"));

        // Whitespace only
        assert!(!Currency::is_valid_code(" "));
        assert!(!Currency::is_valid_code("  "));
        assert!(!Currency::is_valid_code("\t"));
        assert!(!Currency::is_valid_code("\n"));
        assert!(!Currency::is_valid_code("\r"));

        // Starting with valid but ending with lowercase
        assert!(!Currency::is_valid_code("USd"));
        assert!(!Currency::is_valid_code("ABc"));
    }

    #[test]
    fn test_currency_serialization() {
        let currency = Currency {
            code: "USD".to_string(),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&currency).unwrap();
        assert!(json.contains("\"code\":\"USD\""));
        assert!(json.contains("\"created\":\"2024-01-15T10:30:45Z\""));

        // Deserialize from JSON
        let deserialized: Currency = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.code, "USD");
        assert_eq!(deserialized.created, currency.created);
    }

    #[test]
    fn test_currency_deserialization_edge_cases() {
        // Valid JSON
        let valid_json = r#"{"code":"EUR","created":"2024-01-01T00:00:00Z"}"#;
        let currency: Currency = serde_json::from_str(valid_json).unwrap();
        assert_eq!(currency.code, "EUR");

        // Test with milliseconds in timestamp
        let json_with_millis = r#"{"code":"GBP","created":"2024-01-01T00:00:00.123Z"}"#;
        let currency: Currency = serde_json::from_str(json_with_millis).unwrap();
        assert_eq!(currency.code, "GBP");

        // Test with timezone offset
        let json_with_offset = r#"{"code":"JPY","created":"2024-01-01T00:00:00+00:00"}"#;
        let currency: Currency = serde_json::from_str(json_with_offset).unwrap();
        assert_eq!(currency.code, "JPY");
    }

    #[test]
    fn test_currency_deserialization_errors() {
        // Missing code field
        let missing_code = r#"{"created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Currency>(missing_code).is_err());

        // Missing created field
        let missing_created = r#"{"code":"USD"}"#;
        assert!(serde_json::from_str::<Currency>(missing_created).is_err());

        // Invalid date format
        let invalid_date = r#"{"code":"USD","created":"not-a-date"}"#;
        assert!(serde_json::from_str::<Currency>(invalid_date).is_err());

        // Null values
        let null_code = r#"{"code":null,"created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Currency>(null_code).is_err());

        // Wrong types
        let wrong_types = r#"{"code":123,"created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Currency>(wrong_types).is_err());
    }

    #[test]
    fn test_currency_clone() {
        let original = Currency {
            code: "USD".to_string(),
            created: Utc::now(),
        };

        let cloned = original.clone();
        assert_eq!(original.code, cloned.code);
        assert_eq!(original.created, cloned.created);

        // Ensure they are separate instances
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_currency_debug() {
        let currency = Currency {
            code: "USD".to_string(),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
        };

        let debug_str = format!("{:?}", currency);
        assert!(debug_str.contains("Currency"));
        assert!(debug_str.contains("code"));
        assert!(debug_str.contains("USD"));
        assert!(debug_str.contains("created"));
    }

    #[test]
    fn test_regex_initialization() {
        // Test that regex is properly initialized and cached
        assert!(Currency::is_valid_code("USD"));

        // Call again to ensure OnceLock works correctly
        assert!(Currency::is_valid_code("EUR"));
        assert!(!Currency::is_valid_code("usd"));
    }
}
