use super::{OrderId, OrderState, UserId};

use uuid::Uuid;

const ID_COLUMN: &'static str = "id";
const PARENT_COLUMN: &'static str = "parent";
const COMMITTER_COLUMN: &'static str = "committer";
const TIMESTAMP_COLUMN: &'static str = "datetime";
const CHANGE_COLUMN: &'static str = "change";
const DIFF_COLUMN: &'static str = "diff";

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "order_id")]
pub struct OrderDiffId(pub Uuid);

pub type ChangeType = String;

pub struct OrderDiff {
    pub id: OrderDiffId,
    pub parent: OrderId,
    pub committer: UserId,
    pub timestamp: i64,
    pub change: ChangeType,
}
