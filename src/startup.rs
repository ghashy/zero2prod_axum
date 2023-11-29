use axum::routing;
use axum::Router;
use tokio::net::TcpListener;

use axum::serve::Serve;

use bb8_postgres::PostgresConnectionManager;
use secrecy::ExposeSecret;

use crate::configuration::Settings;
use crate::connection_pool::ConnectionPool;
use crate::email_client::EmailClient;
use crate::routes::confirm;
use crate::routes::get_hello;
use crate::routes::health_check;
use crate::routes::subscribe_handler;

mod db_migration;

/// This is a central type of our codebase. `Application` type builds server
/// for both production and testing purposes.
pub struct Application {
    port: u16,
    serve: Serve<Router, Router>,
}

/// Shareable type, we insert it to the main `Router` as state,
/// at the launch stage.
#[derive(Clone)]
pub struct AppState {
    pub base_url: String,
    pub pool: ConnectionPool,
    pub email_client: EmailClient,
}

impl Application {
    /// Build a new server.
    ///
    /// This functions builds a new `Application` with given configuration.
    /// It also configures a pool of connections to the PostgreSQL database.
    pub async fn build(
        configuration: Settings,
    ) -> Result<Application, std::io::Error> {
        let postgres_connection =
            get_postgres_connection_pool(&configuration).await;

        db_migration::run_migration(&postgres_connection).await;

        let timeout = configuration.email_client.timeout_millis();

        let sender_email =
            configuration.email_client.sender().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
            configuration.email_delivery_service,
        )
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let address =
            format!("{}:{}", configuration.app_addr, configuration.app_port);
        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr()?.port();

        let serve = Self::build_server(
            &configuration.app_base_url,
            listener,
            postgres_connection,
            email_client,
        );

        Ok(Self { serve, port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// This function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.serve.await?;
        Ok(())
    }

    /// Configure `Server`.
    fn build_server(
        base_url: &str,
        listener: TcpListener,
        pool: ConnectionPool,
        email_client: EmailClient,
    ) -> Serve<Router, Router> {
        // We do not wrap pool into arc because internally it alreaday has an
        // `Arc`, and copying is cheap.
        let app_state = AppState {
            pool,
            email_client,
            base_url: base_url.to_string(),
        };
        let app =
            Router::new()
                .route("/health_check", routing::get(health_check))
                .route("/hello", routing::get(get_hello))
                .route("/subscriptions", routing::post(subscribe_handler))
                .route("/subscriptions/confirm", routing::get(confirm))
                // DEBUG:
                .fallback(routing::get(
                    |uri: axum::http::Uri,
                     orig_uri: axum::extract::OriginalUri| async move {
                        let uri = uri.path();
                        let orig_uri = orig_uri.path();
                        format!("uri: {}\norig_uri: {}", uri, orig_uri)
                    },
                ))
                .with_state(app_state);

        axum::serve(listener, app)
    }
}

/// Returns a connection pool to the PostgreSQL database.
async fn get_postgres_connection_pool(
    configuration: &Settings,
) -> ConnectionPool {
    let manager = PostgresConnectionManager::new_from_stringlike(
        configuration.database.connection_string().expose_secret(),
        tokio_postgres::NoTls,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}
