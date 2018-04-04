extern crate futures;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate orders_lib as lib;
#[macro_use]
extern crate serde_json;
extern crate stq_http;
extern crate tokio_core;

pub mod common;

use hyper::Method;
use lib::models::Cart;

#[test]
fn healthcheck_returns_ok() {
    let common::Context {
        mut core,
        http_client,
        base_url,
    } = common::setup();

    let uri = format!("{}/healthcheck", base_url);

    assert_eq!(
        core.run(http_client.request::<String>(Method::Get, uri, None, None,)).unwrap(),
        "Ok"
    );
}

#[test]
fn test_carts_service() {
    let common::Context {
        mut core,
        http_client,
        base_url,
    } = common::setup();

    let user_id = 777;
    let product_id = 12345;
    let quantity = 9000;

    let product_id_2 = 67890;
    let quantity_2 = 9001;

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/clear", base_url),
            None,
            Some(user_id.to_string())
        )).unwrap(),
        Cart::default()
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}", base_url, product_id),
            Some(serde_json::to_string(&json!({ "quantity": quantity })).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        Cart {
            products: hashmap!{
                product_id => quantity,
            },
        }
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}", base_url, product_id_2),
            Some(serde_json::to_string(&json!({ "quantity": quantity_2 })).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        Cart {
            products: hashmap!{
                product_id => quantity,
                product_id_2 => quantity_2,
            },
        }
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Delete,
            format!("{}/cart/products/{}", base_url, product_id),
            None,
            Some(user_id.to_string()),
        )).unwrap(),
        Cart {
            products: hashmap!{
                product_id_2 => quantity_2,
            },
        }
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/clear", base_url),
            None,
            Some(user_id.to_string()),
        )).unwrap(),
        Cart { products: hashmap!{} },
    );
}
