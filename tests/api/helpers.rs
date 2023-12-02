//! This is a module with common initialization functions.

use deadpool_postgres::Pool;
use secrecy::ExposeSecret;
use tokio_postgres::NoTls;
use wiremock::MockServer;

use zero2prod_axum::{
    configuration::{DatabaseSettings, Settings},
    startup::{get_postgres_connection_pool, Application},
};

/// This type contains MockServer, and it's address.
/// MockServer represents a email delivery service,
/// such as Postmark.
pub struct TestApp {
    pub address: String,
    pub pool: Pool,
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

pub async fn spawn_postgres_pool(db_config: &DatabaseSettings) -> Pool {
    let mut config = deadpool_postgres::Config::new();
    config.user = Some(db_config.username.clone());
    config.dbname = Some(db_config.username.clone());
    config.host = Some(db_config.host.clone());
    config.password = Some(db_config.password.expose_secret().clone());
    let pool = config
        .create_pool(Some(deadpool::Runtime::Tokio1), NoTls)
        .expect("Failed to build postgres connection pool");
    let _ = pool
        .get()
        .await
        .expect("Failed to get postgres connection from pool");
    pool
}

/// Toggle tracing output by commenting/uncommenting
/// the first lines in this function.
pub async fn spawn_app_locally(mut config: Settings) -> TestApp {
    // let subscriber = tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::INFO)
    //     .with_level(true)
    //     .finish();
    // let _ = tracing::subscriber::set_global_default(subscriber);

    // We should randomize app port
    config.app_port = 0;

    let email_server = MockServer::start().await;

    // Set base_url to our MockServer instead of real email delivery service.
    config.email_client.base_url = email_server.uri();

    let db_config = config.database.clone();
    let application = Application::build(config)
        .await
        .expect("Failed to build application");

    let port = application.port();

    let address = format!("http://127.0.0.1:{}", port);

    // Very important step
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        // This pool is separate from our app's pool
        // pool: spawn_postgres_pool(&db_config).await,
        pool: get_postgres_connection_pool(&db_config).await,
        email_server,
        port,
    }
}
