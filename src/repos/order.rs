use stq_db::repo::*;

use models::*;

static TABLE: &'static str = "orders";

pub trait OrderRepo: DbRepo<Order, NewOrder, OrderMask, OrderUpdate, RepoError> {}

pub type OrderRepoImpl = DbRepoImpl<Order, NewOrder, OrderMask, OrderUpdate>;
impl OrderRepo for OrderRepoImpl {}

pub fn make_order_repo() -> OrderRepoImpl {
    DbRepoImpl::new(TABLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::*;
    use prepare_db;
    use std::sync::Arc;
    use tokio_core::reactor::Core;

    #[test]
    fn test_order_repo() {
        let new_order = NewOrder {
            user_id: 12345,
            products: hashmap! {
                1234 => 9000,
            },
            state: OrderState::New(NewData),
        };

        let mut core = Core::new().unwrap();
        let remote = core.remote();
        let pool = Arc::new(core.run(prepare_db(remote)).unwrap());

        let created_order = pool.run({
            let new_order = new_order.clone();
            move |conn| {
                let conn = Box::new(conn);

                future::ok(())
                    .and_then(move |_| make_order_repo().create(conn, new_order))
                    .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                    .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
            }
        }).wait()
            .unwrap();

        let expected_created_order = Order::from((created_order.id, new_order));

        assert_eq!(created_order, expected_created_order);
    }
}
