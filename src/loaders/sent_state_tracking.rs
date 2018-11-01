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
use tokio_core::reactor::Handle;

use config::{self, Config};
use loaders::ups::{DeliveryState, UpsClient};
use models::{UserLogin, UserRole};
use sentry_integration::log_and_capture_error;
use services::{OrderService, OrderServiceImpl};

use stq_db::pool::Pool as DbPool;
use stq_roles::models::{RepoLogin, RoleEntry};
use stq_static_resources::OrderState;
use stq_types::{OrderId, OrderIdentifier, RoleEntryId, UserId};

#[derive(Clone)]
pub struct SentStateTracking {
    ups_client: UpsClient,
    busy: Arc<Mutex<bool>>,
    db_pool: DbPool,
    config: Option<config::SentOrders>,
    duration: Duration,
}

#[derive(Clone)]
pub struct SentStateTrackingEnvironment {
    pub db_pool: DbPool,
    pub config: Arc<Config>,
    pub handle: Arc<Handle>,
}

impl SentStateTracking {
    /// One hour
    const DEFAULT_DURATION: u64 = 60 * 60;

    pub fn new(env: SentStateTrackingEnvironment) -> SentStateTracking {
        let sent_config = &env.config.sent_orders;
        let access_license_number = sent_config
            .as_ref()
            .map(|config| config.ups_api_access_license_number.as_ref())
            .unwrap_or("")
            .to_string();
        let url = sent_config
            .as_ref()
            .map(|config| config.ups_api_url.as_ref())
            .unwrap_or("")
            .to_string();
        SentStateTracking {
            busy: Arc::new(Mutex::new(false)),
            duration: Self::duration(env.config.sent_orders.as_ref()),
            config: sent_config.clone(),
            db_pool: env.db_pool.clone(),
            ups_client: UpsClient::new(&env.handle, access_license_number, url),
        }
    }

    pub fn start(self) -> impl Stream<Item = (), Error = FailureError> {
        info!("SentStateTracking started");
        let interval = Interval::new_interval(self.duration).map_err(|e| e.context("timer creation error").into());

        interval.and_then(move |_| {
            if let Some(config) = self.config.clone() {
                let busy = *self
                    .busy
                    .lock()
                    .expect("SentStateTrackingEnvironment: poisoned mutex at fetch step");
                if busy {
                    warn!("SentStateTrackingEnvironment: tried to ping SentStateTrackingEnvironment, but it was busy");
                    Either::A(future::ok(()))
                } else {
                    Either::B(self.clone().make_step(config))
                }
            } else {
                error!("SentStateTrackingEnvironment: disabled. Config section [sent_orders] not set.");
                Either::A(future::ok(()))
            }
        })
    }

    fn make_step(self, config: config::SentOrders) -> impl Future<Item = (), Error = FailureError> {
        {
            let mut busy = self.busy.lock().expect("SentStateTracking: poisoned mutex at fetch step");
            *busy = true;
        }
        let busy = self.busy.clone();

        let service = self.create_service();
        let ups_client = self.ups_client.clone();
        service
            .get_orders_with_state(OrderState::Sent, ChronoDuration::days(config.sent_state_duration_days))
            .map(::futures::stream::iter_ok)
            .flatten_stream()
            .inspect(|order| {
                info!("Process order {} with track_id {:?}", order.id, order.track_id);
            }).filter_map(|order| match order.track_id {
                Some(track_id) => Some((order.id, track_id)),
                None => None,
            }).and_then(move |(order_id, track_id)| Self::delivery_state(ups_client.clone(), order_id, track_id))
            .filter(|(_order_id, state)| state == &DeliveryState::Delivered)
            .and_then(move |(order_id, state)| {
                info!("Change order {} with state {}", order_id, state);
                let service = self.create_service();
                service.set_order_state(OrderIdentifier::Id(order_id), OrderState::Delivered, None, None)
            }).then(|result| match result {
                Ok(_) => ::future::ok(()),
                Err(error) => {
                    log_and_capture_error(&error);
                    ::future::ok(())
                }
            }).fold((), fold_ok)
            .then(move |res| {
                let mut busy = busy.lock().expect("SentStateTracking: poisoned mutex at fetch step");
                *busy = false;
                res
            }).and_then(|_| ::future::ok(()))
    }

    fn delivery_state(
        ups_client: UpsClient,
        order_id: OrderId,
        track_id: String,
    ) -> impl Future<Item = (OrderId, DeliveryState), Error = FailureError> {
        ups_client
            .delivery_status(track_id)
            .map_err(From::from)
            .map(move |delivery_state| (order_id, delivery_state))
    }

    fn create_service(&self) -> OrderServiceImpl {
        OrderServiceImpl::new(self.db_pool.clone(), super_user())
    }

    fn duration(delivered_orders: Option<&config::SentOrders>) -> Duration {
        match delivered_orders {
            Some(config) => Duration::from_secs(config.interval_s),
            None => Duration::from_secs(Self::DEFAULT_DURATION),
        }
    }
}

fn fold_ok(_acc: (), _next: ()) -> impl Future<Item = (), Error = FailureError> {
    ::future::ok(())
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
