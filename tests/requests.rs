extern crate futures;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate orders_lib as lib;
extern crate serde_json;
extern crate stq_http;
extern crate stq_static_resources;
extern crate stq_types;
extern crate tokio_core;

pub mod common;

use hyper::{header::{Authorization, Cookie},
            Headers,
            Method};
use lib::models::*;
use stq_http::client::ClientHandle as HttpClientHandle;
use stq_static_resources::OrderState;
use stq_types::*;
use tokio_core::reactor::Core;

struct RpcClient {
    http_client: HttpClientHandle,
    core: Core,
    base_url: String,
    user: UserId,
}

impl RpcClient {
    fn new(base_url: String, user: UserId) -> Self {
        let (core, http_client) = common::make_utils();
        RpcClient {
            http_client,
            core,
            base_url,
            user,
        }
    }

    fn get_cart(&mut self) -> Cart {
        self.core
            .run(self.http_client.request_with_auth_header::<Cart>(
                Method::Get,
                format!("{}/cart/products", self.base_url),
                None,
                Some(self.user.to_string()),
            ))
            .unwrap()
    }

    fn initialize_cart(&mut self, products: Vec<CartItem>) {
        for (i, product) in products.iter().enumerate() {
            self.core
                .run(
                    self.http_client.request_with_auth_header::<Cart>(
                        Method::Post,
                        format!("{}/cart/products/{}/increment", self.base_url, product.product_id),
                        Some(
                            serde_json::to_string(&CartProductIncrementPayload {
                                store_id: product.store_id,
                            }).unwrap(),
                        ),
                        Some(self.user.to_string()),
                    ),
                )
                .unwrap();

            self.core
                .run(self.http_client.request_with_auth_header::<Cart>(
                    Method::Put,
                    format!("{}/cart/products/{}/quantity", self.base_url, product.product_id),
                    Some(serde_json::to_string(&CartProductQuantityPayload { value: product.quantity }).unwrap()),
                    Some(self.user.to_string()),
                ))
                .unwrap();

            self.core
                .run(self.http_client.request_with_auth_header::<Cart>(
                    Method::Put,
                    format!("{}/cart/products/{}/selection", self.base_url, product.product_id),
                    Some(serde_json::to_string(&CartProductSelectionPayload { value: product.selected }).unwrap()),
                    Some(self.user.to_string()),
                ))
                .unwrap();

            self.core
                .run(
                    self.http_client.request_with_auth_header::<Cart>(
                        Method::Put,
                        format!("{}/cart/products/{}/comment", self.base_url, product.product_id),
                        Some(
                            serde_json::to_string(&CartProductCommentPayload {
                                value: product.comment.clone(),
                            }).unwrap(),
                        ),
                        Some(self.user.to_string()),
                    ),
                )
                .unwrap();

            assert_eq!(
                self.get_cart(),
                products
                    .clone()
                    .into_iter()
                    .enumerate()
                    .filter(|(n, _)| *n <= i)
                    .map(|(_, item)| item)
                    .map(CartItem::into_meta)
                    .collect::<Cart>()
            );
        }
    }

    fn clear_cart(&mut self) {
        assert_eq!(
            self.core
                .run(self.http_client.request_with_auth_header::<Cart>(
                    Method::Post,
                    format!("{}/cart/clear", self.base_url),
                    None,
                    Some(self.user.to_string()),
                ))
                .unwrap(),
            hashmap!{},
        );
    }

    fn get_order(&mut self, id: OrderId) -> Option<Order> {
        self.core
            .run(self.http_client.request_with_auth_header::<Option<Order>>(
                Method::Get,
                format!("{}/orders/by-id/{}", self.base_url, id),
                None,
                Some(self.user.to_string()),
            ))
            .unwrap()
    }
}

