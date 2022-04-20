mod common;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = common::spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body).await;
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
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
