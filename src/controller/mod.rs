use futures::future;
use futures::prelude::*;
use hyper;
use hyper::{Delete, Get, Headers, Post, Put, Request};
use std::str::FromStr;
use std::sync::Arc;

use stq_http::client::ClientHandle as HttpClientHandle;
use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::{parse_body, serialize_future, ControllerFuture};
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
    pub cart_factory: Arc<Fn() -> Box<CartService>>,
    pub order_factory: Arc<Fn() -> Box<OrderService>>,
}

pub struct ControllerImpl {
    route_parser: Arc<RouteParser<Route>>,
    service_factory: Arc<ServiceFactory>,
}

impl ControllerImpl {
    pub fn new(db_pool: DbPool, http_client: HttpClientHandle, config: Config) -> Self {
        let cart_factory = Arc::new({
            let db_pool = db_pool.clone();
            move || Box::new(CartServiceImpl::new(db_pool.clone())) as Box<CartService>
        });
        ControllerImpl {
            service_factory: Arc::new(ServiceFactory {
                order_factory: Arc::new({
                    let cart_factory = cart_factory.clone();
                    let http_client = http_client.clone();
                    let config = config.clone();
                    move || {
                        Box::new(OrderServiceImpl {
                            db_pool: db_pool.clone(),
                            cart_service_factory: cart_factory.clone(),
                            order_repo_factory: Arc::new(|| Box::new(make_order_repo())),
                            product_info_source: Arc::new({
                                let http_client = http_client.clone();
                                let config = config.clone();
                                move || {
                                    Box::new(ProductInfoHttpRepoImpl::new(
                                        http_client.clone(),
                                        config.services.stores.clone(),
                                    ))
                                }
                            }),
                        })
                    }
                }),
                cart_factory,
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
                .ok_or_else(|| ControllerError::Forbidden(AuthorizationError::Missing.into()))
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
        Box::new(extract_user_id(headers).and_then(move |user_id| {
            match (method, route) {
                (Get, Some(Route::Cart)) => {
                    if let (Some(from), Some(count)) = parse_query!(uri.query().unwrap_or_default(), "offset" => i32, "count" => i64) {
                        debug!(
                            "Received request for user {} to get {} products starting from {}",
                            user_id, count, from
                        );
                        serialize_future(
                            (service_factory.cart_factory)()
                                .list(user_id, from, count)
                                .map_err(ControllerError::InternalServerError),
                        )
                    } else {
                        serialize_future::<String, _, _>(future::err(ControllerError::UnprocessableEntity(
                            format_err!("Error parsing request from gateway body"),
                        )))
                    }
                }
                (Get, Some(Route::CartProducts)) => serialize_future({
                    debug!("Received request to get cart for user {}", user_id);
                    Box::new(
                        (service_factory.cart_factory)()
                            .get_cart(user_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                (Post, Some(Route::CartClear)) => serialize_future({
                    debug!("Received request to clear cart for user {}", user_id);
                    Box::new(
                        (service_factory.cart_factory)()
                            .clear_cart(user_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                (Delete, Some(Route::CartProduct { product_id })) => serialize_future({
                    debug!(
                        "Received request to delete product {} from user {}'s cart",
                        product_id, user_id
                    );
                    Box::new(
                        (service_factory.cart_factory)()
                            .delete_item(user_id, product_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                (Put, Some(Route::CartProductQuantity { product_id })) => serialize_future(
                    parse_body::<CartProductQuantityPayload>(payload)
                        .inspect(move |params| {
                            debug!(
                                "Received request to set product {} in user {}'s cart to quantity {}",
                                product_id, user_id, params.value
                            );
                        })
                        .and_then(move |params| {
                            (service_factory.cart_factory)()
                                .set_quantity(user_id, product_id, params.value)
                                .map_err(ControllerError::InternalServerError)
                        }),
                ),
                (Put, Some(Route::CartProductSelection { product_id })) => serialize_future(
                    parse_body::<CartProductSelectionPayload>(payload)
                        .inspect(move |params| {
                            debug!(
                                "Received request to set product {}'s selection in user {}'s cart to {}",
                                product_id, user_id, params.value
                            )
                        })
                        .and_then(move |params| {
                            (service_factory.cart_factory)()
                                .set_selection(user_id, product_id, params.value)
                                .map_err(ControllerError::InternalServerError)
                        }),
                ),
                (Post, Some(Route::CartIncrementProduct { product_id })) => serialize_future({
                    parse_body::<CartProductIncrementPayload>(payload).and_then(move |data| {
                        debug!(
                            "Received request to increment product {} quantity for user {}",
                            product_id, user_id
                        );
                        (service_factory.cart_factory)()
                            .increment_item(user_id, product_id, data.store_id)
                            .map_err(ControllerError::InternalServerError)
                    })
                }),
                (Post, Some(Route::CartMerge)) => serialize_future({
                    parse_body::<CartMergePayload>(payload).and_then(move |data| {
                        let user_to = user_id;
                        debug!(
                            "Received request to merge cart from user {} to user {}",
                            data.user_from, user_to
                        );
                        (service_factory.cart_factory)()
                            .merge(data.user_from, user_to)
                            .map_err(ControllerError::InternalServerError)
                    })
                }),
                (Get, Some(Route::Orders)) => serialize_future({
                    debug!("Received request to get orders for user {}", user_id);
                    Box::new(
                        (service_factory.order_factory)()
                            .get_orders_for_user(user_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                (Post, Some(Route::OrderFromCart)) => serialize_future({
                    debug!(
                        "Received request to convert cart into orders for user {}",
                        user_id
                    );
                    Box::new(
                        (service_factory.order_factory)()
                            .convert_cart(user_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                (Put, Some(Route::OrderStatus { order_id })) => serialize_future({
                    debug!("Received request to set order status");
                    parse_body::<OrderState>(payload).and_then(move |order_status| {
                        Box::new(
                            (service_factory.order_factory)()
                                .set_order_state(order_id, order_status)
                                .map_err(ControllerError::InternalServerError),
                        )
                    })
                }),
                (Delete, Some(Route::Order { order_id })) => serialize_future({
                    debug!("Received request to delete order {}", order_id);
                    Box::new(
                        (service_factory.order_factory)()
                            .delete_order(order_id)
                            .map_err(ControllerError::InternalServerError),
                    )
                }),
                // Fallback
                _ => Box::new(future::err(ControllerError::NotFound)),
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::Authorization;
    use hyper::{Method, Uri};
    use serde_json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn make_test_controller(cart_storage: CartServiceMemoryStorage) -> ControllerImpl {
        let cart_factory = Arc::new(move || {
            Box::new(CartServiceMemory {
                inner: cart_storage.clone(),
            }) as Box<CartService>
        });
        ControllerImpl {
            route_parser: Arc::new(routing::make_router()),
            service_factory: Arc::new(ServiceFactory {
                order_factory: Arc::new({
                    let cart_factory = cart_factory.clone();
                    move || {
                        Box::new(OrderServiceMemory {
                            inner: Default::default(),
                            cart_factory: cart_factory.clone(),
                        })
                    }
                }),
                cart_factory,
            }),
        }
    }

    fn run_controller_op(data: CartServiceMemoryStorage, req: Request) -> ControllerFuture {
        make_test_controller(data).call(req)
    }

    #[test]
    fn test_missing_auth_header() {
        let req = Request::new(Method::Get, Uri::default());

        match run_controller_op(Default::default(), req).wait() {
            Ok(v) => panic!("Expected error, received {}", v),
            Err(e) => match e {
                ControllerError::Forbidden(e) => {
                    let e = e.downcast().unwrap();
                    match e {
                        AuthorizationError::Missing => {
                            return;
                        }
                        _ => panic!("Invalid error: {}", e),
                    }
                }
                _ => panic!("Invalid error: {}", e),
            },
        };
    }

    #[test]
    fn test_invalid_auth_header() {
        let mut req = Request::new(Method::Get, Uri::default());
        req.headers_mut()
            .set::<Authorization<String>>(Authorization("12345abc".into()));

        let data = Default::default();

        let result = run_controller_op(data, req).wait();

        match result {
            Ok(v) => panic!("Expected error, received {}", v),
            Err(e) => match e {
                ControllerError::BadRequest(e) => {
                    let e = e.downcast().unwrap();
                    match e {
                        AuthorizationError::Parse { .. } => {
                            return;
                        }
                        _ => panic!("Invalid error: {}", e),
                    }
                }
                _ => panic!("Invalid error: {}", e),
            },
        }
    }

    #[test]
    fn test_get_cart() {
        let user_id = 12345;
        let store_id = 1337;
        let cart = hashmap!{555 => CartItemInfo {
            quantity: 9000,
            selected: true,
            store_id,
        }};
        let storage = hashmap!{user_id => cart.clone()};

        let mut req = Request::new(Method::Get, Uri::from_str("/cart/products").unwrap());
        req.headers_mut()
            .set::<Authorization<String>>(Authorization(user_id.to_string()));

        let data = Arc::new(Mutex::new(storage));

        let expectation = serde_json::to_string(&cart).unwrap();
        let result = run_controller_op(data, req).wait().unwrap();

        assert_eq!(expectation, result);
    }

    #[test]
    fn test_set_cart_quantity_nopayload() {
        let user_id = 12345;
        let product_id = 555;

        let mut req = Request::new(
            Method::Put,
            Uri::from_str(&format!("/cart/products/{}/quantity", product_id)).unwrap(),
        );
        req.headers_mut()
            .set::<Authorization<String>>(Authorization(user_id.to_string()));

        let data = Default::default();

        let result = run_controller_op(data, req).wait();

        match result {
            Ok(v) => panic!("Expected error, received {}", v),
            Err(e) => match e {
                ControllerError::UnprocessableEntity(e) => {
                    e.downcast::<serde_json::Error>().unwrap();
                }
                _ => panic!("Invalid error: {}", e),
            },
        }
    }

    #[test]
    fn test_set_quantity() {
        let user_id = 12345;
        let product_id = 555;
        let quantity = 9000;
        let store_id = 1337;
        let payload = CartProductQuantityPayload { value: quantity };

        let expected_reply = CartItem {
            product_id,
            quantity,
            selected: true,
            store_id,
        };
        let expected_storage: HashMap<UserId, Cart> = hashmap!{
            user_id => hashmap! {
                product_id => CartItemInfo {
                    quantity,
                    selected: true,
                    store_id,
                }
            },
        };

        let mut req = Request::new(
            Method::Put,
            Uri::from_str(&format!("/cart/products/{}/quantity", product_id)).unwrap(),
        );
        req.headers_mut()
            .set::<Authorization<String>>(Authorization(user_id.to_string()));
        req.set_body(serde_json::to_string(&payload).unwrap());

        let data = Arc::new(Mutex::new(hashmap! {
            user_id => hashmap! {
                product_id => CartItemInfo {
                    quantity: 1,
                    selected: true,
                    store_id,
                },
            },
        }));

        let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

        assert_eq!(*data.lock().unwrap(), expected_storage);
        assert_eq!(
            serde_json::from_str::<CartItem>(&resp).unwrap(),
            expected_reply
        );
    }

    #[test]
    fn test_delete_item() {
        let user_id = 12345;
        let store_id = 1337;

        let product_id_keep = 444;
        let quantity_keep = 9000;
        let product_id_remove = 555;
        let quantity_remove = 9100;
        let cart: Cart = hashmap!{
            product_id_keep => CartItemInfo {
                quantity: quantity_keep,
                selected: true,
                store_id,
            },
            product_id_remove => CartItemInfo {
                quantity: quantity_remove,
                selected: true,
                store_id,
            },
        };
        let storage = hashmap!{
            user_id => cart.clone(),
        };

        let expected_reply = CartItem {
            product_id: product_id_remove,
            quantity: quantity_remove,
            selected: true,
            store_id,
        };
        let expected_storage = hashmap! {
            user_id => hashmap! {
                product_id_keep => CartItemInfo {
                    quantity: quantity_keep,
                    selected: true,
                    store_id,
                }
            }
        };

        let mut req = Request::new(
            Method::Delete,
            Uri::from_str(&format!("/cart/products/{}", product_id_remove)).unwrap(),
        );
        req.headers_mut()
            .set::<Authorization<String>>(Authorization(user_id.to_string()));

        let data = Arc::new(Mutex::new(storage));

        let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

        assert_eq!(*data.lock().unwrap(), expected_storage);
        assert_eq!(
            serde_json::from_str::<CartItem>(&resp).unwrap(),
            expected_reply
        );
    }

    #[test]
    fn test_clear_cart() {
        let user_id = 12345;
        let store_id = 1337;

        let data = Arc::new(Mutex::new(hashmap!{
            user_id => hashmap! {
                444 => CartItemInfo {
                    quantity: 9000,
                    selected: true,
                    store_id,
                },
                555 => CartItemInfo {
                    quantity: 9010,
                    selected: true,
                    store_id,
                },
                666 => CartItemInfo {
                    quantity: 9020,
                    selected: true,
                    store_id,
                },
            },
        }));
        let expected_cart = Cart::default();
        let expected_storage = hashmap!{
            user_id => expected_cart.clone(),
        };

        let mut req = Request::new(Method::Post, Uri::from_str("/cart/clear").unwrap());
        req.headers_mut()
            .set::<Authorization<String>>(Authorization(user_id.to_string()));

        let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

        assert_eq!(*data.lock().unwrap(), expected_storage);
        assert_eq!(serde_json::from_str::<Cart>(&resp).unwrap(), expected_cart);
    }
}
