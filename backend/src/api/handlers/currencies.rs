use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;

use crate::{
    api::AppState,
    db::queries::currency::{create_currency, list_currencies},
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
            "Currency code must be 3 uppercase letters".to_string(),
        ));
    }

    let currency = create_currency(&state.pool, &input.code).await?;

    Ok((StatusCode::CREATED, Json(currency)))
}

pub async fn handle_list_currencies(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Currency>>> {
    let currencies = list_currencies(&state.pool).await?;
    Ok(Json(currencies))
}
