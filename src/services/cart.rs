use futures::future;
use futures::prelude::*;
use std;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    /// Increase item's quantity by 1
    fn increment_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem>;
    /// Set item to desired quantity in user's cart
    fn set_quantity(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<CartItem>;
    /// Set selection of the item in user's cart
    fn set_selection(&self, user_id: i32, product_id: i32, selected: bool) -> ServiceFuture<Option<CartItem>>;
    /// Delete item from user's cart
    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem>;
    /// Clear user's cart
    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    /// Iterate over cart
    fn list(&self, user_id: i32, from: i32, count: i64) -> ServiceFuture<Cart>;
}

type ProductRepoFactory = Arc<Fn() -> Box<ProductRepo> + Send + Sync>;

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
            repo_factory: Arc::new(|| Box::new(ProductRepoImpl)),
        }
    }
}

fn get_cart_from_repo(repo_factory: ProductRepoFactory, conn: RepoConnection, user_id: i32) -> RepoConnectionFuture<Cart> {
    Box::new(
        (repo_factory)()
            .get(
                conn,
                CartProductMask {
                    user_id: Some(user_id),
                    ..Default::default()
                },
            )
            .map(|(products, conn)| {
                let mut cart = Cart::default();
                for product in products.into_iter() {
                    cart.insert(
                        product.product_id,
                        CartItemInfo {
                            quantity: product.quantity,
                            selected: product.selected,
                        },
                    );
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

    fn increment_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem> {
        debug!(
            "Adding 1 item {} into cart for user {}",
            product_id, user_id
        );

        let repo_factory = self.repo_factory.clone();
        let output = Arc::new(Mutex::new(None));
        Box::new(
            self.db_pool
                .run({
                    let output = output.clone();
                    move |conn| {
                        log::acquired_db_connection(&conn);
                        let conn = conn.transaction();
                        conn.map_err(|(e, trans)| (RepoError::from(e), Box::new(trans) as RepoConnection))
                            .map(|conn| Box::new(conn) as RepoConnection)
                            .and_then({
                                let repo_factory = repo_factory.clone();
                                move |conn| {
                                    (repo_factory)().get(
                                        conn,
                                        CartProductMask {
                                            user_id: Some(user_id),
                                            product_id: Some(product_id),
                                            ..Default::default()
                                        },
                                    )
                                }
                            })
                            .and_then({
                                let repo_factory = repo_factory.clone();
                                move |(products, conn)| {
                                    let new_product = if let Some(mut product) = products.first().cloned() {
                                        product.quantity += 1;
                                        <(CartProductId, NewCartProduct)>::from(product).1
                                    } else {
                                        NewCartProduct {
                                            user_id,
                                            product_id,
                                            quantity: 1,
                                            selected: true,
                                        }
                                    };
                                    (repo_factory)().insert(conn, new_product)
                                }
                            })
                            .map(|(product, conn)| (CartItem::from(product), conn))
                            .inspect(move |&(ref cart_item, ref conn)| {
                                *output.lock().unwrap() = Some(cart_item.clone());
                            })
                            .and_then(|(_, conn)| conn.commit2())
                            .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                            .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                    }
                })
                .map({
                    let output = output.clone();
                    move |_| {
                        let g = output.lock().unwrap();
                        g.clone().unwrap()
                    }
                })
                .map_err(RepoError::from),
        )
    }

    fn set_quantity(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<CartItem> {
        debug!(
            "Setting item {} to quantity {} in cart for user {}.",
            product_id, quantity, user_id
        );

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    repo_factory()
                        .insert(
                            Box::new(conn),
                            NewCartProduct {
                                user_id,
                                product_id,
                                quantity,
                                selected: true,
                            },
                        )
                        .map(|(product, conn)| (CartItem::from(product), conn))
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                })
                .map_err(RepoError::from),
        )
    }

    fn set_selection(&self, user_id: i32, product_id: i32, selected: bool) -> ServiceFuture<Option<CartItem>> {
        debug!(
            "Setting selection for item {} for user {} to {}",
            product_id, user_id, selected
        );

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            log::acquired_db_connection(&conn);
            (repo_factory)()
                .update(
                    Box::new(conn),
                    CartProductMask {
                        user_id: Some(user_id),
                        product_id: Some(product_id),
                        ..Default::default()
                    },
                    CartProductUpdateData {
                        selected: Some(selected),
                    },
                )
                .map(|(mut v, conn)| {
                    (
                        if v.is_empty() {
                            None
                        } else {
                            Some(v.remove(0).into())
                        },
                        conn,
                    )
                })
                .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
        }))
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem> {
        debug!("Deleting item {} for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    (repo_factory)()
                        .remove(
                            Box::new(conn),
                            CartProductMask {
                                user_id: Some(user_id),
                                product_id: Some(product_id),
                                ..Default::default()
                            },
                        )
                        .map(move |(_, conn)| {
                            (
                                CartItem {
                                    product_id,
                                    quantity: 0,
                                    selected: true,
                                },
                                conn,
                            )
                        })
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
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
                    (repo_factory)()
                        .remove(
                            Box::new(conn),
                            CartProductMask {
                                user_id: Some(user_id),
                                ..Default::default()
                            },
                        )
                        .and_then(move |(_, conn)| get_cart_from_repo(repo_factory, conn, user_id))
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                })
                .map_err(RepoError::from),
        )
    }

    fn list(&self, user_id: i32, from: i32, count: i64) -> ServiceFuture<Cart> {
        debug!(
            "Getting {} cart items starting from {} for user {}",
            count, from, user_id
        );

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    (repo_factory)()
                        .list(Box::new(conn), user_id, from, count)
                        .map(|(products, conn)| {
                            let mut cart = Cart::default();
                            for product in products.into_iter() {
                                let (id, info) = product.into();
                                cart.insert(id, info);
                            }
                            (cart, conn)
                        })
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
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

    fn increment_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem> {
        unimplemented!()
    }

    fn set_quantity(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<CartItem> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        let info = cart.entry(product_id).or_insert(CartItemInfo {
            quantity: 0,
            selected: true,
        });
        info.quantity = quantity;

        Box::new(future::ok(CartItem::from((product_id, info.clone()))))
    }

    fn set_selection(&self, user_id: i32, product_id: i32, selected: bool) -> ServiceFuture<Option<CartItem>> {
        unimplemented!()
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<CartItem> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        cart.remove(&product_id);

        Box::new(future::ok(CartItem {
            product_id,
            quantity: 0,
            selected: true,
        }))
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        std::mem::swap(cart, &mut Cart::default());

        Box::new(future::ok(cart.clone()))
    }

    fn list(&self, user_id: i32, from: i32, count: i64) -> ServiceFuture<Cart> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prepare_db;
    use tokio_core::reactor::Core;

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
            CartItem {
                product_id: set_a.0,
                quantity: set_a.1,
                selected: true,
            },
            core.run(repo.set_quantity(user_id, set_a.0, set_a.1))
                .unwrap()
        );

        // Check DB contents
        assert_eq!(
            hashmap! {
                set_a.0 => CartItemInfo {
                    quantity: set_a.1,
                    selected: true,
                },
            },
            core.run(repo.get_cart(user_id)).unwrap()
        );

        // Amend the first product
        assert_eq!(
            CartItem {
                product_id: set_b.0,
                quantity: set_b.1,
                selected: true,
            },
            core.run(repo.set_quantity(user_id, set_b.0, set_b.1))
                .unwrap()
        );

        // Add the last product
        assert_eq!(
            CartItem {
                product_id: set_c.0,
                quantity: set_c.1,
                selected: true,
            },
            core.run(repo.set_quantity(user_id, set_c.0, set_c.1))
                .unwrap()
        );

        // Check DB contents
        assert_eq!(
            hashmap! {
                5555 => CartItemInfo {
                    quantity: 9010,
                    selected: true,
                },
                4444 => CartItemInfo {
                    quantity: 8000,
                    selected: true,
                },
            },
            core.run(repo.get_cart(user_id)).unwrap()
        );

        // Delete the last item
        assert_eq!(
            CartItem {
                product_id: set_c.0,
                quantity: 0,
                selected: true,
            },
            core.run(repo.delete_item(user_id, set_c.0)).unwrap()
        );

        // Clear user cart
        assert_eq!(Cart::default(), core.run(repo.clear_cart(user_id)).unwrap());
    }
}
