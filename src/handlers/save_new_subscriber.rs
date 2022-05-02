use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::new_subscriber::NewSubscriber;
use crate::events::subscription_created::SubscriptionCreated;

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(config, pg_pool, nats_connection, new_subscriber)
)]
pub async fn save_new_subscriber(
    config: &Config,
    pg_pool: &PgPool,
    nats_connection: &async_nats::Connection,
    new_subscriber: NewSubscriber,
) -> anyhow::Result<()> {
    let subscription_token = Uuid::new_v4();
    let mut tx = pg_pool.begin().await?;
    let subscription_id = SubscriptionQueries::insert_subscriber(&mut tx, &new_subscriber).await?;
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
    Ok(())
}
