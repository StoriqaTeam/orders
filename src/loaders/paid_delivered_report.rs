use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::prelude::*;
use csv::Writer;
use failure::Error as FailureError;
use failure::Fail;
use futures::future;
use futures::future::Either;
use futures::prelude::*;
use futures::stream;
use tokio::timer::Interval;
use tokio_core::reactor::Handle;

use config::{self, Config};
use loaders::s3::S3Client;
use models::{OrderDiffFilter, UserLogin, UserRole};
use sentry_integration::log_and_capture_error;
use services::{OrderService, OrderServiceImpl, ServiceFuture};

use stq_api::orders::Order;
use stq_db::pool::Pool as DbPool;
use stq_roles::models::{RepoLogin, RoleEntry};
use stq_static_resources::{Currency, OrderState};
use stq_types::*;

#[derive(Clone)]
pub struct PaidDeliveredReport {
    busy: Arc<Mutex<bool>>,
    s3: S3Client,
    db_pool: DbPool,
    config: Option<config::PaidDeliveredReports>,
    interval: Duration,
}

#[derive(Clone)]
pub struct PaidDeliveredReportEnvironment {
    pub db_pool: DbPool,
    pub config: Arc<Config>,
    pub handle: Arc<Handle>,
}

struct UploadData {
    now: DateTime<Utc>,
    state: OrderState,
    orders: Vec<Order>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CsvOrder {
    id: OrderId,
    created_from: CartItemId,
    conversion_id: ConversionId,
    slug: OrderSlug,
    customer: UserId,
    store: StoreId,
    product: ProductId,
    price: ProductPrice,
    currency: Currency,
    quantity: Quantity,
    receiver_name: String,
    receiver_phone: String,
    receiver_email: String,
    state: OrderState,
    delivery_company: Option<String>,
    track_id: Option<String>,
    pre_order: bool,
    pre_order_days: i32,
    coupon_id: Option<CouponId>,
    coupon_percent: Option<i32>,
    coupon_discount: Option<ProductPrice>,
    product_discount: Option<ProductPrice>,
    total_amount: ProductPrice,
    //address
    administrative_area_level_1: Option<String>,
    administrative_area_level_2: Option<String>,
    country: Option<String>,
    locality: Option<String>,
    political: Option<String>,
    postal_code: Option<String>,
    route: Option<String>,
    street_number: Option<String>,
    address: Option<String>,
    place_id: Option<String>,
}

impl PaidDeliveredReport {
    /// One hour
    const DEFAULT_DURATION: u64 = 60 * 60;

    pub fn new(env: PaidDeliveredReportEnvironment) -> Result<PaidDeliveredReport, FailureError> {
        let config = &env.config.paid_delivered_report;

        let s3 = if let Some(s3_config) = env.config.s3.clone() {
            S3Client::new(s3_config)?
        } else {
            S3Client::create_dummy()
        };

        Ok(PaidDeliveredReport {
            busy: Arc::new(Mutex::new(false)),
            s3,
            interval: Self::interval(env.config.paid_delivered_report.as_ref()),
            config: config.clone(),
            db_pool: env.db_pool.clone(),
        })
    }

    pub fn start(self) -> impl Stream<Item = (), Error = FailureError> {
        info!("PaidDeliveredReport started with config {:?}", self.config);
        let interval = Interval::new(Instant::now(), self.interval).map_err(|e| e.context("timer creation error").into());

        interval.and_then(move |_| {
            if let Some(config) = self.config.clone() {
                let busy = *self.busy.lock().expect("PaidDeliveredReport: poisoned mutex at fetch step");
                if busy {
                    warn!("PaidDeliveredReport: tried to ping PaidDeliveredReport, but it was busy");
                    Either::A(future::ok(()))
                } else {
                    Either::B(self.clone().make_step(config))
                }
            } else {
                warn!("PaidDeliveredReport: disabled. Config section [paid_delivered_report] not set.");
                Either::A(future::ok(()))
            }
        })
    }

