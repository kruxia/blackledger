use crate::api::home::home;
use axum::{routing::get, Router};

pub mod home;

pub fn api_router() -> Router {
    Router::new().route("/", get(home))
}
