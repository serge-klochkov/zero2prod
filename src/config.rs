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
}

fn load_config() -> anyhow::Result<Config> {
    let env = envy::from_env::<Config>();
    match env {
        // if we could load the config using the existing env variables - use that
        Ok(config) => Ok(config),
        // otherwise, try to load the .env file
        Err(_) => {
            // simulate https://www.npmjs.com/package/dotenv behavior
            let mut file = File::open(".env")?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            for line in content.lines() {
                let eq_pos = line
                    .find('=')
                    .unwrap_or_else(|| panic!("Expected env variable pairs, got {}", content));
                let key = &line[..eq_pos];
                let value = &line[(eq_pos + 1)..];
                std::env::set_var(key, value);
            }
            match envy::from_env::<Config>() {
                Ok(config) => Ok(config),
                Err(e) => panic!("Failed to read the config from env: {}", e),
            }
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = load_config().unwrap();
}
