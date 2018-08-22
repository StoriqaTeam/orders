use stq_db::repo::*;
use stq_db::statement::*;

use models::*;

const TABLE: &str = "order_diffs";

pub struct DummyOrderDiffUpdater {}
impl Updater for DummyOrderDiffUpdater {
    fn into_update_builder(self, _table: &'static str) -> UpdateBuilder {
        unreachable!()
    }
}

pub trait OrderDiffRepo: DbRepo<DbOrderDiff, OrderDiffInserter, OrderDiffFilter, DummyOrderDiffUpdater, RepoError> {}

pub type OrderDiffRepoImpl = DbRepoImpl<DbOrderDiff, OrderDiffInserter, OrderDiffFilter, DummyOrderDiffUpdater>;
impl OrderDiffRepo for OrderDiffRepoImpl {}

type Repo = OrderDiffRepoImpl;

pub fn make_su_repo() -> Repo {
    Repo::new(TABLE)
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
pub fn make_repo(_login_data: UserLogin) -> Repo {
    make_su_repo()
}
