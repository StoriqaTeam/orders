use failure::Error as FailureError;
use futures::prelude::*;
use serde_json;
use tokio_core::reactor::Handle;

use super::*;
use stq_api::orders::*;
use stq_http::client::{Client as HttpClient, ClientHandle as HttpClientHandle, Config as HttpConfig};

#[derive(Clone)]
pub struct SagaClient {
    url: String,
    handle: HttpClientHandle,
}

pub trait SagaService {
    fn set_order_completed(&self, order: Order) -> Box<Future<Item = (), Error = FailureError>>;
}

impl SagaClient {
    pub fn new(handle: &Handle, url: String) -> SagaClient {
        let client = HttpClient::new(
            &HttpConfig {
                http_client_retries: 3,
                http_client_buffer_size: 3,
                timeout_duration_ms: 5000,
            },
            handle,
        );
        let client_handle = client.handle();
        handle.spawn(client.stream().for_each(|_| Ok(())));
        SagaClient {
            handle: client_handle,
            url,
        }
    }

    fn base_url(&self) -> String {
        self.url.clone()
    }

    fn request_url(&self, request: &str) -> String {
        format!("{}/{}", self.base_url(), request)
    }
}

impl SagaService for SagaClient {
    fn set_order_completed(&self, order: Order) -> Box<Future<Item = (), Error = FailureError>> {
        let request_path = format!("orders/{}/set_payment_state", order.id);
        let url = self.request_url(&request_path);
        let payload = OrderPaymentStateRequest {
            state: PaymentState::PaymentToSellerNeeded,
        };
        let self_clone = self.clone();
        Box::new(
            serde_json::to_string(&payload)
                .map_err(FailureError::from)
                .into_future()
                .and_then(move |body| {
                    self_clone
                        .handle
                        .request::<()>(::hyper::Method::Post, url, Some(body), None)
                        .map_err(From::from)
                })
                .map_err(From::from),
        )
    }
}
