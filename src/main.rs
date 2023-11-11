use crate::api::api_router;
use axum::Router;

pub mod api;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().nest("/api", api_router());

    // run it with hyper on localhost:8000
    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
