use axum::response::IntoResponse;

use axum::body::Body;
use hyper::Request;
use hyper::StatusCode;

pub async fn health_check(_: Request<Body>) -> impl IntoResponse {
    tracing::info!("Healthy!");
    StatusCode::OK
}

pub async fn get_hello(_: Request<Body>) -> impl IntoResponse {
    (StatusCode::OK, "Hello from rust-backend!")
}
