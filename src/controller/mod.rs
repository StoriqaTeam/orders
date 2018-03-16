use futures;
use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Post, Put, Request};
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
}

pub struct ControllerImpl {
    pub db_pool: DbPool,
    pub route_parser: Arc<RouteParser<Route>>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        let mut route_parser: RouteParser<Route> = Default::default();
        route_parser.add_route(r"^/cart/products", || Route::CartProducts);

        ControllerImpl {
            db_pool,
            route_parser: Arc::new(route_parser),
        }
    }
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let user_id = request.headers().get::<hyper::header::From>();
        match (request.method(), self.route_parser.test(request.path())) {
            (&Put, Some(Route::CartProducts(product_id))) => {
                serialize_future(parse_body::<models::SetProductParams>(request.body()).and_then({
                    let db_pool = self.db_pool.clone();
                    move |params| {
                        ProductsRepoImpl::new(db_pool.clone())
                            .set_item(user_id, product_id, params.quantity)
                            .map_err(|e| ControllerError::InternalServerError(e.into()))
                    }
                }))
            }
            (&Delete, Some(Route::Item)) => {
                serialize_future(parse_body::<models::DeleteItems>(request.body()).and_then({
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
