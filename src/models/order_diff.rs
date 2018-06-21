use super::OrderId;
use super::OrderState;

use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "order_id")]
pub struct OrderDiffId(pub Uuid);

pub struct OrderDiff {
    pub id: OrderDiffId,
    parent: OrderId,
    committer: user_id,
    datetime: i64,
    state: OrderState,
}
