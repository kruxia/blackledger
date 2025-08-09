mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::get,
};
use blackledger::{
    api,
    auth::{AuthConfig, AuthUser, Claims, JwtValidator},
    error::ApiError,
};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, jwk::JwkSet,
};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

// Use HS256 for simpler testing without RSA key generation issues
fn create_test_secret() -> Vec<u8> {
    b"test_secret_key_for_jwt_validation_testing_only".to_vec()
}

fn create_test_jwt_hs256(claims: &Claims) -> String {
    let secret = create_test_secret();
    let header = Header::new(Algorithm::HS256);
    encode(&header, claims, &EncodingKey::from_secret(&secret)).unwrap()
}

async fn setup_app_with_auth(pool: PgPool, mock_server: &MockServer) -> axum::Router {
    // Mock JWKS endpoint with a dummy RSA key
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
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
            })),
        )
        .mount(mock_server)
        .await;

    let auth_config = AuthConfig {
        enabled: true,
        jwks_url: Some(mock_server.uri()),
        audience: None,
        issuer: None,
    };

    let jwt_validator = Arc::new(
        JwtValidator::new(auth_config)
            .await
            .expect("Failed to create JWT validator"),
    );

    let app_state = api::AppState {
        pool,
        jwt_validator,
    };

    api::router(app_state.clone()).with_state(app_state)
}

#[tokio::test]
async fn test_jwt_validator_disabled_auth() {
    let config = AuthConfig {
        enabled: false,
        jwks_url: None,
        audience: None,
        issuer: None,
    };

    let validator = JwtValidator::new(config).await.unwrap();

    // Any token should work when auth is disabled
    let result = validator.validate_token("any-token").await;
    assert!(result.is_ok());

    let claims = result.unwrap();
    assert_eq!(claims.sub, "anonymous");
    assert_eq!(claims.email, Some("anonymous@localhost".to_string()));
}

#[tokio::test]
async fn test_jwt_validation_with_hs256() {
    // Test JWT validation logic with simpler HS256 algorithm
    let secret = create_test_secret();

    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        iss: Some("test-issuer".to_string()),
        aud: Some("test-audience".to_string()),
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
    };

    let token = create_test_jwt_hs256(&claims);

    // Validate the token
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["test-audience"]);
    validation.set_issuer(&["test-issuer"]);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_ok());
    let token_data = result.unwrap();
    assert_eq!(token_data.claims.sub, "user123");
    assert_eq!(
        token_data.claims.email,
        Some("user@example.com".to_string())
    );
}

#[tokio::test]
async fn test_expired_token_hs256() {
    let secret = create_test_secret();

    // Create expired claims
    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize, // Expired
        iat: (chrono::Utc::now() - chrono::Duration::hours(2)).timestamp() as usize,
        iss: None,
        aud: None,
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
    };

    let token = create_test_jwt_hs256(&claims);

    // Try to validate the expired token
    let validation = Validation::new(Algorithm::HS256);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_err());
    // The error should be about expiration
    let error_str = format!("{:?}", result.unwrap_err());
    assert!(error_str.contains("ExpiredSignature"));
}

#[tokio::test]
async fn test_wrong_audience_hs256() {
    let secret = create_test_secret();

    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        iss: None,
        aud: Some("wrong-audience".to_string()),
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
    };

    let token = create_test_jwt_hs256(&claims);

    // Validate with different expected audience
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["expected-audience"]);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_err());
    let error_str = format!("{:?}", result.unwrap_err());
    assert!(error_str.contains("InvalidAudience"));
}

#[tokio::test]
async fn test_wrong_issuer_hs256() {
    let secret = create_test_secret();

    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        iss: Some("wrong-issuer".to_string()),
        aud: None,
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
    };

    let token = create_test_jwt_hs256(&claims);

    // Validate with different expected issuer
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["expected-issuer"]);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_err());
    let error_str = format!("{:?}", result.unwrap_err());
    assert!(error_str.contains("InvalidIssuer"));
}

