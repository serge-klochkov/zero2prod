use crate::domain::new_subscriber::NewSubscriber;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::subscription_status::SubscriptionStatus;

pub struct SubscriptionQueries {
    pg_pool: Arc<PgPool>,
}

impl SubscriptionQueries {
    pub fn new(pg_pool: Arc<PgPool>) -> Self {
        Self { pg_pool }
    }

    #[tracing::instrument(name = "Insert new subscription", skip(self))]
    pub async fn insert_subscriber(&self, new_subscriber: &NewSubscriber) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
                INSERT INTO subscriptions (id, email, name, status, subscribed_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT DO NOTHING
            "#,
        )
        .bind(id)
        .bind(new_subscriber.email.as_ref())
        .bind(new_subscriber.name.as_ref())
        .bind(SubscriptionStatus::Pending)
        .bind(Utc::now())
        .execute(self.pg_pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(id)
    }

    #[tracing::instrument(name = "Update subscription status", skip(self))]
    pub async fn update_subscription_status(
        &self,
        subscription_id: &Uuid,
        status: SubscriptionStatus,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
                UPDATE subscriptions 
                SET status = $1
                WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(subscription_id)
        .execute(self.pg_pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(())
    }

    #[tracing::instrument(name = "Store subscription token in the database", skip(self))]
    pub async fn store_token(
        &self,
        subscriber_id: &Uuid,
        subscription_token: &Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
                INSERT INTO subscription_tokens (subscription_token, subscriber_id)
                VALUES ($1, $2)
            "#,
        )
        .bind(subscription_token.to_string().as_str())
        .bind(subscriber_id)
        .execute(self.pg_pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Fetch subscription email by token from the database",
        skip(self)
    )]
    pub async fn fetch_subscription_id_by_token(
        &self,
        subscription_token: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        let result = sqlx::query!(
            r#"
                SELECT subscriber_id 
                FROM subscription_tokens 
                WHERE subscription_token = $1
            "#,
            subscription_token
        )
        .fetch_optional(self.pg_pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(result.map(|r| r.subscriber_id))
    }
}
