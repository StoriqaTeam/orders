use super::types::ServiceFuture;
use models::*;
use repos;
use repos::*;
use types::*;

use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_db::repo::*;
use stq_db::statement::*;
use stq_types::*;

/// Service that provides operations for interacting with user carts
pub trait CartService {
    /// Get user's cart contents
    fn get_cart(&self, customer: CartCustomer) -> ServiceFuture<Cart>;
    /// Increase item's quantity by 1
    fn increment_item(&self, customer: CartCustomer, product_id: ProductId, store_id: StoreId) -> ServiceFuture<Cart>;
    /// Set item to desired quantity in user's cart
    fn set_quantity(&self, customer: CartCustomer, product_id: ProductId, quantity: Quantity) -> ServiceFuture<Cart>;
    /// Set selection of the item in user's cart
    fn set_selection(&self, customer: CartCustomer, product_id: ProductId, selected: bool) -> ServiceFuture<Cart>;
    /// Set comment for item in user's cart
    fn set_comment(&self, customer: CartCustomer, product_id: ProductId, comment: String) -> ServiceFuture<Cart>;
    /// Delete item from user's cart
    fn delete_item(&self, customer: CartCustomer, product_id: ProductId) -> ServiceFuture<Cart>;
    /// Clear user's cart
    fn clear_cart(&self, customer: CartCustomer) -> ServiceFuture<Cart>;
    /// Iterate over cart
    fn list(&self, customer: CartCustomer, from: ProductId, count: i32) -> ServiceFuture<Cart>;
    /// Merge carts
    fn merge(&self, from: CartCustomer, to: CartCustomer) -> ServiceFuture<Cart>;
}

pub type ProductRepoFactory = Rc<Fn() -> Box<CartItemRepo>>;

/// Default implementation of user cart service
pub struct CartServiceImpl {
    db_pool: DbPool,
    repo_factory: ProductRepoFactory,
}

impl CartServiceImpl {
    /// Create new cart service with provided DB connection pool
    pub fn new(db_pool: DbPool, login_data: UserLogin) -> Self {
        Self {
            db_pool,
            repo_factory: Rc::new({
                let login_data = login_data.clone();
                move || Box::new(repos::cart_item::make_repo(login_data.clone()))
            }),
        }
    }
}

fn get_cart_from_repo(repo_factory: &ProductRepoFactory, conn: RepoConnection, customer: CartCustomer) -> RepoConnectionFuture<Cart> {
    Box::new((repo_factory)().select(
        conn,
        CartItemFilter {
            customer: Some(customer.into()),
            ..Default::default()
        },
    ))
}

impl CartService for CartServiceImpl {
    fn get_cart(&self, customer: CartCustomer) -> ServiceFuture<Cart> {
        debug!("Getting cart for customer {}.", customer);
        Box::new(self.db_pool.run({
            let repo_factory = self.repo_factory.clone();
            move |conn| {
                (repo_factory)().select(
                    conn,
                    CartItemFilter {
                        customer: Some(customer.into()),
                        ..Default::default()
                    },
                )
            }
        }))
    }

