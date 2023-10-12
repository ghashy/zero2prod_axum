use serde::Deserialize;

use secrecy::{ExposeSecret, Secret};

use crate::domain::SubscriberEmail;

// ───── Body ─────────────────────────────────────────────────────────────── //

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_port: u16,
    pub app_addr: String,
    /// If this parameter set to non-zero length String, use unix sockets.
    pub socket_dir: String,
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
