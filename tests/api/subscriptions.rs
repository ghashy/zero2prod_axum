use crate::helpers::spawn_app_locally;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod_axum::configuration::Settings;

#[tokio::test]
async fn req_subscribe_returns_a_200_for_valid_form_data_and_subscriber_persists_in_db(
) {
    let config = Settings::load_configuration().unwrap();
    let app = spawn_app_locally(config).await;

    let body = "name=hello%20world&email=helloworld%40gmail.com";

    // Add path and request type to already EXISTING mock server,
    // (we are sending relay mail request to a mock server
    // instead of a real email delivery service).
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body).await;

    // CHECK RESPONSE
    assert_eq!(
        200,
        response.status().as_u16(),
        "The API failed with correct post request.\nResponse: {}",
        response.text().await.unwrap().as_str()
    );

    let saved = app
        .pool
        .get()
        .await
        .unwrap()
        .query(
            "SELECT email, name, status FROM subscriptions WHERE name = 'hello world'",
            &[],
        )
        .await
        .expect("Failed to fetch saved subscription.");

    // CHECK IF ENTRY IN DB EXISTS
    assert_eq!(saved[0].get::<&str, &str>("email"), "helloworld@gmail.com");
    assert_eq!(saved[0].get::<&str, &str>("name"), "hello world");
    assert_eq!(saved[0].get::<&str, &str>("status"), "pending_confirmation");

    // REMOVE TEST DATA FROM THE DATABASE.
    let _ = app
        .pool
        .get()
        .await
        .unwrap()
        .simple_query("DELETE FROM subscriptions WHERE name = 'hello world'")
        .await
        .expect("Failed to fetch saved subscription.");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let config = Settings::load_configuration().unwrap();
    let app = spawn_app_locally(config).await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "emtpy name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definetely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body).await;

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
    let app = spawn_app_locally(config).await;

    let test_cases = vec![
        ("name=le%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body).await;

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}",
            error_message
        );
    }
}

/// Test that sending request `create a new subscription` to our APP
/// sends a request to email delivery serivce to send to recipient
/// a confirmation email and both plain body && html body contains
/// single grammarly valid link.
/// NOTE: actually code which has responsibility for sending email
/// is in `crate::routes::subscriptions::send_confirmation_email`
/// request handler.
#[tokio::test]
async fn req_subscribe_sends_a_confirmation_email_with_a_link() {
    let config = Settings::load_configuration().unwrap();

    // Arrange
    let app = spawn_app_locally(config).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Send request to OUR app, 'POST, create a new subscription`
    let _response = app.post_subscriptions(body.into()).await;
    // println!("\nReponse: {}", response.text().await.unwrap().as_str());

    // Assert
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    // Get the first intercepted request
    let email_received_request =
        &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_received_request);
    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);

    // REMOVE TEST DATA FROM THE DATABASE.
    let _ = app
        .pool
        .get()
        .await
        .unwrap()
        .simple_query("DELETE FROM subscriptions WHERE name = 'le guin'")
        .await
        .expect("Failed to fetch saved subscription.");

    // Mock asserts on drop
}
