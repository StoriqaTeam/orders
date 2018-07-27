use std::collections::HashMap;
use std::rc::Rc;

use chrono::prelude::*;
use futures::future;
use futures::prelude::*;

use super::types::ServiceFuture;
use errors::*;
use models::*;
use repos::*;
use types::*;

use stq_db::repo::*;
use stq_static_resources::OrderState;
use stq_types::*;

#[derive(Clone, Debug)]
pub enum RoleRemoveFilter {
    Id(RoleId),
    Meta((UserId, Option<UserRole>)),
}

pub trait OrderService {
    fn convert_cart(
        &self,
        conversion_id: Option<ConversionId>,
        user_id: UserId,
        seller_prices: HashMap<ProductId, ProductSellerPrice>,
        address: AddressFull,
        receiver_name: String,
    ) -> ServiceFuture<Vec<Order>>;
    fn revert_cart_conversion(&self, convertation_id: ConversionId) -> ServiceFuture<()>;
    // fn create_order(&self) -> ServiceFuture<Order>;
    fn get_order(&self, id: OrderIdentifier) -> ServiceFuture<Option<Order>>;
    fn get_order_diff(&self, id: OrderIdentifier) -> ServiceFuture<Vec<OrderDiff>>;
    fn get_orders_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Order>>;
    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>>;
    fn delete_order(&self, id: OrderIdentifier) -> ServiceFuture<()>;
    fn set_order_state(
        &self,
        order_id: OrderIdentifier,
        state: OrderState,
        comment: Option<String>,
        track_id: Option<String>,
    ) -> ServiceFuture<Option<Order>>;
    /// Search using the terms provided.
    fn search(&self, terms: OrderSearchTerms) -> ServiceFuture<Vec<Order>>;

    fn get_roles_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Role>>;
    fn create_role(&self, item: Role) -> ServiceFuture<Role>;
    fn remove_role(&self, filter: RoleRemoveFilter) -> ServiceFuture<Option<Role>>;
    fn remove_all_roles(&self, user_id: UserId) -> ServiceFuture<Vec<Role>>;
}

pub struct OrderServiceImpl {
    pub cart_repo_factory: Rc<Fn() -> Box<ProductRepo>>,
    pub order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    pub order_diff_repo_factory: Rc<Fn() -> Box<OrderDiffRepo>>,
    pub roles_repo_factory: Rc<Fn() -> Box<RolesRepo>>,
    pub db_pool: DbPool,
    pub calling_user: UserId,
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(
        &self,
        conversion_id: Option<ConversionId>,
        customer_id: UserId,
        seller_prices: HashMap<ProductId, ProductSellerPrice>,
        address: AddressFull,
        receiver_name: String,
    ) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diffs_repo_factory = self.order_diff_repo_factory.clone();
        let cart_repo_factory = self.cart_repo_factory.clone();
        let calling_user = self.calling_user;

