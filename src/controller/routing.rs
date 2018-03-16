use stq_router::*;

#[derive(Clone, Copy, Debug)]
pub enum Route {
    CartProducts,
    CartProduct { product_id: i32 },
    CartClear,
}

pub fn make_router() -> RouteParser<Route> {
    let mut route_parser: RouteParser<Route> = Default::default();
    route_parser.add_route_with_params(r"^/cart/products/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|product_id| Route::CartProduct { product_id })
    });
    route_parser.add_route(r"^/cart/products", || Route::CartProducts);
    route_parser.add_route(r"^/cart/clear", || Route::CartClear);

    route_parser
}
