use futures::prelude::*;
use futures_state_stream::StateStream;

use models::*;
use repos::{RepoConnection, RepoConnectionFuture};
use util;

#[derive(Clone, Debug)]
pub struct ProductMask {
    user_id: Option<i32>,
    product_id: Option<i32>,
}

pub trait ProductRepo {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<Vec<Product>>;
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
        let ProductMask { user_id, product_id } = mask;

        let mut query_builder = util::SimpleQueryBuilder::new(util::SimpleQueryOperation::Select, "cart_items");

        if let Some(v) = user_id {
            query_builder = query_builder.with_arg("user_id", v);
        }

        if let Some(v) = product_id {
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
}
