//! This is a module with common initialization functions.

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use deadpool_postgres::{Client, Pool};
use secrecy::{ExposeSecret, Secret};
use sha3::Digest;
use tokio_postgres::NoTls;
use uuid::Uuid;
use wiremock::MockServer;

use zero2prod_axum::{
    configuration::{DatabaseSettings, Settings},
    startup::{get_postgres_connection_pool, Application},
};

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store_in_db(&self, client: &Client) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // We don't care about the exact Argon2 parameters here
        // given that it's for testing purposes!
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        client
            .execute(
                "INSERT INTO users (user_id, username, password_hash)
                    VALUES ($1, $2, $3)",
                &[&self.user_id, &self.username, &password_hash],
            )
            .await
            .expect("Failed to create test users");
    }
}

/// This type contains MockServer, and it's address.
/// MockServer represents a email delivery service,
/// such as Postmark.
pub struct TestApp {
    db_username: String,
    db_config_with_root_cred: DatabaseSettings,
    test_user: TestUser,
    pub address: String,
    pub pool: Pool,
    pub email_server: MockServer,
    pub port: u16,
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLink(pub reqwest::Url);

impl TestApp {
    /// Toggle tracing output by commenting/uncommenting
    /// the first lines in this function.
    pub async fn spawn_app(mut config: Settings) -> TestApp {
        if let Ok(_) = std::env::var("TEST_TRACING") {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .without_time()
                .compact()
                .with_level(true)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

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
        let client = &pool.get().await.unwrap();
        client.simple_query(&create_role).await.unwrap();
        client.simple_query(&create_schema).await.unwrap();

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

        let test_user = TestUser::generate();
        test_user.store_in_db(&pool.get().await.unwrap()).await;

        TestApp {
            db_username,
            db_config_with_root_cred,
            address,
            pool,
            email_server,
            port,
            test_user,
        }
    }

    /// This function sends Post request to our TestApp,
    /// to /subscriptions path. If successful, it will create
    /// a line in postgres db.
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

        let link = get_link(&body["text_body"].as_str().unwrap());

        ConfirmationLink(link)
    }

    pub async fn post_newsletters(
        &self,
        body: serde_json::Value,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", self.address))
            .json(&body)
            .basic_auth(
                &self.test_user.username,
                Some(&self.test_user.password),
            )
            .send()
            .await
            .expect("Failed to execute request.")
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

// ───── Helpers ──────────────────────────────────────────────────────────── //

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
