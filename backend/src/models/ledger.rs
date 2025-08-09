use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ledger {
    pub id: i64,
    pub name: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLedger {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLedger {
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_ledger_serialization() {
        let ledger = Ledger {
            id: 42,
            name: "General Ledger".to_string(),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&ledger).unwrap();
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"name\":\"General Ledger\""));
        assert!(json.contains("\"created\":\"2024-01-15T10:30:45Z\""));

        // Deserialize from JSON
        let deserialized: Ledger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 42);
        assert_eq!(deserialized.name, "General Ledger");
        assert_eq!(deserialized.created, ledger.created);
    }

    #[test]
    fn test_ledger_deserialization_edge_cases() {
        // Valid JSON with different id types
        let json_i64_max = format!(
            r#"{{"id":{},"name":"MaxID","created":"2024-01-01T00:00:00Z"}}"#,
            i64::MAX
        );
        let ledger: Ledger = serde_json::from_str(&json_i64_max).unwrap();
        assert_eq!(ledger.id, i64::MAX);

        // Test with negative id
        let json_negative = r#"{"id":-1,"name":"Negative","created":"2024-01-01T00:00:00Z"}"#;
        let ledger: Ledger = serde_json::from_str(json_negative).unwrap();
        assert_eq!(ledger.id, -1);

        // Test with zero id
        let json_zero = r#"{"id":0,"name":"Zero","created":"2024-01-01T00:00:00Z"}"#;
        let ledger: Ledger = serde_json::from_str(json_zero).unwrap();
        assert_eq!(ledger.id, 0);

        // Test with empty name
        let json_empty_name = r#"{"id":1,"name":"","created":"2024-01-01T00:00:00Z"}"#;
        let ledger: Ledger = serde_json::from_str(json_empty_name).unwrap();
        assert_eq!(ledger.name, "");

        // Test with special characters in name
        let json_special =
            r#"{"id":1,"name":"Test & Co. \"Ledger\" <2024>","created":"2024-01-01T00:00:00Z"}"#;
        let ledger: Ledger = serde_json::from_str(json_special).unwrap();
        assert_eq!(ledger.name, "Test & Co. \"Ledger\" <2024>");

        // Test with Unicode in name
        let json_unicode = r#"{"id":1,"name":"会計帳簿 📊","created":"2024-01-01T00:00:00Z"}"#;
        let ledger: Ledger = serde_json::from_str(json_unicode).unwrap();
        assert_eq!(ledger.name, "会計帳簿 📊");
    }

    #[test]
    fn test_ledger_deserialization_errors() {
        // Missing id field
        let missing_id = r#"{"name":"Test","created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Ledger>(missing_id).is_err());

        // Missing name field
        let missing_name = r#"{"id":1,"created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Ledger>(missing_name).is_err());

        // Missing created field
        let missing_created = r#"{"id":1,"name":"Test"}"#;
        assert!(serde_json::from_str::<Ledger>(missing_created).is_err());

        // Invalid id type (string instead of number)
        let invalid_id = r#"{"id":"not_a_number","name":"Test","created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Ledger>(invalid_id).is_err());

        // Invalid date format
        let invalid_date = r#"{"id":1,"name":"Test","created":"invalid-date"}"#;
        assert!(serde_json::from_str::<Ledger>(invalid_date).is_err());

        // Null values
        let null_id = r#"{"id":null,"name":"Test","created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Ledger>(null_id).is_err());

        let null_name = r#"{"id":1,"name":null,"created":"2024-01-01T00:00:00Z"}"#;
        assert!(serde_json::from_str::<Ledger>(null_name).is_err());
    }

    #[test]
    fn test_create_ledger_serialization() {
        let create_ledger = CreateLedger {
            name: "New Ledger".to_string(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&create_ledger).unwrap();
        assert_eq!(json, r#"{"name":"New Ledger"}"#);

        // Deserialize from JSON
        let deserialized: CreateLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "New Ledger");
    }

    #[test]
    fn test_create_ledger_edge_cases() {
        // Empty name
        let json_empty = r#"{"name":""}"#;
        let create: CreateLedger = serde_json::from_str(json_empty).unwrap();
        assert_eq!(create.name, "");

        // Very long name
        let long_name = "A".repeat(1000);
        let create_long = CreateLedger {
            name: long_name.clone(),
        };
        let json = serde_json::to_string(&create_long).unwrap();
        let deserialized: CreateLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, long_name);

        // Special characters and Unicode
        let special_name = "Test 日本語 العربية 🎯 \n\t\r";
        let create_special = CreateLedger {
            name: special_name.to_string(),
        };
        let json = serde_json::to_string(&create_special).unwrap();
        let deserialized: CreateLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, special_name);
    }

    #[test]
    fn test_create_ledger_errors() {
        // Missing name field
        let missing_name = r#"{}"#;
        assert!(serde_json::from_str::<CreateLedger>(missing_name).is_err());

        // Null name
        let null_name = r#"{"name":null}"#;
        assert!(serde_json::from_str::<CreateLedger>(null_name).is_err());

        // Wrong type for name
        let wrong_type = r#"{"name":123}"#;
        assert!(serde_json::from_str::<CreateLedger>(wrong_type).is_err());

        // Extra fields (should still work due to serde's default behavior)
        let extra_fields = r#"{"name":"Test","extra":"field"}"#;
        let result: CreateLedger = serde_json::from_str(extra_fields).unwrap();
        assert_eq!(result.name, "Test");
    }

    #[test]
    fn test_update_ledger_serialization() {
        // With Some value
        let update_some = UpdateLedger {
            name: Some("Updated Name".to_string()),
        };
        let json_some = serde_json::to_string(&update_some).unwrap();
        assert_eq!(json_some, r#"{"name":"Updated Name"}"#);

        // With None value
        let update_none = UpdateLedger { name: None };
        let json_none = serde_json::to_string(&update_none).unwrap();
        assert_eq!(json_none, r#"{"name":null}"#);

        // Deserialize Some
        let deserialized_some: UpdateLedger = serde_json::from_str(&json_some).unwrap();
        assert_eq!(deserialized_some.name, Some("Updated Name".to_string()));

        // Deserialize None
        let deserialized_none: UpdateLedger = serde_json::from_str(&json_none).unwrap();
        assert_eq!(deserialized_none.name, None);
    }

    #[test]
    fn test_update_ledger_edge_cases() {
        // Missing field (should default to None)
        let missing_field = r#"{}"#;
        let update: UpdateLedger = serde_json::from_str(missing_field).unwrap();
        assert_eq!(update.name, None);

        // Explicit null
        let explicit_null = r#"{"name":null}"#;
        let update: UpdateLedger = serde_json::from_str(explicit_null).unwrap();
        assert_eq!(update.name, None);

        // Empty string
        let empty_string = r#"{"name":""}"#;
        let update: UpdateLedger = serde_json::from_str(empty_string).unwrap();
        assert_eq!(update.name, Some("".to_string()));

        // Unicode and special characters
        let special = r#"{"name":"Updated 更新 🔄"}"#;
        let update: UpdateLedger = serde_json::from_str(special).unwrap();
        assert_eq!(update.name, Some("Updated 更新 🔄".to_string()));
    }

    #[test]
    fn test_ledger_clone() {
        let original = Ledger {
            id: 100,
            name: "Original Ledger".to_string(),
            created: Utc::now(),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.created, cloned.created);

        // Ensure they are separate instances
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_create_ledger_clone() {
        let original = CreateLedger {
            name: "Original".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_update_ledger_clone() {
        let original = UpdateLedger {
            name: Some("Update".to_string()),
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_ne!(&original as *const _, &cloned as *const _);
    }

    #[test]
    fn test_ledger_debug() {
        let ledger = Ledger {
            id: 42,
            name: "Test Ledger".to_string(),
            created: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
        };

        let debug_str = format!("{:?}", ledger);
        assert!(debug_str.contains("Ledger"));
        assert!(debug_str.contains("id: 42"));
        assert!(debug_str.contains("name:"));
        assert!(debug_str.contains("Test Ledger"));
        assert!(debug_str.contains("created:"));
    }

    #[test]
    fn test_create_ledger_debug() {
        let create = CreateLedger {
            name: "Debug Test".to_string(),
        };

        let debug_str = format!("{:?}", create);
        assert!(debug_str.contains("CreateLedger"));
        assert!(debug_str.contains("name:"));
        assert!(debug_str.contains("Debug Test"));
    }

    #[test]
    fn test_update_ledger_debug() {
        let update = UpdateLedger {
            name: Some("Debug Update".to_string()),
        };

        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("UpdateLedger"));
        assert!(debug_str.contains("name:"));
        assert!(debug_str.contains("Some"));
        assert!(debug_str.contains("Debug Update"));
    }
}
