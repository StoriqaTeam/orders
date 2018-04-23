use futures::future;
use futures::prelude::*;
use futures_state_stream::StateStream;
use stq_db::statement::*;

use super::{RepoConnection, RepoConnectionFuture};
use errors::*;
use models::*;

static TABLE: &'static str = "orders";

pub trait OrderRepo {
    fn add(&self, conn: RepoConnection, new_order: NewOrder) -> RepoConnectionFuture<Order>;
    fn get(&self, conn: RepoConnection, mask: OrderMask) -> RepoConnectionFuture<Vec<Order>>;
    fn update(&self, conn: RepoConnection, mask: OrderMask, data: OrderUpdateData) -> RepoConnectionFuture<Order>;
    fn remove(&self, conn: RepoConnection, mask: OrderMask) -> RepoConnectionFuture<()>;
}

#[derive(Clone, Debug, Default)]
pub struct OrderRepoImpl;

impl OrderRepo for OrderRepoImpl {
    fn add(&self, conn: RepoConnection, new_order: NewOrder) -> RepoConnectionFuture<Order> {
        let (statement, args) = new_order.into_insert_builder(TABLE).build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(mut rows, connection)| (Order::from(rows.remove(0)), connection)),
        )
    }

    fn get(&self, conn: RepoConnection, mask: OrderMask) -> RepoConnectionFuture<Vec<Order>> {
        let (statement, args) = mask.into_filtered_operation_builder(FilteredOperation::Select, TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(rows, connection)| {
                    (
                        rows.into_iter().map(Order::from).collect::<Vec<Order>>(),
                        connection,
                    )
                }),
        )
    }

    fn update(&self, conn: RepoConnection, mask: OrderMask, data: OrderUpdateData) -> RepoConnectionFuture<Order> {
        let (statement, args) = OrderUpdate { mask, data }
            .into_update_builder(TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .and_then(|(mut rows, connection)| {
                    if rows.is_empty() {
                        future::err((RepoError::NotFound, connection))
                    } else {
                        future::ok((Order::from(rows.remove(0)), connection))
                    }
                }),
        )
    }

    fn remove(&self, conn: RepoConnection, mask: OrderMask) -> RepoConnectionFuture<()> {
        let (statement, args) = mask.into_filtered_operation_builder(FilteredOperation::Delete, TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(_rows, connection)| ((), connection)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    .and_then(move |_| OrderRepoImpl::default().add(conn, new_order))
                    .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                    .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
            }
        }).wait()
            .unwrap();

        let expected_created_order = Order::from((created_order.id, new_order));

        assert_eq!(created_order, expected_created_order);
    }
}
