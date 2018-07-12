use std::str::FromStr;

use chrono::prelude::*;
use failure;
use geo::Point as GeoPoint;
use tokio_postgres::rows::Row;

use stq_db::statement::*;
use stq_static_resources::OrderState;
use stq_types::*;

use super::common::*;

const ID_COLUMN: &'static str = "id";
const SLUG_COLUMN: &'static str = "slug";
const CUSTOMER_COLUMN: &'static str = "customer";
const STORE_COLUMN: &'static str = "store";
const PRODUCT_COLUMN: &'static str = "product";
const PRICE_COLUMN: &'static str = "price";
const QUANTITY_COLUMN: &'static str = "quantity";
const RECEIVER_NAME_COLUMN: &'static str = "receiver_name";

const LOCATION_COLUMN: &'static str = "location";
const ADMINISTRATIVE_AREA_LEVEL_1_COLUMN: &'static str = "administrative_area_level_1";
const ADMINISTRATIVE_AREA_LEVEL_2_COLUMN: &'static str = "administrative_area_level_2";
const COUNTRY_COLUMN: &'static str = "country";
const LOCALITY_COLUMN: &'static str = "locality";
const POLITICAL_COLUMN: &'static str = "political";
const POSTAL_CODE_COLUMN: &'static str = "postal_code";
const ROUTE_COLUMN: &'static str = "route";
const STREET_NUMBER_COLUMN: &'static str = "street_number";
const ADDRESS_COLUMN: &'static str = "address";
const PLACE_ID_COLUMN: &'static str = "place_id";

