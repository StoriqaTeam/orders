extern crate futures;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate orders_lib as lib;
extern crate serde_json;
extern crate stq_http;
extern crate tokio_core;

pub mod common;

use hyper::{header::{Authorization, Cookie},
            Headers,
            Method};
use lib::models::*;

#[test]
fn test_carts_service() {
    let common::Context {
        mut core,
        http_client,
        base_url,
    } = common::setup();

    let user_id = 777;
    let user_id_2 = 24361345;

    let store_id = 1337;
    let product_id = 12345;
    let quantity = 9000;

    let product_id_2 = 67890;
    let mut quantity_2 = 0;

    let product_id_3 = 88888;
    let quantity_3 = 9002;

    for id in vec![user_id, user_id_2] {
        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/clear", base_url),
                None,
                Some(id.to_string())
            )).unwrap(),
            hashmap!{},
        );
    }

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
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Get,
            format!("{}/cart/products", base_url),
            None,
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
        core.run(http_client.request::<Cart>(
            Method::Get,
            format!("{}/cart/products", base_url),
            None,
            Some({
                let mut h = Headers::new();

                let mut c = Cookie::new();
                c.set("SESSION_ID", user_id.to_string());

                h.set(c);
                h
            }),
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
        core.run(http_client.request::<Cart>(
            Method::Get,
            format!("{}/cart/products", base_url),
            None,
            Some({
                let mut h = Headers::new();

                h.set(Authorization(user_id.to_string()));

                let mut c = Cookie::new();
                c.set("SESSION_ID", (user_id + 1000).to_string());

                h.set(c);
                h
            }),
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
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Get,
            format!("{}/cart/products", base_url),
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

    {
        // Merge testing
        let user_from = user_id_2;
        let to_user = user_id;
        let from_existing_product_id = product_id_2;
        let from_existing_product_quantity = 912673;
        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/products/{}/increment", base_url, from_existing_product_id),
                Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
                Some(user_from.to_string()),
            )).unwrap(),
            hashmap! {
                from_existing_product_id => CartItemInfo {
                    quantity: 1,
                    selected: true,
                    store_id,
                },
            },
        );

        let from_new_product_id = 2351143;
        let from_new_product_quantity = 2324;
        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/products/{}/increment", base_url, from_new_product_id),
                Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
                Some(user_from.to_string()),
            )).unwrap(),
            hashmap! {
                from_existing_product_id => CartItemInfo {
                    quantity: 1,
                    selected: true,
                    store_id,
                },
                from_new_product_id => CartItemInfo {
                    quantity: 1,
                    selected: true,
                    store_id,
                },
            },
        );

        assert_eq!(
            core.run(
                http_client.request_with_auth_header::<Option<CartItem>>(
                    Method::Put,
                    format!("{}/cart/products/{}/quantity", base_url, from_existing_product_id),
                    Some(
                        serde_json::to_string(&CartProductQuantityPayload {
                            value: from_existing_product_quantity,
                        }).unwrap(),
                    ),
                    Some(user_from.to_string()),
                )
            ).unwrap(),
            Some(CartItem {
                product_id: from_existing_product_id,
                quantity: from_existing_product_quantity,
                selected: true,
                store_id,
            }),
        );

        assert_eq!(
            core.run(
                http_client.request_with_auth_header::<Option<CartItem>>(
                    Method::Put,
                    format!("{}/cart/products/{}/quantity", base_url, from_new_product_id),
                    Some(
                        serde_json::to_string(&CartProductQuantityPayload {
                            value: from_new_product_quantity,
                        }).unwrap(),
                    ),
                    Some(user_from.to_string()),
                )
            ).unwrap(),
            Some(CartItem {
                product_id: from_new_product_id,
                quantity: from_new_product_quantity,
                selected: true,
                store_id,
            }),
        );

        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/merge", base_url),
                Some(serde_json::to_string(&CartMergePayload { user_from }).unwrap()),
                Some(to_user.to_string()),
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
                from_new_product_id => CartItemInfo {
                    quantity: from_new_product_quantity,
                    selected: true,
                    store_id,
                }
            },
        );
        // End of merge testing
    }

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
