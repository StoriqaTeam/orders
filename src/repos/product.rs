use models::*;

use stq_db::repo::*;

static TABLE: &'static str = "cart_items";

pub trait ProductRepo: DbRepo<CartProduct, CartProductInserter, CartProductMask, CartProductUpdate, RepoError> {}

pub type ProductRepoImpl = DbRepoImpl<CartProduct, CartProductInserter, CartProductMask, CartProductUpdate>;

impl ProductRepo for ProductRepoImpl {}

pub fn make_product_repo() -> ProductRepoImpl {
    ProductRepoImpl::new(TABLE)
}
