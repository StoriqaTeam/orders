use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Headers, Post, Put, Request};
use std::str::FromStr;
use std::sync::Arc;

use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::{parse_body, serialize_future, ControllerFuture};
use stq_router::RouteParser;

use errors::*;
use models;
use services::*;
pub mod routing;
use self::routing::*;
use types::*;

pub struct ServiceFactory {
    pub system_factory: Arc<Fn() -> Box<SystemService>>,
    pub cart_factory: Arc<Fn() -> Box<CartService>>,
}

pub struct ControllerImpl {
    pub route_parser: Arc<RouteParser<Route>>,
    pub service_factory: Arc<ServiceFactory>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        ControllerImpl {
            service_factory: Arc::new(ServiceFactory {
                system_factory: Arc::new(|| Box::new(SystemServiceImpl::default())),
                cart_factory: Arc::new(move || Box::new(CartServiceImpl::new(db_pool.clone()))),
            }),
            route_parser: Arc::new(routing::make_router()),
        }
    }
}

pub fn extract_user_id(headers: Headers) -> Box<Future<Item = i32, Error = ControllerError>> {
    Box::new(
        future::result(
            headers
                .get::<hyper::header::Authorization<String>>()
                .map(|auth| auth.0.clone())
                .ok_or_else(|| ControllerError::BadRequest(AuthorizationError::Missing.into()))
                .and_then(|string_id| {
                    i32::from_str(&string_id).map_err(|e| {
                        ControllerError::BadRequest(
                            AuthorizationError::Parse {
                                raw: string_id,
                                error: e.into(),
                            }.into(),
                        )
                    })
                }),
        ).inspect(|user_id| debug!("Extracted user_id: {}", user_id)),
    )
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let (method, uri, _, headers, payload) = request.deconstruct();

        let service_factory = self.service_factory.clone();
        let route_parser = self.route_parser.clone();

        let route = route_parser.test(uri.path());
        match (&method, route) {
            // GET /healthcheck
            (&Get, Some(Route::Healthcheck)) => {
                debug!("Received healthcheck request");
                serialize_future((service_factory.system_factory)().healthcheck())
            }
            _ => {
                Box::new(extract_user_id(headers).and_then(move |user_id| {
                    match (method, route) {
                        (Get, Some(Route::CartProducts)) => serialize_future({
                            debug!("Received request to get cart for user {}", user_id);
                            Box::new(
                                (service_factory.cart_factory)()
                                    .get_cart(user_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            )
                        }),
                        (Post, Some(Route::CartClear)) => serialize_future({
                            debug!("Received request to clear cart for user {}", user_id);
                            Box::new(
                                (service_factory.cart_factory)()
                                    .clear_cart(user_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            )
                        }),
                        (Delete, Some(Route::CartProduct { product_id })) => serialize_future({
                            debug!("Received request to delete product {} from user {}'s cart", product_id, user_id);
                            Box::new(
                                (service_factory.cart_factory)()
                                    .delete_item(user_id, product_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            )
                        }),
                        (Put, Some(Route::CartProduct { product_id })) => serialize_future(
                            parse_body::<models::UpsertCart>(payload)
                                .inspect(move |params| {
                                    debug!(
                                        "Received request to set product {} in user {}'s cart to quantity {}",
                                        product_id, user_id, params.quantity
                                    );
                                })
                                .and_then(move |params| {
                                    (service_factory.cart_factory)()
                                        .set_item(user_id, product_id, params.quantity)
                                        .map_err(|e| ControllerError::InternalServerError(e.into()))
                                }),
                        ),
                        // Fallback
                        _ => Box::new(future::err(ControllerError::NotFound)),
                    }
                }))
            }
        }
    }
}
