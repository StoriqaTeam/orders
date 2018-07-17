use chrono::prelude::*;
use failure;
use failure::Fail;
use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Headers, Post, Put, Request};
use std::rc::Rc;

use stq_http::controller::{Controller, ControllerFuture};
use stq_http::request_util::{parse_body, serialize_future};
use stq_router::RouteParser;
use stq_types::*;

use config::*;
use errors::*;
use models::*;
use repos::*;
use services::*;
pub mod routing;
use self::routing::*;
use types::*;

pub struct ServiceFactory {
    pub cart_factory: Rc<Fn(UserId) -> Box<CartService>>,
    pub order_factory: Rc<Fn(UserId) -> Box<OrderService>>,
}

pub struct ControllerImpl {
    route_parser: Rc<RouteParser<Route>>,
    service_factory: Rc<ServiceFactory>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool, _config: Config) -> Self {
        ControllerImpl {
            service_factory: Rc::new(ServiceFactory {
                order_factory: Rc::new({
                    let db_pool = db_pool.clone();
                    move |calling_user| {
                        Box::new(OrderServiceImpl {
                            calling_user,
                            db_pool: db_pool.clone(),
                            cart_repo_factory: Rc::new(|| Box::new(make_product_repo())),
                            order_diff_repo_factory: Rc::new(|| Box::new(make_order_diffs_repo())),
                            order_repo_factory: Rc::new(|| Box::new(make_order_repo())),
                            roles_repo_factory: Rc::new(|| Box::new(make_su_repo())),
                        })
                    }
                }),
                cart_factory: Rc::new({
                    let db_pool = db_pool.clone();
                    move |calling_user| Box::new(CartServiceImpl::new(calling_user, db_pool.clone())) as Box<CartService>
                }),
            }),
            route_parser: Rc::new(routing::make_router()),
        }
    }
}

