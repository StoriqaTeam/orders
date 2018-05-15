use futures::prelude::*;
use futures_state_stream::StateStream;
use stq_db::repo::*;
use tokio_postgres::types::ToSql;

use models::*;

static TABLE: &'static str = "cart_items";

pub trait ProductRepo
    : DbRepo<CartProduct, UpsertCartProduct, CartProductMask, CartProductUpdate, RepoError>
    + DbRepoInsert<CartProduct, CartProductNewInserter, RepoError> {
    fn list(&self, conn: RepoConnection, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<CartProduct>>;
}

pub type ProductRepoImpl = DbRepoImpl;

impl ProductRepo for ProductRepoImpl {
    fn list(&self, conn: RepoConnection, user_id: i32, from: i32, count: i64) -> RepoConnectionFuture<Vec<CartProduct>> {
        let statement = format!(
            "SELECT * FROM {} WHERE {} = $1 AND {} >= $2 LIMIT $3;",
            TABLE, "user_id", "product_id"
        );
        let args: Vec<Box<ToSql + Send>> = vec![Box::new(user_id), Box::new(from), Box::new(count)];

        Box::new(
            conn.prepare2(&statement)
                .and_then({ move |(statement, conn)| conn.query2(&statement, args).collect() })
                .map(|(rows, conn)| (rows.into_iter().map(From::from).collect::<_>(), conn)),
        )
    }
}

pub fn make_product_repo() -> ProductRepoImpl {
    ProductRepoImpl::new(TABLE)
}
