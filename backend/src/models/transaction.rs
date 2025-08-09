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
///     effective: None,  // Will default to created timestamp
///     memo: Some("Sale of goods".to_string()),
///     meta: None,
///     entries: vec![
///         CreateEntry {
///             account_id: 1001,  // Cash account
///             currency: "USD".to_string(),
///             debit: Some(dec!(100.00)),
///             credit: None,
///             account_version: None,
///         },
///         CreateEntry {
///             account_id: 4001,  // Revenue account
///             currency: "USD".to_string(),
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
    pub effective: Option<DateTime<Utc>>, // Optional, defaults to created timestamp
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
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub debit: Option<rust_decimal::Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub credit: Option<rust_decimal::Decimal>,
    #[serde(default, rename = "version")]
    pub account_version: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_transaction_serialization() {
        let transaction = Transaction {
            id: 1000,
            ledger_id: 1,
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
            effective: Utc.with_ymd_and_hms(2024, 1, 16, 0, 0, 0).unwrap(),
            memo: Some("Test transaction".to_string()),
            meta: Some(json!({"invoice": "INV-001", "customer": "ACME Corp"})),
            entries: vec![],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&transaction).unwrap();
        assert!(json.contains(r#""id":1000"#));
        assert!(json.contains(r#""ledger_id":1"#));
        assert!(json.contains(r#""memo":"Test transaction""#));
        assert!(json.contains(r#""created":"2024-01-15T10:30:45Z""#));
        assert!(json.contains(r#""effective":"2024-01-16T00:00:00Z""#));
        assert!(json.contains(r#""invoice":"INV-001""#));
        assert!(json.contains(r#""customer":"ACME Corp""#));
        assert!(json.contains(r#""entries":[]"#));

        // Deserialize from JSON
        let deserialized: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1000);
        assert_eq!(deserialized.ledger_id, 1);
        assert_eq!(deserialized.memo, Some("Test transaction".to_string()));
        assert_eq!(deserialized.created, transaction.created);
        assert_eq!(deserialized.effective, transaction.effective);
        assert_eq!(deserialized.entries.len(), 0);
    }

    #[test]
    fn test_transaction_with_nulls() {
        let transaction = Transaction {
            id: 1,
            ledger_id: 1,
            created: Utc::now(),
            effective: Utc::now(),
            memo: None,
            meta: None,
            entries: vec![],
        };

        let json = serde_json::to_string(&transaction).unwrap();
        assert!(json.contains(r#""memo":null"#));
        assert!(json.contains(r#""meta":null"#));

        let deserialized: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.memo, None);
        assert_eq!(deserialized.meta, None);
    }

    #[test]
    fn test_transaction_with_entries() {
        // Note: We're testing serialization but Entry is defined in another module
        // This tests that the entries field works correctly in Transaction
        let json = r#"{
            "id": 100,
            "ledger_id": 1,
            "created": "2024-01-01T00:00:00Z",
            "effective": "2024-01-01T00:00:00Z",
            "memo": "Test",
            "meta": null,
            "entries": []
        }"#;

        let transaction: Transaction = serde_json::from_str(json).unwrap();
        assert_eq!(transaction.id, 100);
        assert_eq!(transaction.entries.len(), 0);
    }

    #[test]
    fn test_transaction_meta_variations() {
        // Test with different meta JSON values
        let test_cases = vec![
            (Some(json!({})), "empty object"),
            (Some(json!({"key": "value"})), "simple object"),
            (Some(json!({"nested": {"key": "value"}})), "nested object"),
            (Some(json!([1, 2, 3])), "array"),
            (Some(json!("string")), "string"),
            (Some(json!(123)), "number"),
            (Some(json!(true)), "boolean"),
            // Note: json!(null) creates Some(Null), not None
            // We'll test it separately
        ];

        for (meta_value, description) in test_cases {
            let transaction = Transaction {
                id: 1,
                ledger_id: 1,
                created: Utc::now(),
                effective: Utc::now(),
                memo: None,
                meta: meta_value.clone(),
                entries: vec![],
            };

            let json = serde_json::to_string(&transaction).unwrap();
            let deserialized: Transaction = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.meta, meta_value, "Failed for {}", description);
        }

        // Test json!(null) separately
        // When we have Some(json!(null)), it serializes to "meta":null
        // which deserializes back to None, not Some(Null)
        let transaction_with_null = Transaction {
            id: 1,
            ledger_id: 1,
            created: Utc::now(),
            effective: Utc::now(),
            memo: None,
            meta: Some(json!(null)),
            entries: vec![],
        };
        let json = serde_json::to_string(&transaction_with_null).unwrap();
        assert!(json.contains(r#""meta":null"#));
        // When deserialized, a JSON null in meta field becomes None
        let deserialized: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.meta, None);
    }

    #[test]
    fn test_transaction_deserialization_errors() {
        // Missing required fields
        let missing_id = r#"{"ledger_id":1,"created":"2024-01-01T00:00:00Z","effective":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Transaction>(missing_id).is_err());

        let missing_ledger_id =
            r#"{"id":1,"created":"2024-01-01T00:00:00Z","effective":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Transaction>(missing_ledger_id).is_err());

        // Invalid date format
        let invalid_date =
            r#"{"id":1,"ledger_id":1,"created":"invalid","effective":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Transaction>(invalid_date).is_err());

        // Invalid types
        let invalid_id = r#"{"id":"not_a_number","ledger_id":1,"created":"2024-01-01T00:00:00Z","effective":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Transaction>(invalid_id).is_err());
    }

    #[test]
    fn test_create_transaction_serialization() {
        let create = CreateTransaction {
            ledger_id: 1,
            effective: Some(Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap()),
            memo: Some("Monthly rent payment".to_string()),
            meta: Some(json!({"property": "123 Main St"})),
            entries: vec![
                CreateEntry {
                    account_id: 1001,
                    currency: "USD".to_string(),
                    debit: Some(dec!(1500.00)),
                    credit: None,
                    account_version: Some(10),
                },
                CreateEntry {
                    account_id: 2001,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(1500.00)),
                    account_version: Some(20),
                },
            ],
        };

        let json = serde_json::to_string(&create).unwrap();
        assert!(json.contains(r#""ledger_id":1"#));
        assert!(json.contains(r#""effective":"2024-01-15T00:00:00Z""#));
        assert!(json.contains(r#""memo":"Monthly rent payment""#));
        assert!(json.contains(r#""property":"123 Main St""#));
        assert!(json.contains(r#""acct":1001"#)); // Note: renamed field
        assert!(json.contains(r#""acct":2001"#));
        assert!(json.contains(r#""currency":"USD""#));
        assert!(json.contains(r#""debit":"1500""#) || json.contains(r#""debit":"1500.00""#));
        assert!(json.contains(r#""credit":"1500""#) || json.contains(r#""credit":"1500.00""#));
        assert!(json.contains(r#""version":10"#));
        assert!(json.contains(r#""version":20"#));

        let deserialized: CreateTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ledger_id, 1);
        assert_eq!(deserialized.memo, Some("Monthly rent payment".to_string()));
        assert_eq!(deserialized.entries.len(), 2);
        assert_eq!(deserialized.entries[0].account_id, 1001);
        assert_eq!(deserialized.entries[0].debit, Some(dec!(1500.00)));
        assert_eq!(deserialized.entries[1].credit, Some(dec!(1500.00)));
    }

    #[test]
    fn test_create_transaction_with_nulls() {
        let create = CreateTransaction {
            ledger_id: 1,
            effective: None,
            memo: None,
            meta: None,
            entries: vec![],
        };

        let json = serde_json::to_string(&create).unwrap();
        assert!(json.contains(r#""effective":null"#));
        assert!(json.contains(r#""memo":null"#));
        assert!(json.contains(r#""meta":null"#));
        assert!(json.contains(r#""entries":[]"#));

        let deserialized: CreateTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.effective, None);
        assert_eq!(deserialized.memo, None);
        assert_eq!(deserialized.meta, None);
        assert_eq!(deserialized.entries.len(), 0);
    }

    #[test]
    fn test_create_entry_serialization() {
        // Test debit entry
        let debit_entry = CreateEntry {
            account_id: 1000,
            currency: "EUR".to_string(),
            debit: Some(dec!(250.75)),
            credit: None,
            account_version: Some(42),
        };

        let json = serde_json::to_string(&debit_entry).unwrap();
        assert!(json.contains(r#""acct":1000"#)); // Field renamed
        assert!(json.contains(r#""currency":"EUR""#)); // Field renamed
        assert!(json.contains(r#""debit":"250.75""#));
        assert!(!json.contains(r#""credit":"#) || json.contains(r#""credit":null"#));
        assert!(json.contains(r#""version":42"#)); // Field renamed

        // Test credit entry
        let credit_entry = CreateEntry {
            account_id: 2000,
            currency: "GBP".to_string(),
            debit: None,
            credit: Some(dec!(999.99)),
            account_version: None,
        };

        let json = serde_json::to_string(&credit_entry).unwrap();
        assert!(json.contains(r#""acct":2000"#));
        assert!(json.contains(r#""currency":"GBP""#));
        assert!(!json.contains(r#""debit":"#) || json.contains(r#""debit":null"#));
        assert!(json.contains(r#""credit":"999.99""#));
        assert!(!json.contains(r#""version":"#) || json.contains(r#""version":null"#));
    }

    #[test]
    fn test_create_entry_deserialization() {
        // Test with renamed fields
        let json = r#"{"acct":1234,"currency":"USD","debit":"100.50","credit":null,"version":10}"#;
        let entry: CreateEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.account_id, 1234);
        assert_eq!(entry.currency, "USD");
        assert_eq!(entry.debit, Some(Decimal::from_str("100.50").unwrap()));
        assert_eq!(entry.credit, None);
        assert_eq!(entry.account_version, Some(10));

        // Test with missing optional fields
        let json_minimal = r#"{"acct":5678,"currency":"JPY"}"#;
        let entry: CreateEntry = serde_json::from_str(json_minimal).unwrap();
        assert_eq!(entry.account_id, 5678);
        assert_eq!(entry.currency, "JPY");
        assert_eq!(entry.debit, None);
        assert_eq!(entry.credit, None);
        assert_eq!(entry.account_version, None);
    }

    #[test]
    fn test_create_entry_decimal_precision() {
        let test_cases = vec![
            ("0", dec!(0)),
            ("0.01", dec!(0.01)),
            ("0.001", dec!(0.001)),
            ("0.000000001", dec!(0.000000001)),
            (
                "123456789.123456789",
                Decimal::from_str("123456789.123456789").unwrap(),
            ),
            ("-500.50", dec!(-500.50)),
        ];

        for (decimal_str, expected) in test_cases {
            let json = format!(r#"{{"acct":1,"currency":"USD","debit":"{}"}}"#, decimal_str);
            let entry: CreateEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(entry.debit, Some(expected), "Failed for {}", decimal_str);
        }
    }

    #[test]
    fn test_create_entry_large_numbers() {
        // Test with very large account IDs and versions
        let entry = CreateEntry {
            account_id: i64::MAX,
            currency: "BTC".to_string(),
            debit: Some(Decimal::from_str("999999999999999.999999999").unwrap()),
            credit: None,
            account_version: Some(i64::MAX),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: CreateEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.account_id, i64::MAX);
        assert_eq!(deserialized.account_version, Some(i64::MAX));
        assert_eq!(
            deserialized.debit,
            Some(Decimal::from_str("999999999999999.999999999").unwrap())
        );
    }

    #[test]
    fn test_create_entry_deserialization_errors() {
        // Missing required fields
        let missing_acct = r#"{"currency":"USD","debit":"100"}"#;
        assert!(serde_json::from_str::<CreateEntry>(missing_acct).is_err());

        let missing_currency = r#"{"acct":1,"debit":"100"}"#;
        assert!(serde_json::from_str::<CreateEntry>(missing_currency).is_err());

        // Invalid decimal format
        let invalid_decimal = r#"{"acct":1,"currency":"USD","debit":"not_a_number"}"#;
        assert!(serde_json::from_str::<CreateEntry>(invalid_decimal).is_err());

        // Invalid types
        let invalid_acct = r#"{"acct":"not_a_number","currency":"USD"}"#;
        assert!(serde_json::from_str::<CreateEntry>(invalid_acct).is_err());
    }

    #[test]
    fn test_transaction_clone() {
        let transaction = Transaction {
            id: 100,
            ledger_id: 1,
            created: Utc::now(),
            effective: Utc::now(),
            memo: Some("Test".to_string()),
            meta: Some(json!({"key": "value"})),
            entries: vec![],
        };

        let cloned = transaction.clone();
        assert_eq!(transaction.id, cloned.id);
        assert_eq!(transaction.ledger_id, cloned.ledger_id);
        assert_eq!(transaction.memo, cloned.memo);
        assert_eq!(transaction.meta, cloned.meta);
        assert_eq!(transaction.entries.len(), cloned.entries.len());

        // Ensure separate instances
        assert_ne!(&transaction as *const _, &cloned as *const _);
    }

    #[test]
    fn test_create_transaction_clone() {
        let create = CreateTransaction {
            ledger_id: 1,
            effective: Some(Utc::now()),
            memo: Some("Test".to_string()),
            meta: Some(json!({"test": true})),
            entries: vec![CreateEntry {
                account_id: 1,
                currency: "USD".to_string(),
                debit: Some(dec!(100)),
                credit: None,
                account_version: None,
            }],
        };

        let cloned = create.clone();
        assert_eq!(create.ledger_id, cloned.ledger_id);
        assert_eq!(create.memo, cloned.memo);
        assert_eq!(create.entries.len(), cloned.entries.len());
        assert_eq!(create.entries[0].account_id, cloned.entries[0].account_id);

        // Ensure separate instances
        assert_ne!(&create as *const _, &cloned as *const _);
    }

    #[test]
    fn test_transaction_debug() {
        let transaction = Transaction {
            id: 42,
            ledger_id: 1,
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
            effective: Utc.with_ymd_and_hms(2024, 1, 16, 0, 0, 0).unwrap(),
            memo: Some("Debug test".to_string()),
            meta: None,
            entries: vec![],
        };

        let debug_str = format!("{:?}", transaction);
        assert!(debug_str.contains("Transaction"));
        assert!(debug_str.contains("id: 42"));
        assert!(debug_str.contains("ledger_id: 1"));
        assert!(debug_str.contains("memo: Some"));
        assert!(debug_str.contains("Debug test"));
        assert!(debug_str.contains("entries: []"));
    }

    #[test]
    fn test_multi_currency_entries() {
        let create = CreateTransaction {
            ledger_id: 1,
            effective: None,
            memo: Some("Multi-currency transaction".to_string()),
            meta: None,
            entries: vec![
                CreateEntry {
                    account_id: 1,
                    currency: "USD".to_string(),
                    debit: Some(dec!(100)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: 2,
                    currency: "EUR".to_string(),
                    debit: Some(dec!(85)),
                    credit: None,
                    account_version: None,
                },
                CreateEntry {
                    account_id: 3,
                    currency: "USD".to_string(),
                    debit: None,
                    credit: Some(dec!(100)),
                    account_version: None,
                },
                CreateEntry {
                    account_id: 4,
                    currency: "EUR".to_string(),
                    debit: None,
                    credit: Some(dec!(85)),
                    account_version: None,
                },
            ],
        };

        let json = serde_json::to_string(&create).unwrap();
        let deserialized: CreateTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.entries.len(), 4);

        // Verify multi-currency structure is preserved
        let usd_entries: Vec<&CreateEntry> = deserialized
            .entries
            .iter()
            .filter(|e| e.currency == "USD")
            .collect();
        assert_eq!(usd_entries.len(), 2);

        let eur_entries: Vec<&CreateEntry> = deserialized
            .entries
            .iter()
            .filter(|e| e.currency == "EUR")
            .collect();
        assert_eq!(eur_entries.len(), 2);
    }
}
