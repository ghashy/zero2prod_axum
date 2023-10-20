use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

use crate::domain::SubscriberEmail;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_port: u16,
    pub app_addr: String,
    /// If this parameter set to non-zero length String, use unix sockets.
    pub unix_socket: String,
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
    pub socket_file: String,
    pub database_name: String,
}

impl DatabaseSettings {
    /// `tokio-postgres` will try to connect to unix first, and then to tcp.
    pub fn connection_string(&self) -> secrecy::Secret<String> {
        secrecy::Secret::new(format!(
            "user={} password={} dbname={} host={},{} port={} application_name={}",
            self.username,
            self.password.expose_secret(),
            self.database_name,
            self.socket_file,
            self.host,
            self.port,
            "zero2prod"
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
