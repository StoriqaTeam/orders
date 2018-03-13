use futures;
use hyper::Request;
use std::sync::Arc;

use stq_http;
use stq_router;
use stq_router::RouteParser;
use stq_http::controller::Controller;
use stq_http::request_util::{serialize_future, ControllerFuture};

use types;
use types::*;

#[derive(Clone, Copy, Debug)]
pub enum Route {
    Item,
}

#[derive(Debug)]
pub struct ControllerImpl {
    pub db_pool: DbPool,
    pub route_parser: Arc<RouteParser<Route>>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        let mut route_parser = Default::default();
        route_parser.add_route(r"^/items", || Route::Item);

        ControllerImpl {
            db_pool,
            route_parser,
        }
    }
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        match (request.method(), self.route_parser.test(request.path())) {
            (&Post, Some(Route::Item)) => {
                self.db_pool.run(|conn| {
                    serialize_future(futures::future::ok(()))
                })
            }
        }
    }
}
