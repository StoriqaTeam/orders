use futures::prelude::*;
use futures_state_stream::StateStream;

use models::*;
use repos::{RepoConnection, RepoConnectionFuture};
use util;

#[derive(Clone, Copy, Debug, Default)]
pub struct ProductMask {
    pub user_id: Option<i32>,
    pub product_id: Option<i32>,
}

pub trait ProductRepo {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<Vec<Product>>;
    fn insert(self: Box<Self>, item: NewProduct) -> RepoConnectionFuture<()>;
}

pub struct ProductRepoImpl {
    connection: RepoConnection,
}

impl ProductRepoImpl {
    pub fn new(connection: RepoConnection) -> Self {
        Self { connection }
    }
}

impl ProductRepo for ProductRepoImpl {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<Vec<Product>> {
        let mut query_builder = util::SimpleQueryBuilder::new(util::SimpleQueryOperation::Select, "cart_items");

        if let Some(v) = mask.user_id {
            query_builder = query_builder.with_arg("user_id", v);
        }

        if let Some(v) = mask.product_id {
            query_builder = query_builder.with_arg("product_id", v);
        }

        let (statement, args) = query_builder.build();

        Box::new(
            self.connection
                .prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(rows, conn)| (rows.into_iter().map(Product::from).collect::<Vec<Product>>(), conn)),
        )
    }

    fn insert(self: Box<Self>, item: NewProduct) -> RepoConnectionFuture<()> {
        let (statement, args) = util::SimpleQueryBuilder::new(util::SimpleQueryOperation::Insert, "cart_items")
            .with_arg("user_id", item.user_id)
            .with_arg("product_id", item.product_id)
            .with_arg("quantity", item.quantity)
            .with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = $3")
            .build();

        Box::new(
            self.connection
                .prepare2(&statement)
                .and_then(move |(statement, conn)| conn.query2(&statement, args).collect())
                .map(|(_rows, conn)| ((), conn)),
        )
    }
}
