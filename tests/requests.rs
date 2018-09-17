extern crate futures;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate orders_lib as lib;
extern crate serde_json;
extern crate stq_api;
extern crate stq_http;
extern crate stq_static_resources;
extern crate stq_types;
extern crate tokio;

pub mod common;

use futures::{future, prelude::*};
use std::collections::HashMap;
use stq_api::orders::*;
use stq_api::rpc_client::*;
use stq_static_resources::{Currency, OrderState};
use stq_types::*;

struct RpcClient {
    pub inner: RestApiClient,
}

impl RpcClient {
    pub fn new<S>(base_url: &S, user: UserId) -> Self
    where
        S: ToString,
    {
        Self {
            inner: RestApiClient::new(base_url, Some(user)),
        }
    }

    pub fn set_cart_items(&self, products: Cart) -> Cart {
        let mut out = Cart::new();

        for product in products {
            self.inner.delete_item(product.customer, product.product_id).wait().unwrap();
            self.inner
                .increment_item(product.customer, product.product_id, product.store_id)
                .wait()
                .unwrap();
            self.inner
                .set_quantity(product.customer, product.product_id, product.quantity)
                .wait()
                .unwrap();
            self.inner
                .set_selection(product.customer, product.product_id, product.selected)
                .wait()
                .unwrap();
            out = self
                .inner
                .set_comment(product.customer, product.product_id, product.comment)
                .wait()
                .unwrap();
        }

        out
    }

    pub fn create_product(&self, customer: CartCustomer, product_id: ProductId, store_id: StoreId) -> CartItem {
        let rsp = self.inner.increment_item(customer, product_id, store_id).wait().unwrap();
        let v = rsp
            .into_iter()
            .filter(|cart_item| cart_item.product_id == product_id)
            .next()
            .unwrap();
        assert_eq!(
            v,
            CartItem {
                id: v.id,
                customer,
                product_id,
                quantity: Quantity(1),
                selected: true,
                comment: String::new(),
                store_id,
            },
        );

        v
    }
}

