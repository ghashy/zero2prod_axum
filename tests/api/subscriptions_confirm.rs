//! tests/api/subscriptions_confirm.rs
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};
use zero2prod_axum::configuration::Settings;

use crate::helpers::spawn_app_locally;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // Arrange
    let app = spawn_app_locally(Settings::load_configuration().unwrap()).await;

    // Act
    let response =
        reqwest::get(&format!("{}/subscriptions/confirm", app.address))
            .await
            .unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Arrange
    let app = spawn_app_locally(Settings::load_configuration().unwrap()).await;
    let body = "name=confirm%20name&email=confirmname%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    // Act
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);

    // REMOVE TEST DATA FROM THE DATABASE.
    let _ = app
        .pool
        .get()
        .await
        .unwrap()
        .simple_query("DELETE FROM subscriptions WHERE name = 'confirm name'")
        .await
        .expect("Failed to fetch saved subscription.");
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber_in_db() {
    // Arrange
    let app = spawn_app_locally(Settings::load_configuration().unwrap()).await;
    let body = "name=john%20smit&email=johnsmit%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // Act
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert
    let saved = app
        .pool
        .get()
        .await
        .unwrap()
        .query_one("SELECT email, name, status FROM subscriptions WHERE name = \'john smit\'", &[])
        .await
        .unwrap();

    assert_eq!(saved.get::<&str, &str>("email"), "johnsmit@gmail.com");
    assert_eq!(saved.get::<&str, &str>("name"), "john smit");
    assert_eq!(saved.get::<&str, &str>("status"), "confirmed");

    // REMOVE TEST DATA FROM THE DATABASE.
    let _ = app
        .pool
        .get()
        .await
        .unwrap()
        .simple_query("DELETE FROM subscriptions WHERE name = 'john smit'")
        .await
        .expect("Failed to fetch saved subscription.");
}
