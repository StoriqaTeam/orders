use stq_router::*;

use models::*;

#[derive(Clone, Copy, Debug)]
pub enum Route {
    Cart,
    CartProducts,
    CartIncrementProduct { product_id: ProductId },
    CartProduct { product_id: ProductId },
    CartProductQuantity { product_id: ProductId },
    CartProductSelection { product_id: ProductId },
    CartClear,
    CartMerge,
    OrderFromCart,
    OrderSearch,
    Orders,
    OrdersByStore { store_id: StoreId },
    Order { order_id: OrderIdentifier },
    OrderStatus { order_id: OrderIdentifier },
    OrdersAllowedStatuses,
}

pub fn make_router() -> RouteParser<Route> {
    let mut route_parser: RouteParser<Route> = Default::default();
    route_parser.add_route(r"^/cart$", || Route::Cart);
    route_parser.add_route_with_params(r"^/cart/products/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartProduct { product_id })
    });
    route_parser.add_route_with_params(r"^/cart/products/(\d+)/increment$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartIncrementProduct { product_id })
    });
    route_parser.add_route_with_params(r"^/cart/products/(\d+)/quantity$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartProductQuantity { product_id })
    });
    route_parser.add_route_with_params(r"^/cart/products/(\d+)/selection$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartProductSelection { product_id })
    });
    route_parser.add_route(r"^/cart/products$", || Route::CartProducts);
    route_parser.add_route(r"^/cart/clear$", || Route::CartClear);
    route_parser.add_route(r"^/cart/merge$", || Route::CartMerge);
    route_parser.add_route(r"^/orders$", || Route::Orders);
    route_parser.add_route(r"^/orders/create_from_cart$", || Route::OrderFromCart);
    route_parser.add_route(r"^/orders/search", || Route::OrderSearch);
    route_parser.add_route_with_params(r"^/orders/by-store/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(StoreId))
            .map(|store_id| Route::OrdersByStore { store_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-id(\S+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Id))
            .map(|order_id| Route::Order { order_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-slug(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Slug))
            .map(|order_id| Route::Order { order_id })
    });

    route_parser
}
