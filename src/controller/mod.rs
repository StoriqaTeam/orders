use futures;
use futures::future;
use futures::prelude::*;
use hyper::{Delete, Get, Post, Put};
use hyper::Request;
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
    Item,
}

pub struct ControllerImpl {
    pub db_pool: DbPool,
    pub route_parser: Arc<RouteParser<Route>>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        let mut route_parser: RouteParser<Route> = Default::default();
        route_parser.add_route(r"^/items", || Route::Item);

        ControllerImpl {
            db_pool,
            route_parser: Arc::new(route_parser),
        }
    }
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        match (request.method(), self.route_parser.test(request.path())) {
            (&Post, Some(Route::Item)) => {
                serialize_future(parse_body::<models::CartItem>(request.body()).and_then({
                    let db_pool = self.db_pool.clone();
                    move |cart_item| {
                        ProductsRepoImpl::new(db_pool.clone())
                            .add(cart_item)
                            .map_err(|e| ControllerError::InternalServerError(e.into()))
                    }
                }))
            }
            (&Delete, Some(Route::Item)) => {
                serialize_future(parse_body::<models::DeleteCart>(request.body()).and_then({
                    let db_pool = self.db_pool.clone();
                    move |delete_cart| {
                        ProductsRepoImpl::new(db_pool.clone())
                            .clear(delete_cart)
                            .map_err(|e| ControllerError::InternalServerError(e.into()))
                    }
                }))
            }
            // Fallback
            _ => Box::new(future::err(ControllerError::NotFound)),
        }
    }
}