#[test]
fn test_services() {
    let base_url = common::setup();

    tokio::run(future::ok(()).map(move |_| {
        let su_rpc = RpcClient::new(&base_url, UserId(1));

        // Carts
        {
            let u = UserId(777);
            let user_1 = u.into();
            let anon_1 = SessionId(613415346).into();

            let rpc = RpcClient::new(&base_url, u);

            let store_id = StoreId(1337);

            // Clear cart
            for id in vec![user_1, anon_1] {
                assert_eq!(su_rpc.inner.clear_cart(id).wait().unwrap(), hashset![]);
            }

            // Create product and set metadata
            let product_id_1 = ProductId(12345);
            let mut product_1 = rpc.create_product(user_1, product_id_1, store_id);

            assert_eq!(rpc.inner.get_cart(user_1).wait().unwrap(), hashset![product_1.clone()]);

            product_1.quantity = Quantity(5);
            assert_eq!(
                rpc.inner.set_quantity(user_1, product_id_1, Quantity(5)).wait().unwrap(),
                hashset![product_1.clone()]
            );

            product_1.selected = false;
            assert_eq!(
                rpc.inner.set_selection(user_1, product_id_1, false).wait().unwrap(),
                hashset![product_1.clone()]
            );

            product_1.comment = "MyComment".into();
            assert_eq!(
                rpc.inner.set_comment(user_1, product_id_1, "MyComment".into()).wait().unwrap(),
                hashset![product_1.clone()]
            );

            // Create another product and set meta
            let product_id_2 = ProductId(67890);
            let mut product_2 = rpc.create_product(user_1, product_id_2, store_id);

            product_2.quantity.0 += 1;
            assert_eq!(
                rpc.inner.increment_item(user_1, product_id_2, store_id).wait().unwrap(),
                hashset![product_1.clone(), product_2.clone()]
            );

            // Check that we cannot set quantity for the product that does not exist
            let product_id_3 = ProductId(88888);
            let quantity_3 = Quantity(9002);
            for cart_item in rpc.inner.set_quantity(user_1, product_id_3, quantity_3).wait().unwrap() {
                assert_ne!(cart_item.product_id, product_id_3);
            }

            assert_eq!(
                rpc.inner.list(user_1, ProductId(0), 2).wait().unwrap(),
                hashset![product_1.clone(), product_2.clone()],
            );

            assert_eq!(
                rpc.inner.get_cart(user_1).wait().unwrap(),
                hashset![product_1.clone(), product_2.clone()],
            );

            // Test merging
            {
                let user_from = anon_1;
                let to_user = user_1;
                let from_existing_product_id = product_id_2;
                let from_existing_product_quantity = Quantity(912673);
                let mut from_existing_product = rpc.create_product(user_from, from_existing_product_id, store_id);

                let from_new_product_id = ProductId(2351143);
                let from_new_product_quantity = Quantity(2324);
                let mut from_new_product = rpc.create_product(user_from, from_new_product_id, store_id);

                from_existing_product.quantity = from_existing_product_quantity;
                assert_eq!(
                    rpc.inner
                        .set_quantity(user_from, from_existing_product_id, from_existing_product_quantity)
                        .wait()
                        .unwrap(),
                    hashset![from_existing_product.clone(), from_new_product.clone()]
                );

                from_new_product.quantity = from_new_product_quantity;
                assert_eq!(
                    rpc.inner
                        .set_quantity(user_from, from_new_product_id, from_new_product_quantity)
                        .wait()
                        .unwrap(),
                    hashset![from_existing_product.clone(), from_new_product.clone()]
                );

                from_new_product.customer = to_user;

                assert_eq!(
                    rpc.inner.merge(user_from, to_user).wait().unwrap(),
                    hashset![product_1.clone(), product_2.clone(), from_new_product.clone()],
                );
                // End of merge testing
            }

            // Clear cart after testing
            for id in vec![user_1, anon_1] {
                assert_eq!(su_rpc.inner.clear_cart(id).wait().unwrap(), hashset![]);
            }
        }

        // Orders

        {
            let user = UserId(7234212);

            let rpc = RpcClient::new(&base_url, user);

            rpc.inner.clear_cart(user.into()).wait().unwrap();

            let product_id_1 = ProductId(634824);
            let product_id_2 = ProductId(5612213);
            let product_id_3 = ProductId(112314512);

            let cart_fixture = rpc.set_cart_items(hashset![
                CartItem {
                    id: CartItemId::new(),
                    customer: user.into(),
                    product_id: product_id_1,
                    quantity: Quantity(1),
                    selected: true,
                    comment: "Product 1 comment".into(),
                    store_id: StoreId(1001),
                },
                CartItem {
                    id: CartItemId::new(),
                    customer: user.into(),
                    product_id: product_id_2,
                    quantity: Quantity(25),
                    selected: true,
                    comment: "Product 2 comment".into(),
                    store_id: StoreId(1001),
                },
                CartItem {
                    id: CartItemId::new(),
                    customer: user.into(),
                    product_id: product_id_3,
                    quantity: Quantity(12),
                    selected: false,
                    comment: "Product 3 comment".into(),
                    store_id: StoreId(1001),
                },
            ]);

            let conversion_id = ConversionId::new();
            let prices = vec![
                (
                    product_id_1,
                    ProductSellerPrice {
                        price: ProductPrice(41213.0),
                        currency: Currency::RUB,
                    },
                ),
                (
                    product_id_2,
                    ProductSellerPrice {
                        price: ProductPrice(84301.0),
                        currency: Currency::EUR,
                    },
                ),
            ].into_iter()
            .collect::<HashMap<_, _>>();
            let address = AddressFull {
                country: Some("Matrix".into()),
                locality: Some("Central city".into()),
                ..Default::default()
            };
            let receiver_name = "Mr. Anderson".to_string();
            let receiver_phone = "+14441234567".to_string();

            {
                let new_orders = rpc
                    .inner
                    .convert_cart(
                        Some(conversion_id),
                        user,
                        prices.clone(),
                        address.clone(),
                        receiver_name.clone(),
                        receiver_phone.clone(),
                    ).wait()
                    .unwrap();

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
                        let ProductSellerPrice { price, currency } = prices[&cart_item.product_id].clone();
                        Order {
                            id: db_data.id.clone(),
                            created_from: db_data.created_from.clone(),
                            conversion_id,
                            slug: db_data.slug,
                            customer: user,
                            store: cart_item.store_id,
                            product: cart_item.product_id,
                            price,
                            currency,
                            quantity: cart_item.quantity,
                            address: address.clone(),
                            receiver_name: receiver_name.clone(),
                            receiver_phone: receiver_phone.clone(),
                            state: OrderState::New,
                            payment_status: false,
                            delivery_company: None,
                            track_id: None,
                            created_at: db_data.created_at.clone(),
                            updated_at: db_data.updated_at.clone(),
                        }
                    }).collect::<Vec<_>>();

                assert_eq!(
                    new_orders
                        .clone()
                        .into_iter()
                        .map(|order| (order.id, order))
                        .collect::<HashMap<_, _>>(),
                    created_orders_fixture
                        .clone()
                        .into_iter()
                        .map(|order| (order.id, order))
                        .collect::<HashMap<_, _>>()
                );
                assert_eq!(
                    rpc.inner.get_cart(user.into()).wait().unwrap(),
                    cart_fixture.clone().into_iter().filter(|item| !item.selected).collect::<Cart>()
                );

                rpc.set_cart_items(hashset![cart_fixture.iter().filter(|v| v.selected).next().unwrap().clone()]);

                su_rpc.inner.revert_cart_conversion(conversion_id).wait().unwrap();

                for order in created_orders_fixture.iter() {
                    assert_eq!(rpc.inner.get_order(order.id.into()).wait().unwrap(), None);
                }
                assert_eq!(rpc.inner.get_cart(user.into()).wait().unwrap(), cart_fixture);
            }

            rpc.inner.clear_cart(user.into()).wait().unwrap();
        }
    }));
}
