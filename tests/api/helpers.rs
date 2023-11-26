//! This is a module with common initialization functions.

use std::env::temp_dir;

use bb8_postgres::PostgresConnectionManager;
use secrecy::{ExposeSecret, Secret};
use wiremock::MockServer;

use zero2prod_axum::{
    configuration::Settings, connection_pool::ConnectionPool,
    startup::Application,
};

/// This type contains MockServer, and it's address.
/// MockServer represents a email delivery service,
/// such as Postmark.
pub struct TestApp {
    pub address: String,
    pub pool: ConnectionPool,
    pub email_server: MockServer,
    pub port: u16,
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    /// This function sends Post request to our TestApp,
    /// to /subscriptions path. If successful, it will create
    /// and delete line in postgres db.
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

    /// Extract the confirmation links embedded in the email API.
    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value =
            serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_string();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html_link = get_link(&body["html_body"].as_str().unwrap());
        let text_link = get_link(&body["text_body"].as_str().unwrap());

        ConfirmationLinks {
            html: html_link,
            plain_text: text_link,
        }
    }
}

pub async fn spawn_postgres_pool(
    connection_string: Secret<String>,
) -> ConnectionPool {
    let manager = PostgresConnectionManager::new_from_stringlike(
        connection_string.expose_secret(),
        tokio_postgres::NoTls,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}

pub async fn spawn_app_locally(mut config: Settings) -> TestApp {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let connection_string = config.database.connection_string();

    // We should randomize app port
    config.app_port = 0;

    let email_server = MockServer::start().await;

    // Set base_url to our MockServer instead of real email delivery service.
    config.email_client.base_url = email_server.uri();

    let application = Application::build(config)
        .await
        .expect("Failed to build application");

    let zero2prod_axum::startup::PortType::Tcp(port) = application.port()
    else {
        unreachable!();
    };

    let address = format!("http://127.0.0.1:{}", port);

    // Very important step
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        // This pool is separate from our app's pool
        pool: spawn_postgres_pool(connection_string).await,
        email_server,
        port,
    }
}
