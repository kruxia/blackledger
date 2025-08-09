use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// The normal balance side of an account (Debit or Credit)
///
/// This determines whether increases to the account are recorded
/// as debits or credits:
/// - Assets and Expenses normally have Debit balances
/// - Liabilities, Equity, and Revenue normally have Credit balances
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum NormalBalance {
    #[serde(rename = "DR")]
    #[sqlx(rename = "DR")]
    Debit,
    #[serde(rename = "CR")]
    #[sqlx(rename = "CR")]
    Credit,
}

/// An account in the chart of accounts
///
/// Accounts are organized hierarchically and track balances for specific
/// categories of assets, liabilities, equity, revenue, or expenses.
///
/// # Versioning
///
/// The `version` field contains the ID of the last entry posted to this account,
/// enabling optimistic locking for concurrent transaction posting.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: i64,
    pub ledger_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub number: Option<i16>,
    pub created: DateTime<Utc>,
    pub normal: NormalBalance,
    pub version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub ledger_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub number: Option<i16>,
    pub normal: NormalBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccount {
    pub name: Option<String>,
}

/// Account with balances in multiple currencies
///
/// Groups all currency balances for a single account,
/// matching the Python model's structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalances {
    pub account: Account,
    pub balances: std::collections::HashMap<String, Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;
    use std::collections::HashMap;
    use std::str::FromStr;

    #[test]
    fn test_normal_balance_serialization() {
        // Test Debit serialization
        let debit = NormalBalance::Debit;
        let debit_json = serde_json::to_string(&debit).unwrap();
        assert_eq!(debit_json, r#""DR""#);

        // Test Credit serialization
        let credit = NormalBalance::Credit;
        let credit_json = serde_json::to_string(&credit).unwrap();
        assert_eq!(credit_json, r#""CR""#);

        // Test deserialization
        let debit_parsed: NormalBalance = serde_json::from_str(r#""DR""#).unwrap();
        assert!(matches!(debit_parsed, NormalBalance::Debit));

        let credit_parsed: NormalBalance = serde_json::from_str(r#""CR""#).unwrap();
        assert!(matches!(credit_parsed, NormalBalance::Credit));
    }

    #[test]
    fn test_normal_balance_deserialization_errors() {
        // Invalid values
        assert!(serde_json::from_str::<NormalBalance>(r#""DEBIT""#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#""CREDIT""#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#""dr""#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#""cr""#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#""DB""#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#"null"#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#"123"#).is_err());
        assert!(serde_json::from_str::<NormalBalance>(r#"true"#).is_err());
    }

    #[test]
    fn test_account_serialization() {
        let account = Account {
            id: 100,
            ledger_id: 1,
            parent_id: Some(50),
            name: "Cash".to_string(),
            number: Some(1000),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
            normal: NormalBalance::Debit,
            version: Some(42),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains(r#""id":100"#));
        assert!(json.contains(r#""ledger_id":1"#));
        assert!(json.contains(r#""parent_id":50"#));
        assert!(json.contains(r#""name":"Cash""#));
        assert!(json.contains(r#""number":1000"#));
        assert!(json.contains(r#""normal":"DR""#));
        assert!(json.contains(r#""version":42"#));

        // Deserialize from JSON
        let deserialized: Account = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 100);
        assert_eq!(deserialized.ledger_id, 1);
        assert_eq!(deserialized.parent_id, Some(50));
        assert_eq!(deserialized.name, "Cash");
        assert_eq!(deserialized.number, Some(1000));
        assert!(matches!(deserialized.normal, NormalBalance::Debit));
        assert_eq!(deserialized.version, Some(42));
    }

    #[test]
    fn test_account_with_nulls() {
        let account = Account {
            id: 1,
            ledger_id: 1,
            parent_id: None,
            name: "Root".to_string(),
            number: None,
            created: Utc::now(),
            normal: NormalBalance::Credit,
            version: None,
        };

        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains(r#""parent_id":null"#));
        assert!(json.contains(r#""number":null"#));
        assert!(json.contains(r#""version":null"#));

        let deserialized: Account = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.parent_id, None);
        assert_eq!(deserialized.number, None);
        assert_eq!(deserialized.version, None);
    }

    #[test]
    fn test_account_edge_cases() {
        // Test with i64::MAX for IDs
        let json_max = format!(
            r#"{{"id":{},"ledger_id":{},"parent_id":{},"name":"Max","number":32767,"created":"2024-01-01T00:00:00Z","normal":"DR","version":{}}}"#,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX
        );
        let account: Account = serde_json::from_str(&json_max).unwrap();
        assert_eq!(account.id, i64::MAX);
        assert_eq!(account.version, Some(i64::MAX));

        // Test with i16::MAX for number
        assert_eq!(account.number, Some(i16::MAX));

        // Test with negative number
        let json_negative = r#"{"id":1,"ledger_id":1,"parent_id":null,"name":"Test","number":-100,"created":"2024-01-01T00:00:00Z","normal":"CR","version":null}"#;
        let account: Account = serde_json::from_str(json_negative).unwrap();
        assert_eq!(account.number, Some(-100));

        // Test with empty name
        let json_empty = r#"{"id":1,"ledger_id":1,"parent_id":null,"name":"","number":null,"created":"2024-01-01T00:00:00Z","normal":"DR","version":null}"#;
        let account: Account = serde_json::from_str(json_empty).unwrap();
        assert_eq!(account.name, "");

        // Test with Unicode name
        let json_unicode = r#"{"id":1,"ledger_id":1,"parent_id":null,"name":"資産 💰","number":null,"created":"2024-01-01T00:00:00Z","normal":"DR","version":null}"#;
        let account: Account = serde_json::from_str(json_unicode).unwrap();
        assert_eq!(account.name, "資産 💰");
    }

    #[test]
    fn test_account_deserialization_errors() {
        // Missing required fields
        let missing_id =
            r#"{"ledger_id":1,"name":"Test","created":"2024-01-01T00:00:00Z","normal":"DR"}"#;
        assert!(serde_json::from_str::<Account>(missing_id).is_err());

        let missing_normal =
            r#"{"id":1,"ledger_id":1,"name":"Test","created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Account>(missing_normal).is_err());

        // Invalid types
        let invalid_id = r#"{"id":"not_a_number","ledger_id":1,"name":"Test","created":"2024-01-01T00:00:00Z","normal":"DR"}"#;
        assert!(serde_json::from_str::<Account>(invalid_id).is_err());

        let invalid_normal = r#"{"id":1,"ledger_id":1,"name":"Test","created":"2024-01-01T00:00:00Z","normal":"INVALID"}"#;
        assert!(serde_json::from_str::<Account>(invalid_normal).is_err());

        // Number out of i16 range
        let number_overflow = r#"{"id":1,"ledger_id":1,"name":"Test","number":100000,"created":"2024-01-01T00:00:00Z","normal":"DR"}"#;
        assert!(serde_json::from_str::<Account>(number_overflow).is_err());
    }

    #[test]
    fn test_create_account_serialization() {
        let create = CreateAccount {
            ledger_id: 1,
            parent_id: Some(10),
            name: "Accounts Receivable".to_string(),
            number: Some(1200),
            normal: NormalBalance::Debit,
        };

        let json = serde_json::to_string(&create).unwrap();
        assert!(json.contains(r#""ledger_id":1"#));
        assert!(json.contains(r#""parent_id":10"#));
        assert!(json.contains(r#""name":"Accounts Receivable""#));
        assert!(json.contains(r#""number":1200"#));
        assert!(json.contains(r#""normal":"DR""#));

        let deserialized: CreateAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ledger_id, 1);
        assert_eq!(deserialized.parent_id, Some(10));
        assert_eq!(deserialized.name, "Accounts Receivable");
        assert_eq!(deserialized.number, Some(1200));
        assert!(matches!(deserialized.normal, NormalBalance::Debit));
    }

    #[test]
    fn test_create_account_without_optionals() {
        let create = CreateAccount {
            ledger_id: 1,
            parent_id: None,
            name: "Assets".to_string(),
            number: None,
            normal: NormalBalance::Debit,
        };

        let json = serde_json::to_string(&create).unwrap();
        assert!(json.contains(r#""parent_id":null"#));
        assert!(json.contains(r#""number":null"#));

        let deserialized: CreateAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.parent_id, None);
        assert_eq!(deserialized.number, None);
    }

    #[test]
    fn test_update_account_serialization() {
        // With Some value
        let update_some = UpdateAccount {
            name: Some("Updated Account Name".to_string()),
        };
        let json_some = serde_json::to_string(&update_some).unwrap();
        assert_eq!(json_some, r#"{"name":"Updated Account Name"}"#);

        // With None value
        let update_none = UpdateAccount { name: None };
        let json_none = serde_json::to_string(&update_none).unwrap();
        assert_eq!(json_none, r#"{"name":null}"#);

        // Deserialize
        let deserialized_some: UpdateAccount = serde_json::from_str(&json_some).unwrap();
        assert_eq!(
            deserialized_some.name,
            Some("Updated Account Name".to_string())
        );

        let deserialized_none: UpdateAccount = serde_json::from_str(&json_none).unwrap();
        assert_eq!(deserialized_none.name, None);
    }

    #[test]
    fn test_account_balances_serialization() {
        let account = Account {
            id: 1,
            ledger_id: 1,
            parent_id: None,
            name: "Cash".to_string(),
            number: Some(1000),
            created: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            normal: NormalBalance::Debit,
            version: Some(10),
        };

        let mut balances = HashMap::new();
        balances.insert("USD".to_string(), dec!(1000.50));
        balances.insert("EUR".to_string(), dec!(500.25));
        balances.insert("JPY".to_string(), dec!(100000));

        let account_balances = AccountBalances {
            account: account.clone(),
            balances: balances.clone(),
        };

        let json = serde_json::to_string(&account_balances).unwrap();
        assert!(json.contains(r#""account":"#));
        assert!(json.contains(r#""balances":"#));
        assert!(json.contains(r#""USD":"1000.50""#) || json.contains(r#""USD":"1000.5""#));
        assert!(json.contains(r#""EUR":"500.25""#));
        assert!(json.contains(r#""JPY":"100000""#));

        let deserialized: AccountBalances = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.account.id, 1);
        assert_eq!(deserialized.balances.get("USD"), Some(&dec!(1000.50)));
        assert_eq!(deserialized.balances.get("EUR"), Some(&dec!(500.25)));
        assert_eq!(deserialized.balances.get("JPY"), Some(&dec!(100000)));
    }

    #[test]
    fn test_account_balances_edge_cases() {
        let account = Account {
            id: 1,
            ledger_id: 1,
            parent_id: None,
            name: "Test".to_string(),
            number: None,
            created: Utc::now(),
            normal: NormalBalance::Credit,
            version: None,
        };

        // Empty balances
        let empty_balances = AccountBalances {
            account: account.clone(),
            balances: HashMap::new(),
        };
        let json = serde_json::to_string(&empty_balances).unwrap();
        assert!(json.contains(r#""balances":{}"#));

        // Large decimal values
        let mut large_balances = HashMap::new();
        large_balances.insert(
            "USD".to_string(),
            Decimal::from_str("999999999999999999.999999999").unwrap(),
        );
        large_balances.insert(
            "NEGATIVE".to_string(),
            Decimal::from_str("-999999999999999999.999999999").unwrap(),
        );
        large_balances.insert("ZERO".to_string(), dec!(0));
        large_balances.insert(
            "SMALL".to_string(),
            Decimal::from_str("0.000000001").unwrap(),
        );

        let large_account_balances = AccountBalances {
            account: account.clone(),
            balances: large_balances,
        };
        let json = serde_json::to_string(&large_account_balances).unwrap();
        let deserialized: AccountBalances = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.balances.get("USD"),
            Some(&Decimal::from_str("999999999999999999.999999999").unwrap())
        );
        assert_eq!(
            deserialized.balances.get("NEGATIVE"),
            Some(&Decimal::from_str("-999999999999999999.999999999").unwrap())
        );
        assert_eq!(deserialized.balances.get("ZERO"), Some(&dec!(0)));
    }

    #[test]
    fn test_account_clone() {
        let original = Account {
            id: 1,
            ledger_id: 1,
            parent_id: Some(10),
            name: "Original".to_string(),
            number: Some(100),
            created: Utc::now(),
            normal: NormalBalance::Debit,
            version: Some(5),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.ledger_id, cloned.ledger_id);
        assert_eq!(original.parent_id, cloned.parent_id);
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.number, cloned.number);
        assert_eq!(original.created, cloned.created);
        assert!(matches!(original.normal, NormalBalance::Debit));
        assert!(matches!(cloned.normal, NormalBalance::Debit));
        assert_eq!(original.version, cloned.version);

        // Ensure separate instances
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_normal_balance_copy() {
        let original = NormalBalance::Debit;
        let copied = original; // Copy trait
        assert!(matches!(original, NormalBalance::Debit));
        assert!(matches!(copied, NormalBalance::Debit));
    }

    #[test]
    fn test_account_debug() {
        let account = Account {
            id: 42,
            ledger_id: 1,
            parent_id: Some(10),
            name: "Test Account".to_string(),
            number: Some(1000),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
            normal: NormalBalance::Credit,
            version: Some(100),
        };

        let debug_str = format!("{:?}", account);
        assert!(debug_str.contains("Account"));
        assert!(debug_str.contains("id: 42"));
        assert!(debug_str.contains("ledger_id: 1"));
        assert!(debug_str.contains("parent_id: Some(10)"));
        assert!(debug_str.contains("Test Account"));
        assert!(debug_str.contains("number: Some(1000)"));
        assert!(debug_str.contains("Credit"));
        assert!(debug_str.contains("version: Some(100)"));
    }

    #[test]
    fn test_normal_balance_debug() {
        let debit = NormalBalance::Debit;
        let credit = NormalBalance::Credit;

        assert_eq!(format!("{:?}", debit), "Debit");
        assert_eq!(format!("{:?}", credit), "Credit");
    }
}
