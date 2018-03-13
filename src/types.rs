use bb8;
use bb8_postgres;
use tokio_postgres;

pub type DbPool = bb8::Pool<bb8_postgres::PostgresConnectionManager>;
pub type DbConnection = tokio_postgres::Connection;