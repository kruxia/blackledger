// Tests for API handlers to improve coverage

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid;

#[tokio::test]
async fn test_handle_create_accounts() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create a ledger first
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for Accounts {}", uuid::Uuid::new_v4())
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = ledger_response.status();
    let ledger_body = axum::body::to_bytes(ledger_response.into_body(), usize::MAX)
        .await
        .unwrap();

    if status != StatusCode::CREATED {
        let error_text = String::from_utf8_lossy(&ledger_body);
        panic!(
            "Failed to create ledger. Status: {:?}, Body: {}",
            status, error_text
        );
    }
    assert_eq!(status, StatusCode::CREATED);
    let ledgers: Vec<Value> = serde_json::from_slice(&ledger_body).unwrap();
    assert_eq!(ledgers.len(), 1);
    let ledger_id = ledgers[0]["id"].as_i64().unwrap();

    // Test creating multiple accounts in batch
    let accounts = vec![
        json!({
            "ledger_id": ledger_id,
            "name": "Cash Account",
            "number": 1000,
            "normal": "DR"
        }),
        json!({
            "ledger_id": ledger_id,
            "name": "Revenue Account",
            "number": 4000,
            "normal": "CR"
        }),
    ];

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&accounts).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created_accounts: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(created_accounts.len(), 2);
    assert_eq!(created_accounts[0]["name"], "Cash Account");
    assert_eq!(created_accounts[1]["name"], "Revenue Account");
}

#[tokio::test]
async fn test_handle_update_account() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create ledger and account
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for Update {}", uuid::Uuid::new_v4())
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

    // Create an account
    let account_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "ledger_id": ledger_id,
                        "name": "Original Name",
                        "number": 1000,
                        "normal": "DR"
                    })])
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let account_body = axum::body::to_bytes(account_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let accounts: Vec<Value> = serde_json::from_slice(&account_body).unwrap();
    let account_id = accounts[0]["id"].as_i64().unwrap();

    // Update the account (only name can be updated)
    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/accounts/{}", account_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "name": "Updated Name"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
    let updated_body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let updated_account: Value = serde_json::from_slice(&updated_body).unwrap();
    assert_eq!(updated_account["name"], "Updated Name");
    assert_eq!(updated_account["number"], 1000); // Number remains unchanged
}

#[tokio::test]
async fn test_handle_search_accounts() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create test data
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

    // Create multiple accounts
    let accounts = vec![
        json!({
            "ledger_id": ledger_id,
            "name": "Cash Account",
            "number": 1000,
            "normal": "DR"
        }),
        json!({
            "ledger_id": ledger_id,
            "name": "Bank Account",
            "number": 1100,
            "normal": "DR"
        }),
        json!({
            "ledger_id": ledger_id,
            "name": "Revenue Account",
            "number": 4000,
            "normal": "CR"
        }),
    ];

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/accounts")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&accounts).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Search with name filter
    let search_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/accounts?ledger_id={}&name=Cash", ledger_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);
    let search_body = axum::body::to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();

    // Debug: print response body if parsing fails
    let search_result: Value = match serde_json::from_slice(&search_body) {
        Ok(v) => v,
        Err(e) => {
            let body_str = String::from_utf8_lossy(&search_body);
            panic!(
                "Failed to parse search response: {:?}. Body: {}",
                e, body_str
            );
        }
    };
    let found_accounts = search_result["data"].as_array().unwrap();
    assert_eq!(found_accounts.len(), 1);
    assert_eq!(found_accounts[0]["name"], "Cash Account");

    // Test pagination metadata
    assert_eq!(search_result["pagination"]["page"], 1);
    assert_eq!(search_result["pagination"]["size"], 100); // Default size from SearchParams
    assert_eq!(search_result["pagination"]["total"], 1);
}

#[tokio::test]
async fn test_handle_get_accounts_with_balances() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create test data
    let ledger_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ledgers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&vec![json!({
                        "name": format!("Test Ledger for Balances {}", uuid::Uuid::new_v4())
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

    // Post a transaction to create balances
    let tx_response = app
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

    // Check transaction was posted successfully
    if tx_response.status() != StatusCode::CREATED {
        let tx_body = axum::body::to_bytes(tx_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let tx_error = String::from_utf8_lossy(&tx_body);
        panic!(
            "Failed to post transaction. Status: {:?}, Error: {}",
            StatusCode::UNPROCESSABLE_ENTITY,
            tx_error
        );
    }

    // Get accounts with balances
    let balances_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/accounts/balances?ledger_id={}", ledger_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(balances_response.status(), StatusCode::OK);
    let balances_body = axum::body::to_bytes(balances_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let balances_result: Value = serde_json::from_slice(&balances_body).unwrap();
    let account_balances = balances_result["data"].as_array().unwrap();

    // Should have 2 accounts with balances
    assert_eq!(account_balances.len(), 2);

    // Find and verify cash account balance
    let cash_balance = account_balances
        .iter()
        .find(|a| a["account"]["id"] == cash_id)
        .unwrap();

    // Debug: print the cash balance structure
    eprintln!("Cash balance object: {:?}", cash_balance);

    // Check if balance exists and verify its value
    assert!(
        cash_balance["balances"].get("USD").is_some(),
        "Cash account should have USD balance"
    );
    assert_eq!(cash_balance["balances"]["USD"], "100.00");

    // Find and verify revenue account balance
    let revenue_balance = account_balances
        .iter()
        .find(|a| a["account"]["id"] == revenue_id)
        .unwrap();

    // Debug: print the revenue balance structure
    eprintln!("Revenue balance object: {:?}", revenue_balance);

    // Check if balance exists and verify its value
    assert!(
        revenue_balance["balances"].get("USD").is_some(),
        "Revenue account should have USD balance"
    );
    assert_eq!(revenue_balance["balances"]["USD"], "100.00"); // Revenue CR account gets +100 credit
}
