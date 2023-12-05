use axum::routing;
use axum::Router;
use deadpool_postgres::Manager;
use deadpool_postgres::ManagerConfig;
use deadpool_postgres::Pool;
// use native_tls::Identity;
use tokio::net::TcpListener;

use axum::serve::Serve;

use secrecy::ExposeSecret;
use tokio_postgres::NoTls;

use crate::configuration::DatabaseSettings;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::confirm;
use crate::routes::get_hello;
use crate::routes::health_check;
use crate::routes::subscribe_handler;

pub mod db_migration;

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
    pub pool: Pool,
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
            get_postgres_connection_pool(&configuration.database);

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
        pool: Pool,
        email_client: EmailClient,
    ) -> Serve<Router, Router> {
        // We do not wrap pool into arc because internally it alreaday has an
        // `Arc`, and copying is cheap.
        let app_state = AppState {
            pool,
            email_client,
            base_url: base_url.to_string(),
        };
        let app = Router::new()
            .route("/health_check", routing::get(health_check))
            .route("/hello", routing::get(get_hello))
            .route("/subscriptions", routing::post(subscribe_handler))
            .route("/subscriptions/confirm", routing::get(confirm))
            .with_state(app_state);

        axum::serve(listener, app)
    }
}

/// Returns a connection pool to the PostgreSQL database.
pub fn get_postgres_connection_pool(configuration: &DatabaseSettings) -> Pool {
    let pg_config = get_pg_conf(configuration);
    // let connector = get_ssl_connector();
    let connector = NoTls;
    let manager_config = ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    };
    let manager = Manager::from_config(pg_config, connector, manager_config);
    let pool = Pool::builder(manager).max_size(16).build().unwrap();
    pool
}

fn get_pg_conf(configuration: &DatabaseSettings) -> tokio_postgres::Config {
    let mut config = tokio_postgres::Config::new();
    config.user(&configuration.username);
    config.dbname(&configuration.database_name);
    config.host(&configuration.host);
    config.password(&configuration.password.expose_secret());
    config
}

// fn get_ssl_connector() -> postgres_native_tls::MakeTlsConnector {
//     let root_file = std::fs::read("db/center/out/myCA.crt").unwrap();
//     let root = native_tls::Certificate::from_pem(&root_file).unwrap();
//     let mut builder = native_tls::TlsConnector::builder();
//     builder.add_root_certificate(root);

//     // Accept invalid host ssl only in development.
//     #[cfg(debug_assertions)]
//     let builder = builder.danger_accept_invalid_hostnames(true);

//     let connector = builder.build().unwrap();
//     postgres_native_tls::MakeTlsConnector::new(connector)
// }
