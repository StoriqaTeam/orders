use chrono::prelude::*;
use failure::Fallible;
use std::str::FromStr;
use stq_api::orders::*;
use stq_db::statement::*;
use stq_static_resources::Currency;
use stq_static_resources::OrderState;
use stq_types::*;
use tokio_postgres::rows::Row;
use uuid::Uuid;

use super::*;

const ID_COLUMN: &str = "id";
const CREATED_FROM_COLUMN: &str = "created_from";
const CONVERSION_ID_COLUMN: &str = "conversion_id";
const SLUG_COLUMN: &str = "slug";
const CUSTOMER_COLUMN: &str = "customer";
const STORE_COLUMN: &str = "store";
const PRODUCT_COLUMN: &str = "product";
const PRICE_COLUMN: &str = "price";
const CURRENCY_COLUMN: &str = "currency";
const QUANTITY_COLUMN: &str = "quantity";
const RECEIVER_NAME_COLUMN: &str = "receiver_name";
const RECEIVER_PHONE_COLUMN: &str = "receiver_phone";
const RECEIVER_EMAIL_COLUMN: &str = "receiver_email";

const ADMINISTRATIVE_AREA_LEVEL_1_COLUMN: &str = "administrative_area_level_1";
const ADMINISTRATIVE_AREA_LEVEL_2_COLUMN: &str = "administrative_area_level_2";
const COUNTRY_COLUMN: &str = "country";
const LOCALITY_COLUMN: &str = "locality";
const POLITICAL_COLUMN: &str = "political";
const POSTAL_CODE_COLUMN: &str = "postal_code";
const ROUTE_COLUMN: &str = "route";
const STREET_NUMBER_COLUMN: &str = "street_number";
const ADDRESS_COLUMN: &str = "address";
const PLACE_ID_COLUMN: &str = "place_id";

const TRACK_ID_COLUMN: &str = "track_id";
const CREATED_AT_COLUMN: &str = "created_at";
const UPDATED_AT_COLUMN: &str = "updated_at";
const STATE_COLUMN: &str = "state";
const PAYMENT_STATUS_COLUMN: &str = "payment_status";
const DELIVERY_COMPANY_COLUMN: &str = "delivery_company";
const PRE_ORDER_COLUMN: &str = "pre_order";
const PRE_ORDER_DAYS_COLUMN: &str = "pre_order_days";
const COUPON_ID_COLUMN: &str = "coupon_id";
const PRODUCT_DISCOUNT_COLUMN: &str = "product_discount";
const COUPON_PERCENT_COLUMN: &str = "coupon_percent";
const COUPON_DISCOUNT_COLUMN: &str = "coupon_discount";
const TOTAL_AMOUNT_COLUMN: &str = "total_amount";
const COMPANY_PACKAGE_ID_COLUMN: &str = "company_package_id";
const DELIVERY_PRICE_COLUMN: &str = "delivery_price";
const SHIPPING_ID_COLUMN: &str = "shipping_id";

pub fn write_address_into_inserter(addr: AddressFull, mut b: InsertBuilder) -> InsertBuilder {
    if let Some(v) = addr.administrative_area_level_1 {
        b = b.with_arg(ADMINISTRATIVE_AREA_LEVEL_1_COLUMN, v);
    }
    if let Some(v) = addr.administrative_area_level_2 {
        b = b.with_arg(ADMINISTRATIVE_AREA_LEVEL_2_COLUMN, v);
    }
    if let Some(v) = addr.country {
        b = b.with_arg(COUNTRY_COLUMN, v);
    }
    if let Some(v) = addr.locality {
        b = b.with_arg(LOCALITY_COLUMN, v);
    }
    if let Some(v) = addr.political {
        b = b.with_arg(POLITICAL_COLUMN, v);
    }
    if let Some(v) = addr.postal_code {
        b = b.with_arg(POSTAL_CODE_COLUMN, v);
    }
    if let Some(v) = addr.route {
        b = b.with_arg(ROUTE_COLUMN, v);
    }
    if let Some(v) = addr.street_number {
        b = b.with_arg(STREET_NUMBER_COLUMN, v);
    }
    if let Some(v) = addr.address {
        b = b.with_arg(ADDRESS_COLUMN, v);
    }
    if let Some(v) = addr.place_id {
        b = b.with_arg(PLACE_ID_COLUMN, v);
    }

    b
}

