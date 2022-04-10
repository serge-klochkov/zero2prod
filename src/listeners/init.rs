use crate::config::CONFIG;
use crate::events::subscription_created::SubscriptionCreated;

pub async fn init_listeners(nats_connection: &async_nats::Connection) -> anyhow::Result<()> {
    let sub_created = nats_connection
        .queue_subscribe(
            &CONFIG.nats_subscription_created_subject,
            &CONFIG.nats_subscription_created_group,
        )
        .await?;
    if let Some(msg) = sub_created.next().await {
        SubscriptionCreated::process(msg).await;
    }
    Ok(())
}
