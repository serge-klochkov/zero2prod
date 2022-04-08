use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::config::CONFIG;

use zero2prod::startup::run;
use zero2prod::telemetry;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static _TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = telemetry::get_subscriber("test".into(), "debug".into());
    telemetry::init_subscriber(subscriber);
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    lazy_static::initialize(&CONFIG);
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let db_name = Uuid::new_v4().to_string();
    let database_url = CONFIG.database_url.expose_secret().as_str();
    let last_slash_index = database_url
        .rfind('/')
        .expect("Malformed DATABASE_URL: could not figure out connection string without db");
    let connection_string = &database_url[0..last_slash_index];
    let db_pool = get_db_pool(connection_string, &db_name).await;

    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
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
