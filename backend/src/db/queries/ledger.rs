use sqlx::PgPool;

use crate::error::{ApiError, ApiResult};
use crate::models::ledger::{Ledger, CreateLedger, UpdateLedger};

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
    let record = sqlx::query!(
        r#"SELECT id, name, created FROM ledger WHERE id = $1"#,
        id
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
    let record = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM ledger"#
    )
    .fetch_one(pool)
    .await?;
    
    Ok(record.count.unwrap_or(0))
}