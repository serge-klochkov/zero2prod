use crate::common::spawn_app;
use uuid::Uuid;

mod common;

/// See test_subscriptions_complete_flows for additional cases

#[tokio::test(flavor = "multi_thread")]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test(flavor = "multi_thread")]
async fn confirmations_with_a_malformed_token_are_rejected_with_a_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscription_token=foobar",
        app.address
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test(flavor = "multi_thread")]
async fn confirmations_with_a_non_existing_token_are_rejected_with_a_401() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscription_token={}",
        app.address,
        Uuid::new_v4().to_string()
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 401);
}
