use futures;
use futures::prelude::*;
use futures_state_stream::*;
use tokio_postgres;
use tokio_postgres::rows::Row;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;
use tokio_postgres::types::ToSql;

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

pub type RepoConnection = Box<Connection + Send>;
pub type RepoFuture<T> = Box<Future<Item = (T, RepoConnection), Error = (RepoError, RepoConnection)> + Send>;

#[derive(Clone, Debug)]
pub struct ProductMask {
    user_id: Option<i32>,
    product_id: Option<i32>,
}

pub trait Connection {
    fn prepare2(self: Box<Self>, query: &str) -> RepoFuture<Statement>;
    fn query2(
        self: Box<Self>,
        statement: &Statement,
        params: Vec<Box<ToSql + Send>>,
    ) -> Box<StateStream<Item = Row, State = RepoConnection, Error = RepoError> + Send>;
    fn commit2(self: Box<Self>) -> RepoFuture<()>;
}

/*
impl RepoConnection for Transaction {
    fn prepare2(self, query: &str) -> RepoFuture<(Statement, Box<RepoConnection>)> {
        Box::new(
            self.prepare(query)
                .map(|(v, conn)| (v, Box::new(conn)))
                .map_err(|(e, conn)| (RepoError::from(e), Box::new(conn))),
        )
    }
}
*/

impl Connection for tokio_postgres::Connection {
    fn prepare2(self: Box<Self>, query: &str) -> RepoFuture<Statement> {
        Box::new(
            self.prepare(query)
                .map(|(v, conn)| (v, Box::new(conn) as RepoConnection))
                .map_err(|(e, conn)| (RepoError::from(e), Box::new(conn) as RepoConnection)),
        )
    }

    fn query2(
        self: Box<Self>,
        statement: &Statement,
        params: Vec<Box<ToSql + Send>>,
    ) -> Box<StateStream<Item = Row, State = RepoConnection, Error = RepoError> + Send> {
        Box::new(
            self.query(statement, &params.iter().map(|v| &**v as &ToSql).collect::<Vec<&ToSql>>())
                .map_err(RepoError::from)
                .map_state(|conn| Box::new(conn) as RepoConnection),
        )
    }

    fn commit2(self: Box<Self>) -> RepoFuture<()> {
        Box::new(futures::future::ok(((), self as RepoConnection)))
    }
}

pub trait ProductRepo {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoFuture<Vec<Product>>;
}

pub struct ProductRepoImpl {
    connection: RepoConnection,
}

impl ProductRepo for ProductRepoImpl {
    fn get(self: Box<Self>, mask: ProductMask) -> RepoFuture<Vec<Product>> {
        let ProductMask { user_id, product_id } = mask;

        let mut query_builder = util::SimpleQueryBuilder::new(util::SimpleQueryOperation::Select, "cart_items");

        if let Some(v) = user_id {
            query_builder = query_builder.with_arg("user_id", v);
        }

        if let Some(v) = product_id {
            query_builder = query_builder.with_arg("product_id", v);
        }

        let (statement, args) = query_builder.build();

        Box::new(self.connection
            .prepare2(&statement)
            .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
            .map(|(rows, conn)| (rows.into_iter().map(Product::from).collect::<Vec<Product>>(), conn)))
    }
}