    fn increment_item(&self, customer: CartCustomer, product_id: ProductId, store_id: StoreId) -> ServiceFuture<Cart> {
        debug!("Adding 1 item {} into cart for customer {}", product_id, customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run({
            move |conn| {
                future::ok(conn)
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |conn| {
                            (repo_factory)().insert_exactly_one(
                                conn,
                                CartItemInserter {
                                    strategy: CartItemMergeStrategy::Incrementer,
                                    data: CartItem {
                                        id: CartItemId::new(),
                                        customer,
                                        product_id,
                                        store_id,

                                        quantity: Quantity(1),
                                        selected: true,
                                        comment: String::new(),
                                    },
                                },
                            )
                        }
                    })
                    .and_then({
                        let repo_factory = repo_factory.clone();
                        move |(_, conn)| {
                            (repo_factory)().select(
                                conn,
                                CartItemFilter {
                                    customer: Some(customer),
                                    ..Default::default()
                                },
                            )
                        }
                    })
            }
        }))
    }

    fn set_quantity(&self, customer: CartCustomer, product_id: ProductId, quantity: Quantity) -> ServiceFuture<Cart> {
        debug!("Setting quantity for item {} for customer {} to {}", product_id, customer, quantity);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartItemUpdater {
                        filter: CartItemFilter {
                            customer: Some(customer.into()),
                            meta_filter: CartItemMetaFilter {
                                product_id: Some(product_id.into()),
                                ..Default::default()
                            },
                        },
                        data: CartItemUpdateData {
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
                            CartItemFilter {
                                customer: Some(customer.into()),
                                ..Default::default()
                            },
                        )
                    }
                })
        }))
    }

    fn set_selection(&self, customer: CartCustomer, product_id: ProductId, selected: bool) -> ServiceFuture<Cart> {
        debug!(
            "Setting selection for item {} for customer {} to {}",
            product_id, customer, selected
        );

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartItemUpdater {
                        filter: CartItemFilter {
                            customer: Some(customer.into()),
                            meta_filter: CartItemMetaFilter {
                                product_id: Some(product_id.into()),
                                ..Default::default()
                            },
                        },
                        data: CartItemUpdateData {
                            selected: Some(selected),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartItemFilter {
                                customer: Some(customer.into()),
                                ..Default::default()
                            },
                        )
                    }
                })
        }))
    }

    fn set_comment(&self, customer: CartCustomer, product_id: ProductId, comment: String) -> ServiceFuture<Cart> {
        debug!("Setting comment for item {} for customer {} to {}", product_id, customer, comment);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .update(
                    conn,
                    CartItemUpdater {
                        filter: CartItemFilter {
                            customer: Some(customer.into()),
                            meta_filter: CartItemMetaFilter {
                                product_id: Some(product_id.into()),
                                ..Default::default()
                            },
                        },
                        data: CartItemUpdateData {
                            comment: Some(comment),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartItemFilter {
                                customer: Some(customer.into()),
                                ..Default::default()
                            },
                        )
                    }
                })
        }))
    }

    fn delete_item(&self, customer: CartCustomer, product_id: ProductId) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for customer {}", product_id, customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)()
                .delete(
                    conn,
                    CartItemFilter {
                        customer: Some(customer.into()),
                        meta_filter: CartItemMetaFilter {
                            product_id: Some(product_id.into()),
                            ..Default::default()
                        },
                    },
                )
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| {
                        (repo_factory)().select(
                            conn,
                            CartItemFilter {
                                customer: Some(customer.into()),
                                ..Default::default()
                            },
                        )
                    }
                })
        }))
    }

    fn clear_cart(&self, customer: CartCustomer) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().delete(
                        conn,
                        CartItemFilter {
                            customer: Some(customer.into()),
                            ..Default::default()
                        },
                    )
                })
                .map(|_| Default::default()),
        )
    }

    fn list(&self, customer: CartCustomer, from: ProductId, count: i32) -> ServiceFuture<Cart> {
        debug!("Getting {} cart items starting from {} for customer {}", count, from, customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (repo_factory)().select_full(
                conn,
                CartItemFilter {
                    customer: Some(customer.into()),
                    meta_filter: CartItemMetaFilter {
                        product_id: Some(Range::From(RangeLimit {
                            value: from,
                            inclusive: true,
                        })),
                        ..Default::default()
                    },
                },
                Some(count),
                None,
            )
        }))
    }

    fn merge(&self, from: CartCustomer, to: CartCustomer) -> ServiceFuture<Cart> {
        debug!("Merging cart contents from {} to {}", from, to);

        let repo_factory = self.repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            future::ok(conn)
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |conn| {
                        (repo_factory)().delete(
                            conn,
                            CartItemFilter {
                                customer: Some(from),
                                ..Default::default()
                            },
                        )
                    }
                })
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(from_items, conn)| {
                        let mut b: RepoConnectionFuture<()> = Box::new(future::ok(((), conn)));
                        for cart_item in from_items {
                            let repo_factory = repo_factory.clone();
                            b = Box::new(b.and_then(move |(_, conn)| {
                                (repo_factory)()
                                    .insert(
                                        conn,
                                        CartItemInserter {
                                            strategy: CartItemMergeStrategy::CollisionNoOp,
                                            data: CartItem {
                                                id: CartItemId::new(),
                                                customer: to,
                                                ..cart_item
                                            },
                                        },
                                    )
                                    .map(|(_, conn)| ((), conn))
                            }));
                        }
                        b
                    }
                })
                .and_then({
                    let repo_factory = repo_factory.clone();
                    move |(_, conn)| get_cart_from_repo(&repo_factory.clone(), conn, to)
                })
        }))
    }
}