    fn make_step(self, _config: config::PaidDeliveredReports) -> impl Future<Item = (), Error = FailureError> {
        {
            let mut busy = self.busy.lock().expect("PaidDeliveredReport: poisoned mutex at fetch step");
            *busy = true;
        }
        let busy = self.busy.clone();
        let now = ::chrono::offset::Utc::now();
        let start_of_yesterday = now.date().pred().and_hms(0, 0, 0);
        let paid_diffs = OrderDiffFilter {
            committed_at_range: ::models::into_range(Some(start_of_yesterday), None),
            state: Some(OrderState::Paid.into()),
            ..Default::default()
        };
        let self_clone = self.clone();
        let paid_order_report = self.create_service().search_by_diffs(paid_diffs).and_then(move |orders| {
            self_clone.upload(UploadData {
                now,
                orders,
                state: OrderState::Paid,
            })
        });

        let delivered_diffs = OrderDiffFilter {
            committed_at_range: ::models::into_range(Some(start_of_yesterday), None),
            state: Some(OrderState::Delivered.into()),
            ..Default::default()
        };
        let self_clone = self.clone();
        let delivered_order_report = self.create_service().search_by_diffs(delivered_diffs).and_then(move |orders| {
            self_clone.upload(UploadData {
                now,
                orders,
                state: OrderState::Delivered,
            })
        });

        stream::futures_unordered(vec![
            Box::new(paid_order_report) as ServiceFuture<()>,
            Box::new(delivered_order_report) as ServiceFuture<()>,
        ]).then(|result| match result {
            Ok(_) => ::future::ok(()),
            Err(error) => {
                log_and_capture_error(&error);
                ::future::ok(())
            }
        }).fold((), fold_ok)
        .then(move |res| {
            let mut busy = busy.lock().expect("PaidDeliveredReport: poisoned mutex at fetch step");
            *busy = false;
            res
        }).and_then(|_| ::future::ok(()))
    }

    fn upload(&self, upload: UploadData) -> impl Future<Item = (), Error = FailureError> {
        let filename = upload.file_name();
        let upload_name = upload.name();
        if upload.is_empty() {
            info!("{} - no entries", upload_name);
            future::Either::A(future::ok(()))
        } else {
            info!("{} - uploading {} entries to s3", upload_name, upload.orders.len());
            let s3 = self.s3.clone();
            let upload_res = future::result(upload.into_csv()).and_then(move |csv| s3.upload(&filename, csv));
            future::Either::B(upload_res)
        }
    }

    fn create_service(&self) -> OrderServiceImpl {
        OrderServiceImpl::new(self.db_pool.clone(), super_user())
    }

    fn interval(config: Option<&config::PaidDeliveredReports>) -> Duration {
        match config {
            Some(config) => Duration::from_secs(config.interval_s),
            None => Duration::from_secs(Self::DEFAULT_DURATION),
        }
    }
}

impl UploadData {
    fn file_name(&self) -> String {
        let start_of_yesterday = self.now.date().pred().and_hms(0, 0, 0);
        let format_string = "%FT%T"; //2018-11-02T07:14:48
        format!(
            "{}_orders_{}_-_{}.csv",
            self.state,
            start_of_yesterday.format(format_string),
            self.now.format(format_string),
        )
    }

    fn name(&self) -> String {
        let start_of_yesterday = self.now.date().pred().and_hms(0, 0, 0);
        let format_string = "%FT%T"; //2018-11-02T07:14:48
        format!(
            "{} orders {} - {}",
            self.state,
            start_of_yesterday.format(format_string),
            self.now.format(format_string),
        )
    }

    fn into_csv(self) -> Result<Vec<u8>, FailureError> {
        let mut writer = Writer::from_writer(Vec::new());
        for order in self.orders {
            let csv_order = CsvOrder::from(order);
            writer.serialize(csv_order)?;
        }
        let res = writer.into_inner()?;
        Ok(res)
    }

    fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
}

impl From<Order> for CsvOrder {
    fn from(order: Order) -> CsvOrder {
        CsvOrder {
            id: order.id,
            created_from: order.created_from,
            conversion_id: order.conversion_id,
            slug: order.slug,
            customer: order.customer,
            store: order.store,
            product: order.product,
            price: order.price,
            currency: order.currency,
            quantity: order.quantity,
            receiver_name: order.receiver_name,
            receiver_phone: order.receiver_phone,
            receiver_email: order.receiver_email,
            state: order.state,
            delivery_company: order.delivery_company,
            track_id: order.track_id,
            pre_order: order.pre_order,
            pre_order_days: order.pre_order_days,
            coupon_id: order.coupon_id,
            coupon_percent: order.coupon_percent,
            coupon_discount: order.coupon_discount,
            product_discount: order.product_discount,
            total_amount: order.total_amount,
            administrative_area_level_1: order.address.administrative_area_level_1,
            administrative_area_level_2: order.address.administrative_area_level_2,
            country: order.address.country,
            locality: order.address.locality,
            political: order.address.political,
            postal_code: order.address.postal_code,
            route: order.address.route,
            street_number: order.address.street_number,
            address: order.address.address,
            place_id: order.address.place_id,
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

fn fold_ok(_acc: (), _next: ()) -> impl Future<Item = (), Error = FailureError> {
    ::future::ok(())
}
