use std::borrow::Cow;

use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};

use crate::domain::SubscriberEmail;

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
}

/// TODO: I want to measure request
/// connection time and set "timeout" more precisely.
impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
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
        })
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
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
    use super::SendEmailRequest;
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
                    // dbg!(_r);
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
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri()).unwrap();

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
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_eq!(mock_server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn send_email_returns_ok_when_request_succeeds() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri()).unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_email_returns_error_when_request_fails() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri()).unwrap();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri()).unwrap();

        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(60));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
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

    fn email_client(base_url: String) -> Result<EmailClient, String> {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }
}
