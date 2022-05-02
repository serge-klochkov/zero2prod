use reqwest::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::email_client::SendEmailRequest;

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body).await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test(flavor = "multi_thread")]
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

#[tokio::test(flavor = "multi_thread")]
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

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let test_app = common::spawn_app().await;
    println!("AppId {}", test_app.config.application_id);
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
            "The API did not return a 400 Bad request when the payload was {}.",
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

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = common::spawn_app().await;
    println!("AppId {}", test_app.config.application_id);
    let body = "name=To%20Confirm&email=to_confirm%40gmail.com";
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.mock_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    let received_requests =
        common::eventually(|| async { test_app.get_received_requests().await }, 100, 50).await;

    let body: SendEmailRequest = serde_json::from_slice(&received_requests[0].body).unwrap();

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
    assert_eq!(
        response.status().as_u16(),
        200,
        "Test database is {}, link is {}",
        &test_app.db_name,
        confirmation_link,
    );

    // Subscription is now confirmed
    let saved = sqlx::query!("SELECT email, name, (status :: TEXT) FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "to_confirm@gmail.com");
    assert_eq!(saved.name, "To Confirm");
    assert_eq!(saved.status, Some("confirmed".to_owned()));

    // Wiremock asserts on drop
}
