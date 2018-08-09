extern crate futures;
extern crate hyper;
extern crate orders_lib as lib;
extern crate serde_json;
extern crate stq_api;
extern crate stq_http;
extern crate stq_static_resources;
extern crate stq_types;
extern crate tokio_core;

pub mod common;

use stq_api::orders::*;
use stq_api::rpc_client::*;
use stq_static_resources::OrderState;
use stq_types::*;
use tokio_core::reactor::Core;

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

    pub fn set_cart_items(&self, products: Vec<CartItem>) {
        let mut core = Core::new().unwrap();
        for product in products {
            core.run(self.inner.delete_item(product.customer, product.product_id)).unwrap();
            core.run(self.inner.increment_item(product.customer, product.product_id, product.store_id))
                .unwrap();
            core.run(self.inner.set_quantity(product.customer, product.product_id, product.quantity))
                .unwrap();
            core.run(self.inner.set_selection(product.customer, product.product_id, product.selected))
                .unwrap();
            core.run(self.inner.set_comment(product.customer, product.product_id, product.comment))
                .unwrap();
        }
    }

    pub fn create_product(&self, customer: CartCustomer, product_id: ProductId, store_id: StoreId) -> CartItem {
        let mut core = Core::new().unwrap();
        let rsp = core.run(self.inner.increment_item(customer, product_id, store_id)).unwrap();
        let v = rsp.into_iter()
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
fn test_carts_service() {
    let base_url = common::setup();
    let mut core = Core::new().unwrap();

    let u = UserId(777);
    let user_1 = CartCustomer::User(u);
    let anon_1 = CartCustomer::Anonymous(SessionId::new());

    let su_rpc = RpcClient::new(&base_url, UserId(1));
    let rpc = RpcClient::new(&base_url, u);

    let store_id = StoreId(1337);

    // Clear cart
    for id in vec![user_1, anon_1] {
        assert_eq!(core.run(su_rpc.inner.clear_cart(id)).unwrap(), vec![]);
    }

    // Create product and set metadata
    let product_id_1 = ProductId(12345);
    let mut product_1 = rpc.create_product(user_1, product_id_1, store_id);

    assert_eq!(core.run(rpc.inner.get_cart(user_1)).unwrap(), vec![product_1.clone()]);

    product_1.quantity = Quantity(5);
    assert_eq!(
        core.run(rpc.inner.set_quantity(user_1, product_id_1, Quantity(5))).unwrap(),
        vec![product_1.clone()]
    );

    product_1.selected = false;
    assert_eq!(
        core.run(rpc.inner.set_selection(user_1, product_id_1, false)).unwrap(),
        vec![product_1.clone()]
    );

    product_1.comment = "MyComment".into();
    assert_eq!(
        core.run(rpc.inner.set_comment(user_1, product_id_1, "MyComment".into())).unwrap(),
        vec![product_1.clone()]
    );

    // Create another product and set meta
    let product_id_2 = ProductId(67890);
    let mut product_2 = rpc.create_product(user_1, product_id_2, store_id);

    product_2.quantity.0 += 1;
    assert_eq!(
        core.run(rpc.inner.increment_item(user_1, product_id_2, store_id)).unwrap(),
        vec![product_1.clone(), product_2.clone()]
    );

    // Check that we cannot set quantity for the product that does not exist
    let product_id_3 = ProductId(88888);
    let quantity_3 = Quantity(9002);
    for cart_item in core.run(rpc.inner.set_quantity(user_1, product_id_3, quantity_3)).unwrap() {
        assert_ne!(cart_item.product_id, product_id_3);
    }

    assert_eq!(
        core.run(rpc.inner.list(user_1, ProductId(0), 2)).unwrap(),
        vec![product_1.clone(), product_2.clone()],
    );

    assert_eq!(
        core.run(rpc.inner.get_cart(user_1)).unwrap(),
        vec![product_1.clone(), product_2.clone()],
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
            core.run(
                rpc.inner
                    .set_quantity(user_from, from_existing_product_id, from_existing_product_quantity)
            ).unwrap(),
            vec![from_existing_product.clone(), from_new_product.clone()]
        );

        from_new_product.quantity = from_new_product_quantity;
        assert_eq!(
            core.run(rpc.inner.set_quantity(user_from, from_new_product_id, from_new_product_quantity))
                .unwrap(),
            vec![from_existing_product.clone(), from_new_product.clone()]
        );

        from_new_product.customer = to_user;

        assert_eq!(
            core.run(rpc.inner.merge(user_from, to_user)).unwrap(),
            vec![product_1.clone(), product_2.clone(), from_new_product.clone()],
        );
        // End of merge testing
    }

    // Clear cart after testing
    for id in vec![user_1, anon_1] {
        assert_eq!(core.run(su_rpc.inner.clear_cart(id)).unwrap(), vec![]);
    }
}

/*
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
        receiver_phone: "+14441234567".into(),
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
                    receiver_phone: convert_cart_payload.receiver_phone.clone(),
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
            core.run(rpc.inner.get_cart()),
            cart_fixture
                .clone()
                .into_iter()
                .filter(|item| !item.selected)
                .map(CartItem::into_meta)
                .collect::<Cart>()
        );

        rpc.set_cart_items(vec![cart_fixture[0].clone()]);

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
*/
