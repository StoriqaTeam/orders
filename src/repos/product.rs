use super::UserId;

use futures::prelude::*;
use futures_state_stream::StateStream;
use stq_db::repo::*;
use tokio_postgres::types::ToSql;

use models::*;

static TABLE: &'static str = "cart_items";

pub trait ProductRepo: DbRepo<CartProduct, CartProductInserter, CartProductMask, CartProductUpdate, RepoError> {}

pub type ProductRepoImpl = DbRepoImpl<CartProduct, CartProductInserter, CartProductMask, CartProductUpdate>;

impl ProductRepo for ProductRepoImpl {}

pub fn make_product_repo() -> ProductRepoImpl {
    ProductRepoImpl::new(TABLE)
}
