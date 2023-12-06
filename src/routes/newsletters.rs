use anyhow::Context;
use axum::extract::{FromRequestParts, Request};
use axum::Json;
use axum::{extract::State, response::IntoResponse};
use deadpool_postgres::Client;
use http::HeaderMap;
use hyper::StatusCode;
use secrecy::Secret;

use crate::domain::SubscriberEmail;
use crate::{
    cornucopia::queries::newsletters::query_confirmed_subscribers,
    error_chain_fmt, startup::AppState,
};

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
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            PublishError::InternalError => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(
    headers: &HeaderMap,
) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix(("Basic"))
        .context("The authorization scheme was not 'Basic");
    let decoded_bytes =
    // FIXME engine should be cached
        base64::engine::GeneralPurpose::new(). (base64encoded_segment, base64::STANDARD)
            .context("The decoded credential string is not a valid UTF 8.")?;

    todo!()
}

#[tracing::instrument(
    name = "Handle request to publish newsletters",
    skip(state, body)
)]
#[axum::debug_handler]
pub async fn publish_newsletters(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<StatusCode, PublishError> {
    let _credentials = basic_authentication(&headers);
    let connection = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get connection from pool: {}", e);
            return Err(PublishError::InternalError);
        }
    };
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
