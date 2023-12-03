//! src/routes/subscriptions_confirm.rs

use axum::extract::{Query, State};
use deadpool_postgres::Client;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{
    cornucopia::queries::subscriptions, startup::AppState,
    validation::subscriber_token::SubscriberToken,
};

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters, state)
)]
pub async fn confirm(
    State(state): State<AppState>,
    Query(parameters): Query<Parameters>,
) -> StatusCode {
    let subscriber_token =
        match SubscriberToken::parse(&parameters.subscription_token) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to parse subscriber token {}", e);
                return StatusCode::BAD_REQUEST;
            }
        };

    let client = match state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get db connection from pool: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let subscriber_id =
        match get_subscriber_id_from_token(&client, &subscriber_token).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                tracing::warn!("Attempt to confirm unexistent token");
                return StatusCode::NOT_FOUND;
            }
            Err(e) => {
                tracing::error!("Failed to load token from db, error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        };
    if let Err(e) = confirm_subscriber(&client, subscriber_id).await {
        tracing::error!(
            "Failed to change status to \'confirmed\' in db, error: {e}"
        );
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    tracing::info!("Subscriber with uuid: {} confirmed", subscriber_id);
    return StatusCode::OK;
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, client)
)]
async fn get_subscriber_id_from_token(
    client: &Client,
    subscription_token: &SubscriberToken,
) -> Result<Option<Uuid>, tokio_postgres::Error> {
    let id = subscriptions::get_subscriber_id_from_token()
        .bind(client, &subscription_token.as_ref())
        .opt()
        .await?;
    Ok(id)
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed",
    skip(subscriber_id, client)
)]
async fn confirm_subscriber(
    client: &Client,
    subscriber_id: Uuid,
) -> Result<(), tokio_postgres::Error> {
    subscriptions::confirm_subscriber()
        .bind(client, &subscriber_id)
        .await?;
    Ok(())
}
