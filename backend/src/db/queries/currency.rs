use sqlx::PgPool;

use crate::error::{ApiError, ApiResult};
use crate::models::currency::Currency;

pub async fn create_currency(
    pool: &PgPool,
    code: &str,
) -> ApiResult<Currency> {
    // Try to insert, and if it already exists, fetch it
    let currency = sqlx::query_as::<_, Currency>(
        r#"
        INSERT INTO currency (code)
        VALUES ($1)
        ON CONFLICT (code) DO UPDATE 
        SET code = EXCLUDED.code  -- No-op update to trigger RETURNING
        RETURNING *
        "#
    )
    .bind(code)
    .fetch_one(pool)
    .await?;

    Ok(currency)
}

pub async fn get_currency_by_code(pool: &PgPool, code: &str) -> ApiResult<Currency> {
    let currency = sqlx::query_as::<_, Currency>(
        r#"SELECT * FROM currency WHERE code = $1"#
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Currency {} not found", code)),
        _ => ApiError::Database(e),
    })?;

    Ok(currency)
}

pub async fn list_currencies(pool: &PgPool) -> ApiResult<Vec<Currency>> {
    let currencies = sqlx::query_as::<_, Currency>(
        r#"SELECT * FROM currency ORDER BY code"#
    )
    .fetch_all(pool)
    .await?;

    Ok(currencies)
}