//! src/routes/login/get.rs

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Form;
use http::header::LOCATION;
use http::{HeaderMap, StatusCode};
use secrecy::Secret;

use crate::authentication;
use crate::authentication::validate_credentials;
use crate::authentication::Credentials;
use crate::error_chain_fmt;
use crate::startup::AppState;

#[derive(serde::Deserialize, Debug)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        let mut headers = HeaderMap::new();
        headers.insert(
            LOCATION,
            format!("/login?error={}", encoded_error).parse().unwrap(),
        );
        headers.into_response()
        // match self {
        //     LoginError::AuthError(_) => {
        //         StatusCode::UNAUTHORIZED.into_response()
        //     }
        //     LoginError::UnexpectedError(_) => {
        //         StatusCode::INTERNAL_SERVER_ERROR.into_response()
        //     }
        // PublishError::UnexpectedError(_) => {
        //     StatusCode::INTERNAL_SERVER_ERROR.into_response()
        // }
        // PublishError::InternalError => {
        //     StatusCode::INTERNAL_SERVER_ERROR.into_response()
        // }
        // PublishError::AuthError(_) => axum::response::Response::builder()
        //     .status(StatusCode::UNAUTHORIZED)
        //     .header(
        //         http::header::WWW_AUTHENTICATE,
        //         http::HeaderValue::from_str(r#"Basic realm="publish""#)
        //             .unwrap(),
        //     )
        //     .body(axum::body::Body::empty())
        //     .unwrap(),
        // }
    }
}

#[tracing::instrument(
    skip(state, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    form: Form<FormData>,
) -> Result<Response, LoginError> {
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, "/".parse().unwrap());
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));

    let connection = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get connection from pool: {}", e);
            return Ok(
                (StatusCode::INTERNAL_SERVER_ERROR, headers).into_response()
            );
        }
    };

    tracing::info!("Checking credentials");
    let user_id = validate_credentials(credentials, &connection)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => {
                LoginError::AuthError(e.into())
            }
            authentication::AuthError::UnexpectedError(_) => {
                LoginError::UnexpectedError(e.into())
            }
        })?;
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
    tracing::info!("Redirect to /");
    Ok((StatusCode::SEE_OTHER, headers).into_response())
}
