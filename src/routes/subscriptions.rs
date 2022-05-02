use crate::config::Config;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::events::subscription_created::SubscriptionCreated;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pg_pool, nats_connection, config),
    fields(
        subscriber_email = %form.email,
        subscriber_name= %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pg_pool: web::Data<PgPool>,
    nats_connection: web::Data<async_nats::Connection>,
    config: web::Data<Config>,
) -> HttpResponse {
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match subscribe_handler(&config, &pg_pool, &nats_connection, new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

// TODO: extract to "handlers"?
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(config, pg_pool, nats_connection, new_subscriber)
)]
pub async fn subscribe_handler(
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
