extern crate futures;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate orders_lib as lib;
#[macro_use]
extern crate serde_json;
extern crate stq_http;

use futures::future;
use futures::prelude::*;
use hyper::header::Authorization;
use hyper::{Method, Request, Uri};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use stq_http::controller::*;
use stq_http::errors::*;
use stq_http::request_util::*;

use lib::controller::*;
use lib::errors::*;
use lib::models::*;
use lib::repos::*;
use lib::services::*;

pub type CartServiceMemoryStorage = Arc<Mutex<HashMap<i32, Cart>>>;

pub struct CartServiceMemory {
    pub inner: CartServiceMemoryStorage,
}

impl CartService for CartServiceMemory {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        Box::new(future::ok(cart.clone()))
    }

    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        cart.products.insert(product_id, quantity);

        Box::new(future::ok(cart.clone()))
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        cart.products.remove(&product_id);

        Box::new(future::ok(cart.clone()))
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        let mut inner = self.inner.lock().unwrap();
        let cart = inner.entry(user_id).or_insert(Cart::default());

        std::mem::swap(cart, &mut Cart::default());

        Box::new(future::ok(cart.clone()))
    }
}

fn make_test_controller(inner: CartServiceMemoryStorage) -> ControllerImpl {
    ControllerImpl {
        route_parser: Arc::new(routing::make_router()),
        service_factory: Arc::new(move || Box::new(CartServiceMemory { inner: inner.clone() })),
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
            ControllerError::BadRequest(e) => {
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
    req.headers_mut().set::<Authorization<String>>(Authorization("12345abc".into()));

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
    let cart = Cart {
        products: hashmap!{555 => 9000},
    };
    let storage = hashmap!{user_id => cart.clone()};

    let mut req = Request::new(Method::Get, Uri::from_str("/cart/products").unwrap());
    req.headers_mut().set::<Authorization<String>>(Authorization(user_id.to_string()));

    let data = Arc::new(Mutex::new(storage));

    let expectation = serde_json::to_string(&cart).unwrap();
    let result = run_controller_op(data, req).wait().unwrap();

    assert_eq!(expectation, result);
}

#[test]
fn test_set_cart_nopayload() {
    let user_id = 12345;
    let product_id = 555;

    let mut req = Request::new(Method::Put, Uri::from_str(&format!("/cart/products/{}", product_id)).unwrap());
    req.headers_mut().set::<Authorization<String>>(Authorization(user_id.to_string()));

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
fn test_set_cart() {
    let user_id = 12345;
    let product_id = 555;
    let quantity = 9000;
    let payload = json!({ "quantity": quantity });

    let expected_cart = Cart {
        products: hashmap!{ product_id => quantity },
    };
    let expected_storage = hashmap!{ user_id => expected_cart.clone() };

    let mut req = Request::new(Method::Put, Uri::from_str(&format!("/cart/products/{}", product_id)).unwrap());
    req.headers_mut().set::<Authorization<String>>(Authorization(user_id.to_string()));
    req.set_body(serde_json::to_string::<serde_json::Value>(&payload).unwrap());

    let data = Default::default();

    let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

    assert_eq!(*data.lock().unwrap(), expected_storage);
    assert_eq!(serde_json::from_str::<Cart>(&resp).unwrap(), expected_cart);
}

#[test]
fn test_delete_item() {
    let user_id = 12345;
    let product_id_keep = 444;
    let quantity_keep = 9000;
    let product_id_remove = 555;
    let quantity_remove = 9100;
    let cart = Cart {
        products: hashmap! {
            product_id_keep => quantity_keep,
            product_id_remove => quantity_remove,
        },
    };
    let storage = hashmap! {
        user_id => cart.clone(),
    };

    let expected_cart = Cart {
        products: hashmap! {
            product_id_keep => quantity_keep,
        },
    };
    let expected_storage = hashmap! {
        user_id => expected_cart.clone(),
    };

    let mut req = Request::new(
        Method::Delete,
        Uri::from_str(&format!("/cart/products/{}", product_id_remove)).unwrap(),
    );
    req.headers_mut().set::<Authorization<String>>(Authorization(user_id.to_string()));

    let data = Arc::new(Mutex::new(storage));

    let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

    assert_eq!(*data.lock().unwrap(), expected_storage);
    assert_eq!(serde_json::from_str::<Cart>(&resp).unwrap(), expected_cart);
}

#[test]
fn test_clear_cart() {
    let user_id = 12345;
    let data = Arc::new(Mutex::new(hashmap! {
        user_id => Cart {
            products: hashmap! {
                444 => 9000,
                555 => 9010,
                666 => 9020,
            },
        },
    }));
    let expected_cart = Cart::default();
    let expected_storage = hashmap! {
        user_id => expected_cart.clone(),
    };

    let mut req = Request::new(Method::Post, Uri::from_str("/cart/clear/").unwrap());
    req.headers_mut().set::<Authorization<String>>(Authorization(user_id.to_string()));

    let resp = run_controller_op(Arc::clone(&data), req).wait().unwrap();

    assert_eq!(*data.lock().unwrap(), expected_storage);
    assert_eq!(serde_json::from_str::<Cart>(&resp).unwrap(), expected_cart);
}
