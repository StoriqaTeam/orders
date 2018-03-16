extern crate bb8;
extern crate bb8_postgres;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;

use bb8_postgres::{TlsMode, PostgresConnectionManager};
use tokio_core::reactor::Core;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    database: String,
}

pub fn start_server(config: Config) {
    let mut core = Core::new().expect("Unexpected error creating event loop core");

    let database_url: String = config
        .database
        .parse()
        .expect("Database URL must be set in configuration");
    let manager = PostgresConnectionManager::new(database_url, TlsMode::None).unwrap();
    let db_pool = bb8::Pool::builder()
        .build(manager, core.remote())
        .expect("Failed to create connection pool");
}