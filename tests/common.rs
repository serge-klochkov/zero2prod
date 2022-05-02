use actix_web::dev::Server;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, PgConnection, PgPool};
use std::future::Future;
use std::net::TcpListener;
use std::time::Duration;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::config::Config;
use zero2prod::email_client::EmailClient;

use zero2prod::startup::run;
use zero2prod::telemetry;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the value
    // of `TEST_LOG` because the sink is part of the type returned by `get_subscriber`,
    // therefore they are not the same type. We could work around it, but this is the
    // most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        telemetry::init_subscriber(subscriber);
    };
});

pub async fn spawn_app() -> TestApp {
    let mut config = Config::new().expect("Failed to load config");
    // set application to the current test suite name
    // this way, NATS subjects will be prefixed differently
    // and we will have no test interference
    config.application_id = std::thread::current().name().unwrap().to_string();

    let _ = Lazy::force(&TRACING); // FIXME: use either Lazy or lazy_static! macro

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let db_name = Uuid::new_v4().to_string();
    let database_url = config.database_url.expose_secret().as_str();
    let last_slash_index = database_url
        .rfind('/')
        .expect("Malformed DATABASE_URL: could not figure out connection string without db");
    let connection_string = &database_url[0..last_slash_index];
    let db_pool = get_db_pool(connection_string, &db_name).await;

    let nats_connection =
        async_nats::connect(&format!("{}:{}", config.nats_host, config.nats_port))
            .await
            .expect("Could not connect to NATS");

    let mock_server = MockServer::builder().start().await;
    let email_client = EmailClient::new(
        "test@example.com",
        &mock_server.uri(),
        Duration::from_millis(1000),
        config.sendgrid_api_key.clone(),
    );

    let server: Server = run(
        listener,
        db_pool.clone(),
        nats_connection.clone(),
        email_client,
        config.clone(),
    )
    .expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        port,
        db_pool,
        db_name,
        mock_server,
        nats_connection,
        config,
    }
}

pub async fn get_db_pool(connection_string: &str, db_name: &str) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(connection_string)
        .await
        .expect("Failed to connect to Postgres");
    sqlx::query(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .execute(&mut connection)
        .await
        .expect("Failed to create database.");
    // Migrate database
    let database_url = format!("{}/{}", connection_string, db_name);
    let connection_pool = PgPool::connect(database_url.as_str())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[allow(dead_code)] // FIXME: associated function is never used: `eventually`
pub async fn eventually<F, Fut, T>(mut f: F, max_tries: u16, wait_between_tries: u16) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut counter = 0;
    loop {
        let result = f().await;
        if counter > max_tries {
            panic!("We tried so many times. Enough.")
        } else if result.is_err() {
            counter += 1;
            std::thread::sleep(Duration::from_millis(wait_between_tries as u64));
        } else {
            println!("Eventually succeeded after {} tries", counter + 1);
            return result.unwrap();
        }
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub db_name: String,
    pub mock_server: MockServer,
    pub nats_connection: async_nats::Connection,
    pub config: Config,
}

impl TestApp {
    #[allow(dead_code)] // FIXME: associated function is never used: `post_subscriptions`
    pub async fn post_subscriptions(&self, body: &str) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_owned())
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_received_requests(&self) -> anyhow::Result<Vec<wiremock::Request>> {
        let maybe_requests = self.mock_server.received_requests().await;
        let requests = maybe_requests.unwrap();
        if requests.len() > 0 {
            Ok(requests)
        } else {
            anyhow::bail!("Mock server has no received requests yet")
        }
    }
}
