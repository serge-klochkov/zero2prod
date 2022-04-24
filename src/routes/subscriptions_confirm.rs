use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::subscription_status::SubscriptionStatus;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(subscription_queries))]
pub async fn subscriptions_confirm(
    parameters: web::Query<Parameters>,
    subscription_queries: web::Data<SubscriptionQueries>,
) -> HttpResponse {
    let fetch_result = subscription_queries
        .fetch_subscription_id_by_token(&parameters.subscription_token)
        .await;
    let subscription_id = match fetch_result {
        Ok(subscription_id) => subscription_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match subscription_id {
        // Non-existing token = 400
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => {
            let update_result = subscription_queries
                .update_subscription_status(&id, SubscriptionStatus::Confirmed)
                .await;
            if update_result.is_err() {
                HttpResponse::InternalServerError().finish()
            } else {
                HttpResponse::Ok().finish()
            }
        }
    }
}
