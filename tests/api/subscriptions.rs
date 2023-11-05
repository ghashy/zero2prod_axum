use crate::helpers::spawn_app_locally;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod_axum::configuration::Settings;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let config = Settings::load_configuration().unwrap();
    let test_app = spawn_app_locally(config).await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body).await;

    assert_eq!(
        200,
        response.status().as_u16(),
        "The API failed with correct post request.\nResponse: {}",
        response.text().await.unwrap().as_str()
    );

    let saved = test_app
        .pool
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
    let messages = test_app
        .pool
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
    let config = Settings::load_configuration().unwrap();
    let test_app = spawn_app_locally(config).await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "emtpy name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definetely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = test_app.post_subscriptions(body).await;

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
    let config = Settings::load_configuration().unwrap();

    let test_app = spawn_app_locally(config).await;

    let test_cases = vec![
        ("name=le%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body).await;

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let config = Settings::load_configuration().unwrap();

    // Arrange
    let app = spawn_app_locally(config).await;
    let body = "name=le%guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriptions(body.into()).await;
    println!("\nReponse: {}", response.text().await.unwrap().as_str());

    // Assert
    // Mock asserts on drop
}
