use std::fs::File;
use std::io::Read;

use secrecy::Secret;
use serde::Deserialize;

use lazy_static::lazy_static;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub application_host: String,
    pub application_port: u16,
    pub database_url: Secret<String>,
    pub nats_host: String,
    pub nats_port: u16,
    pub nats_subscription_created_subject: String,
    pub nats_subscription_created_group: String,
    pub sendgrid_api_key: Secret<String>,
    pub email_client_sender_email: String,
    pub email_client_base_url: String,
    pub email_client_timeout_seconds: u16,
}

fn load_config() -> anyhow::Result<Config> {
    let env = envy::from_env::<Config>();
    match env {
        // if we could load the config using the existing env variables - use that
        Ok(config) => Ok(config),
        // otherwise, try to load the .env file
        Err(_) => {
            // simulate https://www.npmjs.com/package/dotenv behavior
            // load order: OS environment -> .env.local file -> .env file
            let _ = set_env_from_file_content(".env.local");
            set_env_from_file_content(".env")?;
            match envy::from_env::<Config>() {
                Ok(config) => Ok(config),
                Err(e) => panic!("Failed to read the config from env: {}", e),
            }
        }
    }
}

fn set_env_from_file_content(file_path: &str) -> anyhow::Result<()> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    for line in content.lines() {
        match line.find('=') {
            None => {}
            Some(eq_pos) => {
                let key = &line[..eq_pos];
                // we don't want to override already set variables
                if std::env::var(key).is_err() {
                    let value = &line[(eq_pos + 1)..];
                    std::env::set_var(key, value);
                }
            }
        }
    }
    Ok(())
}

lazy_static! {
    pub static ref CONFIG: Config = load_config().unwrap();
}
