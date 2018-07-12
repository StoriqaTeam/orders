use std::rc::Rc;
use std::collections::HashMap;

use chrono::prelude::*;
use failure;
use futures::future;
use futures::prelude::*;

use super::types::ServiceFuture;
use super::CartService;
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
        user_id: UserId,
        prices: HashMap<ProductId, ProductPrice>,
        address: AddressFull,
        receiver_name: String,
    ) -> ServiceFuture<Vec<Order>>;
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
    pub cart_service_factory: Rc<Fn(UserId) -> Box<CartService>>,
    pub order_repo_factory: Rc<Fn() -> Box<OrderRepo>>,
    pub order_diff_repo_factory: Rc<Fn() -> Box<OrderDiffRepo>>,
    pub roles_repo_factory: Rc<Fn() -> Box<RolesRepo>>,
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
                                state: OrderState::PaymentAwaited,
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
                                                        committed_at: Utc::now(),
                                                        state: OrderState::PaymentAwaited,
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
