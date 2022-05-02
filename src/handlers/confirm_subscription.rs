use sqlx::PgPool;

use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::subscription_status::SubscriptionStatus;

pub enum ConfirmSubscriptionResult {
    Success,
    TokenNotFound,
    UpdateSubscriptionStatusFailed,
    DatabaseError,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(pg_pool))]
pub async fn confirm_subscription(
    subscription_token: &str,
    pg_pool: &PgPool,
) -> ConfirmSubscriptionResult {
    match pg_pool.begin().await {
        Ok(mut tx) => {
            let fetch_result =
                SubscriptionQueries::fetch_subscription_id_by_token(&mut tx, subscription_token)
                    .await;
            let subscription_id = match fetch_result {
                Ok(subscription_id) => subscription_id,
                Err(_) => return ConfirmSubscriptionResult::DatabaseError,
            };
            match subscription_id {
                // Non-existing token = 400
                None => ConfirmSubscriptionResult::TokenNotFound,
                Some(id) => {
                    // TODO transaction
                    let update_result = SubscriptionQueries::update_subscription_status(
                        &mut tx,
                        &id,
                        SubscriptionStatus::Confirmed,
                    )
                    .await;
                    let _ = SubscriptionQueries::delete_token(&mut tx, subscription_token).await;
                    let commit_result = tx.commit().await;
                    if update_result.is_err() || commit_result.is_err() {
                        ConfirmSubscriptionResult::DatabaseError
                    } else {
                        ConfirmSubscriptionResult::Success
                    }
                }
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to summon a transaction");
            ConfirmSubscriptionResult::DatabaseError
        }
    }
}
