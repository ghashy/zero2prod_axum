use std::{borrow::Cow, collections::HashMap};

use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};

use crate::domain::SubscriberEmail;

#[derive(Deserialize, Clone, Debug)]
#[serde(try_from = "String")]
pub enum EmailDeliveryService {
    Postmark,
    SMTP,
}

impl TryFrom<String> for EmailDeliveryService {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "postmark" | "Postmark" => Ok(Self::Postmark),
            "smtp" | "SMTP" => Ok(Self::SMTP),
            _ => Err(format!(
                "Can't construct EmailDeliveryService type from {}",
                value
            )),
        }
    }
}

/// This type handles the sending of emails.
/// Internally, it includes a connection pool.
#[derive(Clone)]
pub struct EmailClient {
    /// Every time a `Client` instance is created, `reqwest` initialises a
    /// connection pool under the hood.
    /// We can clone this, because the internal `Arc` will be cloned, and
    /// will be pointing to one client!
    http_client: Client,
    base_url: reqwest::Url,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
    delivery_service: EmailDeliveryService,
}

/// TODO: I want to measure request
/// connection time and set "timeout" more precisely.
impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
        delivery_service: EmailDeliveryService,
    ) -> Result<Self, String> {
        let base_url =
            reqwest::Url::try_from(base_url.as_str()).map_err(|e| {
                format!("Error in `EmailClient`s new fn: {}", e.to_string())
            })?;
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Ok(Self {
            http_client,
            base_url,
            sender,
            authorization_token,
            delivery_service,
        })
    }

    /// This function will send `POST` request to the
    /// email delivery service, `smtp.bz` in this case
    /// with data necessary to send the email to recipient.
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        match self.delivery_service {
            EmailDeliveryService::Postmark => {
                let url = self.base_url.join("/email").unwrap();
                let request_body = SendEmailRequest {
                    from: Cow::Borrowed(self.sender.as_ref()),
                    to: Cow::Borrowed(recipient.as_ref()),
                    subject: Cow::Borrowed(subject),
                    html_body: Cow::Borrowed(html_content),
                    text_body: Cow::Borrowed(text_content),
                };
                self.http_client
                    .post(url)
                    .header(
                        "X-Postmark-Server-Token",
                        self.authorization_token.expose_secret(),
                    )
                    .json(&request_body)
                    .send()
                    .await?
                    .error_for_status()?;
            }

            EmailDeliveryService::SMTP => {
                let url = self.base_url.join("/v1/smtp/send").unwrap();
                let mut map = HashMap::new();
                map.insert("name", "info");
                map.insert("from", self.sender.as_ref());
                map.insert("subject", subject);
                map.insert("to", recipient.as_ref());
                map.insert("html", html_content);
                map.insert("text", text_content);

                let pass = self.authorization_token.expose_secret();
                self.http_client
                    .post(url)
                    .header("Authorization", pass)
                    .form(&map)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SendEmailRequest<'a> {
    from: Cow<'a, str>,
    to: Cow<'a, str>,
    subject: Cow<'a, str>,
    html_body: Cow<'a, str>,
    text_body: Cow<'a, str>,
}

// It is unit tests module, because it needs private type `SendEmailRequest`.
#[cfg(test)]
mod email_client_tests {
    use super::{EmailDeliveryService, SendEmailRequest};
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let request =
                serde_json::from_slice::<SendEmailRequest>(&request.body);
            match request {
                Ok(_r) => {
                    return true;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return false;
                }
            }
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request_postmark() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client =
            email_client(mock_server.uri(), EmailDeliveryService::Postmark)
                .unwrap();

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(SendEmailBodyMatcher)
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request_smtp() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client =
            email_client(mock_server.uri(), EmailDeliveryService::SMTP)
                .unwrap();

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/x-www-form-urlencoded"))
            .and(path("/v1/smtp/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn send_email_returns_ok_when_request_succeeds() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client =
            email_client(mock_server.uri(), EmailDeliveryService::Postmark)
                .unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_email_returns_error_when_request_fails() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client =
            email_client(mock_server.uri(), EmailDeliveryService::Postmark)
                .unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client =
            email_client(mock_server.uri(), EmailDeliveryService::Postmark)
                .unwrap();

        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(60));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert!(result.is_err());
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(&SafeEmail().fake::<String>()).unwrap()
    }

    fn email_client(
        base_url: String,
        delivery_service: EmailDeliveryService,
    ) -> Result<EmailClient, String> {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
            delivery_service,
        )
    }
}
