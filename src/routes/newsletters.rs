use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use deadpool_postgres::Client;
use http::HeaderMap;
use hyper::StatusCode;
use secrecy::{ExposeSecret, Secret};

use crate::cornucopia::queries::newsletters::query_user_id_by_credentials;
use crate::cornucopia::queries::newsletters::{
    query_confirmed_subscribers, QueryUserIdByCredentials,
};
use crate::domain::SubscriberEmail;
use crate::error_chain_fmt;
use crate::startup::AppState;
use crate::telemetry::spawn_blocking_with_tracing;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed: {0}")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Internal error")]
    InternalError,
}

// Same logic to get the full error chain on `Debug`
impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        match self {
            PublishError::UnexpectedError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::InternalError => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::AuthError(_) => axum::response::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    http::header::WWW_AUTHENTICATE,
                    http::HeaderValue::from_str(r#"Basic realm="publish""#)
                        .unwrap(),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        }
    }
}

#[allow(dead_code)]
struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(state, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
#[axum::debug_handler]
pub async fn publish_newsletters(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<StatusCode, PublishError> {
    let credentials =
        match basic_authentication(&headers).map_err(PublishError::AuthError) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to auth: {e}");
                return Err(e);
            }
        };

    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));

    let connection = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get connection from pool: {}", e);
            return Err(PublishError::InternalError);
        }
    };

    let user_id = match validate_credentials(credentials, &connection).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Error: {e}");
            return Err(e);
        }
    };

    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));

    let subscribers = fetch_confirmed_subscribers(connection).await?;
    for subscriber in subscribers.into_iter() {
        state
            .email_client
            .send_email(
                &subscriber,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .map_err(|e| anyhow::Error::new(e))?;
    }
    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(client))]
async fn fetch_confirmed_subscribers(
    client: Client,
) -> Result<Vec<SubscriberEmail>, anyhow::Error> {
    query_confirmed_subscribers()
        .bind(&client)
        .map(|e| match SubscriberEmail::parse(e) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("Failed to parse subscriber email: {}", e);
                None
            }
        })
        .all()
        .await
        .and_then(|v| Ok(v.into_iter().flatten().collect()))
        .context("Failed to fetch confirmed subscribers")
}

// ───── Helpers ──────────────────────────────────────────────────────────── //

/// Great example where `anyhow` is awesome. It can wrap different error types,
/// and return single. It very convenient if we shouldn't sort all kind of errors,
/// but instead just print error to logs.
fn basic_authentication(
    headers: &HeaderMap,
) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic")
        .context("The authorization scheme was not 'Basic")?
        .trim();
    // FIXME engine should be cached
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("The decoded credential string is not a valid UTF 8.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8")?;

    // Split into two segments using ':' as delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A username must be provided in 'Basic' auth.")
        })?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A password must be provided in 'Basic' auth.")
        })?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

// You might have also noticed that we no longer deal with the
// salt directly - PHC string format takes care of it for us, implicitly.
#[tracing::instrument(
    name = "Validate credentials",
    skip(username, password, client)
)]
async fn validate_credentials(
    Credentials { username, password }: Credentials,
    client: &Client,
) -> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) =
        match get_stored_credentials(&username, client).await {
            Ok(Some(query)) => {
                (Some(query.user_id), Secret::new(query.password_hash))
            }
            Ok(None) => (
                None,
                Secret::new(
                    "$argon2id$v=19$m=15000,t=2,p=1$\
            gZiV/M1gPc22ElAH/Jh1Hw$\
            CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                        .to_string(),
                ),
            ),
            Err(e) => {
                return Err(PublishError::AuthError(anyhow::anyhow!(
                    "Unknown username: {e}"
                )))
            }
        };

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, password)
    })
    .await
    .context("Invalid password.")
    .map_err(PublishError::AuthError)??;

    user_id.ok_or_else(|| {
        // We don't tell that it is unknown username
        PublishError::AuthError(anyhow::anyhow!("Failed to auth."))
    })
}

#[tracing::instrument(name = "Get stored credentials", skip(username, client))]
async fn get_stored_credentials(
    username: &str,
    client: &Client,
) -> Result<Option<QueryUserIdByCredentials>, PublishError> {
    query_user_id_by_credentials()
        .bind(client, &username)
        .opt()
        .await
        .context("Failed to perform a query to validate auth credentials.")
        .map_err(PublishError::UnexpectedError)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash =
        PasswordHash::new(&expected_password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")
            .map_err(PublishError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}
