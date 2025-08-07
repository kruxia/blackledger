use sqlx::PgPool;

use crate::api::search::LedgerSearchParams;
use crate::error::{ApiError, ApiResult};
use crate::models::ledger::{CreateLedger, Ledger, UpdateLedger};

pub async fn create_ledger(pool: &PgPool, input: &CreateLedger) -> ApiResult<Ledger> {
    let record = sqlx::query!(
        r#"
        INSERT INTO ledger (name)
        VALUES ($1)
        RETURNING id, name, created
        "#,
        input.name
    )
    .fetch_one(pool)
    .await?;

    Ok(Ledger {
        id: record.id,
        name: record.name,
        created: record.created,
    })
}

pub async fn get_ledger_by_id(pool: &PgPool, id: i64) -> ApiResult<Ledger> {
    let record = sqlx::query!(r#"SELECT id, name, created FROM ledger WHERE id = $1"#, id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::NotFound(format!("Ledger {} not found", id)),
            _ => ApiError::Database(e),
        })?;

    Ok(Ledger {
        id: record.id,
        name: record.name,
        created: record.created,
    })
}

pub async fn update_ledger(pool: &PgPool, id: i64, input: &UpdateLedger) -> ApiResult<Ledger> {
    let record = sqlx::query!(
        r#"
        UPDATE ledger
        SET name = COALESCE($2, name)
        WHERE id = $1
        RETURNING id, name, created
        "#,
        id,
        input.name.as_deref()
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Ledger {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    Ok(Ledger {
        id: record.id,
        name: record.name,
        created: record.created,
    })
}

pub async fn list_ledgers(
    pool: &PgPool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Ledger>> {
    let records = sqlx::query!(
        r#"
        SELECT id, name, created FROM ledger
        ORDER BY created DESC
        LIMIT $1
        OFFSET $2
        "#,
        limit.unwrap_or(100),
        offset.unwrap_or(0)
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Ledger {
            id: r.id,
            name: r.name,
            created: r.created,
        })
        .collect())
}

pub async fn count_ledgers(pool: &PgPool) -> ApiResult<i64> {
    let record = sqlx::query!(r#"SELECT COUNT(*) as count FROM ledger"#)
        .fetch_one(pool)
        .await?;

    Ok(record.count.unwrap_or(0))
}

pub async fn search_ledgers(pool: &PgPool, params: &LedgerSearchParams) -> ApiResult<Vec<Ledger>> {
    let mut query = String::from("SELECT id, name, created FROM ledger WHERE 1=1");
    let mut bindings = vec![];

    // Handle comma-delimited list of IDs
    if let Some(id_filter) = &params.id {
        let ids: Vec<&str> = id_filter.split(',').map(|s| s.trim()).collect();
        if !ids.is_empty() {
            let valid_ids: Vec<i64> = ids
                .iter()
                .filter_map(|id_str| id_str.parse::<i64>().ok())
                .collect();

            if !valid_ids.is_empty() {
                let placeholders: Vec<String> = (1..=valid_ids.len())
                    .map(|i| format!("${}::bigint", bindings.len() + i))
                    .collect();
                query.push_str(&format!(" AND id IN ({})", placeholders.join(", ")));
                for id in valid_ids {
                    bindings.push(id.to_string());
                }
            }
        }
    }

    // Handle comma-delimited regex patterns for names
    if let Some(name_filter) = &params.name {
        let patterns: Vec<&str> = name_filter.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query.push_str(" AND (");
            for (i, pattern) in patterns.iter().enumerate() {
                if i > 0 {
                    query.push_str(" OR ");
                }
                query.push_str(&format!("name ~* ${}", bindings.len() + 1));
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
        query.push_str(" ORDER BY created DESC");
    }

    // Add pagination
    let limit = params.base.get_limit();
    let offset = params.base.get_offset();
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Build the query dynamically
    let mut sql_query = sqlx::query_as::<_, (i64, String, chrono::DateTime<chrono::Utc>)>(&query);
    for binding in bindings {
        sql_query = sql_query.bind(binding);
    }

    let records = sql_query.fetch_all(pool).await?;

    Ok(records
        .into_iter()
        .map(|(id, name, created)| Ledger { id, name, created })
        .collect())
}
