//! src/routes/subscriptions_confirm.rs

use axum::extract::{Query, State};
use bb8::PooledConnection;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{connection_pool::ConnectionPool, startup::AppState};

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
    let subscriber_id = match get_subscriber_id_from_token(
        &state.pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to load token from db, error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    if let Err(e) = confirm_subscriber(&state.pool, subscriber_id).await {
        tracing::error!(
            "Failed to change status to \'confirmed\' in db, error: {e}"
        );
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    return StatusCode::OK;
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, pool)
)]
async fn get_subscriber_id_from_token(
    pool: &ConnectionPool,
    subscription_token: &str,
) -> Result<Uuid, String> {
    let result = pool.get().await.map_err(|e| e.to_string() )?.query(r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#, &[&subscription_token]).await.map_err(|e| e.to_string())?;
    let row = result
        .get(0)
        .ok_or(String::from("Failed to get [0] value from row after query"))?;
    let subscriber_id = row
        .try_get::<&str, Uuid>("subscriber_id")
        .map_err(|e| e.to_string())?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed",
    skip(subscriber_id, pool)
)]
async fn confirm_subscriber(
    pool: &ConnectionPool,
    subscriber_id: Uuid,
) -> Result<(), String> {
    let _ = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .query(
            r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
            &[&subscriber_id],
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
