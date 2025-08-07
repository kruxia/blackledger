//! Transaction validation service
//!
//! Provides comprehensive validation for accounting transactions including:
//! - Entry structure validation
//! - Double-entry balance checking
//! - Currency existence verification
//! - Account existence and ledger membership
//! - Optimistic locking via account versions

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

use crate::db::queries::account::get_account_by_id;
use crate::db::queries::currency::get_currency_by_code;
use crate::error::{ApiError, ApiResult};
use crate::models::transaction::{CreateEntry, CreateTransaction};

/// Validates all aspects of a transaction before posting
///
/// # Validation Steps
///
/// 1. Entry validation (amounts, structure)
/// 2. Double-entry balance per currency
/// 3. Currency code existence
/// 4. Account existence and ledger membership
///
/// # Errors
///
/// Returns `ApiError::Validation` with details about any validation failure
pub async fn validate_transaction(pool: &PgPool, transaction: &CreateTransaction) -> ApiResult<()> {
    validate_entries(&transaction.entries)?;

    validate_double_entry_balance(&transaction.entries)?;

    validate_currencies_exist(pool, &transaction.entries).await?;

    validate_accounts_exist(pool, transaction.ledger_id, &transaction.entries).await?;

    Ok(())
}

/// Validates the structure and amounts of transaction entries
///
/// # Rules
///
/// - Must have at least one entry
/// - Each entry must have either debit OR credit (not both)
/// - Amounts must be positive
/// - Currency codes must be exactly 3 characters
pub fn validate_entries(entries: &[CreateEntry]) -> ApiResult<()> {
    if entries.is_empty() {
        return Err(ApiError::Validation(
            "Transaction must have at least one entry".to_string(),
        ));
    }

    for (idx, entry) in entries.iter().enumerate() {
        match (entry.debit, entry.credit) {
            (Some(debit), None) => {
                if debit <= Decimal::ZERO {
                    return Err(ApiError::Validation(format!(
                        "Entry {} debit amount must be positive",
                        idx
                    )));
                }
            }
            (None, Some(credit)) => {
                if credit <= Decimal::ZERO {
                    return Err(ApiError::Validation(format!(
                        "Entry {} credit amount must be positive",
                        idx
                    )));
                }
            }
            (Some(_), Some(_)) => {
                return Err(ApiError::Validation(format!(
                    "Entry {} cannot have both debit and credit",
                    idx
                )));
            }
            (None, None) => {
                return Err(ApiError::Validation(format!(
                    "Entry {} must have either debit or credit",
                    idx
                )));
            }
        }

        if entry.currency_code.is_empty() {
            return Err(ApiError::Validation(format!(
                "Entry {} must have a currency code",
                idx
            )));
        }

        if entry.currency_code.len() != 3 {
            return Err(ApiError::Validation(format!(
                "Entry {} currency code must be exactly 3 characters",
                idx
            )));
        }
    }

    Ok(())
}

/// Ensures the transaction balances for each currency
///
/// For a valid double-entry transaction:
/// - Sum of debits must equal sum of credits for each currency
/// - Transaction must have non-zero amounts
pub fn validate_double_entry_balance(entries: &[CreateEntry]) -> ApiResult<()> {
    let mut balances: HashMap<String, Decimal> = HashMap::new();

    for entry in entries {
        let balance = balances
            .entry(entry.currency_code.clone())
            .or_insert(Decimal::ZERO);

        if let Some(debit) = entry.debit {
            *balance += debit;
        }

        if let Some(credit) = entry.credit {
            *balance -= credit;
        }
    }

    for (currency, balance) in balances {
        if balance != Decimal::ZERO {
            return Err(ApiError::Validation(format!(
                "Transaction does not balance for currency {}: {}",
                currency, balance
            )));
        }
    }

    let total_debits: Decimal = entries.iter().filter_map(|e| e.debit).sum();

    let total_credits: Decimal = entries.iter().filter_map(|e| e.credit).sum();

    if total_debits == Decimal::ZERO && total_credits == Decimal::ZERO {
        return Err(ApiError::Validation(
            "Transaction has no amounts".to_string(),
        ));
    }

    Ok(())
}

