//! This is a module with common initialization functions.

use bb8_postgres::PostgresConnectionManager;
use secrecy::{ExposeSecret, Secret};
use wiremock::MockServer;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use zero2prod_axum::{
    configuration::Settings, connection_pool::ConnectionPool,
    startup::Application,
};

// ───── Body ─────────────────────────────────────────────────────────────── //

pub struct TestApp {
    pub address: String,
    pub pool: ConnectionPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(
        &self,
        body: &'static str,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_pool(connection_string: Secret<String>) -> ConnectionPool {
    let manager = PostgresConnectionManager::new_from_stringlike(
        connection_string.expose_secret(),
        tokio_postgres::NoTls,
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

    let zero2prod_axum::startup::PortType::Tcp(port) = application.port()
    else {
        panic!();
    };

    let address = format!("http://127.0.0.1:{}", port);
    // Very important step
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        // This pool is separate from our app's pool
        pool: spawn_pool(connection_string).await,
        email_server: MockServer::start().await,
    }
}
