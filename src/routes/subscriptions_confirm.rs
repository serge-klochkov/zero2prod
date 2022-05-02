use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::handlers::confirm_subscription::{confirm_subscription, ConfirmSubscriptionResult};

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pg_pool))]
pub async fn subscriptions_confirm(
    parameters: web::Query<Parameters>,
    pg_pool: web::Data<PgPool>,
) -> HttpResponse {
    match confirm_subscription(&parameters.subscription_token, &pg_pool).await {
        ConfirmSubscriptionResult::Success => HttpResponse::Ok().finish(),
        ConfirmSubscriptionResult::TokenNotFound => HttpResponse::Unauthorized().finish(),
        ConfirmSubscriptionResult::UpdateSubscriptionStatusFailed => {
            HttpResponse::InternalServerError().finish()
        }
        ConfirmSubscriptionResult::DatabaseError => HttpResponse::InternalServerError().finish(),
    }
}