pub async fn validate_currencies_exist(pool: &PgPool, entries: &[CreateEntry]) -> ApiResult<()> {
    let unique_currencies: HashSet<&str> =
        entries.iter().map(|e| e.currency_code.as_str()).collect();

    for currency_code in unique_currencies {
        get_currency_by_code(pool, currency_code)
            .await
            .map_err(|_| {
                ApiError::Validation(format!("Currency {} does not exist", currency_code))
            })?;
    }

    Ok(())
}

pub async fn validate_accounts_exist(
    pool: &PgPool,
    ledger_id: i64,
    entries: &[CreateEntry],
) -> ApiResult<()> {
    let unique_accounts: HashSet<i64> = entries.iter().map(|e| e.account_id).collect();

    for account_id in unique_accounts {
        let account = get_account_by_id(pool, account_id)
            .await
            .map_err(|_| ApiError::Validation(format!("Account {} does not exist", account_id)))?;

        if account.ledger_id != ledger_id {
            return Err(ApiError::Validation(format!(
                "Account {} does not belong to ledger {}",
                account_id, ledger_id
            )));
        }
    }

    Ok(())
}

/// Validates account versions for optimistic locking
///
/// If an entry specifies an account_version, this ensures it matches
/// the current version in the database, preventing concurrent modifications.
///
/// Uses SELECT FOR UPDATE to lock the account row during validation.
pub async fn validate_account_versions(
    _pool: &PgPool,
    entries: &[CreateEntry],
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> ApiResult<()> {
    for entry in entries {
        if let Some(expected_version) = entry.account_version {
            let row = sqlx::query(r#"SELECT version FROM account WHERE id = $1 FOR UPDATE"#)
                .bind(entry.account_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(|_| {
                    ApiError::NotFound(format!("Account {} not found", entry.account_id))
                })?;

            let current_version: Option<i64> = row.get("version");

            if current_version != Some(expected_version) {
                return Err(ApiError::OptimisticLockError);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_validate_entries_empty() {
        let entries = vec![];
        let result = validate_entries(&entries);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one entry")
        );
    }

    #[test]
    fn test_validate_entries_negative_amounts() {
        let entries = vec![CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(-100)),
            credit: None,
            account_version: None,
        }];
        let result = validate_entries(&entries);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_validate_entries_both_debit_and_credit() {
        let entries = vec![CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: Some(dec!(100)),
            credit: Some(dec!(100)),
            account_version: None,
        }];
        let result = validate_entries(&entries);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot have both"));
    }

    #[test]
    fn test_validate_entries_neither_debit_nor_credit() {
        let entries = vec![CreateEntry {
            account_id: 1,
            currency_code: "USD".to_string(),
            debit: None,
            credit: None,
            account_version: None,
        }];
        let result = validate_entries(&entries);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must have either"));
    }

    #[test]
    fn test_validate_entries_invalid_currency_code() {
        let entries = vec![CreateEntry {
            account_id: 1,
            currency_code: "US".to_string(),
            debit: Some(dec!(100)),
            credit: None,
            account_version: None,
        }];
        let result = validate_entries(&entries);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exactly 3 characters")
        );
    }

    #[test]
    fn test_validate_double_entry_balance_valid() {
        let entries = vec![
            CreateEntry {
                account_id: 1,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 2,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100)),
                account_version: None,
            },
        ];
        let result = validate_double_entry_balance(&entries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_double_entry_balance_unbalanced() {
        let entries = vec![
            CreateEntry {
                account_id: 1,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 2,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(50)),
                account_version: None,
            },
        ];
        let result = validate_double_entry_balance(&entries);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not balance"));
    }

    #[test]
    fn test_validate_double_entry_balance_multi_currency() {
        let entries = vec![
            CreateEntry {
                account_id: 1,
                currency_code: "USD".to_string(),
                debit: Some(dec!(100)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 2,
                currency_code: "USD".to_string(),
                debit: None,
                credit: Some(dec!(100)),
                account_version: None,
            },
            CreateEntry {
                account_id: 3,
                currency_code: "EUR".to_string(),
                debit: Some(dec!(85)),
                credit: None,
                account_version: None,
            },
            CreateEntry {
                account_id: 4,
                currency_code: "EUR".to_string(),
                debit: None,
                credit: Some(dec!(85)),
                account_version: None,
            },
        ];
        let result = validate_double_entry_balance(&entries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_double_entry_balance_zero_amounts() {
        let entries = vec![];
        let result = validate_double_entry_balance(&entries);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no amounts"));
    }
}
