use futures::prelude::*;
use tokio_postgres;

use types;
use types::*;

pub fn run(db_pool: DbPool) -> Box<Future<Item = (), Error = tokio_postgres::Error>> {
    Box::new(
        db_pool
            .run(|conn| {
                conn.batch_execute(
                    "
                CREATE TABLE IF NOT EXISTS cart_items (
                    user_id  BIGINT NOT NULL,
                    product  BIGINT NOT NULL,
                    quantity BIGINT NOT NULL,

                    UNIQUE (user_id, product)
                )
                ",
                ).map(|conn| ((), conn))
            })
            .map(|_| ()),
    )
}
