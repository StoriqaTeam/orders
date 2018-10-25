use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Duration as ChronoDuration;
use failure::Error as FailureError;
use failure::Fail;
use futures::future;
use futures::future::Either;
use futures::prelude::*;
use tokio::timer::Interval;

use config::{self, Config};
use models::{UserLogin, UserRole};
use services::{OrderService, OrderServiceImpl};

use stq_db::pool::Pool as DbPool;
use stq_roles::models::{RepoLogin, RoleEntry};
use stq_types::{RoleEntryId, UserId};

#[derive(Clone)]
pub struct DeliveredStateTracking {
    busy: Arc<Mutex<bool>>,
    db_pool: DbPool,
    config: Option<config::DeliveredOrders>,
    duration: Duration,
}

#[derive(Clone)]
pub struct DeliveredStateTrackingEnvironment {
    pub db_pool: DbPool,
    pub config: Arc<Config>,
}

impl DeliveredStateTracking {
    /// One hour
    const DEFAULT_DURATION: u64 = 60 * 60;

    pub fn new(env: DeliveredStateTrackingEnvironment) -> DeliveredStateTracking {
        DeliveredStateTracking {
            busy: Arc::new(Mutex::new(false)),
            duration: Self::duration(env.config.delivered_orders.as_ref()),
            config: env.config.delivered_orders.clone(),
            db_pool: env.db_pool.clone(),
        }
    }

    pub fn start(self) -> impl Stream<Item = (), Error = FailureError> {
        info!("DeliveredStateTracking started with config {:?}.", self.config.as_ref());
        let interval = Interval::new_interval(self.duration).map_err(|e| e.context("timer creation error").into());

        interval.and_then(move |_| {
            if let Some(config) = self.config.clone() {
                let busy = *self.busy.lock().expect("DeliveredStateTracking: poisoned mutex at fetch step");
                if busy {
                    warn!("DeliveredStateTracking: tried to ping DeliveredStateTracking, but it was busy");
                    Either::A(future::ok(()))
                } else {
                    Either::B(self.clone().make_step(config))
                }
            } else {
                error!("DeliveredStateTracking: disabled. Config section [delivered_orders] not set.");
                Either::A(future::ok(()))
            }
        })
    }

    fn make_step(self, config: config::DeliveredOrders) -> impl Future<Item = (), Error = FailureError> {
        {
            let mut busy = self.busy.lock().expect("DeliveredStateTracking: poisoned mutex at fetch step");
            *busy = true;
        }
        let busy = self.busy.clone();
        let max_delivered_state_duration = ChronoDuration::days(config.delivery_state_duration_days);
        let self_clone = self.clone();
        let service = OrderServiceImpl::new(self_clone.db_pool.clone(), super_user());
        service
            .track_delivered_orders(max_delivered_state_duration)
            .then(move |res| {
                let mut busy = busy.lock().expect("DeliveredStateTracking: poisoned mutex at fetch step");
                *busy = false;
                res
            }).and_then(|_| ::future::ok(()))
    }

    fn duration(delivered_orders: Option<&config::DeliveredOrders>) -> Duration {
        match delivered_orders {
            Some(config) => Duration::from_secs(config.interval_s),
            None => Duration::from_secs(DeliveredStateTracking::DEFAULT_DURATION),
        }
    }
}

fn super_user() -> UserLogin {
    RepoLogin::User {
        caller_id: UserId(1),
        caller_roles: vec![RoleEntry {
            id: RoleEntryId::new(),
            user_id: UserId(1),
            role: UserRole::Superadmin,
        }],
    }
}
