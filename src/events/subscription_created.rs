use crate::config::CONFIG;
use async_nats::Message;
use serde::{Deserialize, Serialize};

use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::email_client::EmailClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionCreated {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl SubscriptionCreated {
    #[tracing::instrument(
        name = "Processing SubscriptionCreated event",
        skip(email_client, message),
        fields(
            message_subject = %message.subject,
        )
    )]
    pub async fn process(email_client: &EmailClient, message: Message) -> anyhow::Result<()> {
        match serde_json::from_slice::<SubscriptionCreated>(&message.data) {
            Ok(event) => {
                email_client
                    .send_email(
                        event.email,
                        "Subscription confirmation",
                        &format!("Hello {}", event.name.as_ref()),
                    )
                    .await?
                // tracing::info!("SubscriptionCreated event processed: {:?}", event)
            }
            Err(_) => {
                tracing::error!("Could not deserialize message"); // TODO log the message itself
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
                &CONFIG.nats_subscription_created_subject,
                serde_json::to_vec(&event)?,
            )
            .await?;
        Ok(())
    }
}
