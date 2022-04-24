use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscriber_email::SubscriberEmail;
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

    #[tracing::instrument(name = "Insert subscriptions", skip(self))]
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

    #[tracing::instrument(name = "Mark subscription as failed", skip(self))]
    pub async fn mark_subscription_as_failed(&self, email: &SubscriberEmail) -> anyhow::Result<()> {
        sqlx::query(
            r#"
                UPDATE subscriptions 
                SET status = 'failed'
                WHERE email = $1
            "#,
        )
        .bind(email.as_ref())
        .execute(self.pg_pool.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
        Ok(())
    }
}
