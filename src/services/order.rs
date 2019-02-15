use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use chrono::prelude::*;
use chrono::Duration as ChronoDuration;
use futures::future;
use futures::prelude::*;

use super::types::ServiceFuture;
use errors::*;
use models::*;
use repos;
use repos::*;
use types::*;

use stq_api::orders::*;
use stq_db::{connection::BoxedConnection, repo::*};
use stq_static_resources::{CommitterRole, OrderState};
use stq_types::*;

pub const ZERO_DISCOUNT: f64 = 0.0001;

#[derive(Clone, Debug)]
pub enum RoleRemoveFilter {
    Id(RoleId),
    Meta((UserId, Option<UserRole>)),
}

pub trait OrderService {
    fn convert_cart(&self, payload: ConvertCartPayload) -> ServiceFuture<Vec<Order>>;
    fn create_buy_now(&self, payload: BuyNow, conversion_id: Option<ConversionId>) -> ServiceFuture<Vec<Order>>;
    fn delete_order_and_revert_cart_conversion(&self, convertation_id: ConversionId) -> ServiceFuture<()>;
    fn get_order(&self, id: OrderIdentifier) -> ServiceFuture<Option<Order>>;
    fn get_order_diff(&self, id: OrderIdentifier) -> ServiceFuture<Vec<OrderDiff>>;
    fn get_orders_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Order>>;
    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>>;
    fn get_orders_with_state(&self, state: OrderState, from: DateTime<Utc>) -> ServiceFuture<Vec<Order>>;
    fn search_by_diffs(&self, diff_filter: OrderDiffFilter) -> ServiceFuture<Vec<Order>>;
    fn delete_order(&self, id: OrderIdentifier) -> ServiceFuture<()>;
    fn set_order_state(
        &self,
        order_id: OrderIdentifier,
        state: OrderState,
        comment: Option<String>,
        track_id: Option<String>,
        committer_role: CommitterRole,
    ) -> ServiceFuture<Option<Order>>;
    /// Search using the terms provided.
    fn search(&self, terms: OrderSearchTerms) -> ServiceFuture<Vec<Order>>;
    fn track_delivered_orders(&self, max_delivered_state_duration: ChronoDuration) -> ServiceFuture<Vec<Order>>;
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
            db_pool,
            login_data,
        }
    }
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(&self, payload: ConvertCartPayload) -> ServiceFuture<Vec<Order>> {
        use self::RepoLogin::*;
        let ConvertCartPayload {
            conversion_id,
            user_id,
            seller_prices,
            address,
            receiver_name,
            receiver_phone,
            receiver_email,
            coupons,
            delivery_info,
            product_info,
            uuid,
            currency_type,
        } = payload;

        let order_repo_factory = self.order_repo_factory.clone();
        let order_diffs_repo_factory = self.order_diff_repo_factory.clone();
        let cart_repo_factory = self.cart_repo_factory.clone();
        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };
        let mut transaction_id = TransactionId::new(uuid);

        Box::new(self.db_pool.run(move |conn| {
            (cart_repo_factory)()
                .delete(
                    conn,
                    CartItemFilter {
                        customer: Some(user_id.into()),
                        meta_filter: CartItemMetaFilter {
                            selected: Some(true),
                            currency_type,
                            ..Default::default()
                        },
                    },
                )
                // Create orders from cart items
                .and_then(move |(cart, conn)| {
                    let mut order_items = Vec::new();
                    for cart_item in cart {
                        if let Some(seller_price) = seller_prices.get(&cart_item.product_id).cloned() {
                            transaction_id = transaction_id.next();
                            let ProductSellerPrice { price, currency, discount } = seller_price;
                            let (company_package_id, shipping_id, delivery_name, delivery_price) =
                                match delivery_info.get(&cart_item.product_id).cloned() {
                                    None => (None, None, None, 0.0),
                                    Some(delivery_info) => (
                                        Some(delivery_info.company_package_id.clone()),
                                        Some(delivery_info.shipping_id.clone()),
                                        Some(delivery_info.name.clone()),
                                        delivery_info.price,
                                    ),
                                };

                            let coupon_percent = cart_item
                                .coupon_id
                                .and_then(|coupon_id| coupons.get(&coupon_id))
                                .map(|coupon| coupon.percent);

                            let product_cashback = product_info.get(&cart_item.product_id).and_then(|product| product.cashback);

                            let TotalAmount {
                                total_amount,
                                coupon_discount,
                                product_discount,
                            } = calculate_total_amount(cart_item.quantity, price, discount, coupon_percent, delivery_price);
                            order_items.push((
                                OrderInserter {
                                    id: None,
                                    created_from: Some(cart_item.id),
                                    conversion_id,
                                    customer: user_id,
                                    store: cart_item.store_id,
                                    product: cart_item.product_id,
                                    quantity: cart_item.quantity,
                                    price,
                                    currency,
                                    address: address.clone(),
                                    receiver_name: receiver_name.clone(),
                                    receiver_phone: receiver_phone.clone(),
                                    receiver_email: receiver_email.clone(),
                                    state: OrderState::New,
                                    delivery_company: delivery_name,
                                    track_id: None,
                                    pre_order: cart_item.pre_order,
                                    pre_order_days: cart_item.pre_order_days,
                                    coupon_id: cart_item.coupon_id,
                                    coupon_percent,
                                    coupon_discount,
                                    product_discount,
                                    total_amount,
                                    company_package_id,
                                    delivery_price,
                                    shipping_id,
                                    uuid: transaction_id.clone().into(),
                                    product_cashback,
                                    currency_type: cart_item.currency_type,
                                },
                                cart_item.comment,
                            ))
                        } else {
                            return Err((
                                format_err!("Missing price information for product {}", cart_item.product_id)
                                    .context(Error::MissingPrice)
                                    .into(),
                                conn,
                            ));
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
                                    (order_repo_factory)()
                                        .insert_exactly_one(conn, new_order)
                                        .and_then(move |(inserted_order, conn)| {
                                            (order_diffs_repo_factory)()
                                                .insert_exactly_one(
                                                    conn,
                                                    OrderDiffInserter {
                                                        parent: inserted_order.0.id,
                                                        committer: calling_user,
                                                        committed_at: Utc::now(),
                                                        state: OrderState::New,
                                                        comment: Some(comment),
                                                        committer_role: CommitterRole::Customer,
                                                    },
                                                )
                                                .map(|(_, conn)| (inserted_order, conn))
                                        })
                                        .map({
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

    fn delete_order_and_revert_cart_conversion(&self, conversion_id: ConversionId) -> ServiceFuture<()> {
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
                    merge_cart_from_orders(conn, cart_repo_factory, orders_with_diffs).map(|conn| ((), conn))
                })
        }))
    }

    fn create_buy_now(&self, payload: BuyNow, conversion_id: Option<ConversionId>) -> ServiceFuture<Vec<Order>> {
        use self::RepoLogin::*;

        let order_repo_factory = self.order_repo_factory.clone();
        let order_diffs_repo_factory = self.order_diff_repo_factory.clone();
        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };

        Box::new(self.db_pool.run(move |conn| {
            let coupon_percent = payload.coupon.as_ref().map(|c| c.percent);
            let coupon_id = payload.coupon.as_ref().map(|c| c.id);
            let (company_package_id, shipping_id, delivery_name, delivery_price) = match payload.delivery_info.clone() {
                None => (None, None, None, 0.0),
                Some(delivery_info) => (
                    Some(delivery_info.company_package_id.clone()),
                    Some(delivery_info.shipping_id.clone()),
                    Some(delivery_info.name.clone()),
                    delivery_info.price,
                ),
            };

            let TotalAmount {
                total_amount,
                coupon_discount,
                product_discount,
            } = calculate_total_amount(
                payload.quantity,
                payload.price.price,
                payload.price.discount,
                coupon_percent,
                delivery_price,
            );

            let order_item = (
                OrderInserter {
                    id: None,
                    created_from: None,
                    conversion_id,
                    customer: payload.customer_id,
                    store: payload.store_id,
                    product: payload.product_id,
                    quantity: payload.quantity,
                    price: payload.price.price,
                    currency: payload.price.currency,
                    address: payload.address,
                    receiver_name: payload.receiver_name,
                    receiver_phone: payload.receiver_phone,
                    receiver_email: payload.receiver_email,
                    state: OrderState::New,
                    delivery_company: delivery_name,
                    track_id: None,
                    pre_order: payload.pre_order,
                    pre_order_days: payload.pre_order_days,
                    coupon_id,
                    coupon_percent,
                    coupon_discount,
                    product_discount,
                    total_amount,
                    company_package_id,
                    delivery_price,
                    shipping_id,
                    uuid: payload.uuid,
                    product_cashback: payload.product_info.cashback,
                    currency_type: payload.price.currency.currency_type(),
                },
                "Buy now".to_string(),
            );

            let order_items = vec![order_item];
            let mut out: RepoConnectionFuture<Vec<Order>>;
            out = Box::new(future::ok((Default::default(), conn)));

            for (new_order, comment) in order_items {
                out = Box::new(out.and_then({
                    let comment = comment.clone();
                    let order_repo_factory = order_repo_factory.clone();
                    let order_diffs_repo_factory = order_diffs_repo_factory.clone();
                    move |(mut out_data, conn)| {
                        // Insert new order along with the record in history
                        (order_repo_factory)()
                            .insert_exactly_one(conn, new_order)
                            .and_then(move |(inserted_order, conn)| {
                                let order_diff = OrderDiffInserter {
                                    parent: inserted_order.0.id,
                                    committer: calling_user,
                                    committed_at: Utc::now(),
                                    state: OrderState::New,
                                    comment: Some(comment),
                                    committer_role: CommitterRole::Customer,
                                };

                                (order_diffs_repo_factory)()
                                    .insert_exactly_one(conn, order_diff)
                                    .map(|(_, conn)| (inserted_order, conn))
                            })
                            .map({
                                move |(order, conn): (DbOrder, RepoConnection)| {
                                    out_data.push(order.0);
                                    (out_data, conn)
                                }
                            })
                    }
                }));
            }

            out
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
            }
            .and_then(move |id| match id {
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
        committer_role: CommitterRole,
    ) -> ServiceFuture<Option<Order>> {
        use self::RepoLogin::*;

        let cart_repo_factory = self.cart_repo_factory.clone();
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();
        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };

        set_order_state(
            order_id,
            state,
            comment,
            track_id,
            cart_repo_factory,
            order_repo_factory,
            order_diff_repo_factory,
            db_pool,
            calling_user,
            committer_role,
        )
    }

    fn track_delivered_orders(&self, max_delivered_state_duration: ChronoDuration) -> ServiceFuture<Vec<Order>> {
        use self::RepoLogin::User;
        let now = ::chrono::offset::Utc::now();
        let old_order_date = now - max_delivered_state_duration;
        let search_old_delivered_diffs = move |order_id: OrderId| -> OrderDiffFilter {
            OrderDiffFilter {
                parent: Some(order_id).map(From::from),
                state: Some(OrderState::Delivered.into()),
                committed_at_range: ::models::common::into_range(None, Some(old_order_date)),
                ..OrderDiffFilter::default()
            }
        };
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();

        let search_delivered_orders = OrderSearchTerms {
            state: Some(OrderState::Delivered),
            ..OrderSearchTerms::default()
        };

        let calling_user = match self.login_data.clone() {
            User { caller_id, .. } => caller_id,
            _ => UserId(-1),
        };

        let cart_repo_factory = self.cart_repo_factory.clone();
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diff_repo_factory2 = self.order_diff_repo_factory.clone();
        let db_pool2 = self.db_pool.clone();

        let result = self
            .search(search_delivered_orders)
            .map(move |delivered_orders| {
                delivered_orders
                    .into_iter()
                    .map(move |delivered_order| search_old_delivered_diffs(delivered_order.id))
                    .map(move |diff_filter| {
                        let db_pool = db_pool.clone();
                        let order_diff_repo_factory = order_diff_repo_factory.clone();
                        db_pool.run(move |conn| (order_diff_repo_factory)().select(conn, diff_filter))
                    })
            })
            .and_then(::futures::future::join_all)
            .map(move |order_diffs| order_diffs.into_iter().flatten().map(|diff| diff.0.parent).collect::<Vec<_>>())
            .map(move |old_delivered_orders_ids| {
                old_delivered_orders_ids.into_iter().map(move |old_delivered_order_id| {
                    info!("Updating order state for order {}", old_delivered_order_id);
                    set_order_state(
                        OrderIdentifier::Id(old_delivered_order_id),
                        OrderState::Complete,
                        None,
                        None,
                        cart_repo_factory.clone(),
                        order_repo_factory.clone(),
                        order_diff_repo_factory2.clone(),
                        db_pool2.clone(),
                        calling_user,
                        CommitterRole::System,
                    )
                })
            })
            .and_then(::futures::future::join_all)
            .map(|orders| orders.into_iter().filter_map(|order| order).collect());

        Box::new(result)
    }

    fn get_orders_with_state(&self, state: OrderState, from: DateTime<Utc>) -> ServiceFuture<Vec<Order>> {
        let search_orders_in_state = OrderSearchTerms {
            state: Some(state),
            ..OrderSearchTerms::default()
        };
        let orders_in_state = self.search(search_orders_in_state);

        let search_diffs = {
            OrderDiffFilter {
                state: Some(state).map(From::from),
                committed_at_range: ::models::common::into_range(Some(from), None),
                ..OrderDiffFilter::default()
            }
        };

        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool = self.db_pool.clone();

        let diffs_in_max_state_duration = db_pool.run(move |conn| (order_diff_repo_factory)().select(conn, search_diffs));

        let result = orders_in_state
            .join(diffs_in_max_state_duration)
            .map(|(orders_in_state, recent_diffs)| {
                let orders_in_state_ids: HashSet<OrderId> = orders_in_state.iter().map(|order| order.id).collect();
                let recent_diffs_ids: HashSet<OrderId> = recent_diffs.iter().map(|diff| diff.0.parent).collect();
                let mut by_id: HashMap<OrderId, Order> = orders_in_state.into_iter().map(|order| (order.id, order)).collect();
                orders_in_state_ids
                    .intersection(&recent_diffs_ids)
                    .filter_map(|order_id| by_id.remove(&order_id))
                    .collect()
            });
        Box::new(result)
    }

    fn search_by_diffs(&self, diff_filter: OrderDiffFilter) -> ServiceFuture<Vec<Order>> {
        let order_diff_repo_factory = self.order_diff_repo_factory.clone();
        let db_pool_diff = self.db_pool.clone();

        let db_pool_order = self.db_pool.clone();
        let order_repo_factory = self.order_repo_factory.clone();

        let result = db_pool_diff
            .run(move |conn| (order_diff_repo_factory)().select(conn, diff_filter))
            .map(|diffs| diffs.into_iter().map(|diff| diff.0.parent).collect::<Vec<OrderId>>())
            .map(|oder_ids| OrderFilter {
                ids: Some(oder_ids.into()),
                ..Default::default()
            })
            .and_then(move |filter| db_pool_order.run(move |conn| (order_repo_factory)().select(conn, filter)))
            .map(|db_orders| db_orders.into_iter().map(|db_order| db_order.0).collect());

        Box::new(result)
    }
}

struct TotalAmount {
    coupon_discount: Option<ProductPrice>,
    product_discount: Option<ProductPrice>,
    total_amount: ProductPrice,
}

fn calculate_total_amount(
    quantity: Quantity,
    product_price: ProductPrice,
    product_discount_percent: Option<f64>,
    coupon_discount_percent: Option<i32>,
    delivery_price_per_product: f64,
) -> TotalAmount {
    let product_discount_percent = product_discount_percent.filter(|p| *p > ZERO_DISCOUNT);
    let TotalAmount {
        coupon_discount,
        product_discount,
        total_amount,
    } = match (product_discount_percent, coupon_discount_percent) {
        (Some(product_discount_percent), _) => {
            let product_discount = product_discount_percent * product_price.0;
            let total_amount = if quantity.0 > 0 {
                (product_price.0 - product_discount) * quantity.0 as f64
            } else {
                0.0
            };
            TotalAmount {
                coupon_discount: None,
                product_discount: Some(ProductPrice(product_discount)),
                total_amount: ProductPrice(total_amount),
            }
        }
        (None, Some(coupon_discount_percent)) => {
            let coupon_discount = coupon_discount_percent as f64 / 100.0 * product_price.0;
            let total_amount = if quantity.0 > 0 {
                product_price.0 - coupon_discount + product_price.0 * (quantity.0 - 1) as f64
            } else {
                0.0
            };
            TotalAmount {
                coupon_discount: Some(ProductPrice(coupon_discount)),
                product_discount: None,
                total_amount: ProductPrice(total_amount),
            }
        }
        (None, None) => TotalAmount {
            coupon_discount: None,
            product_discount: None,
            total_amount: ProductPrice(quantity.0 as f64 * product_price.0),
        },
    };
    TotalAmount {
        coupon_discount,
        product_discount,
        total_amount: ProductPrice(total_amount.0 + quantity.0 as f64 * delivery_price_per_product),
    }
}

fn set_order_state(
    order_id: OrderIdentifier,
    state: OrderState,
    comment: Option<String>,
    track_id: Option<String>,
    cart_repo_factory: Rc<Fn() -> Box<CartItemRepo>>,
    order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    order_diff_repo_factory: Rc<Fn() -> Box<OrderDiffRepo>>,
    db_pool: DbPool,
    calling_user: UserId,
    committer_role: CommitterRole,
) -> ServiceFuture<Option<Order>> {
    let result = db_pool
        .run(move |conn| {
            (order_repo_factory)().update(
                conn,
                OrderUpdater {
                    mask: order_id.into(),
                    data: OrderUpdateData {
                        state: Some(state),
                        track_id,
                    },
                },
            )
        })
        .map(|mut out_data| out_data.pop())
        // Insert new order diff into database
        .and_then(move |updated_order| {
            db_pool.run(move |conn| {
                if let Some(order) = updated_order {
                    let order_clone = order.clone();
                    Box::new(
                        (order_diff_repo_factory)()
                            .insert_exactly_one(
                                conn,
                                OrderDiffInserter {
                                    parent: order.0.id,
                                    committer: calling_user,
                                    committed_at: Utc::now(),
                                    state: order.0.state,
                                    comment,
                                    committer_role,
                                },
                            )
                            // Revert cart from the order if the payment expired
                            .and_then(move |(_, conn)| match order.0.state {
                                OrderState::AmountExpired => Box::new(
                                    (order_diff_repo_factory)()
                                        .select(
                                            conn,
                                            OrderDiffFilter {
                                                parent: Some(order.0.id.into()),
                                                ..Default::default()
                                            },
                                        )
                                        .and_then(|(order_diffs, conn)| {
                                            let order_with_diffs = vec![(order_clone, order_diffs)];
                                            merge_cart_from_orders(conn, cart_repo_factory, order_with_diffs)
                                        })
                                        .map(|conn| (Some(order.0), conn)),
                                ),
                                _ => Box::new(future::ok((Some(order.0), conn))) as Box<Future<Item = _, Error = _>>,
                            }),
                    )
                } else {
                    Box::new(future::ok((None, conn))) as RepoConnectionFuture<Option<Order>>
                }
            })
        });

    Box::new(result)
}

fn merge_cart_from_orders(
    conn: BoxedConnection<RepoError>,
    cart_repo_factory: Rc<Fn() -> Box<CartItemRepo>>,
    orders_with_diffs: Vec<(DbOrder, Vec<DbOrderDiff>)>,
) -> impl Future<Item = BoxedConnection<RepoError>, Error = (RepoError, BoxedConnection<RepoError>)> {
    let new_cart_items = orders_with_diffs.into_iter().map(|(order, diffs)| {
        let mut cart_item = CartItem {
            id: order.0.created_from,
            customer: CartCustomer::User(order.0.customer),
            product_id: order.0.product,
            quantity: order.0.quantity,
            selected: true,
            comment: "".into(),
            store_id: order.0.store,
            pre_order: false,  // TODO get from order fields
            pre_order_days: 0, // TODO get from order fields
            coupon_id: order.0.coupon_id,
            delivery_method_id: None, // TODO get from order fields
            currency_type: order.0.currency_type,
            user_country_code: None,
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

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_calculates_total_amount() {
        let no_discount = calculate_total_amount(Quantity(2), ProductPrice(100.0), None, None, 0.0);
        assert_eq!(no_discount.total_amount, ProductPrice(200.0));
        assert_eq!(no_discount.product_discount, None);
        assert_eq!(no_discount.coupon_discount, None);

        let no_discount_with_delivery = calculate_total_amount(Quantity(2), ProductPrice(100.0), None, None, 100.0);
        assert_eq!(no_discount_with_delivery.total_amount, ProductPrice(400.0));
        assert_eq!(no_discount_with_delivery.product_discount, None);
        assert_eq!(no_discount_with_delivery.coupon_discount, None);

        let product_discount = calculate_total_amount(Quantity(2), ProductPrice(100.0), Some(0.2), None, 0.0);
        assert_eq!(product_discount.total_amount, ProductPrice(160.0));
        assert_eq!(product_discount.product_discount, Some(ProductPrice(20.0)));
        assert_eq!(product_discount.coupon_discount, None);

        let coupon_discount = calculate_total_amount(Quantity(2), ProductPrice(100.0), None, Some(30), 0.0);
        assert_eq!(coupon_discount.total_amount, ProductPrice(170.0));
        assert_eq!(coupon_discount.product_discount, None);
        assert_eq!(coupon_discount.coupon_discount, Some(ProductPrice(30.0)));

        let product_and_coupon_discount = calculate_total_amount(Quantity(2), ProductPrice(100.0), Some(0.2), Some(25), 0.0);
        assert_eq!(product_and_coupon_discount.total_amount, ProductPrice(160.0));
        assert_eq!(product_and_coupon_discount.product_discount, Some(ProductPrice(20.0)));
        assert_eq!(product_and_coupon_discount.coupon_discount, None);

        let product_and_coupon_discount_with_delivery =
            calculate_total_amount(Quantity(2), ProductPrice(100.0), Some(0.2), Some(25), 100.0);
        assert_eq!(product_and_coupon_discount_with_delivery.total_amount, ProductPrice(360.0));
        assert_eq!(product_and_coupon_discount_with_delivery.product_discount, Some(ProductPrice(20.0)));
        assert_eq!(product_and_coupon_discount_with_delivery.coupon_discount, None);
    }
}
