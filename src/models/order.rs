use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use stq_db::repo::*;
use stq_db::statement::*;
use stq_http::errors::ControllerError;
use tokio_postgres::rows::Row;

const ID_COLUMN: &'static str = "id";
const USER_ID_COLUMN: &'static str = "user_id";
const PRODUCTS_COLUMN: &'static str = "products";
const STATE_ID_COLUMN: &'static str = "state_id";
const STATE_DATA_COLUMN: &'static str = "state_data";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewData;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CancelledData;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NeedPaymentData;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessingData;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfirmedData;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompleteData;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "id", content = "data")]
pub enum OrderState {
    New(NewData),
    Cancelled(CancelledData),
    NeedPayment(NeedPaymentData),
    Processing(ProcessingData),
    Confirmed(ConfirmedData),
    Complete(CompleteData),
}

impl From<OrderState> for (String, Value) {
    fn from(v: OrderState) -> Self {
        match v {
            OrderState::New(data) => ("new".to_string(), serde_json::to_value(data).unwrap()),
            OrderState::Cancelled(data) => ("cancelled".to_string(), serde_json::to_value(data).unwrap()),
            OrderState::NeedPayment(data) => (
                "need_payment".to_string(),
                serde_json::to_value(data).unwrap(),
            ),
            OrderState::Processing(data) => (
                "processing".to_string(),
                serde_json::to_value(data).unwrap(),
            ),
            OrderState::Confirmed(data) => ("confirmed".to_string(), serde_json::to_value(data).unwrap()),
            OrderState::Complete(data) => ("complete".to_string(), serde_json::to_value(data).unwrap()),
        }
    }
}

impl OrderState {
    fn from_tuple<'a>(v: (&'a str, Value)) -> Result<Self, ControllerError> {
        let (state_id, state_data) = v;
        match state_id {
            "new" => Ok(OrderState::New(serde_json::from_value(state_data)?)),
            "cancelled" => Ok(OrderState::Cancelled(serde_json::from_value(state_data)?)),
            "need_payment" => Ok(OrderState::NeedPayment(serde_json::from_value(state_data)?)),
            "processing" => Ok(OrderState::Processing(serde_json::from_value(state_data)?)),
            "confirmed" => Ok(OrderState::Confirmed(serde_json::from_value(state_data)?)),
            "complete" => Ok(OrderState::Complete(serde_json::from_value(state_data)?)),
            &_ => Err(ControllerError::UnprocessableEntity(format_err!(
                "Could not parse state"
            ))),
        }
    }
}

impl<'a> From<(&'a str, Value)> for OrderState {
    fn from(v: (&str, Value)) -> Self {
        OrderState::from_tuple(v).unwrap()
    }
}

pub type OrderId = i32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub user_id: i32,
    pub products: HashMap<i32, i32>,
    pub state: OrderState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewOrder {
    pub user_id: OrderId,
    pub products: HashMap<i32, i32>,
    pub state: OrderState,
}

impl From<(OrderId, NewOrder)> for Order {
    fn from(v: (OrderId, NewOrder)) -> Self {
        let (id, new_order) = v;
        Order {
            id,
            user_id: new_order.user_id,
            products: new_order.products,
            state: new_order.state,
        }
    }
}

impl From<Order> for (OrderId, NewOrder) {
    fn from(v: Order) -> Self {
        (
            v.id,
            NewOrder {
                user_id: v.user_id,
                products: v.products,
                state: v.state,
            },
        )
    }
}

impl From<Row> for Order {
    fn from(row: Row) -> Self {
        let state_id: String = row.get(STATE_ID_COLUMN);
        let state_data: Value = row.get(STATE_DATA_COLUMN);
        Self {
            id: row.get(ID_COLUMN),
            user_id: row.get(USER_ID_COLUMN),
            products: serde_json::from_value(row.get(PRODUCTS_COLUMN)).unwrap(),
            state: OrderState::from((state_id.as_str(), state_data)),
        }
    }
}

impl Inserter for NewOrder {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        let (state_id, state_data) = self.state.into();
        InsertBuilder::new(table)
            .with_arg(USER_ID_COLUMN, self.user_id)
            .with_arg(
                PRODUCTS_COLUMN,
                serde_json::to_value(self.products).unwrap(),
            )
            .with_arg(STATE_ID_COLUMN, state_id)
            .with_arg(STATE_DATA_COLUMN, state_data)
    }
}

#[derive(Clone, Debug, Default)]
pub struct OrderMask {
    pub id: Option<OrderId>,
    pub user_id: Option<i32>,
    pub state_id: Option<String>,
}

impl Filter for OrderMask {
    fn into_filtered_operation_builder(self, op: FilteredOperation, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(op, table);

        if let Some(id) = self.id {
            b = b.with_arg(ID_COLUMN, id);
        }

        if let Some(user_id) = self.user_id {
            b = b.with_arg(USER_ID_COLUMN, user_id);
        }

        if let Some(state_id) = self.state_id {
            b = b.with_arg(STATE_ID_COLUMN, state_id);
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

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(FilteredOperation::Select, table));

        if let Some(state) = data.state {
            let (state_id, state_data) = state.into();
            b = b.with_value(STATE_ID_COLUMN, state_id)
                .with_value(STATE_DATA_COLUMN, state_data);
        }

        b
    }
}
