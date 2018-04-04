use std::env;
use std::net::IpAddr;

use config_crate::{Config as RawConfig, ConfigError, Environment, File};

enum Env {
    Development,
    Test,
    Production,
}

impl Env {
    fn new() -> Self {
        match env::var("RUN_MODE") {
            Ok(ref s) if s == "test" => Env::Test,
            Ok(ref s) if s == "production" => Env::Production,
            _ => Env::Development,
        }
    }

    fn to_string(&self) -> &'static str {
        match self {
            &Env::Development => "development",
            &Env::Production => "production",
            &Env::Test => "test",
        }
    }
}

/// Service configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct Listen {
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Database {
    pub dsn: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server listen address
    pub listen: Listen,
    /// Database settings
    pub db: Database,
}

impl Config {
    /// Creates config from base.toml, which are overwritten by <env>.toml, where
    /// env is one of development, test, production. After that it could be overwritten
    /// by env variables like STQ_ORDERS_LISTEN (this will override `listen` field in config)
    pub fn new() -> Result<Self, ConfigError> {
        let env = Env::new();
        let mut s = RawConfig::new();

        s.merge(File::with_name("config/base"))?;
        // Optional file specific for environment
        s.merge(File::with_name(&format!("config/{}", env.to_string())).required(false))?;

        // Add in settings from the environment (with a prefix of STQ_ORDERS)
        s.merge(Environment::with_prefix("STQ_ORDERS"))?;

        s.try_into()
    }
}
