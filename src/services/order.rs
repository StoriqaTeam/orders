use super::types::ServiceFuture;
use super::CartService;
use errors::*;
use models::*;
use repos::*;
use types::*;

use chrono::prelude::*;
use failure;
use futures::future;
use futures::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use stq_db::repo::*;

pub trait OrderService {
    fn convert_cart(
        &self,
        user_id: UserId,
        prices: HashMap<ProductId, ProductPrice>,
        address: AddressFull,
        receiver_name: String,
    ) -> ServiceFuture<Vec<Order>>;
    // fn create_order(&self) -> ServiceFuture<Order>;
    fn get_order(&self, id: OrderIdentifier) -> ServiceFuture<Option<Order>>;
    fn get_orders_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Order>>;
    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>>;
    fn delete_order(&self, id: OrderIdentifier) -> ServiceFuture<()>;
    // fn set_order_state(&self, order_id: OrderIdentifier, state: OrderState) -> ServiceFuture<Order>;
    /// Search using the terms provided.
    fn search(&self, terms: OrderSearchTerms) -> ServiceFuture<Vec<Order>>;
}

pub struct OrderServiceImpl {
    pub cart_service_factory: Rc<Fn(UserId) -> Box<CartService>>,
    pub order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    pub order_diff_repo_factory: Rc<Fn() -> Box<OrderDiffRepo>>,
    pub db_pool: DbPool,
    pub calling_user: UserId,
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(
        &self,
        customer_id: UserId,
        prices: HashMap<ProductId, ProductPrice>,
        address: AddressFull,
        receiver_name: String,
    ) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let order_diffs_repo_factory = self.order_diff_repo_factory.clone();
        let cart_service_factory = self.cart_service_factory.clone();
        let calling_user = self.calling_user;

        Box::new(
            (cart_service_factory)(calling_user)
                .get_cart(customer_id)
                // Create orders from cart items
                .and_then(move |cart: Cart| {
                    let mut order_items = Vec::new();
                    for (product_id, item) in cart.into_iter() {
                        if item.selected {
                            let price = prices.get(&product_id).cloned().ok_or(failure::Error::from(Error::MissingPrice))?;

                            order_items.push((OrderInserter {
                                id: OrderId::new(),
                                customer: customer_id,
                                store: item.store_id,
                                product: product_id,
                                quantity: item.quantity,
                                price,
                                address: address.clone(),
                                receiver_name: receiver_name.clone(),
                                state: OrderState::New,
                                delivery_company: None,
                                track_id: None,
                            }, item.comment))
                        }
                    }
                    Ok(order_items)
                })
                        // Insert new orders into database
                        .and_then({
                            let db_pool = self.db_pool.clone();
                            move |new_orders| {
                                db_pool.run(move |conn| {
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
                                                        timestamp: Utc::now().naive_utc(),
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
                                })
                            }
                        })
                        // Remove ordered products from cart
                        .and_then({
                            let cart_service_factory = cart_service_factory.clone();
                            move |out_data| {
                                let mut out: ServiceFuture<()> = Box::new(future::ok(()));
                                for order in &out_data {
                                        out = Box::new(out.and_then({
                                            let user_id = order.customer;
                                            let product_id = order.product;
                                            let cart_service_factory = cart_service_factory.clone();
                                            move |_| {
                                                (cart_service_factory)(calling_user).delete_item(user_id, product_id).map(|_| ())
                                            }
                                        }));
                                }
                                out.map(move |_| out_data)
                            }
                        }),
        )
    }

    fn get_order(&self, order_id: OrderIdentifier) -> ServiceFuture<Option<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| (order_repo_factory)().select(conn, OrderFilter::from(order_id)))
                .map(|orders| orders.first().cloned()),
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
                .and_then(move |filter| db_pool.run(move |conn| (order_repo_factory)().select(conn, filter))),
        )
    }

    /*
    fn set_order_state(&self, order_id: OrderId, state: OrderState) -> ServiceFuture<Order> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (order_repo_factory)()
                        .update(
                            Box::new(conn),
                            OrderUpdate {
                                mask: OrderMask {
                                    id: Some(order_id),
                                    ..Default::default()
                                },
                                data: OrderUpdateData { state: Some(state) },
                            },
                        )
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                })
                .and_then(|mut v| match v.pop() {
                    Some(order) => Ok(order),
                    None => Err(format_err!("Order not found")),
                }),
        )
    }
    */
}