        Box::new(self.db_pool.run(move |conn| {
            (cart_repo_factory)()
                    .delete(conn, CartProductMask { user_id: Some(customer_id.into()), selected: Some(true), ..Default::default() })
                    // Create orders from cart items
                    .and_then(move |(cart, conn)| {
                        let mut order_items = Vec::new();
                        for cart_item in cart.into_iter() {
                            if let Some(seller_price) = seller_prices.get(&cart_item.product_id).cloned() {
                                let ProductSellerPrice { price, currency_id } = seller_price;
                                order_items.push((OrderInserter {
                                    id: None,
                                    created_from: Some(cart_item.id),
                                    conversion_id,
                                    customer: customer_id,
                                    store: cart_item.store_id,
                                    product: cart_item.product_id,
                                    quantity: cart_item.quantity,
                                    price,
                                    currency_id,
                                    address: address.clone(),
                                    receiver_name: receiver_name.clone(),
                                    state: OrderState::New,
                                    delivery_company: None,
                                    track_id: None,
                                }, cart_item.comment))
                            } else {
                                return Err((format_err!("Missing price information for product {}", cart_item.product_id).context(Error::MissingPrice).into(), conn));
                            }
                        }
                        Ok((order_items, conn))
                    })
                    // Insert new orders into database
                    .and_then({
                        move |(new_orders, conn)| {
                            let mut out: RepoConnectionFuture<Vec<Order>>;
                            out = Box::new(future::ok((Default::default(), conn)));

                            for (new_order, comment) in new_orders.into_iter() {
                                out = Box::new(out.and_then({
                                    let comment = comment.clone();
                                    let order_repo_factory = order_repo_factory.clone();
                                    let order_diffs_repo_factory = order_diffs_repo_factory.clone();
                                    move |(mut out_data, conn)| {
                                        // Insert new order along with the record in history
                                        (order_repo_factory)().insert_exactly_one(conn, new_order).and_then(move |(inserted_order, conn)| {
                                            (order_diffs_repo_factory)().insert_exactly_one(conn, OrderDiffInserter {
                                                parent: inserted_order.id,
                                                committer: calling_user,
                                                committed_at: Utc::now(),
                                                state: OrderState::New,
                                                comment: Some(comment),
                                            }).map(|(_, conn)| (inserted_order, conn))
                                        }).map({
                                            move |(order, conn): (Order, RepoConnection)| {
                                                out_data.push(order);
                                                (out_data, conn)
                                            }
                                        })
                                    }
                                }));
                            }

                            out
                        }
                    })
        }))
    }

    fn revert_cart_conversion(&self, conversion_id: ConversionId) -> ServiceFuture<()> {
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let cart_repo_factory = self.cart_repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (order_repo_factory)()
                .select(
                    conn,
                    OrderFilter {
                        conversion_id: Some(conversion_id.into()),
                        ..Default::default()
                    },
                )
                .and_then({
                    let order_diff_repo_factory = order_diff_repo_factory.clone();
                    move |(orders, conn)| {
                        let mut out = Box::new(future::ok((Default::default(), conn))) as Box<Future<Item = _, Error = _>>;

                        for order in orders.into_iter() {
                            out = Box::new(out.and_then({
                                let order_diff_repo_factory = order_diff_repo_factory.clone();
                                move |(mut orders_with_diffs, conn): (Vec<(Order, Vec<OrderDiff>)>, _)| {
                                    (order_diff_repo_factory)()
                                        .delete(
                                            conn,
                                            OrderDiffFilter {
                                                parent: Some(order.id.into()),
                                                ..Default::default()
                                            },
                                        )
                                        .map(move |(order_diffs, conn)| {
                                            orders_with_diffs.push((order, order_diffs));
                                            (orders_with_diffs, conn)
                                        })
                                }
                            }));
                        }

                        out
                    }
                })
                .and_then({
                    let order_repo_factory = order_repo_factory.clone();
                    move |(orders_with_diffs, conn)| {
                        (order_repo_factory)()
                            .delete(
                                conn,
                                OrderFilter {
                                    conversion_id: Some(conversion_id.into()),
                                    ..Default::default()
                                },
                            )
                            .map(move |(_, conn)| (orders_with_diffs, conn))
                    }
                })
                .and_then(move |(orders_with_diffs, conn)| {
                    let new_cart_items = orders_with_diffs.into_iter().map(|(order, diffs)| {
                        let mut cart_item = NewCartProduct {
                            id: order.created_from,
                            user_id: order.customer,
                            product_id: order.product,
                            quantity: order.quantity,
                            selected: true,
                            comment: "".into(),
                            store_id: order.store,
                        };
                        for diff in diffs {
                            if diff.state == OrderState::New {
                                if let Some(comment) = diff.comment {
                                    cart_item.comment = comment;
                                    break;
                                }
                            }
                        }
                        CartProductInserter::Replacer(cart_item)
                    });

                    let mut out = Box::new(future::ok(conn)) as Box<Future<Item = _, Error = _>>;

                    for cart_item in new_cart_items {
                        out = Box::new(out.and_then({
                            let cart_repo_factory = cart_repo_factory.clone();
                            move |conn| (cart_repo_factory)().insert_exactly_one(conn, cart_item).map(|(_, conn)| conn)
                        }))
                    }

                    out.map(|conn| ((), conn))
                })
        }))
    }

    fn get_order(&self, order_id: OrderIdentifier) -> ServiceFuture<Option<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| (order_repo_factory)().select(conn, OrderFilter::from(order_id)))
                .map(|orders| orders.first().cloned()),
        )
    }

    fn get_order_diff(&self, order_id: OrderIdentifier) -> ServiceFuture<Vec<OrderDiff>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();
        Box::new(
            match order_id {
                OrderIdentifier::Id(id) => Box::new(future::ok(Some(id))) as ServiceFuture<Option<OrderId>>,
                OrderIdentifier::Slug(_slug) => Box::new(
                    db_pool
                        .run(move |conn| (order_repo_factory)().select(conn, OrderFilter::from(order_id)))
                        .map(|orders| orders.first().map(|order| order.id)),
                ),
            }.and_then(move |id| match id {
                None => Box::new(future::ok(vec![])) as ServiceFuture<Vec<OrderDiff>>,
                Some(id) => Box::new(db_pool.run(move |conn| (order_diff_repo_factory)().select(conn, OrderDiffFilter::from(id)))),
            }),
        )
    }

    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (order_repo_factory)().select(
                conn,
                OrderFilter {
                    store: Some(store_id.into()),
                    ..Default::default()
                },
            )
        }))
    }

    fn get_orders_for_user(&self, customer: UserId) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (order_repo_factory)().select(
                conn,
                OrderFilter {
                    customer: Some(customer.into()),
                    ..Default::default()
                },
            )
        }))
    }

    fn delete_order(&self, order_id: OrderIdentifier) -> ServiceFuture<()> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| (order_repo_factory)().delete(conn, OrderFilter::from(order_id)))
                .map(|_| ()),
        )
    }

    fn search(&self, terms: OrderSearchTerms) -> ServiceFuture<Vec<Order>> {
        let db_pool = self.db_pool.clone();
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            future::result(terms.make_filter())
                .map(|filter| filter.with_ordering(true))
                .and_then(move |filter| db_pool.run(move |conn| (order_repo_factory)().select(conn, filter))),
        )
    }

    fn set_order_state(
        &self,
        order_id: OrderIdentifier,
        state: OrderState,
        comment: Option<String>,
        track_id: Option<String>,
    ) -> ServiceFuture<Option<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();
        let calling_user = self.calling_user;
        Box::new(
            db_pool
                .run(move |conn| {
                    (order_repo_factory)()
                        .update(
                            conn,
                            OrderUpdater {
                                mask: order_id.into(),
                                data: OrderUpdateData { state: Some(state), track_id },
                            },
                        )
                })
                .map(|mut out_data| out_data.pop() )
                // Insert new order diff into database
                .and_then(move |updated_order| {
                    db_pool.run(move |conn| {
                        if let Some(order) = updated_order {
                            Box::new(
                                (order_diff_repo_factory)().insert_exactly_one(conn, OrderDiffInserter {
                                parent: order.id,
                                committer: calling_user,
                                committed_at: Utc::now(),
                                state: order.state.clone(),
                                comment: comment,
                            }).map(move |(_, c)| (Some(order), c))
                            )
                        } else {
                            Box::new(future::ok((None,conn))) as RepoConnectionFuture<Option<Order>>
                        }
                    })
                }),
        )
    }

    fn get_roles_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Role>> {
        let roles_repo_factory = self.roles_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (roles_repo_factory)().select(
                        conn,
                        RoleFilter {
                            user_id: Some(user_id.into()),
                            ..Default::default()
                        },
                    )
                })
                .map_err(move |e| e.context(format!("Failed to get roles for user {}", user_id.0)).into()),
        )
    }
    fn create_role(&self, item: Role) -> ServiceFuture<Role> {
        let roles_repo_factory = self.roles_repo_factory.clone();
        Box::new(
            self.db_pool
                .run({
                    let item = item.clone();
                    move |conn| (roles_repo_factory)().insert_exactly_one(conn, item)
                })
                .map_err(move |e| e.context(format!("Failed to create role: {:?}", item)).into()),
        )
    }
    fn remove_role(&self, filter: RoleRemoveFilter) -> ServiceFuture<Option<Role>> {
        let roles_repo_factory = self.roles_repo_factory.clone();
        Box::new(
            self.db_pool
                .run({
                    let filter = filter.clone();
                    move |conn| {
                        (roles_repo_factory)().delete(
                            conn,
                            match filter {
                                RoleRemoveFilter::Id(id) => RoleFilter {
                                    id: Some(id).map(From::from),
                                    ..Default::default()
                                },
                                RoleRemoveFilter::Meta((user_id, role)) => RoleFilter {
                                    user_id: Some(user_id).map(From::from),
                                    role: role.map(From::from),
                                    ..Default::default()
                                },
                            },
                        )
                    }
                })
                .map(|mut v| v.pop())
                .map_err(move |e| e.context(format!("Failed to remove role: {:?}", filter)).into()),
        )
    }
    fn remove_all_roles(&self, user_id: UserId) -> ServiceFuture<Vec<Role>> {
        let roles_repo_factory = self.roles_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (roles_repo_factory)().delete(
                        conn,
                        RoleFilter {
                            user_id: Some(user_id.into()),
                            ..Default::default()
                        },
                    )
                })
                .map_err(move |e| e.context(format!("Failed to remove all roles for user {}", user_id.0)).into()),
        )
    }
}
