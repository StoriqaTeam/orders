use failure;
use futures::prelude::*;
use std::sync::Arc;
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

pub type ProductRepoFactory = Arc<Fn(RepoConnection) -> Box<ProductRepo> + Send + Sync>;

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

fn get_cart_from_repo(repo_factory: ProductRepoFactory, conn: RepoConnection, user_id: i32) -> RepoConnectionFuture<Cart> {
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

impl CartService for CartServiceImpl {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Getting cart for user {}.", user_id);
        Box::new(
            self.db_pool
                .run({
                    let repo_factory = self.repo_factory.clone();
                    move |conn| {
                        log::acquired_db_connection(&conn);
                        get_cart_from_repo(repo_factory, Box::new(conn), user_id)
                            .map(|(v, conn)| (v, conn.unwrap_tokio_postgres().unwrap()))
                            .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres().unwrap()))
                    }
                })
                .map_err(RepoError::from),
        )
    }

    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart> {
        debug!("Setting item {} to quantity {} in cart for user {}.", product_id, quantity, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    repo_factory(Box::new(conn))
                        .insert(NewProduct {
                            user_id,
                            product_id,
                            quantity,
                        })
                        .and_then({ move |(_, conn)| get_cart_from_repo(repo_factory, conn, user_id) })
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres().unwrap()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres().unwrap(),
                            )
                        })
                })
                .map_err(RepoError::from),
        )
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    (repo_factory)(Box::new(conn))
                        .remove(ProductMask {
                            user_id: Some(user_id),
                            product_id: Some(product_id),
                            ..Default::default()
                        })
                        .and_then(move |(_, conn)| get_cart_from_repo(repo_factory, conn, user_id))
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres().unwrap()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres().unwrap(),
                            )
                        })
                })
                .map_err(RepoError::from),
        )
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    (repo_factory)(Box::new(conn))
                        .remove(ProductMask {
                            user_id: Some(user_id),
                            ..Default::default()
                        })
                        .and_then(move |(_, conn)| get_cart_from_repo(repo_factory, conn, user_id))
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres().unwrap()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres().unwrap(),
                            )
                        })
                })
                .map_err(RepoError::from),
        )
    }
}
