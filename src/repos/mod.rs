pub mod product;
pub use self::product::*;

use errors::RepoError;

use futures;
use futures::prelude::*;
use futures_state_stream::*;
use tokio_postgres;
use tokio_postgres::rows::Row;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;
use tokio_postgres::types::ToSql;

pub type RepoConnection = Box<Connection + Send>;
pub type RepoFuture<T> = Box<Future<Item = (T, RepoConnection), Error = (RepoError, RepoConnection)> + Send>;

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