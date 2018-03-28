use futures::prelude::*;
use futures_state_stream::*;
use std::sync::{Arc, Mutex};
use tokio_postgres::Connection;
use tokio_postgres::rows::Row;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;

use models::*;
use util;

#[derive(Clone, Debug, Fail)]
pub enum RepoError {
    #[fail(display = "Connection failure: {}", reason)]
    Connection { reason: String },
}

impl From<tokio_postgres::Error> for RepoError {
    fn from(v: tokio_postgres::Error) -> Self {
        RepoError::Connection {
            reason: format!("{:?}", v),
        }
    }
}

pub type RepoFuture<T> = Box<Future<Item = (T, Box<RepoConnection>), Error = (RepoError, Box<RepoConnection>)>>;

#[derive(Clone, Debug)]
pub struct ProductMask {
    user_id: Option<i32>,
    product_id: Option<i32>,
}

pub trait RepoConnection {
    fn prepare(self, query: &str) -> RepoFuture<(Statement, Box<RepoConnection>)>;
    fn query(self, &Statement) -> Box<StateStream<Item = Row, State = Box<RepoConnection>, Error = RepoError> + Send>;
    fn commit(self) -> RepoFuture<Box<RepoConnection>>;
}

impl RepoConnection for Transaction {
    fn prepare(self, query: &str) -> RepoFuture<(Statement, Box<RepoConnection>)> {
        self.prepare(query)
            .map(|(v, conn)| (v, Box::new(conn)))
            .map_err(|(e, conn)| (RepoError::from(e), Box::new(conn)))
    }
}

impl RepoConnection for Connection {
    fn prepare(self, query: &str) -> RepoFuture<(Statement, Box<RepoConnection>)> {
        self.prepare(query)
            .map(|(v, conn)| (v, Box::new(conn)))
            .map_err(|(e, conn)| (RepoError::from(e), Box::new(conn)))
    }
}

pub trait ProductRepo {
    fn get(self, mask: ProductMask) -> RepoFuture<(Box<ProductRepo>, Vec<Product>)>;
}

pub struct ProductRepoImpl {
    connection: Box<RepoConnection>,
}

impl ProductRepo for RepoConnection {
    fn get(self, mask: ProductMask) -> RepoFuture<Box<RepoConnection>> {
        let out = Arc::new(Mutex::new(Cart::default()));

        let ProductMask { user_id, product_id } = mask;

        let mut query_builder = util::SimpleQueryBuilder::new("SELECT * from cart_items");

        if let Some(v) = user_id {
            query_builder = query_builder.with_arg("user_id", v);
        }

        if let Some(v) = product_id {
            query_builder = query_builder.with_arg("product_id", v);
        }

        let (statement, args) = query_builder.build();

        self.transaction
            .prepare(&statement)
            .and_then({ move |(statement, conn)| conn.query(&statement, &args.map(|v| &*v)) })
            .map(Box::from)
    }
}
