//! src/routes/login/get.rs

use axum::{body::Body, extract::Query, response::IntoResponse};
use http::{header::CONTENT_TYPE, HeaderMap, StatusCode};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(Query(query): Query<QueryParams>) -> impl IntoResponse {
    let error_html = match query.error {
        Some(error_message) => format!("<p><i>{error_message}</i></p>"),
        None => "".into(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/html".parse().unwrap());
    (
        StatusCode::OK,
        headers,
        Body::new(format!(
            r#"<!DOCTYPE html>
                <html lang="en">
                    <head>
                        <meta http-equiv="content-type" content="text/html; charset=utf-8">
                        <title>Login</title>
                    </head>
                    <body>
                        {error_html}
                        <form action="/login" method="post">
                            <label>Username
                                <input
                                    type="text"
                                    placeholder="Enter Username"
                                    name="username"
                                >
                            </label>
                            <label>Password
                                <input
                                    type="password"
                                    placeholder="Enter Password"
                                    name="password"
                                >
                            </label>
                            <button type="submit">Login</button>
                        </form>
                    </body>
                </html>"#
        )),
    )
}
