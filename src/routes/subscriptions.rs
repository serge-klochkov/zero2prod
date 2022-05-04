use crate::config::Config;
use crate::domain::new_subscriber::NewSubscriber;
use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;
use crate::handlers::save_new_subscriber::{save_new_subscriber, SaveNewSubscriberOutput};
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

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
        subscriber_name = %form.name
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
    match save_new_subscriber(&config, &pg_pool, &nats_connection, new_subscriber).await {
        Ok(SaveNewSubscriberOutput::AlreadySubscribed) => HttpResponse::Conflict().finish(),
        Ok(SaveNewSubscriberOutput::Success | SaveNewSubscriberOutput::ResendConfirmation) => {
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!(error = ?err, "Failed to add a new subscriber");
            HttpResponse::InternalServerError().finish()
        }
    }
}
