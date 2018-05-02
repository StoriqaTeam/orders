use futures::prelude::*;
use futures_state_stream::StateStream;
use stq_db::statement::*;
use tokio_postgres::types::ToSql;

use models::*;
use repos::{RepoConnection, RepoConnectionFuture};

static TABLE: &'static str = "cart_items";

pub trait ProductRepo {
    fn get(&self, conn: RepoConnection, mask: CartProductMask) -> RepoConnectionFuture<Vec<CartProduct>>;
    fn insert(&self, conn: RepoConnection, item: NewCartProduct) -> RepoConnectionFuture<CartProduct>;
    fn update(&self, conn: RepoConnection, mask: CartProductMask, data: CartProductUpdateData) -> RepoConnectionFuture<Vec<CartProduct>>;
    fn remove(&self, conn: RepoConnection, mask: CartProductMask) -> RepoConnectionFuture<()>;
    fn list(&self, conn: RepoConnection, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<CartProduct>>;
}

#[derive(Clone, Debug, Default)]
pub struct ProductRepoImpl;

impl ProductRepo for ProductRepoImpl {
    fn get(&self, conn: RepoConnection, mask: CartProductMask) -> RepoConnectionFuture<Vec<CartProduct>> {
        let (statement, args) = mask.into_filtered_operation_builder(FilteredOperation::Select, TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(rows, conn)| {
                    (
                        rows.into_iter()
                            .map(CartProduct::from)
                            .collect::<Vec<CartProduct>>(),
                        conn,
                    )
                }),
        )
    }

    fn insert(&self, conn: RepoConnection, item: NewCartProduct) -> RepoConnectionFuture<CartProduct> {
        let (statement, args) = item.into_insert_builder(TABLE)
            .with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = $3")
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(mut rows, conn)| (CartProduct::from(rows.remove(0)), conn)),
        )
    }

    fn update(&self, conn: RepoConnection, mask: CartProductMask, data: CartProductUpdateData) -> RepoConnectionFuture<Vec<CartProduct>> {
        let (statement, args) = CartProductUpdate { mask, data }
            .into_update_builder(TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(rows, conn)| {
                    (
                        rows.into_iter()
                            .map(CartProduct::from)
                            .collect::<Vec<CartProduct>>(),
                        conn,
                    )
                }),
        )
    }

    fn remove(&self, conn: RepoConnection, mask: CartProductMask) -> RepoConnectionFuture<()> {
        let (statement, args) = mask.into_filtered_operation_builder(FilteredOperation::Delete, TABLE)
            .build();

        Box::new(
            conn.prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(_rows, conn)| ((), conn)),
        )
    }

    fn list(&self, conn: RepoConnection, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<CartProduct>> {
        let statement = format!(
            "SELECT * FROM {} WHERE {} = $1 AND {} >= $2 LIMIT $3;",
            TABLE, "user_id", "product_id"
        );
        let args: Vec<Box<ToSql + Send>> = vec![Box::new(user_id), Box::new(from), Box::new(count)];

        Box::new(
            conn.prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(rows, conn)| {
                    (
                        rows.into_iter()
                            .map(CartProduct::from)
                            .collect::<Vec<CartProduct>>(),
                        conn,
                    )
                }),
        )
    }
}
