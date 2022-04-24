use crate::config::CONFIG;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::email_client::EmailClient;
use crate::events::subscription_created::SubscriptionCreated;

pub async fn init_listeners(
    nats_connection: &async_nats::Connection,
    subscription_queries: &SubscriptionQueries,
    email_client: &EmailClient,
) -> anyhow::Result<()> {
    let sub_created = nats_connection
        .queue_subscribe(
            &CONFIG.nats_subscription_created_subject,
            &CONFIG.nats_subscription_created_group,
        )
        .await?;
    if let Some(msg) = sub_created.next().await {
        let _ = SubscriptionCreated::process(email_client, subscription_queries, msg).await;
    }
    Ok(())
}
