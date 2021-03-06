use claim::assert_err;
use std::time::Duration;

use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::{Paragraph, Sentence};
use fake::Fake;
use serde_json::{from_slice, Value};
use wiremock::matchers::{any, header, header_exists, method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};
use zero2prod::config::Config;

use zero2prod::domain::subscriber_email::SubscriberEmail;
use zero2prod::email_client::EmailClient;

struct MatchSendEmailBody;

impl Match for MatchSendEmailBody {
    fn matches(&self, request: &Request) -> bool {
        let result: Result<Value, _> = from_slice(&request.body);
        if let Ok(body) = result {
            body.get("from").is_some()
                && body.get("personalizations").is_some()
                && body.get("subject").is_some()
                && body.get("content").is_some()
        } else {
            false
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn succeeds_if_the_server_returns_200() {
    // Arrange
    let mock_server = MockServer::start().await;
    let sender = random_email();
    let recipient = random_email();
    let subject: String = Sentence(1..2).fake();
    let content: String = Paragraph(1..10).fake();

    // Mock
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .and(header("content-type", "application/json"))
        .and(header_exists("authorization"))
        .and(MatchSendEmailBody)
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Act
    let email_client = create_email_client(sender.as_ref(), &mock_server, 1000);
    let _ = email_client
        .send_email(&recipient, &subject, &content)
        .await
        .unwrap();

    // Wiremock assertions performed on Drop
}

#[tokio::test(flavor = "multi_thread")]
async fn fails_when_sending_takes_too_long() {
    // Arrange
    let mock_server = MockServer::start().await;
    let sender = random_email();
    let recipient = random_email();
    let subject: String = Sentence(1..2).fake();
    let content: String = Paragraph(1..10).fake();

    // Mock
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(180)))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Act
    let email_client = create_email_client(sender.as_ref(), &mock_server, 100);
    let outcome = email_client
        .send_email(&recipient, &subject, &content)
        .await;

    assert_err!(outcome);

    // Wiremock assertions performed on Drop
}

#[tokio::test(flavor = "multi_thread")]
async fn fails_if_the_server_returns_500() {
    // Arrange
    let mock_server = MockServer::start().await;
    let sender = random_email();
    let subscriber_email = random_email();
    let subject: String = Sentence(1..2).fake();
    let content: String = Paragraph(1..10).fake();
    Mock::given(any())
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Act
    let email_client = create_email_client(sender.as_ref(), &mock_server, 1000);
    let outcome = email_client
        .send_email(&subscriber_email, &subject, &content)
        .await;

    // Assert
    assert_err!(outcome);

    // Wiremock assertions performed on Drop
}

fn random_email() -> SubscriberEmail {
    SubscriberEmail::parse(SafeEmail().fake()).unwrap()
}

fn create_email_client(sender: &str, mock_server: &MockServer, timeout_millis: u16) -> EmailClient {
    let mut config = Config::new().expect("Failed to load config");
    config.email_client_sender_email = sender.to_owned();
    config.email_client_base_url = mock_server.uri();
    config.email_client_timeout_millis = timeout_millis;
    EmailClient::new(&config)
}
