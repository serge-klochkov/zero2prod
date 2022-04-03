use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    form: web::Form<FormData>, // Retrieving a connection from the application state!
    pg_pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email=%form.email,
        subscriber_name=%form.name
    );
    let _request_span_guard = request_span.enter();
    match sqlx::query(
        r#"
            INSERT INTO subscriptions (id, email, name, subscribed_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&form.email)
    .bind(&form.name)
    .bind(Utc::now())
    .execute(pg_pool.get_ref())
    .instrument(tracing::info_span!(
        "Saving new subscriber details in the database",
        %request_id
    ))
    .await
    {
        Ok(_) => {
            tracing::info!("Saved new subscriptions details");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
