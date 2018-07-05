use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_db::repo::*;
use stq_db::statement::*;

use super::types::ServiceFuture;
use models::*;
use repos::*;
use types::*;

/// Service that provides operations for interacting with user carts
pub trait CartService {
    /// Get user's cart contents
    fn get_cart(&self, user_id: UserId) -> ServiceFuture<Cart>;
    /// Increase item's quantity by 1
    fn increment_item(&self, user_id: UserId, product_id: ProductId, store_id: StoreId) -> ServiceFuture<Cart>;
    /// Set item to desired quantity in user's cart
    fn set_quantity(&self, user_id: UserId, product_id: ProductId, quantity: Quantity) -> ServiceFuture<Cart>;
    /// Set selection of the item in user's cart
    fn set_selection(&self, user_id: UserId, product_id: ProductId, selected: bool) -> ServiceFuture<Cart>;
    /// Set comment for item in user's cart
    fn set_comment(&self, user_id: UserId, product_id: ProductId, comment: String) -> ServiceFuture<Cart>;
    /// Delete item from user's cart
    fn delete_item(&self, user_id: UserId, product_id: ProductId) -> ServiceFuture<Cart>;
    /// Clear user's cart
    fn clear_cart(&self, user_id: UserId) -> ServiceFuture<Cart>;
    /// Iterate over cart
    fn list(&self, user_id: UserId, from: ProductId, count: i32) -> ServiceFuture<Cart>;
    /// Merge carts
    fn merge(&self, from: UserId, to: UserId) -> ServiceFuture<Cart>;
}

pub type ProductRepoFactory = Rc<Fn() -> Box<ProductRepo>>;

/// Default implementation of user cart service
pub struct CartServiceImpl {
    calling_user: UserId,
    db_pool: DbPool,
    repo_factory: ProductRepoFactory,
}

impl CartServiceImpl {
    /// Create new cart service with provided DB connection pool
    pub fn new(calling_user: UserId, db_pool: DbPool) -> Self {
        Self {
            calling_user,
            db_pool,
            repo_factory: Rc::new(|| Box::new(make_product_repo())),
        }
    }
}

fn get_cart_from_repo(repo_factory: ProductRepoFactory, conn: RepoConnection, user_id: UserId) -> RepoConnectionFuture<Cart> {
    Box::new(
        (repo_factory)()
            .select(
                conn,
                CartProductMask {
                    user_id: Some(user_id.into()),
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
    fn get_cart(&self, user_id: UserId) -> ServiceFuture<Cart> {
        debug!("Getting cart for user {}.", user_id);
        Box::new(self.db_pool.run({
            let repo_factory = self.repo_factory.clone();
            move |conn| get_cart_from_repo(repo_factory, conn, user_id)
        }))
    }

    fn increment_item(&self, user_id: UserId, product_id: ProductId, store_id: StoreId) -> ServiceFuture<Cart> {
        debug!("Adding 1 item {} into cart for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run({
            move |conn| {
                future::ok(conn)
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |conn| {
                            (repo_factory)().insert_exactly_one(
                                conn,
                                CartProductInserter::Incrementer(NewCartProduct::new(user_id, product_id, store_id)),
                            )
                        }
                    })
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |(_, conn)| {
                            (repo_factory)().select(
                                conn,
                                CartProductMask {
                                    user_id: Some(user_id.into()),
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

    fn set_quantity(&self, user_id: UserId, product_id: ProductId, quantity: Quantity) -> ServiceFuture<Cart> {
        debug!("Setting quantity for item {} for user {} to {}", product_id, user_id, quantity);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartProductUpdater {
                        mask: CartProductMask {
                            user_id: Some(user_id.into()),
                            product_id: Some(product_id.into()),
                            ..Default::default()
                        },
                        data: CartProductUpdateData {
                            quantity: Some(quantity),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartProductMask {
                                user_id: Some(user_id.into()),
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
        }))
    }

    fn set_selection(&self, user_id: UserId, product_id: ProductId, selected: bool) -> ServiceFuture<Cart> {
        debug!("Setting selection for item {} for user {} to {}", product_id, user_id, selected);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartProductUpdater {
                        mask: CartProductMask {
                            user_id: Some(user_id.into()),
                            product_id: Some(product_id.into()),
                            ..Default::default()
                        },
                        data: CartProductUpdateData {
                            selected: Some(selected.into()),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartProductMask {
                                user_id: Some(user_id.into()),
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
        }))
    }

    fn set_comment(&self, user_id: UserId, product_id: ProductId, comment: String) -> ServiceFuture<Cart> {
        debug!("Setting comment for item {} for user {} to {}", product_id, user_id, comment);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartProductUpdater {
                        mask: CartProductMask {
                            user_id: Some(user_id.into()),
                            product_id: Some(product_id.into()),
                            ..Default::default()
                        },
                        data: CartProductUpdateData {
                            comment: Some(comment.into()),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartProductMask {
                                user_id: Some(user_id.into()),
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
        }))
    }

    fn delete_item(&self, user_id: UserId, product_id: ProductId) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for user {}", product_id, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .delete(
                    conn,
                    CartProductMask {
                        user_id: Some(user_id.into()),
                        product_id: Some(product_id.into()),
                        ..Default::default()
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartProductMask {
                                user_id: Some(user_id.into()),
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
        }))
    }

    fn clear_cart(&self, user_id: UserId) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().delete(
                        conn,
                        CartProductMask {
                            user_id: Some(user_id.into()),
                            ..Default::default()
                        },
                    )
                })
                .map(|_| Default::default()),
        )
    }

    fn list(&self, user_id: UserId, from: ProductId, count: i32) -> ServiceFuture<Cart> {
        debug!("Getting {} cart items starting from {} for user {}", count, from, user_id);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .select_full(
                    conn,
                    CartProductMask {
                        user_id: Some(user_id.into()),
                        product_id: Some(Range::From(RangeLimit {
                            value: from,
                            inclusive: true,
                        })),
                        ..Default::default()
                    },
                    Some(count),
                    None,
                )
                .map(|(products, conn)| (products.into_iter().map(<(ProductId, CartItemInfo)>::from).collect::<Cart>(), conn))
        }))
    }

    fn merge(&self, from: UserId, to: UserId) -> ServiceFuture<Cart> {
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
                                user_id: Some(from.into()),
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
                                    .insert(conn, CartProductInserter::CollisionNoOp(new_cart_product))
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
