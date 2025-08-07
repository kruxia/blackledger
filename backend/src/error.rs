use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Transaction does not balance")]
    UnbalancedTransaction,

    #[error("Account version mismatch")]
    OptimisticLockError,

    #[error("Duplicate key: {0}")]
    DuplicateKey(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::Validation(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::UnbalancedTransaction => (
                StatusCode::BAD_REQUEST,
                "Transaction does not balance".to_string(),
            ),
            ApiError::OptimisticLockError => {
                (StatusCode::CONFLICT, "Account version mismatch".to_string())
            }
            ApiError::DuplicateKey(ref msg) => (StatusCode::CONFLICT, msg.clone()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                match e {
                    sqlx::Error::RowNotFound => {
                        (StatusCode::NOT_FOUND, "Resource not found".to_string())
                    }
                    sqlx::Error::Database(db_err) => {
                        if let Some(constraint) = db_err.constraint() {
                            (
                                StatusCode::CONFLICT,
                                format!("Constraint violation: {}", constraint),
                            )
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Database error".to_string(),
                            )
                        }
                    }
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error".to_string(),
                    ),
                }
            }
            ApiError::Internal(ref e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_error_response() {
        let error = ApiError::Validation("Invalid input".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_not_found_error_response() {
        let error = ApiError::NotFound("Resource not found".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unbalanced_transaction_error_response() {
        let error = ApiError::UnbalancedTransaction;
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_optimistic_lock_error_response() {
        let error = ApiError::OptimisticLockError;
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_duplicate_key_error_response() {
        let error = ApiError::DuplicateKey("Duplicate entry".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_unauthorized_error_response() {
        let error = ApiError::Unauthorized;
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
