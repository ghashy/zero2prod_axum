//! This is a module with common initialization functions.

use deadpool_postgres::Pool;
use secrecy::{ExposeSecret, Secret};
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
    db_username: String,
    db_config_with_root_cred: DatabaseSettings,
    pub address: String,
    pub pool: Pool,
    pub email_server: MockServer,
    pub port: u16,
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLink {
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
    pub fn get_confirmation_link(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLink {
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

        let text_link = get_link(&body["text_body"].as_str().unwrap());

        ConfirmationLink {
            plain_text: text_link,
        }
    }
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
    let mut db_config = config.database.clone();

    // Connect as an admin user
    let pool = get_postgres_connection_pool(&db_config);
    let db_username = generate_username();

    // Create new random user account in pg
    let create_role =
        format!("CREATE ROLE {0} WITH LOGIN PASSWORD '{0}';", &db_username);
    let create_schema =
        format!("CREATE SCHEMA {0} AUTHORIZATION {0};", &db_username);
    pool.get()
        .await
        .unwrap()
        .simple_query(&create_role)
        .await
        .unwrap();
    pool.get()
        .await
        .unwrap()
        .simple_query(&create_schema)
        .await
        .unwrap();

    drop(pool);
    db_config.username = db_username.clone();
    db_config.password = Secret::new(db_username.clone());

    // Connect as a new user
    let pool = get_postgres_connection_pool(&db_config);

    let email_server = MockServer::start().await;

    // Set base_url to our MockServer instead of real email delivery service.
    config.email_client.base_url = email_server.uri();
    config.app_port = 0;
    // For Drop
    let db_config_with_root_cred = config.database.clone();

    // Store db_config with test user in config destined for Application::build
    config.database = db_config;

    let application = Application::build(config)
        .await
        .expect("Failed to build application");

    let port = application.port();

    let address = format!("http://127.0.0.1:{}", port);

    // Very important step
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        db_username,
        db_config_with_root_cred,
        address,
        pool,
        email_server,
        port,
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Clean pg
        let db_config = self.db_config_with_root_cred.clone();
        let db_username = self.db_username.clone();
        // Spawn a new thread, because internally sync postgres client uses
        // tokio runtime, but we are already in tokio runtime here. To
        // spawn a new tokio runtime, we should do it inside new thread.
        let _ = std::thread::spawn(move || {
            let mut client = get_sync_postgres_client(&db_config);
            let create_role = format!("DROP SCHEMA {0} CASCADE;", db_username);
            let create_schema = format!("DROP ROLE {0};", db_username);
            client.simple_query(&create_role).unwrap();
            client.simple_query(&create_schema).unwrap();
        })
        .join();
    }
}

pub fn generate_username() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "test_{}",
        std::iter::repeat_with(|| {
            rand::Rng::sample(&mut rng, rand::distributions::Alphanumeric)
        })
        .map(|b| char::from(b).to_lowercase().next().unwrap())
        .take(5)
        .collect::<String>()
    )
}

pub fn get_sync_postgres_client(
    configuration: &DatabaseSettings,
) -> postgres::Client {
    postgres::Client::connect(
        configuration.connection_string().expose_secret(),
        NoTls,
    )
    .unwrap()
}
