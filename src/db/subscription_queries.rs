use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::db::types::Tx;
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscription_status::SubscriptionStatus;

pub struct SubscriptionQueries;

pub struct SubscriptionRecord {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub status: SubscriptionStatus,
    pub subscribed_at: DateTime<Utc>,
}

impl SubscriptionQueries {
    #[tracing::instrument(name = "Insert new subscription", skip(tx))]
    pub async fn insert_subscriber(
        tx: &mut Tx<'_>,
        new_subscriber: &NewSubscriber,
        status: SubscriptionStatus,
    ) -> anyhow::Result<Uuid> {
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
        .bind(status)
        .bind(Utc::now())
        .execute(tx)
        .await?;
        Ok(id)
    }

    #[tracing::instrument(name = "Update subscription status", skip(tx))]
    pub async fn update_subscription_status(
        tx: &mut Tx<'_>,
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
        .execute(tx)
        .await?;
        Ok(())
    }

    #[tracing::instrument(name = "Store subscription token in the database", skip(tx))]
    pub async fn store_token(
        tx: &mut Tx<'_>,
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
        .execute(tx)
        .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Fetch subscription id by token from the database",
        skip(executor)
    )]
    pub async fn fetch_subscription_id_by_token<'a, E>(
        executor: E,
        subscription_token: &str,
    ) -> anyhow::Result<Option<Uuid>>
    where
        E: Executor<'a, Database = Postgres>,
    {
        let result = sqlx::query!(
            r#"
                SELECT subscriber_id 
                FROM subscription_tokens 
                WHERE subscription_token = $1
            "#,
            subscription_token
        )
        .fetch_optional(executor)
        .await?;
        Ok(result.map(|r| r.subscriber_id))
    }

    #[tracing::instrument(name = "Delete subscription token from the database", skip(tx))]
    pub async fn delete_token(tx: &mut Tx<'_>, subscription_token: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
                DELETE FROM subscription_tokens
                WHERE subscription_token = $1
            "#,
        )
        .bind(subscription_token)
        .execute(tx)
        .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Fetching a subscription by email from the database",
        skip(executor)
    )]
    pub async fn fetch_subscription_by_email<'a, E>(
        executor: E,
        email: &str,
    ) -> anyhow::Result<Option<SubscriptionRecord>>
    where
        E: Executor<'a, Database = Postgres>,
    {
        let maybe_record = sqlx::query_as!(
            SubscriptionRecord,
            r#"
                SELECT id, email, name, subscribed_at, status AS "status: _" 
                FROM subscriptions 
                WHERE email = $1
            "#,
            email,
        )
        .fetch_optional(executor)
        .await?;
        Ok(maybe_record)
    }
}
