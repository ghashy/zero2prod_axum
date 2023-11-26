use axum::extract::State;
use axum::Form;

use hyper::StatusCode;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use tokio_postgres::Transaction;
use uuid::Uuid;

use crate::connection_pool::ConnectionPool;
use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use crate::startup::AppState;

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip_all,
    fields(
        request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
    level = "info"
)]
pub async fn subscribe_handler(
    State(state): State<AppState>,
    form: Form<FormData>,
) -> StatusCode {
    let request_id = uuid::Uuid::new_v4();
    tracing::Span::current().record("request_id", request_id.to_string());

    // Convert `&FormData` into a `NewSubscriber`.
    let new_subscriber = match (&form.0).try_into() {
        Ok(sub) => sub,
        Err(e) => {
            return {
                tracing::warn!("Bad request, error: {e}");
                StatusCode::BAD_REQUEST
            }
        }
    };

    let pool = state.pool;

    tracing::info!(
        "Adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    let mut connection = match pool.get().await.map_err(|e| match e {
        bb8::RunError::User(e) => e.to_string(),
        bb8::RunError::TimedOut => String::from("Error: bb8::TimedOut"),
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get PG connection from pool: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let mut transaction = match connection.transaction().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "Failed to start transaction on pg connection: {e}"
            );
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    if let Err(e) =
        insert_subscriber_to_db(&new_subscriber, &mut transaction, request_id)
            .await
    {
        tracing::error!("Failed to execute query: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let subscription_token = generate_subscription_token();
    if let Err(e) =
        store_token(&mut transaction, request_id, &subscription_token).await
    {
        tracing::error!("Failed to store token in db, error: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // We are ignoring email delivery errors for now.
    if let Err(e) = send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    {
        tracing::error!("Failed to send confirmation email, error: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(e) = transaction.commit().await {
        tracing::error!("Failed to send confirmation email, error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        tracing::info!("New subscriber details have been saved");
        StatusCode::OK
    }
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

/// Error can happen in both cases: when trying to get a connection to Postgres
/// from the `pool` and when trying to `await` Postgres query, so return error
/// type is `String`.
#[tracing::instrument(skip_all)]
async fn insert_subscriber_to_db<'a>(
    subscriber: &NewSubscriber,
    transaction: &mut Transaction<'a>,
    id: uuid::Uuid,
) -> Result<(), String> {
    transaction
        .query_opt(
            r#"INSERT INTO subscriptions(id, email, name, subscribed_at, status)
                       VALUES ($1, $2, $3, $4, 'pending_confirmation')
                    "#,
            &[
                // These types implements `ToSql` trait.
                &id,
                &subscriber.email.as_ref(),
                &subscriber.name.as_ref(),
                &std::time::SystemTime::now(),
            ],
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tracing::instrument(
    name = "Store subscription token in the database"
    skip(subscription_token, transaction)
)]
async fn store_token<'a>(
    transaction: &mut Transaction<'a>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), String> {
    transaction
        .query(
            r#"INSERT INTO subscription_tokens
                (subscription_token, subscriber_id)
              VALUES ($1, $2)"#,
            &[&subscription_token, &subscriber_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Using 25 characters we get roughly ~10^45 possible tokens -
/// it should be more than enough for our use case.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
