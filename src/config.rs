use std::env;
use std::net::IpAddr;
use stq_logging;

use config_crate::{Config as RawConfig, ConfigError, Environment, File};
use sentry_integration::SentryConfig;

/// Service configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Listen {
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Database {
    pub dsn: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Server listen address
    pub listen: Listen,
    /// Database settings
    pub db: Database,
    /// GrayLog settings
    pub graylog: Option<stq_logging::GrayLogConfig>,
    /// Sentry settings
    pub sentry: Option<SentryConfig>,
    /// Delivered Orders settings
    pub delivered_orders: Option<DeliveredOrders>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveredOrders {
    ///
    pub interval_s: u64,
    pub delivery_state_duration_days: i64,
}

static ENV_PREFIX: &'static str = "STQ_ORDERS";

/// Creates new app config struct
/// #Examples
/// ```
/// use orders_lib::*;
///
/// let config = Config::new();
/// ```
impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = RawConfig::new();
        s.merge(File::with_name("config/base"))?;

        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        // Add in settings from the environment (with a prefix of STQ_ORDERS)
        s.merge(Environment::with_prefix(ENV_PREFIX))?;

        s.try_into()
    }
}
