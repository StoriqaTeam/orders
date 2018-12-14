use chrono::prelude::*;
use failure::{self, Fallible, ResultExt};
use futures::{future, prelude::*};
use hyper::{self, Delete, Get, Headers, Post, Put, Request};
use std::rc::Rc;
use stq_api::orders::*;
use stq_http::{
    controller::{Controller, ControllerFuture},
    errors::ErrorMessageWrapper,
    request_util::{parse_body, serialize_future},
};
use stq_roles::{
    routing::Controller as RoleController,
    service::{get_login_data, RoleService, RoleServiceImpl},
};
use stq_types::*;
use validator::Validate;

use config::*;
use errors::*;
use models::*;
use sentry_integration::log_and_capture_error;
use services::*;
use types::*;

pub type ServiceFactoryFuture<T> = Box<Future<Item = Box<T>, Error = failure::Error>>;

pub struct ServiceFactory {
    pub role: Rc<Fn(UserLogin) -> Box<RoleService<UserRole>>>,
    pub cart: Rc<Fn(UserLogin) -> Box<CartService>>,
    pub order: Rc<Fn(UserLogin) -> Box<OrderService>>,
}

pub struct ControllerImpl {
    db_pool: DbPool,
    service_factory: Rc<ServiceFactory>,
}

impl ControllerImpl {
    pub fn new(db_pool: &DbPool, _config: &Config) -> Self {
        ControllerImpl {
            service_factory: Rc::new(ServiceFactory {
                role: Rc::new({
                    let db_pool = db_pool.clone();
                    move |login_data| Box::new(RoleServiceImpl::new(db_pool.clone(), login_data))
                }),
                order: Rc::new({
                    let db_pool = db_pool.clone();
                    move |login_data| Box::new(OrderServiceImpl::new(db_pool.clone(), login_data))
                }),
                cart: Rc::new({
                    let db_pool = db_pool.clone();
                    move |login_data| Box::new(CartServiceImpl::new(db_pool.clone(), login_data)) as Box<CartService>
                }),
            }),
            db_pool: db_pool.clone(),
        }
    }
}

pub fn extract_user_id(headers: &Headers) -> Fallible<Option<UserId>> {
    let string_id: String = if let Some(auth) = headers.get::<hyper::header::Authorization<String>>() {
        auth.0.clone()
    } else if let Some(s) = headers.get::<hyper::header::Cookie>().and_then(|c| c.get("SESSION_ID")) {
        s.to_string()
    } else {
        return Ok(None);
    };

    let user_id = string_id
        .parse()
        .map_err(failure::Error::from)
        .context(format!("Failed to parse user ID: {}", string_id))
        .context(Error::UserIdParse)?;

    debug!("Extracted user_id: {}", user_id);

    Ok(Some(user_id))
}

