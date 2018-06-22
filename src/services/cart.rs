use futures::future;
use futures::prelude::*;
use std::sync::Arc;
use stq_db::repo::*;
use stq_db::statement::*;

use super::types::ServiceFuture;
use models::*;
use repos::*;
use types::*;

/// Service that provides operations for interacting with user carts
pub trait CartService {
    /// Get user's cart contents
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    /// Increase item's quantity by 1
    fn increment_item(&self, user_id: i32, product_id: i32, store_id: i32) -> ServiceFuture<Cart>;
    /// Set item to desired quantity in user's cart
    fn set_quantity(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Option<CartItem>>;
    /// Set selection of the item in user's cart
    fn set_selection(&self, user_id: i32, product_id: i32, selected: bool) -> ServiceFuture<Option<CartItem>>;
    /// Delete item from user's cart
    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Option<CartItem>>;
    /// Clear user's cart
    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    /// Iterate over cart
    fn list(&self, user_id: i32, from: i32, count: i64) -> ServiceFuture<Cart>;
    /// Merge carts
    fn merge(&self, from: i32, to: i32) -> ServiceFuture<Cart>;
}

pub type ProductRepoFactory = Arc<Fn() -> Box<ProductRepo> + Send + Sync>;

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
            repo_factory: Arc::new(|| Box::new(make_product_repo())),
        }
    }
}

fn get_cart_from_repo(repo_factory: ProductRepoFactory, conn: RepoConnection, user_id: i32) -> RepoConnectionFuture<Cart> {
    Box::new(
        (repo_factory)()
            .select(
                conn,
                CartProductMask {
                    user_id: Some(user_id),
                    ..Default::default()
                },
            )
            .map(|(products, conn)| {
                let mut cart = Cart::default();
                for product in products.into_iter() {
                    let (k, v) = <(ProductId, CartItemInfo)>::from(product);
                    cart.insert(k, v);
                }
                (cart, conn)
            }),
    )
}

impl CartService for CartServiceImpl {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Getting cart for user {}.", user_id);
        Box::new(self.db_pool.run({
            let repo_factory = self.repo_factory.clone();
            move |conn| get_cart_from_repo(repo_factory, conn, user_id)
        }))
    }

    fn increment_item(&self, user_id: i32, product_id: i32, store_id: i32) -> ServiceFuture<Cart> {
        debug!("Adding 1 item {} into cart for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run({
            move |conn| {
                future::ok(conn)
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |conn| {
                            (repo_factory)().select(
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
                                product.decompose().1
                            } else {
                                NewCartProduct {
                                    user_id,
                                    product_id,
                                    quantity: 1,
                                    selected: true,
                                    store_id,
                                }
                            };
                            (repo_factory)().insert_exactly_one(conn, CartProductInserter::Upserter(new_product))
                        }
                    })
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |(_, conn)| {
                            (repo_factory)().select(
                                conn,
                                CartProductMask {
                                    user_id: Some(user_id),
                                    ..Default::default()
                                },
                            )
                        }
                    })
                    .map({
                        move |(rows, conn)| {
                            (
                                rows.into_iter()
                                    .map(CartProduct::from)
                                    .map(<(ProductId, CartItemInfo)>::from)
                                    .collect::<Cart>(),
                                conn,
                            )
                        }
                    })
            }
        }))
    }

    fn set_quantity(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Option<CartItem>> {
        debug!("Setting quantity for item {} for user {} to {}", product_id, user_id, quantity);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().update(
                        conn,
                        CartProductUpdate {
                            mask: CartProductMask {
                                user_id: Some(user_id),
                                product_id: Some(product_id),
                                ..Default::default()
                            },
                            data: CartProductUpdateData {
                                quantity: Some(quantity),
                                ..Default::default()
                            },
                        },
                    )
                })
                .map(|mut v| v.pop()),
        )
    }

    fn set_selection(&self, user_id: i32, product_id: i32, selected: bool) -> ServiceFuture<Option<CartItem>> {
        debug!("Setting selection for item {} for user {} to {}", product_id, user_id, selected);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().update(
                        conn,
                        CartProductUpdate {
                            mask: CartProductMask {
                                user_id: Some(user_id),
                                product_id: Some(product_id),
                                ..Default::default()
                            },
                            data: CartProductUpdateData {
                                selected: Some(selected),
                                ..Default::default()
                            },
                        },
                    )
                })
                .map(|mut v| v.pop()),
        )
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Option<CartItem>> {
        debug!("Deleting item {} for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().delete(
                        conn,
                        CartProductMask {
                            user_id: Some(user_id),
                            product_id: Some(product_id),
                            ..Default::default()
                        },
                    )
                })
                .map(|mut rows| rows.pop().map(CartItem::from)),
        )
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .delete(
                    Box::new(conn),
                    CartProductMask {
                        user_id: Some(user_id),
                        ..Default::default()
                    },
                )
                .and_then(move |(_, conn)| get_cart_from_repo(repo_factory, conn, user_id))
        }))
    }

    fn list(&self, user_id: i32, from: i32, count: i64) -> ServiceFuture<Cart> {
        debug!("Getting {} cart items starting from {} for user {}", count, from, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .select_full(
                    conn,
                    CartProductMask {
                        user_id: user_id,
                        product_id: Some(Range::From(RangeLimit {
                            value: from,
                            inclusive: true,
                        })),

                        ..Default::default()
                    },
                )
                .map(|(products, conn)| {
                    let mut cart = Cart::default();
                    for product in products.into_iter() {
                        let (id, info) = product.into();
                        cart.insert(id, info);
                    }
                    (cart, conn)
                })
        }))
    }

    fn merge(&self, from: i32, to: i32) -> ServiceFuture<Cart> {
        debug!("Merging cart contents from user {} to user {}", from, to);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            future::ok(conn)
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |conn| {
                        (repo_factory)().delete(
                            conn,
                            CartProductMask {
                                user_id: Some(from),
                                ..Default::default()
                            },
                        )
                    }
                })
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(from_products, conn)| {
                        let mut b: RepoConnectionFuture<()> = Box::new(future::ok(((), conn)));
                        for product in from_products {
                            let repo_factory = repo_factory.clone();
                            b = Box::new(b.and_then(move |(_, conn)| {
                                let mut new_cart_product = product.decompose().1;
                                new_cart_product.user_id = to;
                                (repo_factory)()
                                    .insert(conn, CartProductInserter::Upserter(new_cart_product))
                                    .map(|(_, conn)| ((), conn))
                            }));
                        }
                        b
                    }
                })
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| get_cart_from_repo(repo_factory.clone(), conn, to)
                })
        }))
    }
}
