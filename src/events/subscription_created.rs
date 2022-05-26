use crate::config::Config;
use crate::db::subscription_queries::SubscriptionQueries;
use anyhow::Context;
use async_nats::Message;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::domain::subscription_status::SubscriptionStatus;
use crate::email_client::EmailClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionCreated {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub subscription_token: Uuid,
    pub subscription_id: Uuid,
}

impl SubscriptionCreated {
    #[tracing::instrument(
        name = "Processing SubscriptionCreated event",
        skip(config, email_client, pg_pool, message),
        fields(
            message_subject = %message.subject,
        )
    )]
    pub async fn process(
        config: &Config,
        email_client: &EmailClient,
        pg_pool: &PgPool,
        message: Message,
    ) -> anyhow::Result<()> {
        match serde_json::from_slice::<SubscriptionCreated>(&message.data) {
            Ok(event) => {
                let text_content = format!(
                    "Welcome to our newsletter!\n\
                    Visit {}/subscriptions/confirm?subscription_token={} \
                    to confirm your subscription",
                    config.application_base_url(),
                    event.subscription_token
                );
                let mail_send_result = email_client
                    .send_email(&event.email, "Subscription confirmation", &text_content)
                    .await;
                // TODO: should be proper retry mechanism with different retry + final fail branches
                match mail_send_result {
                    Ok(_) => {
                        tracing::info!("SubscriptionCreated event email sent")
                    }
                    Err(err) => {
                        tracing::error!(
                            error = %err,
                            "Failed to send SubscriptionCreated event mail, \
                            setting the subscription status to failed",
                        );
                        let mut tx = pg_pool
                            .begin()
                            .await
                            .context("Failed to acquire a transaction")?;
                        let update_result = SubscriptionQueries::update_subscription_status(
                            &mut tx,
                            &event.subscription_id,
                            SubscriptionStatus::Failed,
                        )
                        .await;
                        let _ = SubscriptionQueries::delete_token(
                            &mut tx,
                            &event.subscription_token.to_string(),
                        )
                        .await;
                        match update_result {
                            Ok(_) => {}
                            Err(_) => {
                                tracing::error!("Failed to mark subscription as failed")
                            }
                        }
                        tx.commit()
                            .await
                            .context("Failed to commit the transaction")?;
                    }
                }
            }
            Err(_) => {
                tracing::error!("Could not deserialize message");
            }
        };
        Ok(())
    }

    pub fn subscribe(
        nats_connection: Arc<async_nats::Connection>,
        config: Arc<Config>,
        email_client: Arc<EmailClient>,
        pg_pool: Arc<PgPool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let sub_created = nats_connection
                .queue_subscribe(
                    &config.nats_subscription_created_subject(),
                    &config.nats_subscription_created_group(),
                )
                .await
                .expect("Failed to subscribe to SubscriptionCreated subject");
            if let Some(msg) = sub_created.next().await {
                let _ = SubscriptionCreated::process(&config, &email_client, &pg_pool, msg).await;
            }
        })
    }
}
