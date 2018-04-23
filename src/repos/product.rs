use futures::prelude::*;
use futures_state_stream::StateStream;
use stq_db::statement::*;
use tokio_postgres::types::ToSql;

use models::*;
use repos::{RepoConnection, RepoConnectionFuture};

static TABLE: &'static str = "cart_items";

#[derive(Clone, Copy, Debug, Default)]
pub struct ProductMask {
    pub user_id: Option<i32>,
    pub product_id: Option<i32>,
}

pub trait ProductRepo {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<Vec<Product>>;
    fn insert(self: Box<Self>, item: NewProduct) -> RepoConnectionFuture<()>;
    fn remove(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<()>;
    fn list(self: Box<Self>, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<Product>>;
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
        let mut query_builder = FilteredOperationBuilder::new(FilteredOperation::Select, TABLE);

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
        let (statement, args) = InsertBuilder::new(TABLE)
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

    fn remove(self: Box<Self>, mask: ProductMask) -> RepoConnectionFuture<()> {
        let mut query_builder = FilteredOperationBuilder::new(FilteredOperation::Delete, TABLE);

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
                .map(|(_rows, conn)| ((), conn)),
        )
    }

    fn list(self: Box<Self>, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<Product>> {
        let statement = format!(
            "SELECT * FROM {} WHERE {} = $1 AND {} >= $2 LIMIT $3;",
            TABLE, "user_id", "product_id"
        );
        let args: Vec<Box<ToSql + Send>> = vec![Box::new(user_id), Box::new(from), Box::new(count)];

        Box::new(
            self.connection
                .prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(rows, conn)| (rows.into_iter().map(Product::from).collect::<Vec<Product>>(), conn)),
        )
    }
}