#[tokio::test]
async fn test_auth_user_extraction_missing_header() {
    let config = AuthConfig {
        enabled: true,
        jwks_url: Some("http://example.com/jwks".to_string()),
        audience: None,
        issuer: None,
    };

    let validator = Arc::new(JwtValidator {
        config,
        decoding_keys: Arc::new(HashMap::new()),
    });

    async fn protected_route(user: AuthUser) -> String {
        format!("Hello, {}!", user.sub)
    }

    let app = Router::new()
        .route("/protected", get(protected_route))
        .with_state(validator.clone());

    // Request without Authorization header
    let request = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_user_extraction_invalid_header_format() {
    let config = AuthConfig {
        enabled: true,
        jwks_url: Some("http://example.com/jwks".to_string()),
        audience: None,
        issuer: None,
    };

    let validator = Arc::new(JwtValidator {
        config,
        decoding_keys: Arc::new(HashMap::new()),
    });

    async fn protected_route(user: AuthUser) -> String {
        format!("Hello, {}!", user.sub)
    }

    let app = Router::new()
        .route("/protected", get(protected_route))
        .with_state(validator.clone());

    // Request with invalid Authorization header format (missing "Bearer ")
    let request = Request::builder()
        .uri("/protected")
        .header(AUTHORIZATION, "InvalidToken")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_user_extraction_disabled_auth() {
    let config = AuthConfig {
        enabled: false,
        jwks_url: None,
        audience: None,
        issuer: None,
    };

    let validator = Arc::new(JwtValidator::new(config).await.unwrap());

    async fn protected_route(user: AuthUser) -> String {
        format!("Hello, {}!", user.sub)
    }

    let app = Router::new()
        .route("/protected", get(protected_route))
        .with_state(validator.clone());

    // Request without Authorization header should work when auth is disabled
    let request = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "Hello, test-user!");
}

#[tokio::test]
async fn test_jwks_fetch_failure() {
    // Mock server that returns error
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let config = AuthConfig {
        enabled: true,
        jwks_url: Some(mock_server.uri()),
        audience: None,
        issuer: None,
    };

    let result = JwtValidator::new(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwks_with_no_valid_keys() {
    let mock_server = MockServer::start().await;

    // JWKS with no keys
    let jwks = JwkSet { keys: vec![] };

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
        .mount(&mock_server)
        .await;

    let config = AuthConfig {
        enabled: true,
        jwks_url: Some(mock_server.uri()),
        audience: None,
        issuer: None,
    };

    let result = JwtValidator::new(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwks_with_invalid_key() {
    let mock_server = MockServer::start().await;

    // Create JWK with invalid parameters - just return raw JSON
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "kid": "invalid-key",
            "n": "invalid-base64",
            "e": "invalid"
        }]
    });

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
        .mount(&mock_server)
        .await;

    let config = AuthConfig {
        enabled: true,
        jwks_url: Some(mock_server.uri()),
        audience: None,
        issuer: None,
    };

    // Should succeed but with no valid keys
    let result = JwtValidator::new(config).await;
    assert!(result.is_err()); // No valid keys parsed
}

#[tokio::test]
async fn test_malformed_jwt() {
    // Test with a mock validator that has empty keys
    let config = AuthConfig {
        enabled: true,
        jwks_url: Some("http://example.com/jwks".to_string()),
        audience: None,
        issuer: None,
    };

    let validator = JwtValidator {
        config,
        decoding_keys: Arc::new(HashMap::new()),
    };

    // Test with malformed JWT
    let result = validator.validate_token("not.a.jwt").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ApiError::Unauthorized => {}
        _ => panic!("Expected Unauthorized error for malformed JWT"),
    }
}

#[tokio::test]
async fn test_invalid_signature_hs256() {
    let secret = create_test_secret();

    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        iss: None,
        aud: None,
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
    };

    let mut token = create_test_jwt_hs256(&claims);

    // Corrupt the signature part of the JWT
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3 {
        token = format!("{}.{}.corrupted_signature", parts[0], parts[1]);
    }

    // Try to validate with invalid signature
    let validation = Validation::new(Algorithm::HS256);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_err());
    let error_str = format!("{:?}", result.unwrap_err());
    assert!(error_str.contains("InvalidSignature"));
}

#[tokio::test]
async fn test_health_check_no_auth_required() {
    let pool = common::setup_test_db().await;
    let mock_server = MockServer::start().await;
    let app = setup_app_with_auth(pool, &mock_server).await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_routes_require_auth() {
    let pool = common::setup_test_db().await;
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

#[tokio::test]
async fn test_missing_claims_hs256() {
    let secret = create_test_secret();

    // Create claims with missing optional fields
    let claims = Claims {
        sub: "user123".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
        iss: None,
        aud: None,
        email: None, // Missing email
        name: None,  // Missing name
    };

    let token = create_test_jwt_hs256(&claims);

    // Validate token with missing optional claims
    let validation = Validation::new(Algorithm::HS256);

    let result = decode::<Claims>(&token, &DecodingKey::from_secret(&secret), &validation);

    assert!(result.is_ok());
    let token_data = result.unwrap();
    assert_eq!(token_data.claims.sub, "user123");
    assert_eq!(token_data.claims.email, None);
    assert_eq!(token_data.claims.name, None);
}

#[tokio::test]
async fn test_validator_with_mock_jwks() {
    let mock_server = MockServer::start().await;

    // Create a valid RSA JWK (this is a real RSA public key in JWK format)
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "kid": "test-key-1",
            "use": "sig",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbIS",
            "e": "AQAB",
            "alg": "RS256"
        }]
    });

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
        .mount(&mock_server)
        .await;

    let config = AuthConfig {
        enabled: true,
        jwks_url: Some(mock_server.uri()),
        audience: None,
        issuer: None,
    };

    // Should successfully create validator with valid JWK
    let result = JwtValidator::new(config).await;
    assert!(result.is_ok());

    let validator = result.unwrap();
    // Validator should have loaded the key
    assert_eq!(validator.decoding_keys.len(), 1);
    assert!(validator.decoding_keys.contains_key("test-key-1"));
}
