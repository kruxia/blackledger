use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::{
    api::{AppState, pagination::PaginatedResponse, search::CurrencySearchParams},
    db::queries::currency::{count_currencies, create_currencies_batch, search_currencies},
    error::ApiResult,
    models::currency::Currency,
};

#[derive(Debug, Deserialize)]
pub struct CreateCurrencyRequest {
    pub code: String,
}

pub async fn handle_create_currencies(
    State(state): State<AppState>,
    Json(input): Json<Vec<CreateCurrencyRequest>>,
) -> ApiResult<(StatusCode, Json<Vec<Currency>>)> {
    // Validate all currency codes first
    for req in &input {
        if !Currency::is_valid_code(&req.code) {
            return Err(crate::error::ApiError::Validation(format!(
                "Invalid currency code: {}",
                req.code
            )));
        }
    }

    let codes: Vec<String> = input.into_iter().map(|req| req.code).collect();
    let currencies = create_currencies_batch(&state.pool, &codes).await?;

    Ok((StatusCode::CREATED, Json(currencies)))
}

pub async fn handle_search_currencies(
    State(state): State<AppState>,
    Query(params): Query<CurrencySearchParams>,
) -> ApiResult<Json<PaginatedResponse<Currency>>> {
    // Run search and count queries concurrently
    let (currencies_result, count_result) = tokio::join!(
        search_currencies(&state.pool, &params),
        count_currencies(&state.pool, &params)
    );

    let currencies = currencies_result?;
    let total = count_result?;

    let pagination = params.base.to_pagination_params();
    let response = PaginatedResponse::new(currencies, &pagination, Some(total));
    Ok(Json(response))
}
