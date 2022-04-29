use std::net::TcpListener;

use crate::config::CONFIG;
use crate::db::subscription_queries::SubscriptionQueries;
use crate::email_client::EmailClient;
use crate::events::subscription_created::SubscriptionCreated;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe, subscriptions_confirm};

pub fn run(
    listener: TcpListener,
    pg_pool: PgPool,
    nats_connection: async_nats::Connection,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let pg_pool_data = web::Data::new(pg_pool);
    let nats_connection_data = web::Data::new(nats_connection);
    let email_client_data = web::Data::new(email_client);

    let nats_connection_data_clone = nats_connection_data.clone();
    let email_client_data_clone = email_client_data.clone();
    let subscription_queries_data =
        web::Data::new(SubscriptionQueries::new(pg_pool_data.into_inner()));
    let subscription_queries_data_clone = subscription_queries_data.clone();

    let _ = tokio::spawn(async move {
        let sub_created = nats_connection_data_clone
            .queue_subscribe(
                &CONFIG.nats_subscription_created_subject,
                &CONFIG.nats_subscription_created_group,
            )
            .await
            .expect("Failed to subscribe to SubscriptionCreated subject");
        if let Some(msg) = sub_created.next().await {
            let _ = SubscriptionCreated::process(
                &email_client_data_clone,
                &subscription_queries_data_clone,
                msg,
            )
            .await;
        }
    });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route(
                "/subscriptions/confirm",
                web::get().to(subscriptions_confirm),
            )
            .app_data(subscription_queries_data.clone())
            .app_data(nats_connection_data.clone())
            .app_data(email_client_data.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
