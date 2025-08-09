use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Entry {
    pub id: i64,
    pub ledger_id: i64,
    pub transaction_id: i64,
    pub account_id: i64,
    #[sqlx(rename = "currency")]
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub debit: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub credit: Option<Decimal>,
}

impl Entry {
    pub fn amount(&self) -> Decimal {
        match (self.debit, self.credit) {
            (Some(debit), None) => debit,
            (None, Some(credit)) => -credit,
            _ => Decimal::ZERO,
        }
    }

    pub fn is_valid(&self) -> bool {
        match (self.debit, self.credit) {
            (Some(debit), None) => debit > Decimal::ZERO,
            (None, Some(credit)) => credit > Decimal::ZERO,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_entry_amount() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(100.00)),
            credit: None,
        };

        assert_eq!(entry.amount(), dec!(100.00));

        entry.debit = None;
        entry.credit = Some(dec!(50.00));
        assert_eq!(entry.amount(), dec!(-50.00));

        entry.debit = None;
        entry.credit = None;
        assert_eq!(entry.amount(), dec!(0));

        // Both debit and credit (invalid but should return 0)
        entry.debit = Some(dec!(100.00));
        entry.credit = Some(dec!(50.00));
        assert_eq!(entry.amount(), dec!(0));
    }

    #[test]
    fn test_entry_amount_edge_cases() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency: "USD".to_string(),
            debit: None,
            credit: None,
        };

        // Very large numbers
        entry.debit = Some(Decimal::from_str("999999999999999.999999999").unwrap());
        entry.credit = None;
        assert_eq!(
            entry.amount(),
            Decimal::from_str("999999999999999.999999999").unwrap()
        );

        entry.debit = None;
        entry.credit = Some(Decimal::from_str("999999999999999.999999999").unwrap());
        assert_eq!(
            entry.amount(),
            Decimal::from_str("-999999999999999.999999999").unwrap()
        );

        // Very small numbers
        entry.debit = Some(Decimal::from_str("0.000000001").unwrap());
        entry.credit = None;
        assert_eq!(entry.amount(), Decimal::from_str("0.000000001").unwrap());

        entry.debit = None;
        entry.credit = Some(Decimal::from_str("0.000000001").unwrap());
        assert_eq!(entry.amount(), Decimal::from_str("-0.000000001").unwrap());

        // Negative values (shouldn't happen in practice but test behavior)
        entry.debit = Some(dec!(-100));
        entry.credit = None;
        assert_eq!(entry.amount(), dec!(-100));

        entry.debit = None;
        entry.credit = Some(dec!(-100));
        assert_eq!(entry.amount(), dec!(100)); // -(-100) = 100
    }

    #[test]
    fn test_entry_is_valid() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency: "USD".to_string(),
            debit: Some(dec!(100.00)),
            credit: None,
        };

        assert!(entry.is_valid());

        entry.debit = None;
        entry.credit = Some(dec!(50.00));
        assert!(entry.is_valid());

        entry.debit = Some(dec!(100.00));
        entry.credit = Some(dec!(50.00));
        assert!(!entry.is_valid());

        entry.debit = None;
        entry.credit = None;
        assert!(!entry.is_valid());

        entry.debit = Some(dec!(0));
        entry.credit = None;
        assert!(!entry.is_valid());

        entry.debit = None;
        entry.credit = Some(dec!(0));
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_entry_is_valid_edge_cases() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency: "USD".to_string(),
            debit: None,
            credit: None,
        };

        // Very small positive amounts are valid
        entry.debit = Some(Decimal::from_str("0.000000001").unwrap());
        entry.credit = None;
        assert!(entry.is_valid());

        entry.debit = None;
        entry.credit = Some(Decimal::from_str("0.000000001").unwrap());
        assert!(entry.is_valid());

        // Negative amounts are invalid
        entry.debit = Some(dec!(-1));
        entry.credit = None;
        assert!(!entry.is_valid());

        entry.debit = None;
        entry.credit = Some(dec!(-1));
        assert!(!entry.is_valid());

        // Very large amounts are valid
        entry.debit = Some(Decimal::from_str("999999999999999.999999999").unwrap());
        entry.credit = None;
        assert!(entry.is_valid());
    }

    #[test]
    fn test_entry_serialization() {
        let entry = Entry {
            id: 1000,
            ledger_id: 1,
            transaction_id: 500,
            account_id: 100,
            currency: "USD".to_string(),
            debit: Some(dec!(1234.56)),
            credit: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""id":1000"#));
        assert!(json.contains(r#""ledger_id":1"#));
        assert!(json.contains(r#""transaction_id":500"#));
        assert!(json.contains(r#""account_id":100"#));
        assert!(json.contains(r#""currency":"USD""#));
        assert!(json.contains(r#""debit":"1234.56""#));
        assert!(json.contains(r#""credit":null"#));

        // Deserialize from JSON
        let deserialized: Entry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1000);
        assert_eq!(deserialized.ledger_id, 1);
        assert_eq!(deserialized.transaction_id, 500);
        assert_eq!(deserialized.account_id, 100);
        assert_eq!(deserialized.currency, "USD");
        assert_eq!(deserialized.debit, Some(dec!(1234.56)));
        assert_eq!(deserialized.credit, None);
    }

    #[test]
    fn test_entry_serialization_credit() {
        let entry = Entry {
            id: 2000,
            ledger_id: 2,
            transaction_id: 600,
            account_id: 200,
            currency: "EUR".to_string(),
            debit: None,
            credit: Some(dec!(9999.99)),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""debit":null"#));
        assert!(json.contains(r#""credit":"9999.99""#));

        let deserialized: Entry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.debit, None);
        assert_eq!(deserialized.credit, Some(dec!(9999.99)));
    }

    #[test]
    fn test_entry_deserialization_edge_cases() {
        // Test with string decimals
        let json = r#"{
            "id": 1,
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": "USD",
            "debit": "123.456789",
            "credit": null
        }"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.debit, Some(Decimal::from_str("123.456789").unwrap()));

        // Test with null values
        let json_nulls = r#"{
            "id": 1,
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": "USD",
            "debit": null,
            "credit": null
        }"#;
        let entry: Entry = serde_json::from_str(json_nulls).unwrap();
        assert_eq!(entry.debit, None);
        assert_eq!(entry.credit, None);

        // Test with maximum precision
        let json_precision = r#"{
            "id": 1,
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": "USD",
            "debit": "0.000000001",
            "credit": null
        }"#;
        let entry: Entry = serde_json::from_str(json_precision).unwrap();
        assert_eq!(entry.debit, Some(Decimal::from_str("0.000000001").unwrap()));
    }

    #[test]
    fn test_entry_deserialization_errors() {
        // Missing required fields
        let missing_id = r#"{"ledger_id":1,"transaction_id":1,"account_id":1,"currency":"USD"}"#;
        assert!(serde_json::from_str::<Entry>(missing_id).is_err());

        let missing_currency = r#"{"id":1,"ledger_id":1,"transaction_id":1,"account_id":1}"#;
        assert!(serde_json::from_str::<Entry>(missing_currency).is_err());

        // Invalid decimal format
        let invalid_decimal = r#"{
            "id": 1,
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": "USD",
            "debit": "not_a_number",
            "credit": null
        }"#;
        assert!(serde_json::from_str::<Entry>(invalid_decimal).is_err());

        // Invalid types
        let invalid_id = r#"{
            "id": "not_a_number",
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": "USD",
            "debit": null,
            "credit": null
        }"#;
        assert!(serde_json::from_str::<Entry>(invalid_id).is_err());

        // Currency as number instead of string
        let invalid_currency = r#"{
            "id": 1,
            "ledger_id": 1,
            "transaction_id": 1,
            "account_id": 1,
            "currency": 123,
            "debit": null,
            "credit": null
        }"#;
        assert!(serde_json::from_str::<Entry>(invalid_currency).is_err());
    }

    #[test]
    fn test_entry_with_different_currencies() {
        let entries = vec![
            Entry {
                id: 1,
                ledger_id: 1,
                transaction_id: 1,
                account_id: 1,
                currency: "USD".to_string(),
                debit: Some(dec!(100)),
                credit: None,
            },
            Entry {
                id: 2,
                ledger_id: 1,
                transaction_id: 1,
                account_id: 2,
                currency: "EUR".to_string(),
                debit: Some(dec!(85)),
                credit: None,
            },
            Entry {
                id: 3,
                ledger_id: 1,
                transaction_id: 1,
                account_id: 3,
                currency: "JPY".to_string(),
                debit: None,
                credit: Some(dec!(11000)),
            },
            Entry {
                id: 4,
                ledger_id: 1,
                transaction_id: 1,
                account_id: 4,
                currency: "BTC".to_string(),
                debit: None,
                credit: Some(dec!(0.002345)),
            },
        ];

        for entry in &entries {
            assert!(!entry.currency.is_empty());
            let json = serde_json::to_string(&entry).unwrap();
            let deserialized: Entry = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.currency, entry.currency);
        }
    }

    #[test]
    fn test_entry_clone() {
        let original = Entry {
            id: 100,
            ledger_id: 1,
            transaction_id: 50,
            account_id: 10,
            currency: "USD".to_string(),
            debit: Some(dec!(500.25)),
            credit: None,
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.ledger_id, cloned.ledger_id);
        assert_eq!(original.transaction_id, cloned.transaction_id);
        assert_eq!(original.account_id, cloned.account_id);
        assert_eq!(original.currency, cloned.currency);
        assert_eq!(original.debit, cloned.debit);
        assert_eq!(original.credit, cloned.credit);

        // Ensure separate instances
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_entry_debug() {
        let entry = Entry {
            id: 42,
            ledger_id: 1,
            transaction_id: 100,
            account_id: 10,
            currency: "GBP".to_string(),
            debit: Some(dec!(750.50)),
            credit: None,
        };

        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("Entry"));
        assert!(debug_str.contains("id: 42"));
        assert!(debug_str.contains("ledger_id: 1"));
        assert!(debug_str.contains("transaction_id: 100"));
        assert!(debug_str.contains("account_id: 10"));
        assert!(debug_str.contains("currency:"));
        assert!(debug_str.contains("GBP"));
        assert!(debug_str.contains("debit: Some"));
        assert!(debug_str.contains("credit: None"));
    }

    #[test]
    fn test_entry_with_max_values() {
        let entry = Entry {
            id: i64::MAX,
            ledger_id: i64::MAX,
            transaction_id: i64::MAX,
            account_id: i64::MAX,
            currency: "X".repeat(100), // Very long currency code
            debit: Some(Decimal::from_str("79228162514264337593543950335").unwrap()), // Near max decimal
            credit: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: Entry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, i64::MAX);
        assert_eq!(deserialized.ledger_id, i64::MAX);
        assert_eq!(deserialized.transaction_id, i64::MAX);
        assert_eq!(deserialized.account_id, i64::MAX);
        assert_eq!(deserialized.currency.len(), 100);
        assert!(deserialized.debit.is_some());
    }
}
