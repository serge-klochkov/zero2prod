use anyhow::Context;
use sqlx::PgPool;

use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::subscription_status::SubscriptionStatus;
use crate::handlers::errors::error_chain_fmt;

pub enum ConfirmSubscriptionOutput {
    Success,
    TokenNotFound,
}

#[derive(thiserror::Error)]
#[error(transparent)]
pub struct ConfirmSubscriptionError(#[from] anyhow::Error);

impl std::fmt::Debug for ConfirmSubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(pg_pool))]
pub async fn confirm_subscription(
    subscription_token: &str,
    pg_pool: &PgPool,
) -> Result<ConfirmSubscriptionOutput, ConfirmSubscriptionError> {
    let mut tx = pg_pool
        .begin()
        .await
        .context("Failed to acquire a transaction")?;
    let maybe_subscription_id =
        SubscriptionQueries::fetch_subscription_id_by_token(&mut tx, subscription_token)
            .await
            .context("Failed to fetch a subscription ID by the subscription token")?;
    match maybe_subscription_id {
        // Non-existing token = 400
        None => Ok(ConfirmSubscriptionOutput::TokenNotFound),
        Some(id) => {
            SubscriptionQueries::update_subscription_status(
                &mut tx,
                &id,
                SubscriptionStatus::Confirmed,
            )
            .await
            .context("Failed to update a subscription status to Confirmed")?;
            let _ = SubscriptionQueries::delete_token(&mut tx, subscription_token)
                .await
                .context("Failed to delete the subscription token")?;
            tx.commit()
                .await
                .context("Failed to commit the transaction")?;
            Ok(ConfirmSubscriptionOutput::Success)
        }
    }
}
