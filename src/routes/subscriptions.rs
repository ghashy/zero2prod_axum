use axum::extract::State;
use axum::Form;

use hyper::StatusCode;

use serde::Deserialize;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::connection_pool::ConnectionPool;

// ───── Body ─────────────────────────────────────────────────────────────── //

#[derive(Deserialize, Debug)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
    level = "info"
)]
pub async fn subscribe_handler(
    State(pool): State<ConnectionPool>,
    form: Form<FormData>,
) -> Result<StatusCode, (StatusCode, String)> {
    let request_id = uuid::Uuid::new_v4();
    tracing::Span::current().record("request_id", request_id.to_string());
    tracing::info!(
        "Adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    match insert_subscriber(form, pool, request_id).await {
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
#[tracing::instrument(
    name = "Saving new subscriber details to the Database",
    skip(pool, email, name)
)]
async fn insert_subscriber(
    Form(FormData { email, name }): Form<FormData>,
    pool: ConnectionPool,
    id: uuid::Uuid,
) -> Result<(), String> {
    let connection = pool.get().await.map_err(|e| match e {
        bb8::RunError::User(e) => e.to_string(),
        bb8::RunError::TimedOut => String::from("Error: bb8::TimedOut"),
    })?;
    connection
        .query_opt(
            r#"INSERT INTO subscriptions(id, email, name, subscribed_at)
                       VALUES ($1, $2, $3, $4)
                    "#,
            &[&id, &email, &name, &std::time::SystemTime::now()],
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e.to_string()
        })?;
    Ok(())
}
