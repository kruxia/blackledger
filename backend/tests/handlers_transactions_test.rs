// Tests for transaction handlers to improve coverage

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn test_handle_post_transaction() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Setup test data
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for TX {}", uuid::Uuid::new_v4())
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let ledger_body = axum::body::to_bytes(ledger_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledgers: Vec<Value> = serde_json::from_slice(&ledger_body).unwrap();
    let ledger_id = ledgers[0]["id"].as_i64().unwrap();

    // Create currency
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/currencies")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({"code": "USD"})]).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create accounts
    let accounts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Cash",
                            "number": 1000,
                            "normal": "DR"
                        }),
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Revenue",
                            "number": 4000,
                            "normal": "CR"
                        }),
                    ])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let accounts_body = axum::body::to_bytes(accounts_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let accounts: Vec<Value> = serde_json::from_slice(&accounts_body).unwrap();
    let cash_id = accounts[0]["id"].as_i64().unwrap();
    let revenue_id = accounts[1]["id"].as_i64().unwrap();

    // Test posting a transaction
    let transaction_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "ledger_id": ledger_id,
                        "effective": "2024-01-01T00:00:00Z",
                        "memo": "Test transaction",
                        "meta": {"invoice": "INV-001"},
                        "entries": [
                            {
                                "acct": cash_id,
                                "currency": "USD",
                                "debit": "100.00"
                            },
                            {
                                "acct": revenue_id,
                                "currency": "USD",
                                "credit": "100.00"
                            }
                        ]
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = transaction_response.status();
    let transaction_body = axum::body::to_bytes(transaction_response.into_body(), usize::MAX)
        .await
        .unwrap();

    if status != StatusCode::CREATED {
        let error_msg = String::from_utf8_lossy(&transaction_body);
        panic!(
            "Transaction creation failed with status {}: {}",
            status, error_msg
        );
    }

    assert_eq!(status, StatusCode::CREATED);
    let result: Vec<Value> = serde_json::from_slice(&transaction_body).unwrap();

    assert_eq!(result.len(), 1);
    let transaction = &result[0];
    assert!(transaction["id"].is_i64());
    assert_eq!(transaction["memo"], "Test transaction");
    assert_eq!(transaction["meta"]["invoice"], "INV-001");
    assert_eq!(transaction["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_handle_search_transactions() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Setup test data
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for Search {}", uuid::Uuid::new_v4())
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let ledger_body = axum::body::to_bytes(ledger_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledgers: Vec<Value> = serde_json::from_slice(&ledger_body).unwrap();
    let ledger_id = ledgers[0]["id"].as_i64().unwrap();

    // Create currency and accounts
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/currencies")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({"code": "USD"})]).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let accounts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Cash",
                            "number": 1000,
                            "normal": "DR"
                        }),
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Revenue",
                            "number": 4000,
                            "normal": "CR"
                        }),
                    ])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let accounts_body = axum::body::to_bytes(accounts_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let accounts: Vec<Value> = serde_json::from_slice(&accounts_body).unwrap();
    let cash_id = accounts[0]["id"].as_i64().unwrap();
    let revenue_id = accounts[1]["id"].as_i64().unwrap();

    // Post multiple transactions
    for i in 1..=3 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/transactions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&vec![json!({
                            "ledger_id": ledger_id,
                            "effective": format!("2024-01-{:02}T00:00:00Z", i),
                            "memo": format!("Transaction {}", i),
                            "entries": [
                                {
                                    "acct": cash_id,
                                    "currency": "USD",
                                    "debit": format!("{}.00", i * 100)
                                },
                                {
                                    "acct": revenue_id,
                                    "currency": "USD",
                                    "credit": format!("{}.00", i * 100)
                                }
                            ]
                        })])
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Search transactions with memo filter
    let search_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/transactions?ledger_id={}&memo=Transaction",
                    ledger_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);
    let search_body = axum::body::to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let search_result: Value = serde_json::from_slice(&search_body).unwrap();
    let transactions = search_result["data"].as_array().unwrap();
    assert_eq!(transactions.len(), 3);

    // Test pagination
    let page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/transactions?ledger_id={}&_offset=2&_limit=2",
                    ledger_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(page_response.status(), StatusCode::OK);
    let page_body = axum::body::to_bytes(page_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let page_result: Value = serde_json::from_slice(&page_body).unwrap();
    // Offset 2 with limit 2 should return 1 item since we have 3 total
    assert_eq!(page_result["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_handle_get_transaction() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Setup test data
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for Get {}", uuid::Uuid::new_v4())
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let ledger_body = axum::body::to_bytes(ledger_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ledgers: Vec<Value> = serde_json::from_slice(&ledger_body).unwrap();
    let ledger_id = ledgers[0]["id"].as_i64().unwrap();

    // Create currency and accounts
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/currencies")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({"code": "USD"})]).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let accounts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Cash",
                            "number": 1000,
                            "normal": "DR"
                        }),
                        json!({
                            "ledger_id": ledger_id,
                            "name": "Revenue",
                            "number": 4000,
                            "normal": "CR"
                        }),
                    ])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let accounts_body = axum::body::to_bytes(accounts_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let accounts: Vec<Value> = serde_json::from_slice(&accounts_body).unwrap();
    let cash_id = accounts[0]["id"].as_i64().unwrap();
    let revenue_id = accounts[1]["id"].as_i64().unwrap();

    // Post a transaction
    let post_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/transactions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "ledger_id": ledger_id,
                        "effective": "2024-01-01T00:00:00Z",
                        "memo": "Test transaction",
                        "entries": [
                            {
                                "acct": cash_id,
                                "currency": "USD",
                                "debit": "100.00"
                            },
                            {
                                "acct": revenue_id,
                                "currency": "USD",
                                "credit": "100.00"
                            }
                        ]
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let post_body = axum::body::to_bytes(post_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let posted: Vec<Value> = serde_json::from_slice(&post_body).unwrap();
    let transaction_id = posted[0]["id"].as_i64().unwrap();

    // Get the transaction by ID
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/transactions/{}", transaction_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: Value = serde_json::from_slice(&get_body).unwrap();

    assert_eq!(result["transaction"]["id"], transaction_id);
    assert_eq!(result["transaction"]["memo"], "Test transaction");
    assert_eq!(result["entries"].as_array().unwrap().len(), 2);
}
