//! src/authentication.rs

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use deadpool_postgres::Client;
use secrecy::{ExposeSecret, Secret};

use crate::cornucopia::queries::newsletters::query_user_id_by_credentials;
use crate::cornucopia::queries::newsletters::QueryUserIdByCredentials;
use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[allow(dead_code)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

// You might have also noticed that we no longer deal with the
// salt directly - PHC string format takes care of it for us, implicitly.
#[tracing::instrument(
    name = "Validate credentials",
    skip(username, password, client)
)]
pub async fn validate_credentials(
    Credentials { username, password }: Credentials,
    client: &Client,
) -> Result<uuid::Uuid, AuthError> {
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
                return Err(AuthError::InvalidCredentials(anyhow::anyhow!(
                    "Unknown username: {e}"
                )))
            }
        };

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, password)
    })
    .await
    .context("Invalid password.")
    .map_err(AuthError::UnexpectedError)??;

    user_id.ok_or_else(|| {
        // We don't tell that it is unknown username
        AuthError::InvalidCredentials(anyhow::anyhow!("Failed to auth."))
    })
}

#[tracing::instrument(name = "Get stored credentials", skip(username, client))]
async fn get_stored_credentials(
    username: &str,
    client: &Client,
) -> Result<Option<QueryUserIdByCredentials>, AuthError> {
    query_user_id_by_credentials()
        .bind(client, &username)
        .opt()
        .await
        .context("Failed to perform a query to validate auth credentials.")
        .map_err(AuthError::UnexpectedError)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash =
        PasswordHash::new(&expected_password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")
            .map_err(AuthError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}
