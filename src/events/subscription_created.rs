use crate::config::CONFIG;
use async_nats::Message;
use serde::{Deserialize, Serialize};

use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionCreated {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl SubscriptionCreated {
    #[tracing::instrument(
        name = "Processing SubscriptionCreated event",
        skip(message),
        fields(
            message_subject = %message.subject,
        )
    )]
    pub async fn process(message: Message) {
        let sc: SubscriptionCreated = match serde_json::from_slice(&message.data) {
            Ok(sc) => sc,
            Err(_) => {
                tracing::error!("Could not deserialize message"); // TODO log the message itself
                return;
            }
        };
        tracing::info!("SubscriptionCreated event processed: {:?}", sc);
        // TODO send email here
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