pub fn extract_user_id(headers: Headers) -> Result<UserId, failure::Error> {
    let string_id: String = if let Some(auth) = headers.get::<hyper::header::Authorization<String>>() {
        auth.0.clone()
    } else if let Some(s) = headers.get::<hyper::header::Cookie>().and_then(|c| c.get("SESSION_ID")) {
        s.to_string()
    } else {
        return Err(format_err!("User ID not found in Authorization or Cookie headers")
            .context(Error::MissingUserId)
            .into());
    };

    let user_id = string_id.parse().map_err(|e| -> failure::Error {
        failure::Error::from(e)
            .context(format!("Failed to parse user ID: {}", string_id))
            .context(Error::UserIdParse)
            .into()
    })?;

    debug!("Extracted user_id: {}", user_id);

    Ok(user_id)
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let dt = Local::now();
        let (method, uri, _, headers, payload) = request.deconstruct();

        let service_factory = self.service_factory.clone();
        let route_parser = self.route_parser.clone();

        let route = route_parser.test(uri.path());
        Box::new(
            future::result(extract_user_id(headers))
                .map_err(|e| e.context("Failed to extract user ID").into())
                .and_then(move |calling_user| {
                    match (method, route) {
                        (Get, Some(Route::Cart)) => {
                            if let (Some(from), Some(count)) =
                                parse_query!(uri.query().unwrap_or_default(), "offset" => ProductId, "count" => i32)
                            {
                                debug!(
                                    "Received request for user {} to get {} products starting from {}",
                                    calling_user, count, from
                                );
                                serialize_future((service_factory.cart_factory)(calling_user).list(calling_user, from, count))
                            } else {
                                serialize_future::<String, _, _>(future::err(
                                    format_err!("Error parsing request from gateway body").context(Error::ParseError),
                                ))
                            }
                        }
                        (Get, Some(Route::CartProducts)) => serialize_future({
                            debug!("Received request to get cart for user {}", calling_user);
                            Box::new((service_factory.cart_factory)(calling_user).get_cart(calling_user))
                        }),
                        (Post, Some(Route::CartClear)) => serialize_future({
                            debug!("Received request to clear cart for user {}", calling_user);
                            Box::new((service_factory.cart_factory)(calling_user).clear_cart(calling_user))
                        }),
                        (Delete, Some(Route::CartProduct { product_id })) => serialize_future({
                            debug!(
                                "Received request to delete product {} from user {}'s cart",
                                product_id, calling_user
                            );
                            Box::new((service_factory.cart_factory)(calling_user).delete_item(calling_user, product_id))
                        }),
                        (Put, Some(Route::CartProductQuantity { product_id })) => serialize_future(
                            parse_body::<CartProductQuantityPayload>(payload)
                                .inspect(move |params| {
                                    debug!(
                                        "Received request to set product {} in user {}'s cart to quantity {}",
                                        product_id, calling_user, params.value
                                    );
                                })
                                .and_then(move |params| {
                                    (service_factory.cart_factory)(calling_user).set_quantity(calling_user, product_id, params.value)
                                }),
                        ),
                        (Put, Some(Route::CartProductSelection { product_id })) => serialize_future(
                            parse_body::<CartProductSelectionPayload>(payload)
                                .inspect(move |params| {
                                    debug!(
                                        "Received request to set product {}'s selection in user {}'s cart to {}",
                                        product_id, calling_user, params.value
                                    )
                                })
                                .and_then(move |params| {
                                    (service_factory.cart_factory)(calling_user).set_selection(calling_user, product_id, params.value)
                                }),
                        ),
                        (Put, Some(Route::CartProductComment { product_id })) => serialize_future(
                            parse_body::<CartProductCommentPayload>(payload)
                                .inspect(move |comment_payload| {
                                    debug!(
                                        "Received request to set product {}'s comment in user {}'s cart to {}",
                                        product_id, calling_user, comment_payload.value
                                    )
                                })
                                .and_then(move |comment_payload| {
                                    (service_factory.cart_factory)(calling_user).set_comment(
                                        calling_user,
                                        product_id,
                                        comment_payload.value,
                                    )
                                }),
                        ),
                        (Post, Some(Route::CartIncrementProduct { product_id })) => serialize_future({
                            parse_body::<CartProductIncrementPayload>(payload).and_then(move |data| {
                                debug!(
                                    "Received request to increment product {} quantity for user {}",
                                    product_id, calling_user
                                );
                                (service_factory.cart_factory)(calling_user).increment_item(calling_user, product_id, data.store_id)
                            })
                        }),
                        (Post, Some(Route::CartMerge)) => serialize_future({
                            parse_body::<CartMergePayload>(payload).and_then(move |data| {
                                let user_to = calling_user;
                                debug!("Received request to merge cart from user {} to user {}", data.user_from, user_to);
                                (service_factory.cart_factory)(calling_user).merge(data.user_from, user_to)
                            })
                        }),
                        (Get, Some(Route::Orders)) => serialize_future({
                            debug!("Received request to get orders for user {}", calling_user);
                            Box::new((service_factory.order_factory)(calling_user).get_orders_for_user(calling_user))
                        }),
                        (Get, Some(Route::Order { order_id })) => serialize_future({
                            debug!("Received request to get order {:?}", order_id);
                            Box::new((service_factory.order_factory)(calling_user).get_order(order_id))
                        }),
                        (Get, Some(Route::OrderDiff { order_id })) => serialize_future({
                            debug!("Received request to get order diff {:?}", order_id);
                            Box::new((service_factory.order_factory)(calling_user).get_order_diff(order_id))
                        }),
                        (Put, Some(Route::OrderStatus { order_id })) => serialize_future({
                            parse_body::<UpdateStatePayload>(payload).and_then(move |data| {
                                let user_to = calling_user;
                                debug!(
                                    "Received request to set order {:?} status {:?} for user {} ",
                                    order_id, data.state, user_to
                                );
                                (service_factory.order_factory)(calling_user).set_order_state(
                                    order_id,
                                    data.state,
                                    data.comment,
                                    data.track_id,
                                )
                            })
                        }),
                        (Post, Some(Route::OrderSearch)) => serialize_future({
                            parse_body::<OrderSearchTerms>(payload)
                                .and_then(move |terms| Box::new((service_factory.order_factory)(calling_user).search(terms)))
                        }),
                        (Post, Some(Route::OrderFromCart)) => serialize_future({
                            debug!("Received request to convert cart into orders for user {}", calling_user);
                            parse_body::<ConvertCartPayload>(payload).and_then(move |payload| {
                                Box::new((service_factory.order_factory)(calling_user).convert_cart(
                                    payload.conversion_id,
                                    payload.customer_id,
                                    payload.prices,
                                    payload.address,
                                    payload.receiver_name,
                                ))
                            })
                        }),
                        (Post, Some(Route::OrderFromCartRevert)) => serialize_future({
                            parse_body::<ConvertCartRevertPayload>(payload).and_then(move |payload| {
                                Box::new((service_factory.order_factory)(calling_user).revert_cart_conversion(payload.conversion_id))
                            })
                        }),
                        (Delete, Some(Route::Order { order_id })) => serialize_future({
                            debug!("Received request to delete order {:?}", order_id);
                            Box::new((service_factory.order_factory)(calling_user).delete_order(order_id))
                        }),
                        (Get, Some(Route::RolesByUserId { user_id })) => {
                            debug!("Received request to get roles by user id {}", user_id);
                            serialize_future({ (service_factory.order_factory)(calling_user).get_roles_for_user(user_id) })
                        }
                        (Post, Some(Route::Roles)) => serialize_future({
                            parse_body::<Role>(payload).and_then(move |data| {
                                debug!("Received request to create role {:?}", data);
                                (service_factory.order_factory)(calling_user).create_role(data)
                            })
                        }),
                        (Delete, Some(Route::RolesByUserId { user_id })) => serialize_future({
                            parse_body::<Option<UserRole>>(payload).and_then(move |role| {
                                debug!("Received request to delete role {:?}", role);
                                (service_factory.order_factory)(calling_user).remove_role(RoleRemoveFilter::Meta((user_id, role)))
                            })
                        }),
                        (Delete, Some(Route::RoleById { role_id })) => {
                            debug!("Received request to delete role by id {}", role_id);
                            serialize_future({ (service_factory.order_factory)(calling_user).remove_role(RoleRemoveFilter::Id(role_id)) })
                        }
                        // Fallback
                        _ => Box::new(future::err(Error::InvalidRoute.into())),
                    }
                })
                .then(move |res| {
                    let d = Local::now() - dt;
                    info!(
                        "Response error = {:?}, elapsed time = {}.{:03}",
                        res.as_ref().err(),
                        d.num_seconds(),
                        d.num_milliseconds()
                    );
                    res
                }),
        )
    }
}
