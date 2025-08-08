use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ApiError;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwks_url: Option<String>,
    pub audience: Option<String>,
    pub issuer: Option<String>,
}

#[derive(Clone)]
pub struct JwtValidator {
    pub config: AuthConfig,
    pub decoding_keys: Arc<HashMap<String, DecodingKey>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: Option<String>,
    pub aud: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl JwtValidator {
    pub async fn new(config: AuthConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let decoding_keys = if config.enabled && config.jwks_url.is_some() {
            let jwks_url = config.jwks_url.as_ref().unwrap();
            let jwks = fetch_jwks(jwks_url).await?;
            let keys = precompute_decoding_keys(jwks)?;
            Arc::new(keys)
        } else {
            Arc::new(HashMap::new())
        };

        Ok(Self {
            config,
            decoding_keys,
        })
    }

    pub async fn validate_token(&self, token: &str) -> Result<Claims, ApiError> {
        if !self.config.enabled {
            // If auth is disabled, return a dummy claim
            return Ok(Claims {
                sub: "anonymous".to_string(),
                exp: 0,
                iat: 0,
                iss: None,
                aud: None,
                email: Some("anonymous@localhost".to_string()),
                name: Some("Anonymous User".to_string()),
            });
        }

        let header = decode_header(token).map_err(|_| ApiError::Unauthorized)?;

        let kid = header.kid.ok_or_else(|| ApiError::Unauthorized)?;

        let decoding_key = self
            .decoding_keys
            .get(&kid)
            .ok_or_else(|| ApiError::Unauthorized)?;

        let mut validation = Validation::default();

        if let Some(ref aud) = self.config.audience {
            validation.set_audience(&[aud]);
        }

        if let Some(ref iss) = self.config.issuer {
            validation.set_issuer(&[iss]);
        }

        let token_data = decode::<Claims>(token, decoding_key, &validation)
            .map_err(|_| ApiError::Unauthorized)?;

        Ok(token_data.claims)
    }
}

fn precompute_decoding_keys(
    jwks: JwkSet,
) -> Result<HashMap<String, DecodingKey>, Box<dyn std::error::Error>> {
    let mut keys = HashMap::new();

    for jwk in jwks.keys {
        if let Some(kid) = &jwk.common.key_id {
            match DecodingKey::from_jwk(&jwk) {
                Ok(decoding_key) => {
                    keys.insert(kid.clone(), decoding_key);
                }
                Err(e) => {
                    tracing::warn!("Failed to create decoding key for kid {}: {}", kid, e);
                }
            }
        }
    }

    if keys.is_empty() {
        return Err("No valid keys found in JWKS".into());
    }

    tracing::info!("Precomputed {} decoding keys from JWKS", keys.len());
    Ok(keys)
}

async fn fetch_jwks(url: &str) -> Result<JwkSet, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let jwks = response.json::<JwkSet>().await?;
    Ok(jwks)
}

// Axum extractor for authenticated user
#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    Arc<JwtValidator>: axum::extract::FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let validator = Arc::<JwtValidator>::from_ref(state);

        // If auth is disabled (for testing), return a test user
        if !validator.config.enabled {
            return Ok(AuthUser {
                sub: "test-user".to_string(),
                email: Some("test@example.com".to_string()),
                name: Some("Test User".to_string()),
            });
        }

        // Try to extract the Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response()
            })?;

        // Extract the token from "Bearer <token>"
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Invalid authorization header").into_response()
        })?;

        // Validate the token
        let claims = validator
            .validate_token(token)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token").into_response())?;

        Ok(AuthUser {
            sub: claims.sub,
            email: claims.email,
            name: claims.name,
        })
    }
}
