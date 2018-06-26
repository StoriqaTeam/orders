use stq_db::repo::*;
use stq_db::statement::*;

use models::*;

const TABLE: &'static str = "order_diffs";

pub struct DummyOrderDiffUpdater {}
impl Updater for DummyOrderDiffUpdater {
    fn into_update_builder(self, _table: &'static str) -> UpdateBuilder {
        unreachable!()
    }
}

pub trait OrderDiffRepo: DbRepo<OrderDiff, OrderDiffInserter, OrderDiffFilter, DummyOrderDiffUpdater, RepoError> {}

pub type OrderDiffRepoImpl = DbRepoImpl<OrderDiff, OrderDiffInserter, OrderDiffFilter, DummyOrderDiffUpdater>;
impl OrderDiffRepo for OrderDiffRepoImpl {}

pub fn make_order_diffs_repo() -> OrderDiffRepoImpl {
    DbRepoImpl::new(TABLE)
}
