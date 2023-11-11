use crate::api::home::home;
use axum::{routing::get, Router};

pub mod api;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/api", get(home));

    // run it with hyper on localhost:8000
    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
