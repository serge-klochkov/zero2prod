use config::Config;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string_without_db(&self) -> Secret<String> {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
        .into()
    }
    pub fn connection_string(&self) -> Secret<String> {
        format!(
            "{}/{}",
            self.connection_string_without_db().expose_secret(),
            self.database_name
        )
        .into()
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    Config::builder()
        .add_source(config::File::with_name("configuration"))
        .build()?
        .try_deserialize::<Settings>()
}
