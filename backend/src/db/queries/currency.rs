use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::api::search::CurrencySearchParams;
use crate::error::{ApiError, ApiResult};
use crate::models::currency::Currency;

pub async fn create_currency(pool: &PgPool, code: &str) -> ApiResult<Currency> {
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
    let records = sqlx::query!(r#"SELECT code, created FROM currency ORDER BY code"#)
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

pub async fn search_currencies(
    pool: &PgPool,
    params: &CurrencySearchParams,
) -> ApiResult<Vec<Currency>> {
    let limit = params.base.get_limit() as i64;
    let offset = params.base.get_offset() as i64;

    // Use QueryBuilder for dynamic SQL generation
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT code, created FROM currency WHERE 1=1");

    // Handle comma-delimited regex patterns for currency codes
    if let Some(ref code_patterns) = params.code {
        let patterns: Vec<&str> = code_patterns.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query_builder.push(" AND (");
            let mut first = true;
            for pattern in patterns {
                if !first {
                    query_builder.push(" OR ");
                }
                query_builder.push("code ~* ");
                query_builder.push_bind(pattern);
                first = false;
            }
            query_builder.push(")");
        }
    }

    // Add sorting based on SearchParams with whitelist validation
    const ALLOWED_COLUMNS: &[&str] = &["code", "created"];
    if let Some(order_clause) = params.base.parse_order_by(ALLOWED_COLUMNS) {
        query_builder.push(" ORDER BY ");
        query_builder.push(order_clause);
    } else {
        // Default ordering
        query_builder.push(" ORDER BY code ASC");
    }

    // Add pagination with parameter binding
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute the query
    let query = query_builder.build();
    let rows = query.fetch_all(pool).await?;

    let mut currencies = Vec::new();
    for row in rows {
        let currency = Currency {
            code: row.try_get("code")?,
            created: row.try_get("created")?,
        };
        currencies.push(currency);
    }

    Ok(currencies)
}
