pub mod product;
pub use self::product::*;

use errors::RepoError;

use futures;
use futures::prelude::*;
use futures_state_stream::*;
use std;
use tokio_postgres;
use tokio_postgres::rows::Row;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;
use tokio_postgres::types::ToSql;

pub type BoxedConnection<E> = Box<Connection<E> + Send>;
pub type ConnectionFuture<T, E> = Box<Future<Item = (T, BoxedConnection<E>), Error = (E, BoxedConnection<E>)> + Send>;

pub trait Connection<E>
where
    E: std::convert::From<tokio_postgres::Error>,
{
    fn prepare2(self: Box<Self>, query: &str) -> ConnectionFuture<Statement, E>;
    fn query2(
        self: Box<Self>,
        statement: &Statement,
        params: Vec<Box<ToSql + Send>>,
    ) -> Box<StateStream<Item = Row, State = BoxedConnection<E>, Error = E> + Send>;
    fn commit2(self: Box<Self>) -> ConnectionFuture<(), E>;
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

impl Connection<RepoError> for tokio_postgres::Connection {
    fn prepare2(self: Box<Self>, query: &str) -> ConnectionFuture<Statement, RepoError> {
        Box::new(
            self.prepare(query)
                .map(|(v, conn)| (v, Box::new(conn) as BoxedConnection<RepoError>))
                .map_err(|(e, conn)| (RepoError::from(e), Box::new(conn) as BoxedConnection<RepoError>)),
        )
    }

    fn query2(
        self: Box<Self>,
        statement: &Statement,
        params: Vec<Box<ToSql + Send>>,
    ) -> Box<StateStream<Item = Row, State = BoxedConnection<RepoError>, Error = RepoError> + Send> {
        Box::new(
            self.query(statement, &params.iter().map(|v| &**v as &ToSql).collect::<Vec<&ToSql>>())
                .map_err(RepoError::from)
                .map_state(|conn| Box::new(conn) as BoxedConnection<RepoError>),
        )
    }

    fn commit2(self: Box<Self>) -> ConnectionFuture<(), RepoError> {
        Box::new(futures::future::ok(((), self as BoxedConnection<RepoError>)))
    }
}

pub type RepoConnection = BoxedConnection<RepoError>;
pub type RepoConnectionFuture<T> = ConnectionFuture<T, RepoError>;