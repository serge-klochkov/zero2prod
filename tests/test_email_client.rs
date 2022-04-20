use claim::assert_err;
use std::time::Duration;

use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::{Paragraph, Sentence};
use fake::Fake;
use serde_json::{from_slice, Value};
use wiremock::matchers::{any, header, header_exists, method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

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

#[tokio::test]
async fn send_mail() {
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
        .send_email(recipient, &subject, &content)
        .await
        .unwrap();

    // Wiremock assertions performed on Drop
}

#[tokio::test]
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
    let outcome = email_client.send_email(recipient, &subject, &content).await;

    assert_err!(outcome);

    // Wiremock assertions performed on Drop
}

fn random_email() -> SubscriberEmail {
    SubscriberEmail::parse(SafeEmail().fake()).unwrap()
}

fn create_email_client(sender: &str, mock_server: &MockServer, timeout_millis: u64) -> EmailClient {
    EmailClient::new(
        sender,
        &mock_server.uri(),
        Duration::from_millis(timeout_millis),
    )
}
