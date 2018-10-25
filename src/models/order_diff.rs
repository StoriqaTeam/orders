use chrono::prelude::*;
use tokio_postgres::rows::Row;

use stq_api::orders::*;
use stq_db::statement::*;
use stq_static_resources::OrderState;
use stq_types::*;

use super::*;

const ID_COLUMN: &str = "id";
const PARENT_COLUMN: &str = "parent";
const COMMITTER_COLUMN: &str = "committer";
const COMMITTED_AT_COLUMN: &str = "committed_at";
const STATE_COLUMN: &str = "state";
const COMMENT_COLUMN: &str = "comment";

#[derive(Clone, Debug, PartialEq)]
pub struct DbOrderDiff(pub OrderDiff);

impl From<Row> for DbOrderDiff {
    fn from(row: Row) -> Self {
        DbOrderDiff(OrderDiff {
            id: OrderDiffId(row.get(ID_COLUMN)),
            parent: OrderId(row.get(PARENT_COLUMN)),
            committer: UserId(row.get(COMMITTER_COLUMN)),
            committed_at: row.get(COMMITTED_AT_COLUMN),
            state: row.get(STATE_COLUMN),
            comment: row.get(COMMENT_COLUMN),
        })
    }
}

pub struct OrderDiffInserter {
    pub parent: OrderId,
    pub committer: UserId,
    pub committed_at: DateTime<Utc>,
    pub state: OrderState,
    pub comment: Option<String>,
}

impl Inserter for OrderDiffInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(PARENT_COLUMN, self.parent.0)
            .with_arg(COMMITTER_COLUMN, self.committer.0)
            .with_arg(COMMITTED_AT_COLUMN, self.committed_at)
            .with_arg(STATE_COLUMN, self.state)
            .with_arg(COMMENT_COLUMN, self.comment)
    }
}

#[derive(Clone, Debug, Default)]
pub struct OrderDiffFilter {
    pub do_order: bool,
    pub id: Option<ValueContainer<OrderDiffId>>,
    pub parent: Option<ValueContainer<OrderId>>,
    pub committer: Option<ValueContainer<UserId>>,
    pub committed_at: Option<ValueContainer<DateTime<Utc>>>,
    pub committed_at_range: Option<ValueContainer<Range<DateTime<Utc>>>>,
    pub state: Option<ValueContainer<OrderState>>,
    pub comment: Option<ValueContainer<Option<String>>>,
}

impl OrderDiffFilter {
    pub fn with_ordering(mut self, flag: bool) -> Self {
        self.do_order = flag;
        self
    }
}

impl Filter for OrderDiffFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v.value.0);
        }

        if let Some(v) = self.parent {
            b = b.with_filter(PARENT_COLUMN, v.value.0);
        }

        if let Some(v) = self.committer {
            b = b.with_filter(COMMITTER_COLUMN, v.value.0);
        }

        if let Some(v) = self.committed_at {
            b = b.with_filter(COMMITTED_AT_COLUMN, v.value);
        }

        if let Some(v) = self.committed_at_range {
            b = b.with_filter::<DateTime<Utc>, _>(COMMITTED_AT_COLUMN, v.value);
        }

        if let Some(v) = self.state {
            b = b.with_filter(STATE_COLUMN, v.value);
        }

        if let Some(v) = self.comment {
            b = b.with_filter(COMMENT_COLUMN, v.value);
        }

        if self.do_order {
            b = b.with_extra("ORDER BY committed_at DESC");
        }

        b
    }
}

impl From<OrderId> for OrderDiffFilter {
    fn from(v: OrderId) -> Self {
        Self {
            parent: Some(v.into()),
            ..Default::default()
        }
    }
}
