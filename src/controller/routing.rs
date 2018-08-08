use stq_api::orders::*;
use stq_roles;
use stq_router::*;
use stq_types::*;

pub fn make_router() -> RouteParser<Route> {
    let mut route_parser: RouteParser<Route> = Default::default();
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::User))
            .map(|customer| Route::Cart { customer })
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::Anonymous))
            .map(|customer| Route::Cart { customer })
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products/(\d+)$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::User) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products/(\d+)$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::Anonymous) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products/(\d+)/increment$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::User) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products/(\d+)/increment$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::Anonymous) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products/(\d+)/quantity$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::User) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products/(\d+)/quantity$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::Anonymous) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products/(\d+)/selection$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::User) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products/(\d+)/selection$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::Anonymous) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products/(\d+)/comment$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::User) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products/(\d+)/comment$", |params| {
        if let Some(customer_id_s) = params.get(0) {
            if let Some(product_id_s) = params.get(0) {
                if let Ok(customer) = customer_id_s.parse().map(CartCustomer::Anonymous) {
                    if let Ok(product_id) = product_id_s.parse().map(ProductId) {
                        return Some(Route::CartIncrementProduct { customer, product_id });
                    }
                }
            }
        }
        None
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/products$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::User))
            .map(|customer| Route::CartProducts { customer })
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/products$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::Anonymous))
            .map(|customer| Route::CartProducts { customer })
    });
    route_parser.add_route_with_params(r"^/cart/by-user/(\d+)/clear$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::User))
            .map(|customer| Route::CartClear { customer })
    });
    route_parser.add_route_with_params(r"^/cart/by-session/([a-zA-Z0-9-]+)/clear$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(CartCustomer::Anonymous))
            .map(|customer| Route::CartClear { customer })
    });
    route_parser.add_route(r"^/cart/merge$", || Route::CartMerge);
    route_parser.add_route(r"^/orders$", || Route::Orders);
    route_parser.add_route(r"^/orders/create_from_cart$", || Route::OrderFromCart);
    route_parser.add_route(r"^/orders/create_from_cart/revert$", || Route::OrderFromCartRevert);
    route_parser.add_route(r"^/orders/search", || Route::OrderSearch);
    route_parser.add_route_with_params(r"^/orders/by-store/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok())
            .map(|store_id| Route::OrdersByStore { store_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-id/([a-zA-Z0-9-]+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Id))
            .map(|order_id| Route::Order { order_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-slug/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Slug))
            .map(|order_id| Route::Order { order_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-id/([a-zA-Z0-9-]+)/status$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Id))
            .map(|order_id| Route::OrderStatus { order_id })
    });
    route_parser.add_route_with_params(r"^/orders/by-slug/(\d+)/status$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Slug))
            .map(|order_id| Route::OrderStatus { order_id })
    });
    route_parser.add_route_with_params(r"^/order_diff/by-id/([a-zA-Z0-9-]+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Id))
            .map(|order_id| Route::OrderDiff { order_id })
    });
    route_parser.add_route_with_params(r"^/order_diff/by-slug/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse().ok().map(OrderIdentifier::Slug))
            .map(|order_id| Route::OrderDiff { order_id })
    });

    route_parser = stq_roles::routing::add_routes(route_parser);

    route_parser
}
