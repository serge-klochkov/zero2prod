use wiremock::matchers::any;
use wiremock::{Mock, ResponseTemplate};

mod common;

/// See test_subscriptions_complete_flows for additional cases

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body).await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_persists_a_new_subscriber() {
    let test_app = common::spawn_app().await;
    // Mail send requests are verified in the complete flows tests
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.mock_server)
        .await;
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
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(), 500);
}
