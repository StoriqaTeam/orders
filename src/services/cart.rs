use super::types::ServiceFuture;
use models::*;
use repos;
use repos::*;
use types::*;

use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_api::orders::*;
use stq_db::repo::*;
use stq_db::statement::*;
use stq_static_resources::*;
use stq_types::*;

/// Service that provides operations for interacting with user carts
pub trait CartService {
    /// Get user's cart contents
    fn get_cart(&self, customer: CartCustomer, currency_type: Option<CurrencyType>) -> ServiceFuture<Cart>;
    /// Increase item's quantity by 1
    fn increment_item(&self, customer: CartCustomer, product_id: ProductId, payload: CartProductIncrementPayload) -> ServiceFuture<Cart>;
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
    fn merge(&self, from: CartCustomer, to: CartCustomer, currency_type: Option<CurrencyType>) -> ServiceFuture<Cart>;
    /// Add coupon
    fn add_coupon(&self, customer: CartCustomer, product_id: ProductId, coupon_id: CouponId) -> ServiceFuture<Cart>;
    /// Delete coupon
    fn delete_coupon(&self, customer: CartCustomer, product_id: CouponId) -> ServiceFuture<Cart>;
    /// Delete coupon by product_id
    fn delete_coupon_by_product(&self, customer: CartCustomer, product_id: ProductId) -> ServiceFuture<Cart>;
    /// Set delivery company
    fn set_delivery_method(
        &self,
        customer: CartCustomer,
        product_id: ProductId,
        delivery_method_id: Option<DeliveryMethodId>,
    ) -> ServiceFuture<Cart>;
    /// Delete products from all carts
    fn delete_products_from_all_carts(&self, product_ids: Vec<ProductId>) -> ServiceFuture<()>;

    /// Delete delivery method from all carts
    fn delete_delivery_method_from_all_carts(&self, product_ids: Vec<ProductId>) -> ServiceFuture<()>;
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
            repo_factory: Rc::new({ move || Box::new(repos::cart_item::make_repo(login_data.clone())) }),
        }
    }
}

