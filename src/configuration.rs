use std::{env::VarError, net::Ipv4Addr, path::Path};

use config::FileFormat;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

use crate::{domain::SubscriberEmail, email_client::EmailDeliveryService};

pub enum Environment {
    Local,
    Production,
}
impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.", other)),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app_port: u16,
    pub app_addr: Ipv4Addr,
    pub app_base_url: String,
    /// If this parameter set to non-zero length String, use unix sockets.
    pub unix_socket: String,
    pub email_client: EmailClientSettings,
    pub email_delivery_service: EmailDeliveryService,
}

impl Settings {
    pub fn load_configuration() -> Result<Settings, config::ConfigError> {
        let base_path = std::env::current_dir()
            .expect("Failed to determine the current directory");
        let configuration_directory = base_path.join("configuration");
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");

        // Initialise our configuration reader
        let settings = config::Config::builder()
            .add_source(config::File::new(
                configuration_directory
                    .join(environment.as_str())
                    .to_str()
                    .unwrap(),
                FileFormat::Yaml,
            ))
            .build()?;

        // Try to deserialize the configuration values it read into
        // our `Settings` type.
        settings.try_deserialize()
    }

    pub fn load_configuration_from_env() -> Result<Settings, VarError> {
        let settings = Settings {
            database: DatabaseSettings {
                username: std::env::var("PG_USER")?,
                password: Secret::new(load_passwd_from_file(std::env::var(
                    "PG_PASSWORD_FILE",
                )?)),
                host: std::env::var("PG_HOST")?,
                unix_socket: String::new(),
                database_name: std::env::var("PG_DBNAME")?,
            },
            app_port: std::env::var("APP_PORT")?.parse::<u16>().unwrap(),
            app_addr: std::env::var("APP_ADDR")?.parse::<Ipv4Addr>().unwrap(),
            app_base_url: std::env::var("APP_BASE_URL")?,
            unix_socket: String::new(),
            email_client: EmailClientSettings {
                base_url: std::env::var("EMAIL_CLIENT_BASE_URL")?,
                sender_email: std::env::var("SENDER_EMAIL")?,
                authorization_token: Secret::new(load_passwd_from_file(
                    std::env::var("AUTHORIZATION_TOKEN_FILE")?,
                )),
                timeout: 10000,
            },
            email_delivery_service: std::env::var("EMAIL_DELIVERY_SERVICE")?
                .try_into()
                .unwrap(),
        };
        Ok(settings)
    }
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub unix_socket: String,
    pub database_name: String,
}

impl DatabaseSettings {
    /// `tokio-postgres` will try to connect to unix first, and then to tcp.
    pub fn connection_string(&self) -> secrecy::Secret<String> {
        secrecy::Secret::new(format!(
            "user={} password={} dbname={} host={},{} application_name={}",
            self.username,
            self.password.expose_secret(),
            self.database_name,
            self.unix_socket,
            self.host,
            "zero2prod"
        ))
    }
}

/// This type describes configuration
/// for client, sending emails.
#[derive(Debug, Deserialize)]
pub struct EmailClientSettings {
    /// Email delivery service we use to relay email (Postmark in our case)
    pub base_url: String,
    /// This host email address
    pub sender_email: String,
    /// Token to authorize in Postmark API
    pub authorization_token: Secret<String>,
    /// `request` crate will wait until this timeout when sends emails
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

fn load_passwd_from_file<T: AsRef<Path>>(path: T) -> String {
    std::fs::read_to_string(path).unwrap().trim().to_string()
}
