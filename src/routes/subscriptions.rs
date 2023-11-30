use axum::extract::State;
use axum::Form;

use hyper::StatusCode;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use tokio_postgres::error::SqlState;
use tokio_postgres::Transaction;
use uuid::Uuid;

use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use crate::startup::AppState;

// TODO: WRITE HOW IT WORKS VERY DETAILED
// 1. We get request with form: email, name. If not correct return BAD_REQUEST.
// 2. We check database: is that email in db already? We check subscriber stataus.
// 3. If there are no such email in db, we generate unique token, generate request id (subscriber_id), and store them in db.
// 4. If there are such email, check its status:
//     - If pending, we update token, and send new email with this confirmation token.
//     - If confirmed, return CONFLICT response.

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[derive(PartialEq, Eq, Debug)]
enum SubscriberStatus {
    NonExisting,
    Pending,
    Confirmed,
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

    // TRY to convert `&FormData` into a `NewSubscriber`.
    let new_subscriber = match (&form.0).try_into() {
        Ok(sub) => sub,
        Err(e) => {
            return {
                tracing::warn!("Bad request, error: {e}");
                StatusCode::BAD_REQUEST
            }
        }
    };

    tracing::info!(
        "Adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    // TRY to get connection from pool
    let mut connection = match state.pool.get().await.map_err(|e| match e {
        bb8::RunError::User(e) => e.to_string(),
        bb8::RunError::TimedOut => String::from("Error: bb8::TimedOut"),
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get PG connection from pool: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    // TRY to start a new transaction on db
    let mut transaction = match connection.transaction().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "Failed to start transaction on pg connection: {e}"
            );
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    // Check current subscriber status
    let subscriber_status =
        match get_subscriber_status(&mut transaction, &new_subscriber).await {
            Ok(s) => {
                tracing::info!("Subscriber status: {:?}", s);
                s
            }
            Err(e) => {
                tracing::error!("Db error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        };

    let subscription_token = generate_subscription_token();

    match subscriber_status {
        SubscriberStatus::NonExisting => {
            match insert_subscriber_to_db(
                &new_subscriber,
                &mut transaction,
                request_id,
            )
            .await
            {
                Ok(()) => {
                    tracing::info!("New subscriber details have been saved")
                }
                Err(e) => {
                    tracing::error!("Failed to execute query: {:?}", e);
                    if e.code().is_some_and(|sqlstate| {
                        *sqlstate == SqlState::UNIQUE_VIOLATION
                    }) {
                        return StatusCode::CONFLICT;
                    } else {
                        return StatusCode::INTERNAL_SERVER_ERROR;
                    }
                }
            }

            if let Err(e) =
                store_token(&mut transaction, request_id, &subscription_token)
                    .await
            {
                tracing::error!("Failed to store token in db, error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        SubscriberStatus::Pending => {
            if let Err(e) = update_token(
                &mut transaction,
                new_subscriber.email.as_ref(),
                &subscription_token,
            )
            .await
            {
                tracing::error!("Failed to update token in db, error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        SubscriberStatus::Confirmed => {
            tracing::error!("This email already confirmed: {}", form.email);
            return StatusCode::CONFLICT;
        }
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
        tracing::info!("Db transaction commited!");
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
) -> Result<(), tokio_postgres::error::Error> {
    transaction
        .query_opt(
            "INSERT INTO subscriptions(id, email, name, subscribed_at, status)
                       VALUES ($1, $2, $3, $4, 'pending_confirmation')
                    ",
            &[
                // These types implements `ToSql` trait.
                &id,
                &subscriber.email.as_ref(),
                &subscriber.name.as_ref(),
                &std::time::SystemTime::now(),
            ],
        )
        .await?;
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
) -> Result<(), tokio_postgres::Error> {
    transaction
        .query(
            r#"INSERT INTO subscription_tokens
                (subscription_token, subscriber_id)
              VALUES ($1, $2)"#,
            &[&subscription_token, &subscriber_id],
        )
        .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Update subscriber token in db"
    skip(subscription_token, transaction)
)]
async fn update_token<'a>(
    transaction: &mut Transaction<'a>,
    email: &str,
    subscription_token: &str,
) -> Result<(), tokio_postgres::Error> {
    let rows_modified = transaction
        .execute(
            r#"
            DELETE FROM subscription_tokens
            WHERE subscriber_id = (
                SELECT id
                FROM subscriptions
                WHERE email = $1
            );"#,
            &[&email],
        )
        .await?;
    assert_eq!(rows_modified, 1);
    let rows_modified = transaction
        .execute(
            r#"
            INSERT INTO subscription_tokens
            (subscription_token, subscriber_id)
            VALUES ($1, (
                    SELECT id
                    FROM subscriptions
                    WHERE email = $2
                )
            );"#,
            &[&subscription_token, &email],
        )
        .await?;
    assert_eq!(rows_modified, 1);
    Ok(())
}

#[tracing::instrument(
    name = "Check subscriber current status"
    skip(new_subscriber, transaction)
)]
async fn get_subscriber_status<'a>(
    transaction: &mut Transaction<'a>,
    new_subscriber: &NewSubscriber,
) -> Result<SubscriberStatus, tokio_postgres::Error> {
    let rows = transaction
        .query_opt(
            r#"SELECT status
               FROM subscriptions
               WHERE email = $1"#,
            &[&new_subscriber.email.as_ref()],
        )
        .await?;

    // Return
    if let Some(row) = rows {
        match row.get::<&str, &str>("status") {
            "confirmed" => Ok(SubscriberStatus::Confirmed),
            "pending_confirmation" => Ok(SubscriberStatus::Pending),
            _ => unreachable!(),
        }
    } else {
        Ok(SubscriberStatus::NonExisting)
    }
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
