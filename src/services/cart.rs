use failure;
use futures::future;
use futures::prelude::*;
use std;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_postgres;

use super::types::ServiceFuture;
use errors::*;
use log;
use models::*;
use repos::*;
use types::*;

/// Service that provides operations for interacting with user carts
pub trait CartService {
    /// Get user's cart contents
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    /// Set item to desired quantity in user's cart
    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart>;
    /// Delete item from user's cart
    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart>;
    /// Clear user's cart
    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
}

type ProductRepoFactory = Arc<Fn(RepoConnection) -> Box<ProductRepo> + Send + Sync>;

/// Default implementation of user cart service
pub struct CartServiceImpl {
    db_pool: DbPool,
    repo_factory: ProductRepoFactory,
}

impl CartServiceImpl {
    /// Create new cart service with provided DB connection pool
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
                            .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                            .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
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
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres(),
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
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres(),
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
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| {
                            (
                                tokio_postgres::error::conversion(Box::new(failure::Error::from(e).compat())),
                                conn.unwrap_tokio_postgres(),
                            )
                        })
                })
                .map_err(RepoError::from),
        )
    }
}

pub type CartServiceMemoryStorage = Arc<Mutex<HashMap<i32, Cart>>>;

/// In-memory implementation of cart service
pub struct CartServiceMemory {
    pub inner: CartServiceMemoryStorage,
}

impl CartService for CartServiceMemory {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        Box::new(future::ok(cart.clone()))
    }

    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        cart.products.insert(product_id, quantity);

        Box::new(future::ok(cart.clone()))
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        cart.products.remove(&product_id);

        Box::new(future::ok(cart.clone()))
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        std::mem::swap(cart, &mut Cart::default());

        Box::new(future::ok(cart.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config;
    use bb8;
    use bb8_postgres::PostgresConnectionManager;
    use tokio_core::reactor::{Core, Remote};
    use tokio_postgres::TlsMode;

    fn prepare_db(remote: Remote) -> Box<Future<Item = bb8::Pool<PostgresConnectionManager>, Error = tokio_postgres::Error>> {
        let config = config::Config::new().unwrap();
        let manager = PostgresConnectionManager::new(config.db.dsn.clone(), || TlsMode::None).unwrap();

        bb8::Pool::builder().min_idle(Some(10)).build(manager, remote)
    }

    #[test]
    fn test_products_repo() {
        let mut core = Core::new().unwrap();
        let remote = core.remote();
        let pool = Arc::new(core.run(prepare_db(remote)).unwrap());

        let repo = CartServiceImpl::new(pool);

        let user_id = 1234;

        let set_a = (5555, 9000);
        let set_b = (5555, 9010);
        let set_c = (4444, 8000);

        // Clear user cart before starting
        assert_eq!(Cart::default(), core.run(repo.clear_cart(user_id)).unwrap());

        // Add the first product
        assert_eq!(
            Cart {
                products: vec![set_a].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.set_item(user_id, set_a.0, set_a.1)).unwrap()
        );

        // Check DB contents
        assert_eq!(
            Cart {
                products: vec![set_a].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.get_cart(user_id)).unwrap()
        );

        // Amend the first product
        assert_eq!(
            Cart {
                products: vec![set_a, set_b].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.set_item(user_id, set_b.0, set_b.1)).unwrap()
        );

        // Add the last product
        assert_eq!(
            Cart {
                products: vec![set_a, set_b, set_c].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.set_item(user_id, set_c.0, set_c.1)).unwrap()
        );

        // Check DB contents
        assert_eq!(
            Cart {
                products: vec![set_a, set_b, set_c].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.get_cart(user_id)).unwrap()
        );

        // Delete the last item
        assert_eq!(
            Cart {
                products: vec![set_a, set_b].into_iter().collect::<HashMap<i32, i32>>(),
            },
            core.run(repo.delete_item(user_id, set_c.0)).unwrap()
        );

        // Clear user cart
        assert_eq!(Cart::default(), core.run(repo.clear_cart(user_id)).unwrap());
    }
}