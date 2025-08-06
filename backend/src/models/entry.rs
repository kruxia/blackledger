use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub account_id: Uuid,
    pub currency_code: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub dr: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub cr: Option<Decimal>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created: DateTime<Utc>,
}

impl Entry {
    pub fn amount(&self) -> Decimal {
        match (self.dr, self.cr) {
            (Some(dr), None) => dr,
            (None, Some(cr)) => -cr,
            _ => Decimal::ZERO,
        }
    }

    pub fn is_valid(&self) -> bool {
        match (self.dr, self.cr) {
            (Some(dr), None) => dr > Decimal::ZERO,
            (None, Some(cr)) => cr > Decimal::ZERO,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_entry_amount() {
        let mut entry = Entry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            currency_code: "USD".to_string(),
            dr: Some(dec!(100.00)),
            cr: None,
            description: None,
            metadata: None,
            created: Utc::now(),
        };

        assert_eq!(entry.amount(), dec!(100.00));

        entry.dr = None;
        entry.cr = Some(dec!(50.00));
        assert_eq!(entry.amount(), dec!(-50.00));

        entry.dr = None;
        entry.cr = None;
        assert_eq!(entry.amount(), dec!(0));
    }

    #[test]
    fn test_entry_is_valid() {
        let mut entry = Entry {
            id: Uuid::new_v4(),
            transaction_id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            currency_code: "USD".to_string(),
            dr: Some(dec!(100.00)),
            cr: None,
            description: None,
            metadata: None,
            created: Utc::now(),
        };

        assert!(entry.is_valid());

        entry.dr = None;
        entry.cr = Some(dec!(50.00));
        assert!(entry.is_valid());

        entry.dr = Some(dec!(100.00));
        entry.cr = Some(dec!(50.00));
        assert!(!entry.is_valid());

        entry.dr = None;
        entry.cr = None;
        assert!(!entry.is_valid());

        entry.dr = Some(dec!(0));
        entry.cr = None;
        assert!(!entry.is_valid());
    }
}