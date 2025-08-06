use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::ledger::{Ledger, CreateLedger, UpdateLedger};

pub async fn create_ledger(pool: &PgPool, input: &CreateLedger) -> ApiResult<Ledger> {
    let id = Uuid::new_v4();
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"
        INSERT INTO ledger (id, name, description, metadata)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.metadata)
    .fetch_one(pool)
    .await?;

    Ok(ledger)
}

pub async fn get_ledger_by_id(pool: &PgPool, id: Uuid) -> ApiResult<Ledger> {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"SELECT * FROM ledger WHERE id = $1"#
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Ledger {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    Ok(ledger)
}

pub async fn update_ledger(pool: &PgPool, id: Uuid, input: &UpdateLedger) -> ApiResult<Ledger> {
    let ledger = sqlx::query_as::<_, Ledger>(
        r#"
        UPDATE ledger
        SET name = COALESCE($2, name),
            description = COALESCE($3, description),
            metadata = COALESCE($4, metadata),
            updated = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.metadata)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("Ledger {} not found", id)),
        _ => ApiError::Database(e),
    })?;

    Ok(ledger)
}

pub async fn list_ledgers(
    pool: &PgPool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> ApiResult<Vec<Ledger>> {
    let ledgers = sqlx::query_as::<_, Ledger>(
        r#"
        SELECT * FROM ledger
        ORDER BY created DESC
        LIMIT $1
        OFFSET $2
        "#
    )
    .bind(limit.unwrap_or(100))
    .bind(offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;

    Ok(ledgers)
}