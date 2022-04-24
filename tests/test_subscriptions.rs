use reqwest::Url;
use std::thread;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::config::CONFIG;
use zero2prod::email_client::SendEmailRequest;

mod common;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body).await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    test_app.post_subscriptions(body).await;
    let saved = sqlx::query!("SELECT email, name, (status :: TEXT) FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, Some("pending".to_owned()));
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = common::spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
        let result = sqlx::query("SELECT email, name FROM subscriptions")
            .fetch_all(&test_app.db_pool)
            .await
            .unwrap();
        assert_eq!(
            result.len(),
            0,
            "There should be no saved subscriptions in case of failure"
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_empty() {
    let test_app = common::spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (body, description) in test_cases {
        let response = test_app.post_subscriptions(body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}.",
            description
        );
        let result = sqlx::query("SELECT email, name FROM subscriptions")
            .fetch_all(&test_app.db_pool)
            .await
            .unwrap();
        assert_eq!(
            result.len(),
            0,
            "There should be no saved subscriptions in case of failure"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.mock_server)
        .await;

    let sub_created = test_app
        .nats_connection
        .queue_subscribe(
            &CONFIG.nats_subscription_created_subject,
            &uuid::Uuid::new_v4().to_string(),
        )
        .await
        .unwrap();

    test_app.post_subscriptions(body.into()).await;

    // Wait for the next message in the "SubscriptionCreated" subject (but with different group)
    // this way, we assume that the actual business logic subscriber received the message as well
    // otherwise, we just need to do plain sleep
    if let Some(_) = sub_created.next().await {
        // and then shutdown the NATS subscription immediately to continue the test
        sub_created.unsubscribe().await.unwrap();
        // FIXME: sleep here is still required so the subscriber has time to process the message
        thread::sleep(Duration::from_millis(500));
    }

    let email_request = &test_app.mock_server.received_requests().await.unwrap()[0];
    let body: SendEmailRequest = serde_json::from_slice(&email_request.body).unwrap();

    let links: Vec<_> = linkify::LinkFinder::new()
        .links(body.content.first().unwrap().value.as_ref())
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
    assert_eq!(links.len(), 1);

    // The link from the email should work; replace the port form .env with test app random port
    let mut confirmation_url = Url::parse(links.first().unwrap().as_str()).unwrap();
    assert_eq!(confirmation_url.host_str().unwrap(), "127.0.0.1");

    confirmation_url.set_port(Some(test_app.port)).unwrap();
    let confirmation_link = confirmation_url.as_str();
    let response = reqwest::Client::new()
        .get(confirmation_link)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status().as_u16(), 200);

    // Subscription is now confirmed
    let saved = sqlx::query!("SELECT email, name, (status :: TEXT) FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, Some("confirmed".to_owned()));

    // Wiremock asserts on drop
}
