use chrono::prelude::*;
use failure;
use failure::Fail;
use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Headers, Post, Put, Request};
use std::rc::Rc;
use std::str::FromStr;

use stq_http::controller::{Controller, ControllerFuture};
use stq_http::request_util::{parse_body, serialize_future};
use stq_router::RouteParser;

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
        let cart_factory = Rc::new({
            let db_pool = db_pool.clone();
            move |calling_user| Box::new(CartServiceImpl::new(calling_user,db_pool.clone())) as Box<CartService>
        });
        ControllerImpl {
            service_factory: Rc::new(ServiceFactory {
                order_factory: Rc::new({
                    let cart_factory = cart_factory.clone();
                    move |calling_user| {
                        Box::new(OrderServiceImpl {
                            calling_user,
                            db_pool: db_pool.clone(),
                            cart_service_factory: cart_factory.clone(),
                            order_diff_repo_factory: Rc::new(|| Box::new(make_order_diffs_repo())),
                            order_repo_factory: Rc::new(|| Box::new(make_order_repo())),
                        })
                    }
                }),
                cart_factory,
            }),
            route_parser: Rc::new(routing::make_router()),
        }
    }
}

pub fn extract_user_id(headers: Headers) -> Box<Future<Item = UserId, Error = failure::Error>> {
    Box::new(
        future::result(
            headers
                .get::<hyper::header::Authorization<String>>()
                .map(|auth| auth.0.clone())
                .or(headers
                    .get::<hyper::header::Cookie>()
                    .and_then(|c| c.get("SESSION_ID").map(|v| v.to_string())))
                .ok_or_else(|| {
                    Error::MissingUserId
                        .context("User ID not found in Authorization or Cookie headers")
                        .into()
                })
                .and_then(|string_id| {
                    FromStr::from_str(&string_id).map(UserId).map_err(|e| {
                        e.context(format!("Failed to parse user ID: {}", string_id))
                            .context(Error::UserIdParse)
                            .into()
                    })
                }),
        ).inspect(|user_id| debug!("Extracted user_id: {}", user_id.0)),
    )
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let dt = Local::now();
        let (method, uri, _, headers, payload) = request.deconstruct();

        let service_factory = self.service_factory.clone();
        let route_parser = self.route_parser.clone();

        let route = route_parser.test(uri.path());
        Box::new(
            extract_user_id(headers)
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
                        (Post, Some(Route::OrderFromCart)) => serialize_future({
                            debug!("Received request to convert cart into orders for user {}", calling_user);
                            parse_body::<ConvertCartPayload>(payload).and_then(move |payload| {
                                Box::new((service_factory.order_factory)(calling_user).convert_cart(
                                    calling_user,
                                    payload.prices,
                                    payload.address,
                                    payload.receiver_name,
                                    payload.comments,
                                ))
                            })
                        }),
                        (Delete, Some(Route::Order { order_id })) => serialize_future({
                            debug!("Received request to delete order {:?}", order_id);
                            Box::new((service_factory.order_factory)(calling_user).delete_order(order_id))
                        }),
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
