use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::{
    api::{AppState, search::CurrencySearchParams},
    db::queries::currency::{create_currency, search_currencies},
    error::ApiResult,
    models::currency::Currency,
};

#[derive(Debug, Deserialize)]
pub struct CreateCurrencyRequest {
    pub code: String,
}

pub async fn handle_create_currency(
    State(state): State<AppState>,
    Json(input): Json<CreateCurrencyRequest>,
) -> ApiResult<(StatusCode, Json<Currency>)> {
    // Validate currency code
    if !Currency::is_valid_code(&input.code) {
        return Err(crate::error::ApiError::Validation(
            "Invalid currency code".to_string(),
        ));
    }

    let currency = create_currency(&state.pool, &input.code).await?;

    Ok((StatusCode::CREATED, Json(currency)))
}

pub async fn handle_search_currencies(
    State(state): State<AppState>,
    Query(params): Query<CurrencySearchParams>,
) -> ApiResult<Json<Vec<Currency>>> {
    let currencies = search_currencies(&state.pool, &params).await?;
    Ok(Json(currencies))
}
