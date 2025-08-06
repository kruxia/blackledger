use sqlx::PgPool;

use crate::error::{ApiError, ApiResult};
use crate::models::currency::Currency;

pub async fn create_currency(
    pool: &PgPool,
    code: &str,
) -> ApiResult<Currency> {
    // Try to insert, and if it already exists, fetch it
    let record = sqlx::query!(
        r#"
        INSERT INTO currency (code)
        VALUES ($1)
        ON CONFLICT (code) DO UPDATE 
        SET code = EXCLUDED.code  -- No-op update to trigger RETURNING
        RETURNING code, created
        "#,
        code
    )
    .fetch_one(pool)
    .await?;

    Ok(Currency {
        code: record.code,
        created: record.created,
    })
}

pub async fn get_currency_by_code(pool: &PgPool, code: &str) -> ApiResult<Currency> {
    let record = sqlx::query!(
        r#"SELECT code, created FROM currency WHERE code = $1"#,
        code
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Currency {} not found", code)),
        _ => ApiError::Database(e),
    })?;

    Ok(Currency {
        code: record.code,
        created: record.created,
    })
}

pub async fn list_currencies(pool: &PgPool) -> ApiResult<Vec<Currency>> {
    let records = sqlx::query!(
        r#"SELECT code, created FROM currency ORDER BY code"#
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Currency {
            code: r.code,
            created: r.created,
        })
        .collect())
}