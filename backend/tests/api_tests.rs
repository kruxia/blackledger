mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::api;
use serde_json::{Value, json};
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
async fn test_currency_search_with_regex() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create multiple currencies
    for code in &["USD", "EUR", "GBP", "JPY", "CAD", "AUD"] {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/currencies")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"code": code}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Test single regex pattern
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?code=^U")
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
    assert_eq!(currencies.len(), 1);
    assert_eq!(currencies[0]["code"], "USD");

    // Test comma-delimited patterns
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?code=^U,^E")
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
    assert_eq!(currencies.len(), 2);
    let codes: Vec<String> = currencies.iter().map(|c| c["code"].as_str().unwrap().to_string()).collect();
    assert!(codes.contains(&"USD".to_string()));
    assert!(codes.contains(&"EUR".to_string()));

    // Test case-insensitive matching
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?code=usd")
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
    assert_eq!(currencies.len(), 1);
    assert_eq!(currencies[0]["code"], "USD");

    // Test pattern matching within string
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?code=.*D$")
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
    let codes: Vec<String> = currencies.iter().map(|c| c["code"].as_str().unwrap().to_string()).collect();
    assert!(codes.contains(&"USD".to_string()));
    assert!(codes.contains(&"CAD".to_string()));
    assert!(codes.contains(&"AUD".to_string()));
}

#[tokio::test]
async fn test_currency_pagination_and_sorting() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create multiple currencies for testing
    let currencies = ["USD", "EUR", "GBP", "JPY", "CAD", "AUD", "CHF", "NZD", "SEK", "NOK"];
    for code in &currencies {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/currencies")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"code": code}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Test pagination with limit
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?_limit=3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        eprintln!("Response body: {}", String::from_utf8_lossy(&body));
        panic!("Expected OK, got {}", status);
    }
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let currencies: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(currencies.len(), 3);

    // Test pagination with offset
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?_limit=3&_offset=3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let currencies_page2: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(currencies_page2.len(), 3);
    // Ensure we got different currencies
    assert_ne!(currencies[0]["code"], currencies_page2[0]["code"]);

    // Test sorting ascending
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?_orderby=code")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let sorted_asc: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(sorted_asc[0]["code"], "AUD");

    // Test sorting descending
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?_orderby=-code")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let sorted_desc: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(sorted_desc[0]["code"], "USD");

    // Test combination of filtering, pagination and sorting
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/currencies?code=^[AC]&_orderby=-code&_limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let filtered: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0]["code"], "CHF");
    assert_eq!(filtered[1]["code"], "CAD");
}

#[tokio::test]
async fn test_sql_injection_prevention() {
    let app_state = common::setup_test_app_state().await;
    let app = api::router(app_state.clone()).with_state(app_state);

    // Create a few currencies for testing
    for code in &["USD", "EUR", "GBP"] {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/currencies")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"code": code}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Test SQL injection attempt in orderby field
    let malicious_inputs = vec![
        "code; DROP TABLE currency;--",
        "code' OR '1'='1",
        "code UNION SELECT * FROM users",
        "code/**/OR/**/1=1",
        "code\"; DROP TABLE currency;--",
        "1; DELETE FROM currency",
    ];

    for malicious_input in malicious_inputs {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/currencies?_orderby={}", urlencoding::encode(malicious_input)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should reject with 400
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Expected 400 for SQL injection attempt '{}', got {}",
            malicious_input,
            response.status()
        );
    }

    // Test valid orderby patterns
    let valid_inputs = vec![
        "code",
        "-code",
        "created",
        "-created",
        "code,created",
        "-code,-created",
    ];

    for valid_input in valid_inputs {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/currencies?_orderby={}", valid_input))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Expected OK for valid orderby '{}', got {}",
            valid_input,
            response.status()
        );
    }
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
    assert!(
        updated_ledger["name"]
            .as_str()
            .unwrap()
            .starts_with("Updated Test Ledger")
    );
}

#[tokio::test]
async fn test_account_operations() {
    let app_state = common::setup_test_app_state().await;
    let _pool = app_state.pool.clone();
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
    let _pool = app_state.pool.clone();
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
        eprintln!(
            "Transaction creation failed with status {}: {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn test_unbalanced_transaction_rejection() {
    let app_state = common::setup_test_app_state().await;
    let _pool = app_state.pool.clone();
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
                .body(Body::from(json!({"name": ledger_name}).to_string()))
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
