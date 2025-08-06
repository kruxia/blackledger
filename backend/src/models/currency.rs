use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Currency {
    pub id: Uuid,
    pub code: String,
    pub name: Option<String>,
    pub minor_units: Option<i32>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Currency {
    pub fn is_valid_code(code: &str) -> bool {
        code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_currency_code() {
        assert!(Currency::is_valid_code("USD"));
        assert!(Currency::is_valid_code("EUR"));
        assert!(Currency::is_valid_code("GBP"));
    }

    #[test]
    fn test_invalid_currency_code() {
        assert!(!Currency::is_valid_code("US"));
        assert!(!Currency::is_valid_code("USDD"));
        assert!(!Currency::is_valid_code("usd"));
        assert!(!Currency::is_valid_code("US1"));
    }
}