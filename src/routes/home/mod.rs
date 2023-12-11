//! src/routes/home/mod.rs

use axum::{body::Body, response::IntoResponse};
use http::{header::CONTENT_TYPE, HeaderMap, StatusCode};

pub async fn home() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/html".parse().unwrap());
    (
        StatusCode::OK,
        headers,
        Body::new(include_str!("../../../html/home.html").to_string()),
    )
}
