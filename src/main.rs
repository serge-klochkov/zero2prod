use secrecy::ExposeSecret;
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::config::CONFIG;

use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    lazy_static::initialize(&CONFIG);

    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into());
    telemetry::init_subscriber(subscriber);
    tracing::info!("Connecting to Postgres");

    let connection_pool = PgPool::connect(CONFIG.database_url.expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    let nats_connection =
        async_nats::connect(&format!("{}:{}", CONFIG.nats_host, CONFIG.nats_port)).await?;

    let address = format!("{}:{}", CONFIG.application_host, CONFIG.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, nats_connection)?.await
}
