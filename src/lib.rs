extern crate bb8;
extern crate bb8_postgres;
extern crate chrono;
extern crate config as config_crate;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_state_stream;
extern crate hyper;
#[macro_use]
extern crate log as log_crate;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate stq_http;
extern crate stq_router;
extern crate tokio_core;
extern crate tokio_postgres;

use bb8_postgres::PostgresConnectionManager;
use chrono::prelude::*;
use env_logger::Builder as LogBuilder;
use futures::future;
use futures::prelude::*;
use hyper::server::Http;
use log_crate::LevelFilter as LogLevelFilter;
use std::env;
use std::io::Write;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use tokio_core::reactor::{Core, Remote};
use tokio_postgres::TlsMode;

use stq_http::controller::Application;

mod config;
pub mod controller;
pub mod errors;
pub mod log;
pub mod models;
pub mod repos;
pub mod services;
pub mod types;
pub mod util;

pub use config::*;

pub fn prepare_db(remote: Remote) -> Box<Future<Item = bb8::Pool<PostgresConnectionManager>, Error = tokio_postgres::Error>> {
    let config = config::Config::new().unwrap();
    let manager = PostgresConnectionManager::new(config.dsn.clone(), || TlsMode::None).unwrap();

    bb8::Pool::builder().min_idle(Some(10)).build(manager, remote)
}

/// Starts web server with the provided configuration
pub fn start_server<F: FnOnce() + 'static>(config: config::Config, port: Option<u16>, callback: F) {
    let mut builder = LogBuilder::new();
    builder
        .format(|formatter, record| {
            let now = Utc::now();
            writeln!(formatter, "{} - {} - {}", now.to_rfc3339(), record.level(), record.args())
        })
        .filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    // Prepare logger
    builder.init();

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

    let listen_address = {
        let port = port.unwrap_or(config.listen.port);
        SocketAddr::from((config.listen.host, port))
    };

    let serve = Http::new()
        .serve_addr_handle(&listen_address, &core.handle(), move || {
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
                    handle.spawn(conn.map(|_| ()).map_err(|why| error!("Server Error: {:?}", why)));
                    Ok(())
                }
            })
            .map_err(|_| ()),
    );

    info!("Listening on http://{}", listen_address);
    handle.spawn_fn(move || {
        callback();
        future::ok(())
    });
    core.run(future::empty::<(), ()>()).unwrap();
}
