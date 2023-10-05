//! This is a module with common initialization functions.

use bb8_postgres::PostgresConnectionManager;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use secrecy::{ExposeSecret, Secret};

// ───── Current Crate Imports ────────────────────────────────────────────── //

use zero2prod_axum::{
    configuration::Settings, connection_pool::ConnectionPool,
    startup::Application,
};

// ───── Body ─────────────────────────────────────────────────────────────── //

pub struct TestApp {
    pub address: String,
    pub pool: ConnectionPool,
}

pub async fn spawn_pool(
    connection_string: Secret<String>,
    connector: MakeTlsConnector,
) -> ConnectionPool {
    let manager = PostgresConnectionManager::new_from_stringlike(
        connection_string.expose_secret(),
        connector,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}

pub async fn spawn_app_locally(mut config: Settings) -> TestApp {
    let connection_string = config.database.connection_string();
    // We should randomize app port
    config.app_port = 0;
    let application = Application::build(config)
        .await
        .expect("Failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());
    // Very important step
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        // This pool is separate from our app's pool
        pool: spawn_pool(connection_string, get_connector()).await,
    }
}

pub fn get_connector() -> MakeTlsConnector {
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_ca_file("assets/root.crt").unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    MakeTlsConnector::new(builder.build())
}
