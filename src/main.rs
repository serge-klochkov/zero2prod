use std::net::TcpListener;
use std::time::Duration;

use secrecy::ExposeSecret;
use sqlx::PgPool;

use zero2prod::config::Config;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = Config::new().expect("Failed to load config");

    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);
    tracing::info!("Connecting to Postgres");

    let connection_pool = PgPool::connect(config.database_url.expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    let nats_connection =
        async_nats::connect(&format!("{}:{}", config.nats_host, config.nats_port)).await?;

    let email_client = EmailClient::new(
        &config.email_client_sender_email,
        &config.email_client_base_url,
        Duration::from_secs(config.email_client_timeout_seconds as u64),
        config.sendgrid_api_key.clone(),
    );

    let address = format!("{}:{}", config.application_host, config.application_port);
    let listener = TcpListener::bind(address)?;
    run(
        listener,
        connection_pool,
        nats_connection,
        email_client,
        config,
    )?
    .await
}
