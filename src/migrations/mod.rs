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
                    user_id    INTEGER NOT NULL,
                    product_id BIGINT  NOT NULL,
                    quantity   BIGINT  NOT NULL,

                    CONSTRAINT item UNIQUE (user_id, product_id)
                )
                ",
                ).map(|conn| ((), conn))
            })
            .map(|_| ()),
    )
}
