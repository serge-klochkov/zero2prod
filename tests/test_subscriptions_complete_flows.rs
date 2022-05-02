use crate::common::TestApp;
use reqwest::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::db::subscription_queries::SubscriptionQueries;
use zero2prod::domain::new_subscriber::NewSubscriber;
use zero2prod::domain::subscriber_email::SubscriberEmail;
use zero2prod::domain::subscriber_name::SubscriberName;
use zero2prod::domain::subscription_status::SubscriptionStatus;
use zero2prod::email_client::SendEmailRequest;

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn sends_an_email_and_confirms_subscription_for_a_new_subscriber() {
    let test_app = common::spawn_app().await;
    mock_mail_send(&test_app).await;

    let email = "to_confirm@gmail.com";
    let name = "To Confirm";
    let body = format!("name={}&email={}", name, email);

    test_app.post_subscriptions(&body).await;

    let received_requests =
        common::eventually(|| async { test_app.get_received_requests().await }, 100, 50).await;

    let confirmation_link = extract_confirmation_link(&test_app, &received_requests[0].body);

    // Click the link for the first time: subscription is now confirmed
    follow_link_and_expect_status(&test_app, &confirmation_link, 200).await;
    assert_confirmed_subscription_in_db(&test_app, email, name).await;

    // If we click the link twice, it is expired
    follow_link_and_expect_status(&test_app, &confirmation_link, 401).await;
    // DB entry has not changed
    assert_confirmed_subscription_in_db(&test_app, email, name).await;

    // We cannot subscribe with the same email one more time - it is already confirmed
    // the status Conflict should represent exactly that
    let response = test_app.post_subscriptions(&body).await;
    assert_eq!(response.status().as_u16(), 409)

    // Wiremock asserts on drop
}

#[tokio::test(flavor = "multi_thread")]
async fn sends_an_email_and_confirms_subscription_for_a_previously_failed_one() {
    let test_app = common::spawn_app().await;
    mock_mail_send(&test_app).await;

    let email = "failed_sub@gmail.com";
    let name = "Failed Sub";
    let body = format!("name={}&email={}", name, email);

    insert_new_subscription(&test_app, email, name, SubscriptionStatus::Failed).await;
    test_app.post_subscriptions(&body).await;

    let received_requests =
        common::eventually(|| async { test_app.get_received_requests().await }, 100, 50).await;

    let confirmation_link = extract_confirmation_link(&test_app, &received_requests[0].body);

    // Click the link: failed subscription is now confirmed
    follow_link_and_expect_status(&test_app, &confirmation_link, 200).await;
    assert_confirmed_subscription_in_db(&test_app, email, name).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sends_an_email_and_confirms_subscription_for_a_pending_one() {
    let test_app = common::spawn_app().await;
    mock_mail_send(&test_app).await;

    let email = "pending_sub@gmail.com";
    let name = "Pending Sub";
    let body = format!("name={}&email={}", name, email);

    insert_new_subscription(&test_app, email, name, SubscriptionStatus::Pending).await;
    test_app.post_subscriptions(&body).await;

    let received_requests =
        common::eventually(|| async { test_app.get_received_requests().await }, 100, 50).await;

    let confirmation_link = extract_confirmation_link(&test_app, &received_requests[0].body);

    // Click the link: failed subscription is now confirmed
    follow_link_and_expect_status(&test_app, &confirmation_link, 200).await;
    assert_confirmed_subscription_in_db(&test_app, email, name).await;
}

fn extract_confirmation_link(test_app: &TestApp, body: &[u8]) -> String {
    let body: SendEmailRequest = serde_json::from_slice(body).unwrap();

    let links: Vec<_> = linkify::LinkFinder::new()
        .links(body.content.first().unwrap().value.as_ref())
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
    assert_eq!(links.len(), 1);

    let mut confirmation_url = Url::parse(links.first().unwrap().as_str()).unwrap();
    assert_eq!(confirmation_url.host_str().unwrap(), "127.0.0.1");

    // replace the port form .env with test app random port
    confirmation_url.set_port(Some(test_app.port)).unwrap();
    confirmation_url.to_string()
}

async fn follow_link_and_expect_status(test_app: &TestApp, link: &str, expected_status: u16) {
    let response = reqwest::Client::new()
        .get(link)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(
        response.status().as_u16(),
        expected_status,
        "Test database is {}, link is {}",
        &test_app.db_name,
        link,
    );
}

async fn assert_confirmed_subscription_in_db(test_app: &TestApp, email: &str, name: &str) {
    let saved = sqlx::query!("SELECT email, name, (status :: TEXT) FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, email);
    assert_eq!(saved.name, name);
    assert_eq!(saved.status, Some("confirmed".to_owned()));
}

async fn insert_new_subscription(
    test_app: &TestApp,
    email: &str,
    name: &str,
    status: SubscriptionStatus,
) {
    let mut tx = test_app.db_pool.begin().await.unwrap();
    let sub = NewSubscriber {
        email: SubscriberEmail::parse(email.to_string()).unwrap(),
        name: SubscriberName::parse(name.to_string()).unwrap(),
    };
    SubscriptionQueries::insert_subscriber(&mut tx, &sub, status)
        .await
        .expect("Failed to save a new subscriber");
    tx.commit().await.unwrap();
}

async fn mock_mail_send(test_app: &TestApp) {
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.mock_server)
        .await;
}
