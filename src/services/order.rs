use futures::future;
use futures::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::sync::{Arc, Mutex};
use stq_db::repo::*;

use super::CartService;
use super::types::ServiceFuture;
use models::*;
use repos::*;
use types::*;

pub trait OrderService {
    fn convert_cart(&self, user_id: i32) -> ServiceFuture<HashMap<i32, Order>>;
    fn get_order(&self, order_id: OrderId) -> ServiceFuture<Option<Order>>;
    fn get_orders_for_user(&self, user_id: i32) -> ServiceFuture<Vec<Order>>;
    fn delete_order(&self, order_id: OrderId) -> ServiceFuture<()>;
    fn set_order_state(&self, order_id: OrderId, state: OrderState) -> ServiceFuture<Order>;
}

pub type CartServiceFactory = Arc<Fn() -> Box<CartService> + Send + Sync>;

pub struct OrderServiceImpl {
    pub cart_service_factory: CartServiceFactory,
    pub order_repo_factory: Arc<Fn() -> Box<OrderRepo + Send + Sync> + Send + Sync>,
    pub db_pool: DbPool,
}

struct OrderItem {
    product_id: i32,
    store_id: i32,
    quantity: i32,
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(&self, user_id: i32) -> ServiceFuture<HashMap<i32, Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let cart_service_factory = self.cart_service_factory.clone();

        Box::new(
            (cart_service_factory)()
                .get_cart(user_id)
                // Get store ID for each cart product
                .and_then(|cart: Cart| {
                    future::result(cart.into_iter().map(|(product_id, cart_item_info)| {
                        if cart_item_info.store_id < 0 {
                            return Err(format_err!("Invalid store ID for product {}: {}", product_id, cart_item_info.store_id)).into()
                        }

                        Ok(OrderItem {
                            product_id,
                            store_id: cart_item_info.store_id,
                            quantity: cart_item_info.quantity,
                        })
                    }).collect::<Result<Vec<_>, RepoError>>())
                })
                // Bin cart products into separate orders based on store ID
                .map(move |cart: Vec<OrderItem>| {
                            let mut orders_by_store = HashMap::<i32, NewOrder>::new();
                            for OrderItem {product_id, store_id, quantity} in cart {
                                match orders_by_store.entry(store_id) {
                                    Occupied(mut entry) => {
                                        entry.get_mut().products.insert(product_id, quantity);
                                    }
                                    Vacant(mut entry) => {
                                        let mut v = NewOrder {
                                            user_id,
                                            state: OrderState::Processing(Default::default()),
                                            products: Default::default(),
                                        };
                                        v.products.insert(product_id, quantity);

                                        entry.insert(v);
                                    }
                                }
                            }
                            orders_by_store
                        })
                        // Insert new orders into database
                        .and_then({
                            let db_pool = self.db_pool.clone();
                            move |new_orders_by_store| {
                                db_pool.run(move |conn| {
                                    let mut out: RepoConnectionFuture<HashMap<i32, Order>>;
                                    out = Box::new(future::ok((Default::default(), Box::new(conn) as RepoConnection)));

                                    for (store_id, new_order) in new_orders_by_store.into_iter() {
                                        out = Box::new(out.and_then({
                                            let order_repo_factory = order_repo_factory.clone();
                                            move |(mut out_data, conn)| {
                                                (order_repo_factory)().insert_exactly_one(conn, new_order).map({
                                                    move |(order, conn)| {
                                                        out_data.insert(store_id, order);
                                                        (out_data, conn)
                                                    }
                                                })
                                            }
                                        }));
                                    }

                                    out
                                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                                })
                            }
                        })
                        // Remove ordered products from cart
                        .and_then({
                            let cart_service_factory = cart_service_factory.clone();
                            move |out_data| {
                                let mut out: ServiceFuture<()> = Box::new(future::ok(()));
                                for (_store_id, order) in &out_data {
                                    for (product_id, _) in &order.products {
                                        out = Box::new(out.and_then({
                                            let user_id = user_id.clone();
                                            let product_id = product_id.clone();
                                            let cart_service_factory = cart_service_factory.clone();
                                            move |_| {
                                                (cart_service_factory)().delete_item(user_id, product_id).map(|_| ())
                                            }
                                        }));
                                    }
                                }
                                out.map(move |_| out_data)
                            }
                        }),
        )
    }

    fn get_order(&self, order_id: OrderId) -> ServiceFuture<Option<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (order_repo_factory)()
                        .select(
                            Box::new(conn),
                            OrderMask {
                                id: Some(order_id),
                                ..Default::default()
                            },
                        )
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                })
                .map(|orders| orders.first().cloned()),
        )
    }

    fn get_orders_for_user(&self, user_id: i32) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(self.db_pool.run(move |conn| {
            (order_repo_factory)()
                .select(
                    Box::new(conn),
                    OrderMask {
                        user_id: Some(user_id),
                        ..Default::default()
                    },
                )
                .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
        }))
    }

    fn delete_order(&self, order_id: OrderId) -> ServiceFuture<()> {
        let order_repo_factory = self.order_repo_factory.clone();
        Box::new(
            self.db_pool
                .run(move |conn| {
                    (order_repo_factory)()
                        .delete(
                            Box::new(conn),
                            OrderMask {
                                id: Some(order_id),
                                ..Default::default()
                            },
                        )
                        .map(|(v, conn)| (v, conn.unwrap_tokio_postgres()))
                        .map_err(|(e, conn)| (e, conn.unwrap_tokio_postgres()))
                })
                .map(|_| ()),
        )
    }

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
}

pub type OrderServiceMemoryStorage = Arc<Mutex<HashMap<i32, Order>>>;

/// In-memory implementation of cart service
pub struct OrderServiceMemory {
    pub inner: OrderServiceMemoryStorage,
    pub cart_factory: Arc<Fn() -> Box<CartService> + Send + Sync>,
}

impl OrderService for OrderServiceMemory {
    fn convert_cart(&self, _user_id: i32) -> ServiceFuture<HashMap<i32, Order>> {
        unimplemented!()
    }

    fn get_order(&self, _order_id: OrderId) -> ServiceFuture<Option<Order>> {
        unimplemented!()
    }

    fn get_orders_for_user(&self, _user_id: i32) -> ServiceFuture<Vec<Order>> {
        unimplemented!()
    }

    fn delete_order(&self, _order_id: OrderId) -> ServiceFuture<()> {
        unimplemented!()
    }

    fn set_order_state(&self, _order_id: OrderId, _state: OrderState) -> ServiceFuture<Order> {
        unimplemented!()
    }
}
