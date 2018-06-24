use super::types::ServiceFuture;
use super::CartService;
use models::*;
use repos::*;
use types::*;

use futures::future;
use futures::prelude::*;
use std::rc::Rc;
use stq_db::repo::*;

pub trait OrderService {
    fn convert_cart(&self, user_id: UserId, address: AddressFull, receiver_name: String, comment: String) -> ServiceFuture<Vec<Order>>;
    // fn create_order(&self) -> ServiceFuture<Order>;
    fn get_order(&self, id: OrderIdentifier) -> ServiceFuture<Option<Order>>;
    fn get_orders_for_user(&self, user_id: UserId) -> ServiceFuture<Vec<Order>>;
    fn get_orders_for_store(&self, store_id: StoreId) -> ServiceFuture<Vec<Order>>;
    fn delete_order(&self, id: OrderIdentifier) -> ServiceFuture<()>;
    // fn set_order_state(&self, order_id: OrderIdentifier, state: OrderState) -> ServiceFuture<Order>;
    fn search(&self, filter: OrderSearchFilter) -> ServiceFuture<Vec<Order>>;
}

pub struct OrderServiceImpl {
    pub cart_service_factory: Rc<Fn() -> Box<CartService>>,
    pub order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    pub db_pool: DbPool,
}

struct OrderItem {
    product_id: ProductId,
    store_id: StoreId,
    quantity: Quantity,
}

impl OrderService for OrderServiceImpl {
    fn convert_cart(&self, customer_id: UserId, address: AddressFull, receiver_name: String, comment: String) -> ServiceFuture<Vec<Order>> {
        let order_repo_factory = self.order_repo_factory.clone();
        let cart_service_factory = self.cart_service_factory.clone();

        Box::new(
            (cart_service_factory)()
                .get_cart(customer_id)
                // Create orders from cart items
                .map(move |cart: Cart| {
                    cart.into_iter().filter(|(_, cart_item_info)| cart_item_info.selected).map(|(product_id, item)| {
                        OrderInserter {
                            id: OrderId::new(),
                            customer: customer_id,
                            store: item.store_id,
                            product: product_id,
                            address: address.clone(),
                            receiver_name: receiver_name.clone(),
                            state: OrderState::New(NewData {comment: comment.clone()}),
                            track_id: None,
                        }
                    }).collect::<Vec<_>>()
                })
                        // Insert new orders into database
                        .and_then({
                            let db_pool = self.db_pool.clone();
                            move |new_orders| {
                                db_pool.run(move |conn| {
                                    let mut out: RepoConnectionFuture<Vec<Order>>;
                                    out = Box::new(future::ok((Default::default(), conn)));

                                    for new_order in new_orders.into_iter() {
                                        out = Box::new(out.and_then({
                                            let order_repo_factory = order_repo_factory.clone();
                                            move |(mut out_data, conn)| {
                                                (order_repo_factory)().insert_exactly_one(conn, new_order).map({
                                                    move |(order, conn)| {
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
                                                (cart_service_factory)().delete_item(user_id, product_id).map(|_| ())
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
                    store: Some(store_id),
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
                    customer: Some(customer),
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

    fn search(&self, filter: OrderSearchFilter) -> ServiceFuture<Vec<Order>> {
        unimplemented!()
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
