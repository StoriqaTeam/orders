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
use lib::models::*;

#[test]
fn healthcheck_returns_ok() {
    let common::Context {
        mut core,
        http_client,
        base_url,
    } = common::setup();

    let uri = format!("{}/healthcheck", base_url);

    assert_eq!(
        core.run(http_client.request::<String>(Method::Get, uri, None, None,))
            .unwrap(),
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
    let store_id = 1337;
    let product_id = 12345;
    let quantity = 9000;

    let product_id_2 = 67890;
    let mut quantity_2 = 0;

    let product_id_3 = 88888;
    let quantity_3 = 9002;

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/clear", base_url),
            None,
            Some(user_id.to_string())
        )).unwrap(),
        hashmap!{},
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/products/{}/increment", base_url, product_id),
            Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        hashmap! {
            product_id => CartItemInfo {
                quantity: 1,
                selected: true,
                store_id,
            },
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<CartItem>(
            Method::Put,
            format!("{}/cart/products/{}/quantity", base_url, product_id),
            Some(serde_json::to_string(&CartProductQuantityPayload { value: quantity }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        CartItem {
            product_id,
            quantity,
            selected: true,
            store_id,
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<CartItem>(
            Method::Put,
            format!("{}/cart/products/{}/selection", base_url, product_id),
            Some(serde_json::to_string(&CartProductSelectionPayload { value: false }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        CartItem {
            product_id,
            quantity,
            selected: false,
            store_id,
        }
    );

    quantity_2 += 1;

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/products/{}/increment", base_url, product_id_2),
            Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        hashmap! {
            product_id => CartItemInfo {
                quantity,
                selected: false,
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                store_id,
            },
        },
    );

    quantity_2 += 1;

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/products/{}/increment", base_url, product_id_2),
            Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        hashmap! {
            product_id => CartItemInfo {
                quantity,
                selected: false,
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                store_id,
            },
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Option<CartItem>>(
            Method::Put,
            format!("{}/cart/products/{}/quantity", base_url, product_id_3),
            Some(serde_json::to_string(&CartProductQuantityPayload { value: quantity_3 }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap(),
        None,
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Get,
            format!("{}/cart?offset=0&count=2", base_url),
            None,
            Some(user_id.to_string()),
        )).unwrap(),
        hashmap!{
            product_id => CartItemInfo {
                quantity,
                selected: false,
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                store_id,
            },
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<CartItem>(
            Method::Delete,
            format!("{}/cart/products/{}", base_url, product_id),
            None,
            Some(user_id.to_string()),
        )).unwrap(),
        CartItem {
            product_id,
            quantity,
            selected: false,
            store_id,
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Post,
            format!("{}/cart/clear", base_url),
            None,
            Some(user_id.to_string()),
        )).unwrap(),
        hashmap!{},
    );
}