const TRACK_ID_COLUMN: &'static str = "track_id";
const CREATED_AT_COLUMN: &'static str = "created_at";
const UPDATED_AT_COLUMN: &'static str = "updated_at";
const STATE_COLUMN: &'static str = "state";
const PAYMENT_STATUS_COLUMN: &'static str = "payment_status";
const DELIVERY_COMPANY_COLUMN: &'static str = "delivery_company";


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AddressFull {
    location: Option<GeoPoint<f64>>,
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

impl AddressFull {
    pub fn write_into_inserter(self, mut b: InsertBuilder) -> InsertBuilder {
        if let Some(v) = self.location {
            b = b.with_arg(LOCATION_COLUMN, v);
        }
        if let Some(v) = self.administrative_area_level_1 {
            b = b.with_arg(ADMINISTRATIVE_AREA_LEVEL_1_COLUMN, v);
        }
        if let Some(v) = self.administrative_area_level_2 {
            b = b.with_arg(ADMINISTRATIVE_AREA_LEVEL_2_COLUMN, v);
        }
        if let Some(v) = self.country {
            b = b.with_arg(COUNTRY_COLUMN, v);
        }
        if let Some(v) = self.locality {
            b = b.with_arg(LOCALITY_COLUMN, v);
        }
        if let Some(v) = self.political {
            b = b.with_arg(POLITICAL_COLUMN, v);
        }
        if let Some(v) = self.postal_code {
            b = b.with_arg(POSTAL_CODE_COLUMN, v);
        }
        if let Some(v) = self.route {
            b = b.with_arg(ROUTE_COLUMN, v);
        }
        if let Some(v) = self.street_number {
            b = b.with_arg(STREET_NUMBER_COLUMN, v);
        }
        if let Some(v) = self.address {
            b = b.with_arg(ADDRESS_COLUMN, v);
        }
        if let Some(v) = self.place_id {
            b = b.with_arg(PLACE_ID_COLUMN, v);
        }

        b
    }

    pub fn from_row(row: &Row) -> Self {
        Self {
            location: row.get(LOCATION_COLUMN),
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
}

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize)]
pub struct OrderSlug(pub i32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub slug: OrderSlug,
    pub customer: UserId,
    pub store: StoreId,
    pub product: ProductId,
    pub price: ProductPrice,
    pub quantity: Quantity,
    pub address: AddressFull,
    pub receiver_name: String,
    pub state: OrderState,
    pub payment_status: bool,
    pub delivery_company: Option<String>,
    pub track_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Row> for Order {
    fn from(row: Row) -> Self {
        let id = OrderId(row.get(ID_COLUMN));
        let state_id: String = row.get(STATE_COLUMN);
        Self {
            id,
            slug: OrderSlug(row.get(SLUG_COLUMN)),
            customer: UserId(row.get(CUSTOMER_COLUMN)),
            store: StoreId(row.get(STORE_COLUMN)),
            product: ProductId(row.get(PRODUCT_COLUMN)),
            price: ProductPrice(row.get(PRICE_COLUMN)),
            quantity: Quantity(row.get(QUANTITY_COLUMN)),
            address: AddressFull::from_row(&row),
            receiver_name: row.get(RECEIVER_NAME_COLUMN),
            payment_status: row.get(PAYMENT_STATUS_COLUMN),
            delivery_company: row.get(DELIVERY_COMPANY_COLUMN),
            created_at: row.get(CREATED_AT_COLUMN),
            updated_at: row.get(UPDATED_AT_COLUMN),
            track_id: row.get(TRACK_ID_COLUMN),
            state: OrderState::from_str(&state_id).expect(&format!("Invalid order state ({}) in DB record {}", state_id, id)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderInserter {
    pub id: OrderId,
    pub customer: UserId,
    pub store: StoreId,
    pub product: ProductId,
    pub price: ProductPrice,
    pub quantity: Quantity,
    pub address: AddressFull,
    pub receiver_name: String,
    pub delivery_company: Option<String>,
    pub state: OrderState,
    pub track_id: Option<String>,
}

impl Inserter for OrderInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        let mut b = InsertBuilder::new(table)
            .with_arg(ID_COLUMN, self.id.0)
            .with_arg(CUSTOMER_COLUMN, self.customer.0)
            .with_arg(STORE_COLUMN, self.store.0)
            .with_arg(PRODUCT_COLUMN, self.product.0)
            .with_arg(RECEIVER_NAME_COLUMN, self.receiver_name)
            .with_arg(PRICE_COLUMN, self.price.0)
            .with_arg(QUANTITY_COLUMN, self.quantity.0)
            .with_arg(STATE_COLUMN, self.state.to_string());

        b = self.address.write_into_inserter(b);

        if let Some(v) = self.track_id {
            b = b.with_arg(TRACK_ID_COLUMN, v);
        }

        b
    }
}

pub type AddressMask = AddressFull;

/// Anything that can uniquely identify an Order
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OrderIdentifier {
    Id(OrderId),
    Slug(OrderSlug),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderSearchTerms {
    pub slug: Option<OrderSlug>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub payment_status: Option<bool>,
    pub customer: Option<UserId>,
    pub store: Option<StoreId>,
    pub state: Option<OrderState>,
}

#[derive(Clone, Debug, Default)]
pub struct OrderFilter {
    pub id: Option<ValueContainer<OrderId>>,
    pub slug: Option<ValueContainer<OrderSlug>>,
    pub customer: Option<ValueContainer<UserId>>,
    pub store: Option<ValueContainer<StoreId>>,
    pub product: Option<ValueContainer<ProductId>>,
    pub address: AddressMask,
    pub receiver_name: Option<ValueContainer<String>>,
    pub created_at: Option<ValueContainer<Range<DateTime<Utc>>>>,
    pub updated_at: Option<ValueContainer<Range<DateTime<Utc>>>>,
    pub state: Option<ValueContainer<OrderState>>,
    pub payment_status: Option<ValueContainer<bool>>,
    pub delivery_company: Option<ValueContainer<Option<String>>>,
    pub track_id: Option<ValueContainer<Option<String>>>,
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

impl OrderSearchTerms {
    pub fn make_filter(self) -> Result<OrderFilter, failure::Error> {
        let mut mask = OrderFilter::default();

        mask.slug = self.slug.map(From::from);

        mask.created_at = if let (Some(from), Some(to)) = (self.created_from, self.created_to).clone() {
            Some(
                Range::Between((
                    {
                        RangeLimit {
                            value: from,
                            inclusive: true,
                        }
                    },
                    {
                        RangeLimit {
                            value: to,
                            inclusive: true,
                        }
                    },
                )).into(),
            )
        } else if let Some(value) = self.created_from {
            Some(Range::From({ RangeLimit { value, inclusive: true } }).into())
        } else if let Some(value) = self.created_to {
            Some(Range::To({ RangeLimit { value, inclusive: true } }).into())
        } else {
            None
        };

        mask.payment_status = self.payment_status.map(From::from);
        mask.customer = self.customer.map(From::from);
        mask.store = self.store.map(From::from);
        mask.state = self.state.map(From::from);

        Ok(mask)
    }
}

impl Filter for OrderFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v.value.0);
        }

        if let Some(v) = self.slug {
            b = b.with_filter(SLUG_COLUMN, v.value.0);
        }

        if let Some(v) = self.customer {
            b = b.with_filter(CUSTOMER_COLUMN, v.value.0);
        }

        if let Some(v) = self.state {
            b = b.with_filter(STATE_COLUMN, v.value.to_string());
        }

        if let Some(v) = self.payment_status {
            b = b.with_filter(PAYMENT_STATUS_COLUMN, v.value);
        }

        if let Some(v) = self.track_id {
            b = b.with_filter(TRACK_ID_COLUMN, v.value);
        }
        
        if let Some(v) = self.created_at {
            b = b.with_filter::<DateTime<Utc>, _>(CREATED_AT_COLUMN, v.value);
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
            b = b.with_value(STATE_COLUMN, state.to_string());
        }

        if let Some(track_id) = data.track_id {
            b = b.with_value(TRACK_ID_COLUMN, track_id);
        }

        b
    }
}
