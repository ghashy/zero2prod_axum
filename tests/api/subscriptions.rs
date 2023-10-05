// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::helpers::{spawn_app_locally, TestApp};
use zero2prod_axum::configuration::Settings;

// ───── Body ─────────────────────────────────────────────────────────────── //

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let config = Settings::load_test_configuration();
    let TestApp { address, pool } = spawn_app_locally(config).await;

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &address))
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
        .query(
            "SELECT email, name FROM subscriptions WHERE name = 'le guin'",
            &[],
        )
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
        .simple_query("DELETE FROM subscriptions WHERE name = 'le guin'")
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
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let config = Settings::load_test_configuration();
    let TestApp { address, pool } = spawn_app_locally(config).await;

    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "emtpy name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definetely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", address))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let config = Settings::load_test_configuration();

    let TestApp { address, pool } = spawn_app_locally(config).await;

    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &address))
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
