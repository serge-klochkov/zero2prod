use config::Config;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
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
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");
    // Detect the running environment.
    let environment: String = std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "local".into());
    // Read the "default" configuration file
    let base_src = config::File::from(configuration_directory.join("base")).required(true);
    // Layer on the environment-specific values.
    let env_src = config::File::from(configuration_directory.join(environment)).required(true);
    Config::builder()
        .add_source(base_src)
        .add_source(env_src)
        .build()?
        .try_deserialize::<Settings>()
}
