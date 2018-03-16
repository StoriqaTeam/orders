use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Post, Put, Request};
use std::str::FromStr;
use std::sync::Arc;

use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::{parse_body, serialize_future, ControllerFuture};
use stq_router::RouteParser;

use errors::*;
use models;
use repos::ProductsRepo;
use repos::*;
pub mod routing;
use self::routing::*;
use types::*;

pub struct ControllerImpl {
    pub route_parser: Arc<RouteParser<Route>>,
    pub repo_factory: Arc<Fn() -> Box<ProductsRepo>>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool) -> Self {
        ControllerImpl {
            repo_factory: Arc::new(move || Box::new(ProductsRepoImpl::new(db_pool.clone()))),
            route_parser: Arc::new(routing::make_router()),
        }
    }
}
impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let (method, uri, _, headers, payload) = request.deconstruct();
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
            ).inspect(|user_id| debug!("Extracted user_id: {}", user_id))
                .and_then({
                    let repo_factory = self.repo_factory.clone();
                    let route_parser = self.route_parser.clone();
                    move |user_id| {
                        match (method, route_parser.test(uri.path())) {
                            (Get, Some(Route::CartProducts)) => serialize_future(
                                (repo_factory)()
                                    .get_cart(user_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            ),
                            (Post, Some(Route::CartClear)) => serialize_future(
                                (repo_factory)()
                                    .clear_cart(user_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            ),
                            (Delete, Some(Route::CartProduct { product_id })) => serialize_future(
                                (repo_factory)()
                                    .delete_item(user_id, product_id)
                                    .map_err(|e| ControllerError::InternalServerError(e.into())),
                            ),
                            (Put, Some(Route::CartProduct { product_id })) => {
                                serialize_future(parse_body::<models::UpsertCart>(payload).and_then(move |params| {
                                    (repo_factory)()
                                        .set_item(user_id, product_id, params.quantity)
                                        .map_err(|e| ControllerError::InternalServerError(e.into()))
                                }))
                            }
                            // Fallback
                            _ => Box::new(future::err(ControllerError::NotFound)),
                        }
                    }
                }),
        )
    }
}
