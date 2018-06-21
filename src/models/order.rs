use super::common::*;
use errors::Error;

use failure;
use failure::Fail;
use geo::Point as GeoPoint;
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use stq_db::repo::*;
use stq_db::statement::*;
use tokio_postgres::rows::Row;
use uuid::Uuid;

const ID_COLUMN: &'static str = "id";
const SLUG_COLUMN: &'static str = "slug";
const CUSTOMER_COLUMN: &'static str = "customer";
const PRODUCT_COLUMN: &'static str = "product";
const PRICE_COLUMN: &'static str = "price";
const QUANTITY_COLUMN: &'static str = "quantity";
const SUBTOTAL_COLUMN: &'static str = "subtotal";
const RECEIVER_NAME: &'static str = "receiver_name";
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
const CREATION_DATE_COLUMN: &'static str = "creation_date";
const STATE_ID_COLUMN: &'static str = "state_id";
const STATE_DATA_COLUMN: &'static str = "state_data";
const PAYMENT_STATUS_COLUMN: &'static str = "payment_status";
const DELIVERY_COMPANY_COLUMN: &'static str = "delivery_company";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewData {
    pub comment: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PaidData {
    pub comment: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InProcessingData {
    pub comment: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CancelledData {
    pub comment: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SentData {
    pub comment: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "id", content = "data")]
pub enum OrderState {
    /// State set on order creation.
    #[serde(rename = "new")]
    New(NewData), // on creation
    /// Set after payment by request of billing
    #[serde(rename = "paid")]
    Paid(PaidData),
    /// Order is being processed by store management
    #[serde(rename = "in_processing")]
    InProcessing(InProcessingData),
    /// Can be cancelled by any party before order being sent.
    #[serde(rename = "cancelled")]
    Cancelled(CancelledData),
    /// Wares are on their way to the customer. Tracking ID must be set.
    #[serde(rename = "sent")]
    Sent(SentData),
}

impl OrderState {
    pub fn into_db(self) -> (String, Value) {
        use self::OrderState::*;

        match self {
            New(data) => ("new".to_string(), serde_json::to_value(data).unwrap()),
            Paid(data) => ("paid".to_string(), serde_json::to_value(data).unwrap()),
            InProcessing(data) => ("in_processing".to_string(), serde_json::to_value(data).unwrap()),
            Cancelled(data) => ("cancelled".to_string(), serde_json::to_value(data).unwrap()),
            Sent(data) => ("sent".to_string(), serde_json::to_value(data).unwrap()),
        }
    }

    pub fn from_db<'a>(state_id: &'a str, state_data: Value) -> Result<Self, failure::Error> {
        use self::OrderState::*;

        match state_id {
            "new" => Ok(New(serde_json::from_value(state_data)?)),
            "paid" => Ok(Paid(serde_json::from_value(state_data)?)),
            "in_processing" => Ok(InProcessing(serde_json::from_value(state_data)?)),
            "cancelled" => Ok(Cancelled(serde_json::from_value(state_data)?)),
            "sent" => Ok(Sent(serde_json::from_value(state_data)?)),
            other => Err(Error::ParseError.context(format!("Unknown state_id {}", other)).into()),
        }
    }
}

#[derive(Clone, Debug)]
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
    track_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "order_id")]
pub struct OrderId(pub Uuid);

impl OrderId {
    pub fn new() -> Self {
        OrderId(Uuid::new_v4())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub slug: String,
    pub customer: UserId,
    pub store_id: StoreId,
    pub product_id: ProductId,
    pub address: AddressFull,
    pub receiver_name: String,
    pub state: OrderState,
}

impl From<Row> for Order {
    fn from(row: Row) -> Self {
        let state_id: String = row.get(STATE_ID_COLUMN);
        let state_data: Value = row.get(STATE_DATA_COLUMN);
        Self {
            id: row.get(ID_COLUMN),
            customer: row.get(CUSTOMER_COLUMN),
            product: row.get(PRODUCT_COLUMN),
            state: OrderState::from_db(state_id.as_str(), state_data).unwrap(),
        }
    }
}

impl Inserter for Order {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        let (state_id, state_data) = self.state.into_db();
        InsertBuilder::new(table)
            .with_arg(ID_COLUMN, self.id)
            .with_arg(SLUG_COLUMN, self.slug)
            .with_arg(CUSTOMER_COLUMN, self.customer)
            .with_arg(STORE_ID_COLUMN, self.store_id)
            .with_arg(PRODUCT_COLUMN, self.product_id)
            .with_arg(RECEIVER_NAME_COLUMN, self.receiver_name)
            .with_arg(STATE_ID_COLUMN, state_id)
            .with_arg(STATE_DATA_COLUMN, state_data)
    }
}

#[derive(Clone, Debug, Default)]
pub struct OrderMask {
    pub id: Option<OrderId>,
    pub user_id: Option<UserId>,
    pub state_id: Option<String>,
}

impl Filter for OrderMask {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(id) = self.id {
            b = b.with_filter(ID_COLUMN, id);
        }

        if let Some(user_id) = self.user_id {
            b = b.with_filter(USER_ID_COLUMN, user_id);
        }

        if let Some(state_id) = self.state_id {
            b = b.with_filter(STATE_ID_COLUMN, state_id);
        }

        b
    }
}

pub struct OrderUpdateData {
    pub state: Option<OrderState>,
}

pub struct OrderUpdate {
    pub mask: OrderMask,
    pub data: OrderUpdateData,
}

impl Updater for OrderUpdate {
    fn into_update_builder(self, table: &'static str) -> UpdateBuilder {
        let OrderUpdate { mask, data } = self;

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(table));

        if let Some(state) = data.state {
            let (state_id, state_data) = state.into();
            b = b.with_value(STATE_ID_COLUMN, state_id).with_value(STATE_DATA_COLUMN, state_data);
        }

        b
    }
}
