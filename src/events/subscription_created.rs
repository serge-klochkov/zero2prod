use crate::config::CONFIG;
use crate::db::subscription_queries::SubscriptionQueries;
use async_nats::Message;
use serde::{Deserialize, Serialize};
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
        skip(email_client, subscription_queries),
        fields(
            message_subject = %message.subject,
        )
    )]
    pub async fn process(
        email_client: &EmailClient,
        subscription_queries: &SubscriptionQueries,
        message: Message,
    ) -> anyhow::Result<()> {
        match serde_json::from_slice::<SubscriptionCreated>(&message.data) {
            Ok(event) => {
                let text_content = format!(
                    "Welcome to our newsletter!\n\
                    Visit {}/subscriptions/confirm?subscription_token={} \
                    to confirm your subscription",
                    CONFIG.application_base_url(),
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
                        let update_result = subscription_queries
                            .update_subscription_status(
                                &event.subscription_id,
                                SubscriptionStatus::Failed,
                            )
                            .await;
                        match update_result {
                            Ok(_) => {}
                            Err(_) => {
                                tracing::error!("Failed to mark subscription as failed")
                            }
                        }
                    }
                }
            }
            Err(_) => {
                tracing::error!("Could not deserialize message");
            }
        };
        Ok(())
    }

    #[tracing::instrument(name = "Publish SubscriptionCreated event", skip(nats_connection))]
    pub async fn publish(
        nats_connection: &async_nats::Connection,
        event: SubscriptionCreated,
    ) -> anyhow::Result<()> {
        nats_connection
            .publish(
                &CONFIG.nats_subscription_created_subject(),
                serde_json::to_vec(&event)?,
            )
            .await?;
        Ok(())
    }
}
