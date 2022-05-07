use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::db::transaction::{begin_transaction, commit_transaction};
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscription_status::SubscriptionStatus;
use crate::events::subscription_created::SubscriptionCreated;
use crate::handlers::errors::error_chain_fmt;

pub enum SaveNewSubscriberOutput {
    Success,
    ResendConfirmation,
    AlreadySubscribed,
}

#[derive(thiserror::Error)]
#[error(transparent)]
pub struct SaveNewSubscriberError(#[from] anyhow::Error);

impl std::fmt::Debug for SaveNewSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(config, pg_pool, nats_connection, new_subscriber)
)]
pub async fn save_new_subscriber(
    config: &Config,
    pg_pool: &PgPool,
    nats_connection: &async_nats::Connection,
    new_subscriber: NewSubscriber,
) -> Result<SaveNewSubscriberOutput, SaveNewSubscriberError> {
    let subscription_token = Uuid::new_v4();
    let maybe_subscription =
        SubscriptionQueries::fetch_subscription_by_email(pg_pool, new_subscriber.email.as_ref())
            .await
            .context("Failed to fetch a subscription by the email")?;
    let status: SaveNewSubscriberOutput;
    let subscription_id: Uuid;
    let mut tx = match maybe_subscription {
        Some(sub) if sub.status == SubscriptionStatus::Confirmed => {
            return Ok(SaveNewSubscriberOutput::AlreadySubscribed)
        }
        maybe_subscription => {
            let mut tx = begin_transaction(pg_pool).await?;
            match maybe_subscription {
                // Failed = change to Pending
                Some(sub) if sub.status == SubscriptionStatus::Failed => {
                    SubscriptionQueries::update_subscription_status(
                        &mut tx,
                        &sub.id,
                        SubscriptionStatus::Pending,
                    )
                    .await
                    .context("Failed to update the subscription status to Pending")?;
                    subscription_id = sub.id;
                    status = SaveNewSubscriberOutput::ResendConfirmation;
                }
                // Resend = do nothing
                Some(sub) if sub.status == SubscriptionStatus::Pending => {
                    subscription_id = sub.id;
                    status = SaveNewSubscriberOutput::ResendConfirmation;
                }
                // Subscription does not exist, insert a new one
                _ => {
                    subscription_id = SubscriptionQueries::insert_subscriber(
                        &mut tx,
                        &new_subscriber,
                        SubscriptionStatus::Pending,
                    )
                    .await
                    .context("Failed to insert a new subscription")?;
                    status = SaveNewSubscriberOutput::Success;
                }
            }
            tx
        }
    };
    SubscriptionQueries::store_token(&mut tx, &subscription_id, &subscription_token)
        .await
        .context("Failed to store the subscription token")?;
    commit_transaction(tx).await?;
    let event = SubscriptionCreated {
        email: new_subscriber.email,
        name: new_subscriber.name,
        subscription_token,
        subscription_id,
    };
    nats_connection
        .publish(
            &config.nats_subscription_created_subject(),
            serde_json::to_vec(&event).context("Failed to serialize SubscriptionCreated event")?,
        )
        .await
        .context("Failed to publish SubscriptionCreated event")?;
    Ok(status)
}