impl Controller for ControllerImpl {
    fn call(&self, request: Request) -> ControllerFuture {
        let dt = Local::now();
        let (method, uri, _, headers, payload) = request.deconstruct();

        let service_factory = self.service_factory.clone();

        let route = Route::from_path(uri.path());
        Box::new(
            future::result(extract_user_id(&headers))
                .map_err(|e| e.context("Failed to extract user ID").into())
                .and_then({
                    let db_pool = self.db_pool.clone();
                    let path = uri.path().to_string();
                    let method = method.clone();
                    move |caller_id| {
                        debug!(
                            "Server received Request, method: {}, url: {}, user id: {:?}",
                            method, path, caller_id
                        );
                        get_login_data(&db_pool, caller_id)
                    }
                })
                .and_then({
                    let service_factory = service_factory.clone();
                    move |login_data| {
                        match (method.clone(), route.clone()) {
                            (Get, Some(Route::Cart { customer })) => {
                                return if let (Some(from), Some(count)) =
                                    parse_query!(uri.query().unwrap_or_default(), "offset" => ProductId, "count" => i32)
                                {
                                    debug!(
                                        "Received request to get {} products starting from {} for customer {}",
                                        count, from, customer
                                    );
                                    serialize_future((service_factory.cart)(login_data).list(customer, from, count))
                                } else {
                                    serialize_future::<String, _, _>(future::err(
                                        format_err!("Failed to retrieve query parameters from request").context(Error::ParseError),
                                    ))
                                }
                            }
                            (Get, Some(Route::CartProducts { customer })) => {
                                return serialize_future({
                                    debug!("Received request to get cart for customer {}", customer);
                                    (service_factory.cart)(login_data).get_cart(customer)
                                })
                            }
                            (Post, Some(Route::CartClear { customer })) => {
                                return serialize_future({
                                    debug!("Received request to clear cart for customer {}", customer);
                                    (service_factory.cart)(login_data).clear_cart(customer)
                                })
                            }
                            (Delete, Some(Route::CartProduct { customer, product_id })) => {
                                return serialize_future({
                                    debug!(
                                        "Received request to delete product {} from cart for customer {}",
                                        product_id, customer
                                    );
                                    (service_factory.cart)(login_data).delete_item(customer, product_id)
                                })
                            }
                            (
                                Post,
                                Some(Route::AddCartCoupon {
                                    customer,
                                    product_id,
                                    coupon_id,
                                }),
                            ) => {
                                return serialize_future({
                                    debug!(
                                        "Received request to add coupon {} for product {} to cart for customer {}",
                                        coupon_id, product_id, customer
                                    );
                                    (service_factory.cart)(login_data).add_coupon(customer, product_id, coupon_id)
                                })
                            }
                            (Delete, Some(Route::DeleteCartCoupon { customer, coupon_id })) => {
                                return serialize_future({
                                    debug!(
                                        "Received request to delete coupon {} from cart for customer {}",
                                        coupon_id, customer
                                    );
                                    (service_factory.cart)(login_data).delete_coupon(customer, coupon_id)
                                })
                            }
                            (Delete, Some(Route::DeleteCartCouponByProduct { customer, product_id })) => {
                                return serialize_future({
                                    debug!(
                                        "Received request to delete coupon from product {} from cart for customer {}",
                                        product_id, customer
                                    );
                                    (service_factory.cart)(login_data).delete_coupon_by_product(customer, product_id)
                                })
                            }
                            (Post, Some(Route::CartProductDeliveryMethod { customer, product_id })) => {
                                return serialize_future(parse_body::<CartProductDeliveryMethodIdPayload>(payload).and_then(move |params| {
                                    debug!(
                                        "Received request to set delivery method in cart to {:?} for product {} for customer {}",
                                        params.value, product_id, customer
                                    );

                                    (service_factory.cart)(login_data).set_delivery_method(customer, product_id, Some(params.value))
                                }))
                            }
                            (Delete, Some(Route::CartProductDeliveryMethod { customer, product_id })) => {
                                return serialize_future({
                                    debug!(
                                        "Received request to delete delivery method in cart for product {} for customer {}",
                                        product_id, customer
                                    );
                                    (service_factory.cart)(login_data).set_delivery_method(customer, product_id, None)
                                })
                            }
                            (Put, Some(Route::CartProductQuantity { customer, product_id })) => {
                                return serialize_future(
                                    parse_body::<CartProductQuantityPayload>(payload)
                                        .inspect(move |params| {
                                            debug!(
                                                "Received request to set product {} in cart to quantity {} for customer {}",
                                                product_id, params.value, customer
                                            );
                                        })
                                        .and_then(move |params| {
                                            (service_factory.cart)(login_data).set_quantity(customer, product_id, params.value)
                                        }),
                                )
                            }
                            (Put, Some(Route::CartProductSelection { customer, product_id })) => {
                                return serialize_future(
                                    parse_body::<CartProductSelectionPayload>(payload)
                                        .inspect(move |params| {
                                            debug!(
                                                "Received request to set product {}'s selection in cart to {} for customer {}",
                                                product_id, params.value, customer
                                            )
                                        })
                                        .and_then(move |params| {
                                            (service_factory.cart)(login_data).set_selection(customer, product_id, params.value)
                                        }),
                                )
                            }
                            (Put, Some(Route::CartProductComment { customer, product_id })) => {
                                return serialize_future(
                                    parse_body::<CartProductCommentPayload>(payload)
                                        .inspect(move |comment_payload| {
                                            debug!(
                                                "Received request to set product {}'s comment in cart to {} for customer {}",
                                                product_id, comment_payload.value, customer
                                            )
                                        })
                                        .and_then(move |comment_payload| {
                                            (service_factory.cart)(login_data).set_comment(customer, product_id, comment_payload.value)
                                        }),
                                )
                            }
                            (Post, Some(Route::CartIncrementProduct { customer, product_id })) => {
                                return serialize_future({
                                    parse_body::<CartProductIncrementPayload>(payload).and_then(move |data| {
                                        debug!("Received request to increment product {} for customer {}", product_id, customer);
                                        (service_factory.cart)(login_data).increment_item(customer, product_id, data)
                                    })
                                })
                            }
                            (Post, Some(Route::CartMerge)) => {
                                return serialize_future({
                                    parse_body::<CartMergePayload>(payload).and_then(move |data| {
                                        debug!("Received request to merge cart from customer {} to customer {}", data.from, data.to);
                                        (service_factory.cart)(login_data).merge(data.from, data.to)
                                    })
                                })
                            }
                            (Get, Some(Route::OrdersByUser { user })) => {
                                return serialize_future({
                                    debug!("Received request to get orders for user {}", user);
                                    (service_factory.order)(login_data).get_orders_for_user(user)
                                })
                            }
                            (Get, Some(Route::Order { order_id })) => {
                                return serialize_future({
                                    debug!("Received request to get order {:?}", order_id);
                                    (service_factory.order)(login_data).get_order(order_id)
                                })
                            }
                            (Get, Some(Route::OrderDiff { order_id })) => {
                                return serialize_future({
                                    debug!("Received request to get order diff {:?}", order_id);
                                    (service_factory.order)(login_data).get_order_diff(order_id)
                                })
                            }
                            (Put, Some(Route::OrderStatus { order_id })) => {
                                return serialize_future({
                                    parse_body::<UpdateStatePayload>(payload).and_then(move |data| {
                                        debug!("Received request to set order {:?} status {:?}", order_id, data.state);
                                        (service_factory.order)(login_data).set_order_state(
                                            order_id,
                                            data.state,
                                            data.comment,
                                            data.track_id,
                                            data.committer_role,
                                        )
                                    })
                                })
                            }
                            (Post, Some(Route::OrderSearch)) => {
                                return serialize_future({
                                    parse_body::<OrderSearchTerms>(payload)
                                        .and_then(move |terms| (service_factory.order)(login_data).search(terms))
                                })
                            }
                            (Post, Some(Route::OrderFromCart)) => {
                                return serialize_future({
                                    parse_body::<ConvertCartPayload>(payload).and_then(move |payload| {
                                        debug!("Received request to convert cart into orders for user {}", payload.user_id);
                                        payload
                                            .validate()
                                            .map_err(failure::Error::from)
                                            .context("Failed to validate ConvertCartPayload")
                                            .context(Error::ParseError)
                                            .map_err(failure::Error::from)
                                            .into_future()
                                            .and_then(move |_| (service_factory.order)(login_data).convert_cart(payload))
                                    })
                                });
                            }
                            (Post, Some(Route::OrderFromBuyNow)) => {
                                return serialize_future({
                                    parse_body::<BuyNowPayload>(payload).and_then(move |payload| {
                                        debug!("Received request to create order from buy_now data");
                                        payload
                                            .buy_now
                                            .validate()
                                            .map_err(failure::Error::from)
                                            .context("Failed to validate BuyNowPayload")
                                            .context(Error::ParseError)
                                            .map_err(failure::Error::from)
                                            .into_future()
                                            .and_then(move |_| {
                                                (service_factory.order)(login_data).create_buy_now(payload.buy_now, payload.conversion_id)
                                            })
                                    })
                                })
                            }
                            (Post, Some(Route::OrderFromCartRevert)) => {
                                return serialize_future({
                                    parse_body::<ConvertCartRevertPayload>(payload).and_then(move |payload| {
                                        (service_factory.order)(login_data).revert_cart_conversion(payload.conversion_id)
                                    })
                                })
                            }
                            (Delete, Some(Route::Order { order_id })) => {
                                return serialize_future({
                                    debug!("Received request to delete order {:?}", order_id);
                                    (service_factory.order)(login_data).delete_order(order_id)
                                })
                            }
                            (method, Some(Route::Roles(route))) => {
                                let c = RoleController {
                                    service: (service_factory.role)(login_data).into(),
                                };
                                if let Some(out) = c.call(&method, &route, payload) {
                                    return out;
                                }
                            }
                            _ => {}
                        };

                        // Fallback
                        Box::new(future::err(
                            format_err!("Could not route request {} {}", method, uri.path())
                                .context(Error::InvalidRoute)
                                .into(),
                        ))
                    }
                })
                .then(move |res| {
                    let d = Local::now() - dt;
                    info!(
                        "Response error = {:?}, elapsed time = {}.{:03}",
                        res.as_ref().err(),
                        d.num_seconds(),
                        d.num_milliseconds()
                    );

                    if let Err(ref err) = res {
                        let wrapper = ErrorMessageWrapper::<Error>::from(err);
                        if wrapper.inner.code == 500 {
                            log_and_capture_error(err);
                        }
                    }

                    res
                }),
        )
    }
}
