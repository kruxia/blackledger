use sqlx::PgPool;

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
    let mut query = String::from("SELECT code, created FROM currency WHERE 1=1");
    let mut bindings = vec![];

    // Handle comma-delimited regex patterns for currency codes
    if let Some(code_filter) = &params.code {
        let patterns: Vec<&str> = code_filter.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query.push_str(" AND (");
            for (i, pattern) in patterns.iter().enumerate() {
                if i > 0 {
                    query.push_str(" OR ");
                }
                query.push_str(&format!("code ~* ${}", bindings.len() + 1));
                bindings.push(pattern.to_string());
            }
            query.push_str(")");
        }
    }

    // Add ordering based on SearchParams
    if let Some(order_clause) = params.base.parse_order_by() {
        query.push_str(&format!(" ORDER BY {}", order_clause));
    } else {
        // Default ordering
        query.push_str(" ORDER BY code ASC");
    }

    // Add pagination
    let limit = params.base.get_limit();
    let offset = params.base.get_offset();
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Build the query dynamically
    let mut sql_query = sqlx::query_as::<_, (String, chrono::DateTime<chrono::Utc>)>(&query);
    for binding in bindings {
        sql_query = sql_query.bind(binding);
    }

    let records = sql_query.fetch_all(pool).await?;

    Ok(records
        .into_iter()
        .map(|(code, created)| Currency { code, created })
        .collect())
}
