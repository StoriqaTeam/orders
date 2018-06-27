use super::*;

use chrono::prelude::*;
use std::str::FromStr;
use stq_db::statement::*;
use tokio_postgres::rows::Row;
use uuid::Uuid;

const ID_COLUMN: &'static str = "id";
const PARENT_COLUMN: &'static str = "parent";
const COMMITTER_COLUMN: &'static str = "committer";
const COMMITTED_AT_COLUMN: &'static str = "committed_at";
const STATE_COLUMN: &'static str = "state";
const COMMENT_COLUMN: &'static str = "comment";

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize)]
pub struct OrderDiffId(pub Uuid);

pub struct OrderDiff {
    pub id: OrderDiffId,
    pub parent: OrderId,
    pub committer: UserId,
    pub committed_at: NaiveDateTime,
    pub state: OrderState,
    pub comment: Option<String>,
}

impl From<Row> for OrderDiff {
    fn from(row: Row) -> Self {
        Self {
            id: OrderDiffId(row.get(ID_COLUMN)),
            parent: OrderId(row.get(PARENT_COLUMN)),
            committer: UserId(row.get(COMMITTER_COLUMN)),
            committed_at: row.get(COMMITTED_AT_COLUMN),
            state: OrderState::from_str(&row.get::<String, _>(STATE_COLUMN)).unwrap(),
            comment: row.get(COMMENT_COLUMN),
        }
    }
}

pub struct OrderDiffInserter {
    pub parent: OrderId,
    pub committer: UserId,
    pub committed_at: NaiveDateTime,
    pub state: OrderState,
    pub comment: Option<String>,
}

impl Inserter for OrderDiffInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(PARENT_COLUMN, self.parent.0)
            .with_arg(COMMITTER_COLUMN, self.committer.0)
            .with_arg(COMMITTED_AT_COLUMN, self.committed_at)
            .with_arg(STATE_COLUMN, self.state.to_string())
            .with_arg(COMMENT_COLUMN, self.comment)
    }
}

pub struct OrderDiffFilter;

impl Filter for OrderDiffFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        FilteredOperationBuilder::new(table)
    }
}