impl CartService for CartServiceImpl {
    fn get_cart(&self, customer: CartCustomer, currency_type: Option<CurrencyType>) -> ServiceFuture<Cart> {
        debug!("Getting cart for customer {}.", customer);
        Box::new(
            self.db_pool
                .run({
                    let repo_factory = self.repo_factory.clone();
                    move |conn| {
                        (repo_factory)().select(
                            conn,
                            CartItemFilter {
                                customer: Some(customer),
                                meta_filter: CartItemMetaFilter {
                                    currency_type,
                                    ..Default::default()
                                },
                            },
                        )
                    }
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn increment_item(&self, customer: CartCustomer, product_id: ProductId, payload: CartProductIncrementPayload) -> ServiceFuture<Cart> {
        debug!("Adding 1 item {} into cart for customer {}", product_id, customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run({
                    move |conn| {
                        future::ok(conn)
                            .and_then({
                                let repo_factory = repo_factory.clone();
                                let payload = payload.clone();
                                move |conn| {
                                    (repo_factory)().insert_exactly_one(
                                        conn,
                                        CartItemInserter {
                                            strategy: CartItemMergeStrategy::Incrementer,
                                            data: CartItem {
                                                id: CartItemId::new(),
                                                customer,
                                                product_id,
                                                store_id: payload.store_id,
                                                quantity: Quantity(1),
                                                selected: true,
                                                comment: String::new(),
                                                pre_order: payload.pre_order,
                                                pre_order_days: payload.pre_order_days,
                                                coupon_id: None,
                                                delivery_method_id: None,
                                                currency_type: payload.currency_type,
                                                user_country_code: payload.user_country_code,
                                            },
                                        },
                                    )
                                }
                            })
                            .and_then({
                                let repo_factory = repo_factory.clone();
                                move |(_, conn)| {
                                    let repo: Box<CartItemRepo> = (repo_factory)();
                                    repo.update(
                                        conn,
                                        CartItemUpdater {
                                            data: CartItemUpdateData {
                                                user_country_code: Some(payload.user_country_code),
                                                ..Default::default()
                                            },
                                            filter: CartItemFilter {
                                                customer: Some(customer),
                                                meta_filter: Default::default(),
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
                                            meta_filter: CartItemMetaFilter { ..Default::default() },
                                        },
                                    )
                                }
                            })
                    }
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn set_quantity(&self, customer: CartCustomer, product_id: ProductId, quantity: Quantity) -> ServiceFuture<Cart> {
        debug!("Setting quantity for item {} for customer {} to {}", product_id, customer, quantity);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
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
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn set_selection(&self, customer: CartCustomer, product_id: ProductId, selected: bool) -> ServiceFuture<Cart> {
        debug!(
            "Setting selection for item {} for customer {} to {}",
            product_id, customer, selected
        );

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
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
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn set_comment(&self, customer: CartCustomer, product_id: ProductId, comment: String) -> ServiceFuture<Cart> {
        debug!("Setting comment for item {} for customer {} to {}", product_id, customer, comment);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
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
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn delete_item(&self, customer: CartCustomer, product_id: ProductId) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for customer {}", product_id, customer);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .select_exactly_one(
                            conn,
                            CartItemFilter {
                                customer: Some(customer),
                                meta_filter: CartItemMetaFilter {
                                    product_id: Some(product_id.into()),
                                    ..Default::default()
                                },
                            },
                        )
                        .and_then({
                            let repo_factory = repo_factory.clone();
                            move |(_, conn)| {
                                (repo_factory)()
                                    .delete(
                                        conn,
                                        CartItemFilter {
                                            customer: Some(customer),
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
                                                    customer: Some(customer),
                                                    meta_filter: CartItemMetaFilter { ..Default::default() },
                                                },
                                            )
                                        }
                                    })
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
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
                            customer: Some(customer),
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
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().select_full(
                        conn,
                        CartItemFilter {
                            customer: Some(customer),
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
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn merge(&self, from: CartCustomer, to: CartCustomer, currency_type: Option<CurrencyType>) -> ServiceFuture<Cart> {
        debug!("Merging cart contents from {} to {}", from, to);

        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    future::ok(conn)
                        .and_then({
                            let repo_factory = repo_factory.clone();
                            move |conn| {
                                (repo_factory)().delete(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(from),
                                        meta_filter: CartItemMetaFilter {
                                            currency_type,
                                            ..Default::default()
                                        },
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
                                        let f: Box<CartItemRepo> = (repo_factory)();
                                        f.insert(
                                            conn,
                                            CartItemInserter {
                                                strategy: CartItemMergeStrategy::CollisionNoOp,
                                                data: CartItem { customer: to, ..cart_item },
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
                            move |(_, conn)| {
                                Box::new((repo_factory)().select(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(to),
                                        meta_filter: CartItemMetaFilter {
                                            currency_type,
                                            ..Default::default()
                                        },
                                    },
                                ))
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn add_coupon(&self, customer: CartCustomer, product_id: ProductId, coupon_id: CouponId) -> ServiceFuture<Cart> {
        debug!("Add coupon {} for product {} for customer {}", coupon_id, product_id, customer);
        let repo_factory = self.repo_factory.clone();

        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
                                    meta_filter: CartItemMetaFilter {
                                        product_id: Some(product_id.into()),
                                        ..Default::default()
                                    },
                                },
                                data: CartItemUpdateData {
                                    coupon_id: Some(Some(coupon_id)),
                                    ..Default::default()
                                },
                            },
                        )
                        .and_then({
                            let repo_factory = repo_factory.clone();
                            move |(_, conn)| {
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn delete_coupon_by_product(&self, customer: CartCustomer, product_id: ProductId) -> ServiceFuture<Cart> {
        debug!("Delete coupon for product {} from customer {}", product_id, customer);
        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
                                    meta_filter: CartItemMetaFilter {
                                        product_id: Some(product_id.into()),
                                        ..Default::default()
                                    },
                                },
                                data: CartItemUpdateData {
                                    coupon_id: Some(None),
                                    ..Default::default()
                                },
                            },
                        )
                        .and_then({
                            let repo_factory = repo_factory.clone();
                            move |(_, conn)| {
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn delete_coupon(&self, customer: CartCustomer, coupon_id: CouponId) -> ServiceFuture<Cart> {
        debug!("Delete coupon {} from customer {}", coupon_id, customer);
        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
                                    meta_filter: CartItemMetaFilter {
                                        coupon_id: Some(coupon_id.into()),
                                        ..Default::default()
                                    },
                                },
                                data: CartItemUpdateData {
                                    coupon_id: Some(None),
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
                                        customer: Some(customer),
                                        ..Default::default()
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn set_delivery_method(
        &self,
        customer: CartCustomer,
        product_id: ProductId,
        delivery_method_id: Option<DeliveryMethodId>,
    ) -> ServiceFuture<Cart> {
        debug!(
            "Set delivery method {:?} for product {} for customer {}",
            delivery_method_id, product_id, customer
        );
        let repo_factory = self.repo_factory.clone();

        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)()
                        .update(
                            conn,
                            CartItemUpdater {
                                filter: CartItemFilter {
                                    customer: Some(customer),
                                    meta_filter: CartItemMetaFilter {
                                        product_id: Some(product_id.into()),
                                        ..Default::default()
                                    },
                                },
                                data: CartItemUpdateData {
                                    delivery_method_id: Some(delivery_method_id),
                                    ..Default::default()
                                },
                            },
                        )
                        .and_then({
                            let repo_factory = repo_factory.clone();
                            move |(_, conn)| {
                                (repo_factory)().select_exactly_one(
                                    conn,
                                    CartItemFilter {
                                        customer: Some(customer),
                                        meta_filter: CartItemMetaFilter {
                                            product_id: Some(product_id.into()),
                                            ..Default::default()
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
                                        meta_filter: CartItemMetaFilter { ..Default::default() },
                                    },
                                )
                            }
                        })
                })
                .map(|c| c.into_iter().collect()),
        )
    }

    fn delete_products_from_all_carts(&self, product_ids: Vec<ProductId>) -> ServiceFuture<()> {
        debug!("delete_products_from_all_carts {} products from all carts", product_ids.len());
        let repo_factory = self.repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().delete(
                        conn,
                        CartItemFilter {
                            customer: None,
                            meta_filter: CartItemMetaFilter {
                                product_id: Some(Range::In(product_ids)),
                                ..Default::default()
                            },
                        },
                    )
                })
                .map(|_| ()),
        )
    }

    /// Delete delivery method from all carts
    fn delete_delivery_method_from_all_carts(&self, product_ids: Vec<ProductId>) -> ServiceFuture<()> {
        debug!(
            "delete_delivery_method_from_all_carts {} clear delivery method from all carts",
            product_ids.len()
        );
        let repo_factory = self.repo_factory.clone();

        Box::new(
            self.db_pool
                .run(move |conn| {
                    (repo_factory)().update(
                        conn,
                        CartItemUpdater {
                            filter: CartItemFilter {
                                customer: None,
                                meta_filter: CartItemMetaFilter {
                                    product_id: Some(Range::In(product_ids)),
                                    ..Default::default()
                                },
                            },
                            data: CartItemUpdateData {
                                delivery_method_id: Some(None),
                                comment: Some("Selected delivery has changed/removed by store manager".to_string()),
                                ..Default::default()
                            },
                        },
                    )
                })
                .map(|_| ()),
        )
    }
}
