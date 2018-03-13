extern crate bb8;
extern crate bb8_postgres;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate stq_http;
extern crate stq_router;
extern crate tokio_core;
extern crate tokio_postgres;

use bb8_postgres::PostgresConnectionManager;
use futures::prelude::*;
use hyper::server::Http;
use std::process::exit;
use tokio_core::reactor::Core;
use tokio_postgres::TlsMode;

use stq_http::controller::Application;

mod controller;
mod types;

use controller::*;
use types::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    listen: String,
    dsn: String,
}

pub fn start_server(config: Config) {
    let mut core = Core::new().expect("Unexpected error creating event loop core");

    let manager = PostgresConnectionManager::new(config.dsn.clone(), || TlsMode::None).unwrap();
    let db_pool = bb8::Pool::builder()
        .build(manager, core.remote())
        .wait()
        .expect("Failed to create connection pool");

    let serve = Http::new()
        .serve_addr_handle(&config.listen, &core.handle(), move || {
            let controller = Box::new(controller::ControllerImpl::new(db_pool.clone()));

            // Prepare application
            let app = Application { controller };

            Ok(app)
        })
        .unwrap_or_else(|why| {
            error!("Http Server Initialization Error: {}", why);
            exit(1);
        });
}
