use bb8;
use bb8_postgres;
use std::sync::Arc;
use tokio_postgres;

pub type DbPool = Arc<bb8::Pool<bb8_postgres::PostgresConnectionManager>>;
pub type DbConnection = tokio_postgres::Connection;
