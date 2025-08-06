mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn test_create_and_get_currency() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

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
                        "code": "USD"
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
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

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
                        "name": format!("Test Ledger CRUD {}", 
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos())
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
    let ledger_id = ledger["id"].as_i64().unwrap();

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
                        "name": format!("Updated Test Ledger {}", 
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos())
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
    assert!(updated_ledger["name"]
        .as_str()
        .unwrap()
        .starts_with("Updated Test Ledger"));
}

#[tokio::test]
async fn test_account_operations() {
    let app_state = common::setup_test_app_state().await;
    let pool = app_state.pool.clone();
    let app = api::router(app_state.clone()).with_state(app_state);

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
                        "name": format!("Account Test Ledger {}", 
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos())
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
    let ledger_id = ledger["id"].as_i64().unwrap();

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
                        "number": 1000,
                        "name": "Cash",
                        "normal": "DR"
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
    assert_eq!(account["number"], 1000);
    assert_eq!(account["name"], "Cash");
}

#[tokio::test]
async fn test_transaction_posting() {
    let app_state = common::setup_test_app_state().await;
    let pool = app_state.pool.clone();
    let app = api::router(app_state.clone()).with_state(app_state);

    // Setup: Create ledger, currency, and accounts
    let ledger_id = create_test_ledger(&app).await;
    create_test_currency(&app, "USD").await;
    let cash_account_id = create_test_account(&app, ledger_id, "1000", "Cash", "DR").await;
    let revenue_account_id = create_test_account(&app, ledger_id, "4000", "Revenue", "CR").await;

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
                        "memo": "Test transaction",
                        "entries": [
                            {
                                "account_id": cash_account_id,
                                "currency_code": "USD",
                                "debit": "100.00"
                            },
                            {
                                "account_id": revenue_account_id,
                                "currency_code": "USD",
                                "credit": "100.00"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = transaction_response.status();
    if status != StatusCode::CREATED {
        let body = axum::body::to_bytes(transaction_response.into_body(), usize::MAX)
            .await
            .unwrap();
        eprintln!("Transaction creation failed with status {}: {}", status, String::from_utf8_lossy(&body));
    }
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn test_unbalanced_transaction_rejection() {
    let app_state = common::setup_test_app_state().await;
    let pool = app_state.pool.clone();
    let app = api::router(app_state.clone()).with_state(app_state);

    // Setup
    let ledger_id = create_test_ledger(&app).await;
    create_test_currency(&app, "USD").await;
    let cash_account_id = create_test_account(&app, ledger_id, "1000", "Cash", "DR").await;
    let revenue_account_id = create_test_account(&app, ledger_id, "4000", "Revenue", "CR").await;

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
                        "memo": "Unbalanced transaction",
                        "entries": [
                            {
                                "account_id": cash_account_id,
                                "currency_code": "USD",
                                "debit": "100.00"
                            },
                            {
                                "account_id": revenue_account_id,
                                "currency_code": "USD",
                                "credit": "50.00"  // Doesn't balance!
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
async fn create_test_ledger(app: &axum::Router) -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let ledger_name = format!("Test Ledger {}", timestamp);
    
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": ledger_name}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledger: Value = serde_json::from_slice(&body).unwrap();
    ledger["id"].as_i64().unwrap()
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
                        "code": code
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
    ledger_id: i64,
    number: &str,
    name: &str,
    normal_balance: &str,
) -> i64 {
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
                        "number": number.parse::<i16>().ok(),
                        "name": name,
                        "normal": normal_balance
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
    account["id"].as_i64().unwrap()
}