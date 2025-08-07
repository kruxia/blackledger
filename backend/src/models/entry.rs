use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Entry {
    pub id: i64,
    pub ledger_id: i64,
    pub transaction_id: i64,
    pub account_id: i64,
    #[sqlx(rename = "curr")]
    pub currency_code: String,
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

    #[test]
    fn test_entry_amount() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency_code: "USD".to_string(),
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
    }

    #[test]
    fn test_entry_is_valid() {
        let mut entry = Entry {
            id: 1,
            ledger_id: 1,
            transaction_id: 1,
            account_id: 1,
            currency_code: "USD".to_string(),
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
    }
}
