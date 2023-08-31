use axum::async_trait;
use axum::extract::FromRef;
use axum::extract::FromRequestParts;

use hyper::http::request::Parts;
use hyper::StatusCode;

use postgres_openssl::MakeTlsConnector;

use bb8::Pool;
use bb8::PooledConnection;
use bb8_postgres::PostgresConnectionManager;

// ───── Body ─────────────────────────────────────────────────────────────── //

pub type ConnectionPool = Pool<PostgresConnectionManager<MakeTlsConnector>>;

/// Custom extractor that grabs a connection from the pool
/// which setup is appropriate depends on your application.
pub struct DatabaseConnection(
    pub PooledConnection<'static, PostgresConnectionManager<MakeTlsConnector>>,
);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    ConnectionPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let pool = ConnectionPool::from_ref(state);
        let connection = pool.get_owned().await.map_err(internal_error)?;
        Ok(Self(connection))
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    println!("{}", err);
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
