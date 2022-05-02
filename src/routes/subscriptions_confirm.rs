use crate::db::subscription_queries::SubscriptionQueries;
use crate::domain::subscription_status::SubscriptionStatus;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pg_pool))]
pub async fn subscriptions_confirm(
    parameters: web::Query<Parameters>,
    pg_pool: web::Data<PgPool>,
) -> HttpResponse {
    // TODO transaction
    match pg_pool.begin().await {
        Ok(mut tx) => {
            let fetch_result = SubscriptionQueries::fetch_subscription_id_by_token(
                &mut tx,
                &parameters.subscription_token,
            )
            .await;
            let subscription_id = match fetch_result {
                Ok(subscription_id) => subscription_id,
                Err(_) => return HttpResponse::InternalServerError().finish(),
            };
            match subscription_id {
                // Non-existing token = 400
                None => HttpResponse::Unauthorized().finish(),
                Some(id) => {
                    // TODO transaction
                    let update_result = SubscriptionQueries::update_subscription_status(
                        &mut tx,
                        &id,
                        SubscriptionStatus::Confirmed,
                    )
                    .await;
                    let _ =
                        SubscriptionQueries::delete_token(&mut tx, &parameters.subscription_token)
                            .await;
                    let commit_result = tx.commit().await;
                    if update_result.is_err() || commit_result.is_err() {
                        HttpResponse::InternalServerError().finish()
                    } else {
                        HttpResponse::Ok().finish()
                    }
                }
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to summon a transaction");
            HttpResponse::InternalServerError().finish()
        }
    }
}
