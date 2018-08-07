use std::collections::HashMap;
use std::rc::Rc;

use chrono::prelude::*;
use futures::future;
use futures::prelude::*;

use super::types::ServiceFuture;
use errors::*;
use models::*;
use repos;
use repos::*;
use types::*;

use stq_api::orders::*;
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
        receiver_phone: String,
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
}

pub struct OrderServiceImpl {
    pub db_pool: DbPool,
    pub login_data: UserLogin,
    pub cart_repo_factory: Rc<Fn() -> Box<CartItemRepo>>,
    pub order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    pub order_diff_repo_factory: Rc<Fn() -> Box<OrderDiffRepo>>,
}

impl OrderServiceImpl {
    pub fn new(db_pool: DbPool, login_data: UserLogin) -> Self {
        Self {
            db_pool: db_pool.clone(),
            cart_repo_factory: Rc::new({
                let login_data = login_data.clone();
                move || Box::new(repos::cart_item::make_repo(login_data.clone()))
            }),
            order_diff_repo_factory: Rc::new({
                let login_data = login_data.clone();
                move || Box::new(repos::order_diff::make_repo(login_data.clone()))
            }),
            order_repo_factory: Rc::new({
                let login_data = login_data.clone();
                move || Box::new(repos::order::make_repo(login_data.clone()))
            }),
            login_data,
        }
    }
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(
        &self,
        conversion_id: Option<ConversionId>,
        customer_id: UserId,
        seller_prices: HashMap<ProductId, ProductSellerPrice>,
        address: AddressFull,
        receiver_name: String,
        receiver_phone: String,
    ) -> ServiceFuture<Vec<Order>> {
        use self::RepoLogin::*;

        let order_repo_factory = self.order_repo_factory.clone();
        let order_diffs_repo_factory = self.order_diff_repo_factory.clone();
        let cart_repo_factory = self.cart_repo_factory.clone();
        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };

        Box::new(self.db_pool.run(move |conn| {
            (cart_repo_factory)()
                    .delete(conn, CartItemFilter { customer: Some(CartCustomer::User(customer_id)), meta_filter: CartItemMetaFilter { selected: Some(true), ..Default::default() } })
                    // Create orders from cart items
                    .and_then(move |(cart, conn)| {
                        let mut order_items = Vec::new();
                        for cart_item in cart {
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
                                    receiver_phone: receiver_phone.clone(),
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

                            for (new_order, comment) in new_orders {
                                out = Box::new(out.and_then({
                                    let comment = comment.clone();
                                    let order_repo_factory = order_repo_factory.clone();
                                    let order_diffs_repo_factory = order_diffs_repo_factory.clone();
                                    move |(mut out_data, conn)| {
                                        // Insert new order along with the record in history
                                        (order_repo_factory)().insert_exactly_one(conn, new_order).and_then(move |(inserted_order, conn)| {
                                            (order_diffs_repo_factory)().insert_exactly_one(conn, OrderDiffInserter {
                                                parent: inserted_order.0.id,
                                                committer: calling_user,
                                                committed_at: Utc::now(),
                                                state: OrderState::New,
                                                comment: Some(comment),
                                            }).map(|(_, conn)| (inserted_order, conn))
                                        }).map({
                                            move |(order, conn): (DbOrder, RepoConnection)| {
                                                out_data.push(order.0);
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

                        for order in orders {
                            out = Box::new(out.and_then({
                                let order_diff_repo_factory = order_diff_repo_factory.clone();
                                move |(mut orders_with_diffs, conn): (Vec<(DbOrder, Vec<DbOrderDiff>)>, _)| {
                                    (order_diff_repo_factory)()
                                        .delete(
                                            conn,
                                            OrderDiffFilter {
                                                parent: Some(order.0.id.into()),
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
                        let mut cart_item = CartItem {
                            id: order.0.created_from,
                            customer: CartCustomer::User(order.0.customer),
                            product_id: order.0.product,
                            quantity: order.0.quantity,
                            selected: true,
                            comment: "".into(),
                            store_id: order.0.store,
                        };
                        for diff in diffs {
                            if diff.0.state == OrderState::New {
                                if let Some(comment) = diff.0.comment {
                                    cart_item.comment = comment;
                                    break;
                                }
                            }
                        }
                        CartItemInserter {
                            strategy: CartItemMergeStrategy::Replacer,
                            data: cart_item,
                        }
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
                .map(|mut orders| orders.pop().map(|v| v.0)),
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
                        .map(|mut orders| orders.pop().map(|order| order.0.id)),
                ),
            }.and_then(move |id| match id {
                None => Box::new(future::ok(vec![])) as ServiceFuture<Vec<OrderDiff>>,
                Some(id) => Box::new(
                    db_pool
                        .run(move |conn| (order_diff_repo_factory)().select(conn, OrderDiffFilter::from(id).with_ordering(true)))
                        .map(|v| v.into_iter().map(|v| v.0).collect()),
                ),
            }),
        )
    }

    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (order_repo_factory)().select(
                        conn,
                        OrderFilter {
                            store: Some(store_id.into()),
                            ..Default::default()
                        },
                    )
                })
                .map(|v| v.into_iter().map(|v| v.0).collect()),
        )
    }

    fn get_orders_for_user(&self, customer: UserId) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (order_repo_factory)().select(
                        conn,
                        OrderFilter {
                            customer: Some(customer.into()),
                            ..Default::default()
                        },
                    )
                })
                .map(|v| v.into_iter().map(|v| v.0).collect()),
        )
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
            future::result(OrderFilter::from_search_terms(terms))
                .map(|filter| filter.with_ordering(true))
                .and_then(move |filter| db_pool.run(move |conn| (order_repo_factory)().select(conn, filter)))
                .map(|v| v.into_iter().map(|v| v.0).collect()),
        )
    }

    fn set_order_state(
        &self,
        order_id: OrderIdentifier,
        state: OrderState,
        comment: Option<String>,
        track_id: Option<String>,
    ) -> ServiceFuture<Option<Order>> {
        use self::RepoLogin::*;

        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();
        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };
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
                                parent: order.0.id,
                                committer: calling_user,
                                committed_at: Utc::now(),
                                state: order.0.state.clone(),
                                comment,
                            }).map(move |(_, c)| (Some(order.0), c))
                            )
                        } else {
                            Box::new(future::ok((None, conn))) as RepoConnectionFuture<Option<Order>>
                        }
                    })
                }),
        )
    }
}
