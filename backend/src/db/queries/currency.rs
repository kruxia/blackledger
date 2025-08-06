use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::currency::Currency;

pub async fn create_currency(
    pool: &PgPool,
    code: &str,
    name: Option<&str>,
    minor_units: Option<i32>,
) -> ApiResult<Currency> {
    let id = Uuid::new_v4();
    let currency = sqlx::query_as::<_, Currency>(
        r#"
        INSERT INTO currency (id, code, name, minor_units)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (code) DO UPDATE
        SET name = EXCLUDED.name,
            minor_units = EXCLUDED.minor_units,
            updated = CURRENT_TIMESTAMP
        RETURNING *
        "#
    )
    .bind(id)
    .bind(code)
    .bind(name)
    .bind(minor_units)
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