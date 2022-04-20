use std::net::TcpListener;

use crate::email_client::EmailClient;
use crate::listeners::init::init_listeners;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe};

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
    tokio::spawn(async move {
        init_listeners(&nats_connection_data_clone, &email_client_data_clone)
            .await
            .expect("Failed to init NATS listeners");
    });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(pg_pool_data.clone())
            .app_data(nats_connection_data.clone())
            .app_data(email_client_data.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
