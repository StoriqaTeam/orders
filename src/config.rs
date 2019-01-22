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
    /// Sent Orders settings
    pub sent_orders: Option<SentOrders>,
    /// S3 config
    pub s3: Option<S3>,
    /// Paid and delivered report settings
    pub paid_delivered_report: Option<PaidDeliveredReports>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SentOrders {
    /// State check interval in seconds
    pub interval_s: u64,
    /// UPS api access licence number
    pub ups_api_access_license_number: String,
    /// UPS api url
    pub ups_api_url: String,
    /// How long in days order can be in sent state to be processed
    pub sent_state_duration_days: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveredOrders {
    /// State check interval in seconds
    pub interval_s: u64,
    /// How long in days order has to be in delivered state to be considered completed
    pub delivery_state_duration_days: i64,
    /// Saga url
    pub saga_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaidDeliveredReports {
    pub interval_s: u64,
}

/// AWS S3 credentials
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct S3 {
    pub key: String,
    pub secret: String,
    pub region: String,
    pub bucket: String,
    pub acl: String,
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