pub fn address_from_row(row: &Row) -> AddressFull {
    AddressFull {
        administrative_area_level_1: row.get(ADMINISTRATIVE_AREA_LEVEL_1_COLUMN),
        administrative_area_level_2: row.get(ADMINISTRATIVE_AREA_LEVEL_2_COLUMN),
        country: row.get(COUNTRY_COLUMN),
        locality: row.get(LOCALITY_COLUMN),
        political: row.get(POLITICAL_COLUMN),
        postal_code: row.get(POSTAL_CODE_COLUMN),
        route: row.get(ROUTE_COLUMN),
        street_number: row.get(STREET_NUMBER_COLUMN),
        address: row.get(ADDRESS_COLUMN),
        place_id: row.get(PLACE_ID_COLUMN),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DbOrder(pub Order);

impl From<Row> for DbOrder {
    fn from(row: Row) -> Self {
        DbOrder(Order {
            id: OrderId(row.get(ID_COLUMN)),
            created_from: CartItemId(row.get(CREATED_FROM_COLUMN)),
            conversion_id: ConversionId(row.get(CONVERSION_ID_COLUMN)),
            slug: OrderSlug(row.get(SLUG_COLUMN)),
            customer: UserId(row.get(CUSTOMER_COLUMN)),
            store: StoreId(row.get(STORE_COLUMN)),
            product: ProductId(row.get(PRODUCT_COLUMN)),
            price: ProductPrice(row.get(PRICE_COLUMN)),
            currency: Currency::from_str(row.get(CURRENCY_COLUMN)).unwrap(),
            quantity: Quantity(row.get(QUANTITY_COLUMN)),
            address: address_from_row(&row),
            receiver_name: row.get(RECEIVER_NAME_COLUMN),
            receiver_phone: row.get(RECEIVER_PHONE_COLUMN),
            receiver_email: row.get(RECEIVER_EMAIL_COLUMN),
            payment_status: row.get(PAYMENT_STATUS_COLUMN),
            delivery_company: row.get(DELIVERY_COMPANY_COLUMN),
            created_at: row.get(CREATED_AT_COLUMN),
            updated_at: row.get(UPDATED_AT_COLUMN),
            track_id: row.get(TRACK_ID_COLUMN),
            state: row.get(STATE_COLUMN),
            pre_order: row.get(PRE_ORDER_COLUMN),
            pre_order_days: row.get(PRE_ORDER_DAYS_COLUMN),
            coupon_id: row.get::<Option<i32>, _>(COUPON_ID_COLUMN).map(CouponId),
            coupon_percent: row.get(COUPON_PERCENT_COLUMN),
            coupon_discount: row.get::<Option<f64>, _>(COUPON_DISCOUNT_COLUMN).map(ProductPrice),
            product_discount: row.get::<Option<f64>, _>(PRODUCT_DISCOUNT_COLUMN).map(ProductPrice),
            total_amount: ProductPrice(row.get(TOTAL_AMOUNT_COLUMN)),
            company_package_id: row.get::<Option<i32>, _>(COMPANY_PACKAGE_ID_COLUMN).map(CompanyPackageId),
            delivery_price: row.get(DELIVERY_PRICE_COLUMN),
            shipping_id: row.get::<Option<i32>, _>(SHIPPING_ID_COLUMN).map(ShippingId),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderInserter {
    pub id: Option<OrderId>,
    pub created_from: Option<CartItemId>,
    pub conversion_id: Option<ConversionId>,
    pub customer: UserId,
    pub store: StoreId,
    pub product: ProductId,
    pub price: ProductPrice,
    pub currency: Currency,
    pub quantity: Quantity,
    pub address: AddressFull,
    pub receiver_name: String,
    pub receiver_phone: String,
    pub receiver_email: String,
    pub delivery_company: Option<String>,
    pub state: OrderState,
    pub track_id: Option<String>,
    pub pre_order: bool,
    pub pre_order_days: i32,
    pub coupon_id: Option<CouponId>,
    pub coupon_percent: Option<i32>,
    pub coupon_discount: Option<ProductPrice>,
    pub product_discount: Option<ProductPrice>,
    pub total_amount: ProductPrice,
    pub company_package_id: Option<CompanyPackageId>,
    pub delivery_price: f64,
    pub shipping_id: Option<ShippingId>,
}

impl Inserter for OrderInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        let mut b = InsertBuilder::new(table)
            .with_arg(CUSTOMER_COLUMN, self.customer.0)
            .with_arg(STORE_COLUMN, self.store.0)
            .with_arg(PRODUCT_COLUMN, self.product.0)
            .with_arg(RECEIVER_NAME_COLUMN, self.receiver_name)
            .with_arg(RECEIVER_PHONE_COLUMN, self.receiver_phone)
            .with_arg(RECEIVER_EMAIL_COLUMN, self.receiver_email)
            .with_arg(PRICE_COLUMN, self.price.0)
            .with_arg(CURRENCY_COLUMN, self.currency.to_string())
            .with_arg(QUANTITY_COLUMN, self.quantity.0)
            .with_arg(STATE_COLUMN, self.state)
            .with_arg(PRE_ORDER_COLUMN, self.pre_order)
            .with_arg(PRE_ORDER_DAYS_COLUMN, self.pre_order_days)
            .with_arg(TOTAL_AMOUNT_COLUMN, self.total_amount.0)
            .with_arg(DELIVERY_PRICE_COLUMN, self.delivery_price);

        b = write_address_into_inserter(self.address, b);

        if let Some(v) = self.id {
            b = b.with_arg(ID_COLUMN, v.0);
        }

        if let Some(v) = self.created_from {
            b = b.with_arg(CREATED_FROM_COLUMN, v.0);
        }

        if let Some(v) = self.conversion_id {
            b = b.with_arg(CONVERSION_ID_COLUMN, v.0);
        }

        if let Some(v) = self.track_id {
            b = b.with_arg(TRACK_ID_COLUMN, v);
        }

        if let Some(v) = self.coupon_id {
            b = b.with_arg(COUPON_ID_COLUMN, v.0);
        }

        if let Some(v) = self.coupon_percent {
            b = b.with_arg(COUPON_PERCENT_COLUMN, v);
        }

        if let Some(v) = self.coupon_discount {
            b = b.with_arg(COUPON_DISCOUNT_COLUMN, v.0);
        }

        if let Some(v) = self.product_discount {
            b = b.with_arg(PRODUCT_DISCOUNT_COLUMN, v.0);
        }

        if let Some(v) = self.company_package_id {
            b = b.with_arg(COMPANY_PACKAGE_ID_COLUMN, v.0);
        }

        if let Some(v) = self.shipping_id {
            b = b.with_arg(SHIPPING_ID_COLUMN, v.0);
        }

        b
    }
}

pub type AddressMask = AddressFull;

#[derive(Clone, Debug, Default)]
pub struct OrderFilter {
    pub do_order: bool,
    pub id: Option<ValueContainer<OrderId>>,
    pub ids: Option<ValueContainer<Vec<OrderId>>>,
    pub created_from: Option<ValueContainer<CartItemId>>,
    pub conversion_id: Option<ValueContainer<ConversionId>>,
    pub slug: Option<ValueContainer<OrderSlug>>,
    pub customer: Option<ValueContainer<UserId>>,
    pub store: Option<ValueContainer<StoreId>>,
    pub product: Option<ValueContainer<ProductId>>,
    pub created_at: Option<ValueContainer<Range<DateTime<Utc>>>>,
    pub updated_at: Option<ValueContainer<Range<DateTime<Utc>>>>,
    pub state: Option<ValueContainer<OrderState>>,
    pub payment_status: Option<ValueContainer<bool>>,
    pub delivery_company: Option<ValueContainer<Option<String>>>,
    pub track_id: Option<ValueContainer<Option<String>>>,
    pub pre_order: Option<ValueContainer<bool>>,
    pub pre_order_days: Option<ValueContainer<i32>>,
}

impl From<OrderIdentifier> for OrderFilter {
    fn from(v: OrderIdentifier) -> Self {
        use self::OrderIdentifier::*;

        match v {
            Id(id) => Self {
                id: Some(id.into()),
                ..Default::default()
            },
            Slug(slug) => Self {
                slug: Some(slug.into()),
                ..Default::default()
            },
        }
    }
}

impl OrderFilter {
    pub fn with_ordering(mut self, flag: bool) -> Self {
        self.do_order = flag;
        self
    }

    #[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
    pub fn from_search_terms(terms: OrderSearchTerms) -> Fallible<Self> {
        let mut mask = OrderFilter::default();

        mask.slug = terms.slug.map(From::from);

        mask.created_at = super::into_range(terms.created_from, terms.created_to);
        mask.updated_at = super::into_range(terms.updated_from, terms.updated_to);

        mask.payment_status = terms.payment_status.map(From::from);
        mask.customer = terms.customer.map(From::from);
        mask.store = terms.store.map(From::from);
        mask.state = terms.state.map(From::from);

        Ok(mask)
    }
}

impl Filter for OrderFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v.value.0);
        }

        if let Some(v) = self.ids {
            let ids: Vec<Uuid> = v.value.into_iter().map(|id| id.0).collect();
            b = b.with_filter::<Uuid, _>(ID_COLUMN, ids);
        }

        if let Some(v) = self.created_from {
            b = b.with_filter(CREATED_FROM_COLUMN, v.value.0);
        }

        if let Some(v) = self.conversion_id {
            b = b.with_filter(CONVERSION_ID_COLUMN, v.value.0);
        }

        if let Some(v) = self.slug {
            b = b.with_filter(SLUG_COLUMN, v.value.0);
        }

        if let Some(v) = self.customer {
            b = b.with_filter(CUSTOMER_COLUMN, v.value.0);
        }

        if let Some(v) = self.store {
            b = b.with_filter(STORE_COLUMN, v.value.0);
        }

        if let Some(v) = self.product {
            b = b.with_filter(PRODUCT_COLUMN, v.value.0);
        }

        if let Some(v) = self.created_at {
            b = b.with_filter::<DateTime<Utc>, _>(CREATED_AT_COLUMN, v.value);
        }

        if let Some(v) = self.updated_at {
            b = b.with_filter::<DateTime<Utc>, _>(UPDATED_AT_COLUMN, v.value);
        }

        if let Some(v) = self.state {
            b = b.with_filter(STATE_COLUMN, v.value);
        }

        if let Some(v) = self.payment_status {
            b = b.with_filter(PAYMENT_STATUS_COLUMN, v.value);
        }

        if let Some(v) = self.delivery_company {
            b = b.with_filter(DELIVERY_COMPANY_COLUMN, v.value);
        }

        if let Some(v) = self.track_id {
            b = b.with_filter(TRACK_ID_COLUMN, v.value);
        }

        if let Some(v) = self.pre_order {
            b = b.with_filter(PRE_ORDER_COLUMN, v.value);
        }

        if let Some(v) = self.pre_order_days {
            b = b.with_filter(PRE_ORDER_DAYS_COLUMN, v.value);
        }

        if self.do_order {
            b = b.with_extra("ORDER BY created_at DESC");
        }

        b
    }
}

pub struct OrderUpdateData {
    pub state: Option<OrderState>,
    pub track_id: Option<String>,
}

pub struct OrderUpdater {
    pub mask: OrderFilter,
    pub data: OrderUpdateData,
}

impl Updater for OrderUpdater {
    fn into_update_builder(self, table: &'static str) -> UpdateBuilder {
        let OrderUpdater { mask, data } = self;

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(table));

        if let Some(state) = data.state {
            b = b.with_value(STATE_COLUMN, state);
        }

        if let Some(track_id) = data.track_id {
            b = b.with_value(TRACK_ID_COLUMN, track_id);
        }

        b
    }
}
