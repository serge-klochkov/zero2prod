use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use crate::handlers::confirm_subscription::{confirm_subscription, ConfirmSubscriptionOutput};

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pg_pool))]
pub async fn subscriptions_confirm(
    parameters: web::Query<Parameters>,
    pg_pool: web::Data<PgPool>,
) -> HttpResponse {
    if Uuid::from_str(&parameters.subscription_token).is_ok() {
        match confirm_subscription(&parameters.subscription_token, &pg_pool).await {
            Ok(ConfirmSubscriptionOutput::Success) => HttpResponse::Ok().finish(),
            Ok(ConfirmSubscriptionOutput::TokenNotFound) => HttpResponse::Unauthorized().finish(),
            Err(err) => {
                tracing::error!(error = ?err, "Failed to confirm a subscription");
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        HttpResponse::BadRequest().finish()
    }
}
