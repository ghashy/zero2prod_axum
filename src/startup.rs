use std::net::TcpListener;

use axum::routing;
use axum::routing::IntoMakeService;
use axum::Router;
use axum::Server;

use hyper::server::conn::AddrIncoming;

use bb8_postgres::PostgresConnectionManager;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use secrecy::ExposeSecret;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::configuration::Settings;
use crate::connection_pool::ConnectionPool;
use crate::email_client::EmailClient;
use crate::routes::health_check;
use crate::routes::subscribe_handler;

// ───── Body ─────────────────────────────────────────────────────────────── //

/// This is a central type of our codebase. `Application` type builds server
/// for both production and testing purposes.
pub struct Application {
    server: Server<AddrIncoming, IntoMakeService<Router>>,
    #[allow(unused)]
    port: u16,
}

/// Shareable type, we insert it to the main `Router` as state,
/// at the launch stage.
#[derive(Clone)]
pub struct AppState {
    pub pool: ConnectionPool,
    pub email_client: EmailClient,
}

// Public
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
        )
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let address =
            format!("{}:{}", configuration.app_addr, configuration.app_port);
        let listener = TcpListener::bind(address)?;

        let server = Self::run(listener, postgres_connection, email_client);

        let port = server.local_addr().port();

        Ok(Self { server, port })
    }
    /// Get port on which current application is ran.
    pub fn port(&self) -> u16 {
        self.server.local_addr().port()
    }
    /// This function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        self.server.await
    }
}

// Private
impl Application {
    /// Configure `Server`.
    fn run(
        listener: TcpListener,
        pool: ConnectionPool,
        email_client: EmailClient,
    ) -> Server<AddrIncoming, IntoMakeService<Router>> {
        // We do not wrap pool into arc because internally it alreaday has an
        // `Arc`, and copying is cheap.
        let app_state = AppState { pool, email_client };
        let app = Router::new()
            .route("/health_check", routing::get(health_check))
            .route("/subscriptions", routing::post(subscribe_handler))
            .with_state(app_state);

        axum::Server::from_tcp(listener)
            .expect("Cant create server from tcp listener.")
            .serve(app.into_make_service())
    }
}

/// Returns a connection pool to the PostgreSQL database.
async fn get_postgres_connection_pool(
    configuration: &Settings,
) -> ConnectionPool {
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_ca_file(&configuration.ssl_crt_path).unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    let connector = MakeTlsConnector::new(builder.build());

    let manager = PostgresConnectionManager::new_from_stringlike(
        configuration.database.connection_string().expose_secret(),
        connector,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}
