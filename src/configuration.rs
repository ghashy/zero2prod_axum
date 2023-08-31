use serde::Deserialize;

use secrecy::{ExposeSecret, Secret};

// ───── Body ─────────────────────────────────────────────────────────────── //

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
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

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    // Initialise our configuration reader
    let settings = config::Config::builder()
        .add_source(config::File::with_name("configuration"))
        .build()?;

    // Try to deserialize the configuration values it read into
    // our `Settings` type.
    settings.try_deserialize()
}

pub fn get_test_configuration() -> Settings {
    Settings {
        database: DatabaseSettings {
            username: String::from("ghashy"),
            password: Secret::new(String::from("ghashy")),
            port: 5432,
            host: String::from("127.0.0.1"),
            database_name: String::from("newsletter"),
        },
        application_port: 8000,
    }
}