#[test]
fn test_carts_service() {
    let base_url = common::setup();
    let (mut core, http_client) = common::make_utils();

    let user_id = UserId(777);
    let user_id_2 = UserId(24361345);

    let mut rpc = RpcClient::new(base_url.clone(), user_id);

    let store_id = StoreId(1337);
    let product_id = ProductId(12345);
    let quantity = Quantity(9000);

    let product_id_2 = ProductId(67890);
    let mut quantity_2 = Quantity(0);

    let product_id_3 = ProductId(88888);
    let quantity_3 = Quantity(9002);

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
                quantity: Quantity(1),
                selected: true,
                comment: String::new(),
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
                quantity: Quantity(1),
                selected: true,
                comment: String::new(),
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
                quantity: Quantity(1),
                selected: true,
                comment: String::new(),
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
                c.set("SESSION_ID", UserId(user_id.0 + 1000).to_string());

                h.set(c);
                h
            }),
        )).unwrap(),
        hashmap! {
            product_id => CartItemInfo {
                quantity: Quantity(1),
                selected: true,
                comment: String::new(),
                store_id,
            },
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}/quantity", base_url, product_id),
            Some(serde_json::to_string(&CartProductQuantityPayload { value: quantity }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap()
            .remove(&product_id)
            .unwrap(),
        CartItemInfo {
            quantity,
            selected: true,
            comment: String::new(),
            store_id,
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}/selection", base_url, product_id),
            Some(serde_json::to_string(&CartProductSelectionPayload { value: false }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap()
            .remove(&product_id)
            .unwrap(),
        CartItemInfo {
            quantity,
            selected: false,
            comment: String::new(),
            store_id,
        }
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}/comment", base_url, product_id),
            Some(serde_json::to_string(&CartProductCommentPayload { value: "MyComment".into() }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap()
            .remove(&product_id)
            .unwrap(),
        CartItemInfo {
            quantity,
            selected: false,
            comment: "MyComment".into(),
            store_id,
        }
    );

    quantity_2.0 += 1;

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
                comment: "MyComment".into(),
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                comment: String::new(),
                store_id,
            },
        },
    );

    quantity_2.0 += 1;

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
                comment: "MyComment".into(),
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                comment: String::new(),
                store_id,
            },
        },
    );

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Put,
            format!("{}/cart/products/{}/quantity", base_url, product_id_3),
            Some(serde_json::to_string(&CartProductQuantityPayload { value: quantity_3 }).unwrap()),
            Some(user_id.to_string()),
        )).unwrap()
            .remove(&product_id_3),
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
                comment: "MyComment".into(),
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                comment: String::new(),
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
                comment: "MyComment".into(),
                store_id,
            },
            product_id_2 => CartItemInfo {
                quantity: quantity_2,
                selected: true,
                comment: String::new(),
                store_id,
            },
        },
    );

    {
        // Merge testing
        let user_from = user_id_2;
        let to_user = user_id;
        let from_existing_product_id = product_id_2;
        let from_existing_product_quantity = Quantity(912673);
        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/products/{}/increment", base_url, from_existing_product_id),
                Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
                Some(user_from.to_string()),
            )).unwrap(),
            hashmap! {
                from_existing_product_id => CartItemInfo {
                    quantity: Quantity(1),
                    selected: true,
                comment: String::new(),
                    store_id,
                },
            },
        );

        let from_new_product_id = ProductId(2351143);
        let from_new_product_quantity = Quantity(2324);
        assert_eq!(
            core.run(http_client.request_with_auth_header::<Cart>(
                Method::Post,
                format!("{}/cart/products/{}/increment", base_url, from_new_product_id),
                Some(serde_json::to_string(&CartProductIncrementPayload { store_id }).unwrap()),
                Some(user_from.to_string()),
            )).unwrap(),
            hashmap! {
                from_existing_product_id => CartItemInfo {
                    quantity: Quantity(1),
                    selected: true,
                    comment: String::new(),
                    store_id,
                },
                from_new_product_id => CartItemInfo {
                    quantity: Quantity(1),
                    selected: true,
                    comment: String::new(),
                    store_id,
                },
            },
        );

        assert_eq!(
            core.run(
                http_client.request_with_auth_header::<Cart>(
                    Method::Put,
                    format!("{}/cart/products/{}/quantity", base_url, from_existing_product_id),
                    Some(
                        serde_json::to_string(&CartProductQuantityPayload {
                            value: from_existing_product_quantity,
                        }).unwrap(),
                    ),
                    Some(user_from.to_string()),
                )
            ).unwrap()
                .remove(&from_existing_product_id)
                .unwrap(),
            CartItemInfo {
                quantity: from_existing_product_quantity,
                selected: true,
                comment: String::new(),
                store_id,
            },
        );

        assert_eq!(
            core.run(
                http_client.request_with_auth_header::<Cart>(
                    Method::Put,
                    format!("{}/cart/products/{}/quantity", base_url, from_new_product_id),
                    Some(
                        serde_json::to_string(&CartProductQuantityPayload {
                            value: from_new_product_quantity,
                        }).unwrap(),
                    ),
                    Some(user_from.to_string()),
                )
            ).unwrap()
                .remove(&from_new_product_id)
                .unwrap(),
            CartItemInfo {
                quantity: from_new_product_quantity,
                selected: true,
                comment: String::new(),
                store_id,
            },
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
                    comment: "MyComment".into(),
                    store_id,
                },
                product_id_2 => CartItemInfo {
                    quantity: quantity_2,
                    selected: true,
                    comment: String::new(),
                    store_id,
                },
                from_new_product_id => CartItemInfo {
                    quantity: from_new_product_quantity,
                    selected: true,
                    comment: String::new(),
                    store_id,
                }
            },
        );
        // End of merge testing
    }

    assert_eq!(
        core.run(http_client.request_with_auth_header::<Cart>(
            Method::Delete,
            format!("{}/cart/products/{}", base_url, product_id),
            None,
            Some(user_id.to_string()),
        )).unwrap()
            .remove(&product_id),
        None,
    );

    rpc.clear_cart();
}

