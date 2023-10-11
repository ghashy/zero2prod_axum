use axum::response::IntoResponse;

use hyper::Body;
use hyper::Request;
use hyper::StatusCode;

// ───── Body ─────────────────────────────────────────────────────────────── //

pub async fn health_check(_: Request<Body>) -> impl IntoResponse {
    StatusCode::OK
}

pub async fn get_hello(_: Request<Body>) -> impl IntoResponse {
    (StatusCode::OK, "Hello from rust-backend!")
}
