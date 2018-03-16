use futures;
use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Post, Put, Request};
use std::str::FromStr;
use std::sync::Arc;

use stq_http;
use stq_router;
use stq_router::RouteParser;
use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::{parse_body, serialize_future, ControllerFuture};

use errors;
use errors::*;
use models;
use repos::*;
use repos::ProductsRepo;
use types::*;

#[derive(Clone, Copy, Debug)]
pub enum Route {
    CartProducts,
    CartProduct { product_id: i64 },
}

pub struct ControllerImpl {
    pub db_pool: DbPool,
    pub route_parser: Arc<RouteParser<Route>>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        let mut route_parser: RouteParser<Route> = Default::default();
        route_parser.add_route(r"^/cart/products", || Route::CartProducts);
        route_parser.add_route_with_params(r"^/cart/products/(\d+)$", |params| {
            params
                .get(0)
                .and_then(|string_id| string_id.parse::<i64>().ok())
                .map(|product_id| Route::CartProduct { product_id })
        });

        ControllerImpl {
            db_pool,
            route_parser: Arc::new(route_parser),
        }
    }
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        Box::new(
            future::result(
                request
                    .headers()
                    .get::<hyper::header::Authorization<String>>()
                    .map(|auth| auth.0.clone())
                    .ok_or_else(|| ControllerError::BadRequest(format_err!("Missing user_id")))
                    .and_then(|string_id| {
                        i64::from_str(&string_id).map_err(|e| {
                            ControllerError::BadRequest(format_err!(
                                "Failed to parse user_id: {}",
                                e
                            ))
                        })
                    }),
            ).and_then({
                let db_pool = self.db_pool.clone();
                let route_parser = self.route_parser.clone();
                move |user_id| {
                    match (request.method(), route_parser.test(request.path())) {
                        (&Get, Some(Route::CartProducts)) => serialize_future(
                            ProductsRepoImpl::new(db_pool.clone())
                                .get_cart(user_id)
                                .map_err(|e| ControllerError::InternalServerError(e.into())),
                        ),
                        (&Post, Some(Route::CartProducts)) => serialize_future(
                            ProductsRepoImpl::new(db_pool.clone())
                                .clear_cart(user_id)
                                .map_err(|e| ControllerError::InternalServerError(e.into())),
                        ),
                        (&Delete, Some(Route::CartProduct { product_id })) => serialize_future(
                            ProductsRepoImpl::new(db_pool.clone())
                                .delete_item(user_id, product_id)
                                .map_err(|e| ControllerError::InternalServerError(e.into())),
                        ),
                        (&Put, Some(Route::CartProduct { product_id })) => serialize_future(
                            parse_body::<models::SetProductParams>(request.body()).and_then(
                                move |params| {
                                    ProductsRepoImpl::new(db_pool.clone())
                                        .set_item(user_id, product_id, params.quantity)
                                        .map_err(|e| ControllerError::InternalServerError(e.into()))
                                },
                            ),
                        ),
                        // Fallback
                        _ => Box::new(future::err(ControllerError::NotFound)),
                    }
                }
            }),
        )
    }
}
