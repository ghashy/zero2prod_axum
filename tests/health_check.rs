//! Health integration test.
//!
//! IMPORTANT: Runnig PostgreSQL server is required for testing!

use std::net::TcpListener;

use bb8_postgres::PostgresConnectionManager;

use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};

use postgres_openssl::MakeTlsConnector;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use zero2prod_axum::configuration::get_test_configuration;
use zero2prod_axum::connection_pool::ConnectionPool;

// ───── Body ─────────────────────────────────────────────────────────────── //

/// `tokio::test` is the testing equivalent of `tokio::main`.
/// It also spares you from having to specify the `#[test]` attribute.
///
/// You can inspect what code gets generated using
/// `cargo expand --test health_check` (<- name of the test file).
#[tokio::test]
async fn health_check_test() {
    let addr = spawn_app(spawn_pool(get_connector()).await);
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", addr))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let pool = spawn_pool(get_connector()).await;
    let app_address = spawn_app(pool.clone());

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        200,
        response.status().as_u16(),
        "The API failed with correct post request"
    );

    let saved = pool
        .get()
        .await
        .unwrap()
        .query("SELECT email, name FROM subscriptions", &[])
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(
        saved[0].get::<&str, &str>("email"),
        "ursula_le_guin@gmail.com"
    );
    assert_eq!(saved[0].get::<&str, &str>("name"), "le guin");

    // Remove test data from database.
    let messages = pool
        .get()
        .await
        .unwrap()
        .simple_query("DELETE FROM subscriptions")
        .await
        .expect("Failed to fetch saved subscription.");
    for message in messages {
        match message {
            tokio_postgres::SimpleQueryMessage::Row(data) => {
                println!("GOT ROW: {:?}", data)
            }
            tokio_postgres::SimpleQueryMessage::CommandComplete(n) => {
                println!("GOT COMMAND_COMPLETE: {}", n);
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let app_address = spawn_app(spawn_pool(get_connector()).await);
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            error_message
        );
    }
}

async fn spawn_pool(connector: MakeTlsConnector) -> ConnectionPool {
    let connection_string =
        get_test_configuration().database.connection_string();

    let manager = PostgresConnectionManager::new_from_stringlike(
        connection_string,
        connector,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}

fn spawn_app(pool: ConnectionPool) -> String {
    // ':0' means request random port from system.
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod_axum::startup::run(listener, pool);
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

fn get_connector() -> MakeTlsConnector {
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    builder.set_ca_file("assets/root.crt").unwrap();
    builder.set_verify(SslVerifyMode::NONE);
    MakeTlsConnector::new(builder.build())
}
