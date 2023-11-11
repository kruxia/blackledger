use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

const NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const REPOSITORY: Option<&str> = option_env!("CARGO_PKG_REPOSITORY");

#[derive(Serialize)]
struct Home {
    name: String,
    version: String,
    repository: String,
}

pub async fn home() -> Json<Value> {
    Json(json!(Home {
        name: String::from(NAME.unwrap_or("NULL")),
        repository: String::from(REPOSITORY.unwrap_or("NULL")),
        version: String::from(VERSION.unwrap_or("NULL")),
    }))
}
