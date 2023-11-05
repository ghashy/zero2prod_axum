use axum::extract::State;
use axum::Form;

use hyper::StatusCode;

use serde::Deserialize;

use crate::connection_pool::ConnectionPool;
use crate::domain::NewSubscriber;
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
) -> Result<StatusCode, (StatusCode, String)> {
    let request_id = uuid::Uuid::new_v4();
    tracing::Span::current().record("request_id", request_id.to_string());

    // Convert `&FormData` into a `NewSubscriber`.
    let new_subscriber = match (&form.0).try_into() {
        Ok(sub) => sub,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e)),
    };

    let pool = state.pool;

    tracing::event!(
        tracing::Level::INFO,
        "Adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    match insert_subscriber_to_db(&new_subscriber, pool, request_id).await {
        Ok(_) => {
            tracing::info!("New subscriber details have been saved");
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
}

/// Error can happen in both cases: when trying to get a connection to Postgres
/// from the `pool` and when trying to `await` Postgres query, so return error
/// type is `String`.
#[tracing::instrument(skip_all)]
async fn insert_subscriber_to_db(
    subscriber: &NewSubscriber,
    pool: ConnectionPool,
    id: uuid::Uuid,
) -> Result<(), String> {
    let connection = pool.get().await.map_err(|e| match e {
        bb8::RunError::User(e) => e.to_string(),
        bb8::RunError::TimedOut => String::from("Error: bb8::TimedOut"),
    })?;
    connection
        .query_opt(
            r#"INSERT INTO subscriptions(id, email, name, subscribed_at, status)
                       VALUES ($1, $2, $3, $4, 'confirmed')
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
