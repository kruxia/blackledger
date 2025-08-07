use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use blackledger::{api, auth};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_app_with_auth(pool: PgPool, mock_server: &MockServer) -> axum::Router {
    // Mock JWKS endpoint with a dummy RSA key
    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "keys": [
                {
                    "kty": "RSA",
                    "use": "sig",
                    "kid": "test-key-1",
                    "n": "xjNrLpXNr5HNjyHmZ9Jb9dLzJJV5qR5bfEMNNSWDDaFhsq5lZvhj9wZWW3SCPVF1vebstLfSTDSVmqTFKqCJkSSxSpo5SFYw1ncIrj8f_KKmXA99f_I7TlFdl7Vb5VhWkR9aGEQMJB3qMJM0L1bCCHaZwM0YcS1MqnfFnlcfCllKYZGYJYKNc2tpGpqYVLKkNBPznwLLpeBvuQjsjkFLkJudHhA6pLDsBKK4n02SkYlbnafh23iNDanwUBVqjL0y34mnz7duqBX2NgQJxgFHXspUHp0qFVqLvLh5cEhHnkrF5n2kc4vTnD7UUDRDWGEwFYUq58bFKmGEqXCAH7LBhQ",
                    "e": "AQAB",
                    "alg": "RS256"
                }
            ]
        })))
        .mount(mock_server)
        .await;

    let auth_config = auth::AuthConfig {
        enabled: true,
        jwks_url: Some(format!("{}/.well-known/jwks.json", mock_server.uri())),
        audience: None,
        issuer: None,
    };

    let jwt_validator = Arc::new(
        auth::JwtValidator::new(auth_config)
            .await
            .expect("Failed to create JWT validator"),
    );

    let app_state = api::AppState {
        pool,
        jwt_validator,
    };

    api::router(app_state.clone()).with_state(app_state)
}

#[sqlx::test]
async fn test_health_check_no_auth_required(pool: PgPool) {
    let mock_server = MockServer::start().await;
    let app = setup_app_with_auth(pool, &mock_server).await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_protected_routes_require_auth(pool: PgPool) {
    let mock_server = MockServer::start().await;
    let app = setup_app_with_auth(pool, &mock_server).await;

    // Test that GET /currencies requires auth
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/currencies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test that GET /ledgers requires auth
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ledgers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test that GET /accounts requires auth
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test that GET /transactions requires auth
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/transactions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
