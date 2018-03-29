use futures::prelude::*;
use futures_state_stream::*;
use std::any::Any;
use std::sync::{Arc, Mutex};
use tokio_postgres;

use errors::*;
use log;
use models::*;
use repos::*;
use types::*;

pub type ServiceFuture<T> = Box<Future<Item = T, Error = RepoError>>;

pub trait CartService {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart>;
    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart>;
    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
}

pub type ProductRepoFactory = Arc<Fn(RepoConnection) -> Box<ProductRepo>>;

pub struct CartServiceImpl {
    db_pool: DbPool,
    repo_factory: ProductRepoFactory,
}

impl CartServiceImpl {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            db_pool,
            repo_factory: Arc::new(|conn| Box::new(ProductRepoImpl::new(conn))),
        }
    }
}

impl CartServiceImpl {
    fn _get_cart(repo_factory: ProductRepoFactory, conn: RepoConnection, user_id: i32) -> RepoConnectionFuture<Cart> {
        Box::new(
            (repo_factory)(conn)
                .get(ProductMask {
                    user_id: Some(user_id),
                    ..Default::default()
                })
                .map(|(products, conn)| {
                    let mut cart = Cart::default();
                    for product in products.into_iter() {
                        cart.products.insert(product.product_id, product.quantity);
                    }
                    (cart, conn)
                }),
        )
    }
}

impl CartService for CartServiceImpl {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Getting cart for user {}.", user_id);
        Box::new(
            self.db_pool
                .run({
                    let repo_factory = self.repo_factory.clone();
                    move |conn| {
                        log::acquired_db_connection(&conn);
                        Self::_get_cart(repo_factory, Box::new(conn), user_id)
                    }
                })
                .map_err(RepoError::from),
        )
    }

    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart> {
        debug!("Setting item {} to quantity {} in cart for user {}.", product_id, quantity, user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare(
                        "
                        INSERT INTO cart_items (user_id, product_id, quantitSy)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (user_id, product_id)
                        DO UPDATE SET quantity = $3
                        ;",
                    ).and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id, &product_id, &quantity]))
                        .map(|(_, conn)| conn)
                        .and_then({
                            let repo_factory = self.repo_factory.clone();
                            move |conn| Self::_get_cart(repo_factory, Box::new(conn), user_id)
                        })
                })
                .map_err(RepoError::from),
        )
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for user {}", product_id, user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1 AND product_id = $2;")
                        .and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id, &product_id]))
                        .map(|(_, conn)| conn)
                        .and_then({
                            let repo_factory = self.repo_factory.clone();
                            move |conn| Self::_get_cart(repo_factory, Box::new(conn), user_id)
                        })
                })
                .map_err(RepoError::from),
        )
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1;")
                        .and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id]))
                        .map(|(_, conn)| conn)
                        .and_then(move |conn| Self::_get_cart(Box::new(conn), user_id))
                })
                .map_err(RepoError::from),
        )
    }
}
