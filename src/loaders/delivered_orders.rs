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

use stq_api::orders::OrderSearchTerms;
use stq_db::pool::Pool as DbPool;
use stq_roles::models::{RepoLogin, RoleEntry};
use stq_static_resources::OrderState;
use stq_types::{OrderIdentifier, RoleEntryId, UserId};

#[derive(Clone)]
pub struct DeliveredOrdersLoader {
    busy: Arc<Mutex<bool>>,
    db_pool: DbPool,
    config: Option<config::DeliveredOrders>,
    duration: Duration,
    delivery_state_duration: ChronoDuration,
}

#[derive(Clone)]
pub struct DeliveredOrdersEnvironment {
    pub db_pool: DbPool,
    pub config: Arc<Config>,
}

impl DeliveredOrdersLoader {
    const DEFAULT_DURATION: u64 = 60 * 60 * 24;
    const DEFAULT_DELIVERY_STATE_DURATION_DAYS: i64 = 1;

    pub fn new(env: DeliveredOrdersEnvironment) -> DeliveredOrdersLoader {
        DeliveredOrdersLoader {
            busy: Arc::new(Mutex::new(false)),
            duration: Self::duration(env.config.delivered_orders.as_ref()),
            config: env.config.delivered_orders.clone(),
            db_pool: env.db_pool.clone(),
            delivery_state_duration: Self::delivery_state_duration(env.config.delivered_orders.as_ref()),
        }
    }

    pub fn start(self) -> impl Stream<Item = (), Error = FailureError> {
        info!("DeliveredOrdersLoader started with config {:?}.", self.config.as_ref());
        let interval = Interval::new_interval(self.duration).map_err(|e| e.context("timer creation error").into());

        interval.and_then(move |_| {
            if self.config.as_ref().is_some() {
                let busy = *self.busy.lock().expect("DeliveredOrdersLoader: poisoned mutex at fetch step");
                if busy {
                    warn!("DeliveredOrdersLoader: tried to ping DeliveredOrdersLoader, but it was busy");
                    Either::A(future::ok(()))
                } else {
                    Either::B(self.clone().make_step())
                }
            } else {
                error!("DeliveredOrdersLoader: disabled. Config section [delivered_orders] not set.");
                Either::A(future::ok(()))
            }
        })
    }

    fn make_step(self) -> impl Future<Item = (), Error = FailureError> {
        {
            let mut busy = self.busy.lock().expect("DeliveredOrdersLoader: poisoned mutex at fetch step");
            *busy = true;
        }
        let busy = self.busy.clone();

        let search_delivered_orders = OrderSearchTerms {
            slug: None,
            created_from: None,
            created_to: None,
            payment_status: None,
            customer: None,
            store: None,
            state: Some(OrderState::Delivered),
        };
        let self_clone = self.clone();
        let service = OrderServiceImpl::new(self_clone.db_pool.clone(), super_user());
        service
            .search(search_delivered_orders)
            .map(move |delivered_orders| {
                let now = ::chrono::offset::Utc::now();
                delivered_orders
                    .into_iter()
                    .filter(move |order| (now - order.updated_at) >= self.delivery_state_duration)
                    .map(move |old_delivered_order| {
                        info!("Updating order state for order {}", old_delivered_order.id);
                        service.set_order_state(OrderIdentifier::Id(old_delivered_order.id), OrderState::Complete, None, None)
                    })
            }).and_then(::futures::future::join_all)
            .then(move |res| {
                let mut busy = busy.lock().expect("DeliveredOrdersLoader: poisoned mutex at fetch step");
                *busy = false;
                res
            }).and_then(|_| ::future::ok(()))
    }

    fn duration(delivered_orderes: Option<&config::DeliveredOrders>) -> Duration {
        match delivered_orderes {
            Some(config) => Duration::from_secs(config.interval_s),
            None => Duration::from_secs(DeliveredOrdersLoader::DEFAULT_DURATION),
        }
    }

    fn delivery_state_duration(delivered_orderes: Option<&config::DeliveredOrders>) -> ChronoDuration {
        match delivered_orderes {
            Some(config) => ChronoDuration::days(config.delivery_state_duration_days),
            None => ChronoDuration::days(DeliveredOrdersLoader::DEFAULT_DELIVERY_STATE_DURATION_DAYS),
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
