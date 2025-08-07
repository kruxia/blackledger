mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::Value;
use tower::ServiceExt;

use common::{setup_test_app_state, setup_test_db};

#[tokio::test]
async fn test_health_check_endpoint() {
    let app_state = setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_database_connectivity() {
    let pool = setup_test_db().await;

    // Test that we can execute a simple query
    let result = sqlx::query("SELECT 1 as test").fetch_one(&pool).await;

    assert!(result.is_ok());
}
