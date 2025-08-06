use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwks_url: Option<String>,
    pub audience: Option<String>,
    pub issuer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JwtValidator {
    pub config: AuthConfig,
    pub jwks: Arc<RwLock<Option<JwkSet>>>,
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
        let jwks = if config.enabled && config.jwks_url.is_some() {
            let jwks_url = config.jwks_url.as_ref().unwrap();
            let jwks = fetch_jwks(jwks_url).await?;
            Arc::new(RwLock::new(Some(jwks)))
        } else {
            Arc::new(RwLock::new(None))
        };

        Ok(Self { config, jwks })
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

        let header = decode_header(token)
            .map_err(|_| ApiError::Unauthorized)?;

        let kid = header.kid
            .ok_or_else(|| ApiError::Unauthorized)?;

        let jwks = self.jwks.read().await;
        let jwks = jwks.as_ref()
            .ok_or_else(|| ApiError::Unauthorized)?;

        let jwk = jwks.find(&kid)
            .ok_or_else(|| ApiError::Unauthorized)?;

        let decoding_key = DecodingKey::from_jwk(jwk)
            .map_err(|_| ApiError::Unauthorized)?;

        let mut validation = Validation::default();
        
        if let Some(ref aud) = self.config.audience {
            validation.set_audience(&[aud]);
        }
        
        if let Some(ref iss) = self.config.issuer {
            validation.set_issuer(&[iss]);
        }

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|_| ApiError::Unauthorized)?;

        Ok(token_data.claims)
    }

    pub async fn refresh_jwks(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref jwks_url) = self.config.jwks_url {
            let new_jwks = fetch_jwks(jwks_url).await?;
            let mut jwks = self.jwks.write().await;
            *jwks = Some(new_jwks);
        }
        Ok(())
    }
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
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract the Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response()
            })?;

        // Extract the token from "Bearer <token>"
        let _token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, "Invalid authorization header").into_response()
            })?;

        // For now, just extract basic info from the token
        // In a real implementation, this would validate against the JwtValidator
        Ok(AuthUser {
            sub: "user".to_string(),
            email: Some("user@example.com".to_string()),
            name: Some("Test User".to_string()),
        })
    }
}

// Optional auth extractor that doesn't fail if no auth header is present
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuthUser(Some(user))),
            Err(_) => Ok(OptionalAuthUser(None)),
        }
    }
}