use sqlx::{PgPool, Postgres, QueryBuilder, Row};

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

pub async fn create_ledgers_batch(pool: &PgPool, inputs: &[CreateLedger]) -> ApiResult<Vec<Ledger>> {
    // Start a transaction to ensure atomicity
    let mut tx = pool.begin().await?;
    
    let mut ledgers = Vec::new();
    
    for input in inputs {
        let record = sqlx::query!(
            r#"
            INSERT INTO ledger (name)
            VALUES ($1)
            RETURNING id, name, created
            "#,
            input.name
        )
        .fetch_one(&mut *tx)
        .await?;
        
        ledgers.push(Ledger {
            id: record.id,
            name: record.name,
            created: record.created,
        });
    }
    
    // Commit the transaction
    tx.commit().await?;
    
    Ok(ledgers)
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
    let limit = params.base.get_limit() as i64;
    let offset = params.base.get_offset() as i64;

    // Use QueryBuilder for dynamic SQL generation
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT id, name, created FROM ledger WHERE 1=1");

    // Handle comma-delimited list of IDs
    if let Some(ref id_list) = params.id {
        let ids: Vec<i64> = id_list
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        if !ids.is_empty() {
            query_builder.push(" AND id IN (");
            let mut separated = query_builder.separated(", ");
            for id in ids {
                separated.push_bind(id);
            }
            query_builder.push(")");
        }
    }

    // Handle comma-delimited regex patterns for names
    if let Some(ref name_patterns) = params.name {
        let patterns: Vec<&str> = name_patterns.split(',').map(|s| s.trim()).collect();
        if !patterns.is_empty() {
            query_builder.push(" AND (");
            let mut first = true;
            for pattern in patterns {
                if !first {
                    query_builder.push(" OR ");
                }
                query_builder.push("name ~* ");
                query_builder.push_bind(pattern);
                first = false;
            }
            query_builder.push(")");
        }
    }

    // Add sorting based on SearchParams with whitelist validation
    const ALLOWED_COLUMNS: &[&str] = &["id", "name", "created"];
    if let Some(order_clause) = params.base.parse_order_by(ALLOWED_COLUMNS) {
        query_builder.push(" ORDER BY ");
        query_builder.push(order_clause);
    } else {
        // Default ordering
        query_builder.push(" ORDER BY created DESC");
    }

    // Add pagination with parameter binding
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    // Execute the query
    let query = query_builder.build();
    let rows = query.fetch_all(pool).await?;

    let mut ledgers = Vec::new();
    for row in rows {
        let ledger = Ledger {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            created: row.try_get("created")?,
        };
        ledgers.push(ledger);
    }

    Ok(ledgers)
}
