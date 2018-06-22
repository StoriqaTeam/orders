use stq_db::repo::*;

use models::*;

const TABLE: &'static str = "orders";

pub trait OrderRepo: DbRepo<Order, OrderInserter, OrderFilter, OrderUpdate, RepoError> {}

pub type OrderRepoImpl = DbRepoImpl<Order, OrderInserter, OrderFilter, OrderUpdate>;
impl OrderRepo for OrderRepoImpl {}

pub fn make_order_repo() -> OrderRepoImpl {
    DbRepoImpl::new(TABLE)
}
