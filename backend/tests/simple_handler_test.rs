// Simple test to debug handler issues

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_simple_ledger_creation() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Test health check first
    let health_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(health_response.status(), StatusCode::OK);
    println!("Health check passed");

    // Now test ledger creation
    let ledger_request = vec![json!({
        "name": format!("Simple Test Ledger {}", Uuid::new_v4())
    })];

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&ledger_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);

    println!("Response status: {:?}", status);
    println!("Response body: {}", body_str);

    assert_eq!(
        status,
        StatusCode::CREATED,
        "Expected 201 CREATED, got {:?}. Body: {}",
        status,
        body_str
    );
}
