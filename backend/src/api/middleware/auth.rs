use axum::{
    body::Body,
    extract::State,
    http::{Request, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::auth::JwtValidator;
use crate::error::ApiError;

pub async fn auth_middleware(
    State(validator): State<Arc<JwtValidator>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // If auth is disabled, just pass through
    if !validator.config.enabled {
        return Ok(next.run(request).await);
    }

    // Extract the Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    // Extract the token from "Bearer <token>"
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;

    // Validate the token
    let _claims = validator.validate_token(token).await?;

    // Token is valid, proceed with the request
    Ok(next.run(request).await)
}
