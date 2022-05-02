use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscription_status::SubscriptionStatus;
use crate::events::subscription_created::SubscriptionCreated;

pub enum SaveNewSubscriberResult {
    Success,
    ResendConfirmation,
    AlreadySubscribed,
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
) -> anyhow::Result<SaveNewSubscriberResult> {
    let subscription_token = Uuid::new_v4();
    let mut tx = pg_pool.begin().await?;
    let maybe_subscription =
        SubscriptionQueries::fetch_subscription_by_email(&mut tx, new_subscriber.email.as_ref())
            .await?;
    let status: SaveNewSubscriberResult;
    let subscription_id: Uuid;
    match maybe_subscription {
        Some(sub) if sub.status == SubscriptionStatus::Confirmed => {
            return Ok(SaveNewSubscriberResult::AlreadySubscribed)
        }
        // Failed = change to Pending
        Some(sub) if sub.status == SubscriptionStatus::Failed => {
            SubscriptionQueries::update_subscription_status(
                &mut tx,
                &sub.id,
                SubscriptionStatus::Pending,
            )
            .await?;
            subscription_id = sub.id;
            status = SaveNewSubscriberResult::ResendConfirmation;
        }
        // Pending = do nothing
        Some(sub) => {
            subscription_id = sub.id;
            status = SaveNewSubscriberResult::ResendConfirmation;
        }
        None => {
            subscription_id = SubscriptionQueries::insert_subscriber(
                &mut tx,
                &new_subscriber,
                SubscriptionStatus::Pending,
            )
            .await?;
            status = SaveNewSubscriberResult::Success;
        }
    }
    SubscriptionQueries::store_token(&mut tx, &subscription_id, &subscription_token).await?;
    tx.commit().await?;
    SubscriptionCreated::publish(
        config,
        nats_connection,
        SubscriptionCreated {
            email: new_subscriber.email,
            name: new_subscriber.name,
            subscription_token,
            subscription_id,
        },
    )
    .await?;
    Ok(status)
}
