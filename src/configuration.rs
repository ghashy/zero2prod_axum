use serde::Deserialize;

use secrecy::{ExposeSecret, Secret};

use crate::domain::SubscriberEmail;

// ───── Body ─────────────────────────────────────────────────────────────── //

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_port: u16,
    pub app_addr: String,
    pub ssl_crt_path: String,
    pub email_client: EmailClientSettings,
}

impl Settings {
    pub fn load_configuration() -> Result<Settings, config::ConfigError> {
        // Initialise our configuration reader
        let settings = config::Config::builder()
            .add_source(config::File::with_name("configuration"))
            .build()?;

        // Try to deserialize the configuration values it read into
        // our `Settings` type.
        settings.try_deserialize()
    }
    pub fn load_test_configuration() -> Settings {
        // let database_settings = DatabaseSettings {
        //     username: String::from("ghashy"),
        //     password: Secret::new(String::from("ghashy")),
        //     port: 5432,
        //     host: String::from("127.0.0.1"),
        //     database_name: String::from("newsletter"),
        // };
        // let email_client = EmailClientSettings {
        //     base_url: String::from("http://127.0.0.1"),
        //     sender_email: String::from("sender@gmail.com"),
        //     authorization_token: Secret::new(String::from("my_token")),
        //     timeout: 10000,
        // };
        // Settings {
        //     database: database_settings,
        //     // Random port
        //     app_port: 0,
        //     app_addr: String::from("127.0.0.1"),
        //     ssl_crt_path: String::from("assets/root.crt"),
        //     email_client,
        // }
        Settings::load_configuration().unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> secrecy::Secret<String> {
        secrecy::Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    timeout: u64,
}

impl EmailClientSettings {
    /// Try to parse email from `String` type to safe `SubscriberEmail`.
    pub fn sender(&self) -> Result<SubscriberEmail, &'static str> {
        SubscriberEmail::parse(&self.sender_email)
    }

    pub fn timeout_millis(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout)
    }
}
