mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::{api, models::*};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_create_and_get_currency() {
    let pool = common::setup_test_db().await;
    let app = api::router().with_state(pool);

    // Create a currency
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/currencies")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "code": "USD",
                        "name": "US Dollar",
                        "minor_units": 2
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // List currencies
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    
    let currencies: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert!(currencies.iter().any(|c| c["code"] == "USD"));
}

#[tokio::test]
async fn test_ledger_crud_operations() {
    let pool = common::setup_test_db().await;
    let app = api::router().with_state(pool);

    // Create a ledger
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Test Ledger",
                        "description": "A test ledger for integration tests"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);
    
    let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledger: Value = serde_json::from_slice(&body).unwrap();
    let ledger_id = ledger["id"].as_str().unwrap();

    // Get ledger by ID
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/ledgers/{}", ledger_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    // Update ledger
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/ledgers/{}", ledger_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Updated Test Ledger"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated_ledger: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated_ledger["name"], "Updated Test Ledger");
}

#[tokio::test]
async fn test_account_operations() {
    let pool = common::setup_test_db().await;
    let app = api::router().with_state(pool.clone());

    // First create a ledger
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Account Test Ledger"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(ledger_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledger: Value = serde_json::from_slice(&body).unwrap();
    let ledger_id = ledger["id"].as_str().unwrap();

    // Create an account
    let account_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ledger_id": ledger_id,
                        "number": "1000",
                        "name": "Cash",
                        "normal_balance": "DR"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(account_response.status(), StatusCode::CREATED);
    
    let body = axum::body::to_bytes(account_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let account: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(account["number"], "1000");
    assert_eq!(account["name"], "Cash");
}

#[tokio::test]
async fn test_transaction_posting() {
    let pool = common::setup_test_db().await;
    let app = api::router().with_state(pool.clone());

    // Setup: Create ledger, currency, and accounts
    let ledger_id = create_test_ledger(&app).await;
    create_test_currency(&app, "USD").await;
    let cash_account_id = create_test_account(&app, &ledger_id, "1000", "Cash", "DR").await;
    let revenue_account_id = create_test_account(&app, &ledger_id, "4000", "Revenue", "CR").await;

    // Post a transaction
    let transaction_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ledger_id": ledger_id,
                        "effective": "2024-01-01T00:00:00Z",
                        "description": "Test transaction",
                        "entries": [
                            {
                                "account_id": cash_account_id,
                                "currency_code": "USD",
                                "dr": "100.00"
                            },
                            {
                                "account_id": revenue_account_id,
                                "currency_code": "USD",
                                "cr": "100.00"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(transaction_response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_unbalanced_transaction_rejection() {
    let pool = common::setup_test_db().await;
    let app = api::router().with_state(pool.clone());

    // Setup
    let ledger_id = create_test_ledger(&app).await;
    create_test_currency(&app, "USD").await;
    let cash_account_id = create_test_account(&app, &ledger_id, "1000", "Cash", "DR").await;
    let revenue_account_id = create_test_account(&app, &ledger_id, "4000", "Revenue", "CR").await;

    // Try to post an unbalanced transaction
    let transaction_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ledger_id": ledger_id,
                        "effective": "2024-01-01T00:00:00Z",
                        "description": "Unbalanced transaction",
                        "entries": [
                            {
                                "account_id": cash_account_id,
                                "currency_code": "USD",
                                "dr": "100.00"
                            },
                            {
                                "account_id": revenue_account_id,
                                "currency_code": "USD",
                                "cr": "50.00"  // Doesn't balance!
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(transaction_response.status(), StatusCode::BAD_REQUEST);
}

// Helper functions
async fn create_test_ledger(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Test Ledger"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledger: Value = serde_json::from_slice(&body).unwrap();
    ledger["id"].as_str().unwrap().to_string()
}

async fn create_test_currency(app: &axum::Router, code: &str) {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/currencies")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "code": code,
                        "name": format!("{} Currency", code),
                        "minor_units": 2
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
}

async fn create_test_account(
    app: &axum::Router,
    ledger_id: &str,
    number: &str,
    name: &str,
    normal_balance: &str,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ledger_id": ledger_id,
                        "number": number,
                        "name": name,
                        "normal_balance": normal_balance
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let account: Value = serde_json::from_slice(&body).unwrap();
    account["id"].as_str().unwrap().to_string()
}