#[test]
fn test_orders_conversion() {
    let base_url = common::setup();
    let (mut core, http_client) = common::make_utils();

    let user = UserId(7234212);

    let cart_fixture = vec![
        CartItem {
            product_id: ProductId(634824),
            quantity: Quantity(1),
            selected: true,
            comment: "Product 1 comment".into(),
            store_id: StoreId(1001),
        },
        CartItem {
            product_id: ProductId(5612213),
            quantity: Quantity(25),
            selected: true,
            comment: "Product 2 comment".into(),
            store_id: StoreId(1001),
        },
        CartItem {
            product_id: ProductId(112314512),
            quantity: Quantity(12),
            selected: false,
            comment: "Product 3 comment".into(),
            store_id: StoreId(1001),
        },
    ];

    let convert_cart_payload = ConvertCartPayload {
        conversion_id: Some(ConversionId::new()),
        customer_id: user,
        receiver_name: "Mr. Anderson".into(),
        address: AddressFull {
            country: Some("Matrix".into()),
            locality: Some("Central city".into()),
            ..Default::default()
        },
        prices: hashmap! {
            cart_fixture[0].product_id => ProductSellerPrice { price: ProductPrice(41213.0), currency_id: CurrencyId(1) },
            cart_fixture[1].product_id => ProductSellerPrice { price: ProductPrice(84301.0), currency_id: CurrencyId(2) },
        },
    };

    let mut rpc = RpcClient::new(base_url.clone(), user);

    rpc.clear_cart();
    rpc.initialize_cart(cart_fixture.clone());

    {
        let new_orders = core.run(http_client.request_with_auth_header::<Vec<Order>>(
            Method::Post,
            format!("{}/orders/create_from_cart", base_url),
            Some(serde_json::to_string(&convert_cart_payload).unwrap()),
            Some(user.to_string()),
        )).unwrap();

        let created_orders_fixture = cart_fixture
            .clone()
            .into_iter()
            .filter(|cart_item| cart_item.selected)
            .map(|cart_item| {
                let db_data = new_orders
                    .iter()
                    .filter(|order| order.product == cart_item.product_id)
                    .cloned()
                    .next()
                    .unwrap();
                let ProductSellerPrice { price, currency_id } = convert_cart_payload.prices[&cart_item.product_id];
                Order {
                    id: db_data.id.clone(),
                    created_from: db_data.created_from.clone(),
                    conversion_id: convert_cart_payload.conversion_id.unwrap(),
                    slug: db_data.slug,
                    customer: user,
                    store: cart_item.store_id,
                    product: cart_item.product_id,
                    price,
                    currency_id,
                    quantity: cart_item.quantity,
                    address: convert_cart_payload.address.clone(),
                    receiver_name: convert_cart_payload.receiver_name.clone(),
                    state: OrderState::New,
                    payment_status: false,
                    delivery_company: None,
                    track_id: None,
                    created_at: db_data.created_at.clone(),
                    updated_at: db_data.updated_at.clone(),
                }
            })
            .collect::<Vec<_>>();

        assert_eq!(new_orders, created_orders_fixture);
        assert_eq!(
            rpc.get_cart(),
            cart_fixture
                .clone()
                .into_iter()
                .filter(|item| !item.selected)
                .map(CartItem::into_meta)
                .collect::<Cart>()
        );

        core.run(
            http_client.request_with_auth_header::<()>(
                Method::Post,
                format!("{}/orders/create_from_cart/revert", base_url),
                Some(
                    serde_json::to_string(&ConvertCartRevertPayload {
                        conversion_id: convert_cart_payload.conversion_id.unwrap(),
                    }).unwrap(),
                ),
                Some(user.to_string()),
            ),
        ).unwrap();

        for order in created_orders_fixture.iter() {
            assert_eq!(rpc.get_order(order.id), None);
        }
        assert_eq!(
            rpc.get_cart(),
            cart_fixture.clone().into_iter().map(CartItem::into_meta).collect::<Cart>()
        );
    }

    rpc.clear_cart();
}
