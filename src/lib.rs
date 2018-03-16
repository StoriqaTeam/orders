extern crate bb8;
extern crate bb8_postgres;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_state_stream;
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
use futures::future;
use futures::prelude::*;
use hyper::server::Http;
use std::collections::HashMap;
use std::env::Vars;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio_core::reactor::Core;
use tokio_postgres::TlsMode;

use stq_http::controller::Application;

mod controller;
mod errors;
mod migrations;
mod models;
mod repos;
mod types;

use controller::*;
use types::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    listen: SocketAddr,
    dsn: String,
}

impl Config {
    pub fn from_vars(v: Vars) -> Result<Self, failure::Error> {
        let data = v.collect::<HashMap<String, String>>();

        Ok(Self {
            listen: data.get("LISTEN_ADDR")
                .ok_or(format_err!("Listen address is not specified"))?
                .parse()?,
            dsn: data.get("DATABASE_URL")
                .ok_or(format_err!("Database address is not specified"))?
                .clone(),
        })
    }
}

pub fn start_server(config: Config) {
    // Prepare logger
    env_logger::init();

    let mut core = Core::new().expect("Unexpected error creating event loop core");

    let manager = PostgresConnectionManager::new(config.dsn.clone(), || TlsMode::None).unwrap();
    let db_pool = Arc::new({
        let remote = core.remote();
        core.run(
            bb8::Pool::builder()
                .min_idle(Some(10))
                .build(manager, remote)
                .map_err(|e| format_err!("{}", e)),
        ).expect("Failed to create connection pool")
    });

    migrations::run(db_pool.clone())
        .wait()
        .expect("Failed to prepare database");

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

    let handle = core.handle();
    handle.spawn(
        serve
            .for_each({
                let handle = handle.clone();
                move |conn| {
                    handle.spawn(
                        conn.map(|_| ())
                            .map_err(|why| error!("Server Error: {:?}", why)),
                    );
                    Ok(())
                }
            })
            .map_err(|_| ()),
    );
    core.run(future::empty::<(), ()>()).unwrap();
}
