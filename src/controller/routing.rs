use stq_router::*;

use models::*;

#[derive(Clone, Copy, Debug)]
pub enum Route {
    CartProducts,
    CartIncrementProduct { product_id: i32 },
    CartProduct { product_id: i32 },
    CartClear,
    OrderFromCart,
    Orders,
    Order { order_id: OrderId },
    OrderStatus { order_id: OrderId },
    Healthcheck,
}

pub fn make_router() -> RouteParser<Route> {
    let mut route_parser: RouteParser<Route> = Default::default();
    route_parser.add_route(r"^/healthcheck$", || Route::Healthcheck);
    route_parser.add_route_with_params(r"^/cart/products/(\d+)/increment$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartIncrementProduct { product_id })
    });
    route_parser.add_route_with_params(r"^/cart/products/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartProduct { product_id })
    });
    route_parser.add_route(r"^/cart/products$", || Route::CartProducts);
    route_parser.add_route(r"^/cart/clear$", || Route::CartClear);
    route_parser.add_route(r"^/orders$", || Route::Orders);
    route_parser.add_route(r"^/orders/create_from_cart$", || Route::OrderFromCart);
    route_parser.add_route_with_params(r"^/orders/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|order_id| Route::Order { order_id })
    });
    route_parser.add_route_with_params(r"^/orders/(\d+)/status$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|order_id| Route::OrderStatus { order_id })
    });

    route_parser
}
