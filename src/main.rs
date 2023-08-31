use std::net::TcpListener;

use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};

use postgres_openssl::MakeTlsConnector;

use bb8_postgres::PostgresConnectionManager;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use secrecy::ExposeSecret;
use zero2prod_axum::configuration::get_configuration;
use zero2prod_axum::startup::run;

// ───── Body ─────────────────────────────────────────────────────────────── //

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .pretty()
        .with_level(true)
        .init();
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_ca_file("assets/root.crt").unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    let connector = MakeTlsConnector::new(builder.build());

    // Panic if we can't read configuration
    let configuration =
        get_configuration().expect("Failed to read configuration.");

    let manager = PostgresConnectionManager::new_from_stringlike(
        configuration.database.connection_string().expose_secret(),
        connector,
    )
    .unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener =
        TcpListener::bind(address).expect("Failed to bind random port.");

    if let Err(e) = run(listener, pool).await {
        eprintln!("Error: {}", e);
    }
}
