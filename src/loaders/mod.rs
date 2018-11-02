use std::sync::Arc;

use bb8;
use bb8_postgres::PostgresConnectionManager;
use futures;
use futures::future;
use futures::prelude::*;
use tokio_core::reactor::Core;
use tokio_postgres::TlsMode;

use config::*;
use types::*;

mod delivered_state_tracking;
mod paid_delivered_report;
mod s3;
mod sent_state_tracking;
mod ups;

use self::delivered_state_tracking::*;
use self::paid_delivered_report::*;
use self::sent_state_tracking::*;

pub fn start_delivered_state_tracking(config: Config) {
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let db_pool = {
        let manager = PostgresConnectionManager::new(config.db.dsn.clone(), || TlsMode::None).unwrap();
        let remote = core.remote();
        DbPool::from(
            core.run(
                bb8::Pool::builder()
                    .min_idle(Some(1))
                    .build(manager, remote)
                    .map_err(|e| format_err!("{}", e)),
            ).expect("Failed to create connection pool"),
        )
    };
    let env = DeliveredStateTrackingEnvironment {
        db_pool,
        config: Arc::new(config),
    };
    handle.spawn(create_delivered_state_tracking_loader(env));

    core.run(future::empty::<(), ()>()).unwrap();
}

pub fn start_sent_state_tracking(config: Config) {
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let db_pool = {
        let manager = PostgresConnectionManager::new(config.db.dsn.clone(), || TlsMode::None).unwrap();
        let remote = core.remote();
        DbPool::from(
            core.run(
                bb8::Pool::builder()
                    .min_idle(Some(1))
                    .build(manager, remote)
                    .map_err(|e| format_err!("{}", e)),
            ).expect("Failed to create connection pool"),
        )
    };
    let env = SentStateTrackingEnvironment {
        db_pool,
        config: Arc::new(config),
        handle: handle.clone(),
    };
    handle.spawn(create_sent_state_tracking_loader(env));

    core.run(future::empty::<(), ()>()).unwrap();
}

pub fn start_paid_delivered_reporting(config: Config) {
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let db_pool = {
        let manager = PostgresConnectionManager::new(config.db.dsn.clone(), || TlsMode::None).unwrap();
        let remote = core.remote();
        DbPool::from(
            core.run(
                bb8::Pool::builder()
                    .min_idle(Some(1))
                    .build(manager, remote)
                    .map_err(|e| format_err!("{}", e)),
            ).expect("Failed to create connection pool"),
        )
    };
    let env = PaidDeliveredReportEnvironment {
        db_pool,
        config: Arc::new(config),
        handle: handle.clone(),
    };
    handle.spawn(create_paid_delivered_report(env));

    core.run(future::empty::<(), ()>()).unwrap();
}

fn create_paid_delivered_report(env: PaidDeliveredReportEnvironment) -> impl Future<Item = (), Error = ()> {
    future::result(PaidDeliveredReport::new(env))
        .map(|loader| loader.start())
        .flatten_stream()
        .or_else(|e| {
            error!("Error in paid delivered report: {:?}.", e);
            futures::future::ok(())
        }).for_each(|_| futures::future::ok(()))
}

fn create_delivered_state_tracking_loader(env: DeliveredStateTrackingEnvironment) -> impl Future<Item = (), Error = ()> {
    let loader = DeliveredStateTracking::new(env);

    let stream = loader.start();
    stream
        .or_else(|e| {
            error!("Error in delivered state tracking loader: {:?}.", e);
            futures::future::ok(())
        }).for_each(|_| futures::future::ok(()))
}

fn create_sent_state_tracking_loader(env: SentStateTrackingEnvironment) -> impl Future<Item = (), Error = ()> {
    let loader = SentStateTracking::new(env);

    let stream = loader.start();
    stream
        .or_else(|e| {
            error!("Error in sent state tracking loader: {:?}.", e);
            futures::future::ok(())
        }).for_each(|_| futures::future::ok(()))
}
