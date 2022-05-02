use sqlx::PgPool;

use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::subscription_status::SubscriptionStatus;

pub enum ConfirmSubscriptionResult {
    Success,
    TokenNotFound,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(pg_pool))]
pub async fn confirm_subscription(
    subscription_token: &str,
    pg_pool: &PgPool,
) -> anyhow::Result<ConfirmSubscriptionResult> {
    let mut tx = pg_pool.begin().await?;
    let maybe_subscription_id =
        SubscriptionQueries::fetch_subscription_id_by_token(&mut tx, subscription_token).await?;
    match maybe_subscription_id {
        // Non-existing token = 400
        None => Ok(ConfirmSubscriptionResult::TokenNotFound),
        Some(id) => {
            SubscriptionQueries::update_subscription_status(
                &mut tx,
                &id,
                SubscriptionStatus::Confirmed,
            )
            .await?;
            let _ = SubscriptionQueries::delete_token(&mut tx, subscription_token).await?;
            tx.commit().await?;
            Ok(ConfirmSubscriptionResult::Success)
        }
    }
}
