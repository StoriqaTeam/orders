use futures::future;
use futures::prelude::*;
use hyper::Method::Get;
use stq_http::client::ClientHandle as HttpClientHandle;

use super::{RepoError, RepoFuture};
use models::*;

pub trait ProductInfoRepo {
    fn get_store_id(&self, product_id: i32) -> RepoFuture<i32>;
}

#[derive(Clone)]
pub struct ProductInfoRepoImpl {
    http_client: HttpClientHandle,
    stores_addr: String,
}

impl ProductInfoRepoImpl {
    pub fn new(http_client: HttpClientHandle, stores_addr: String) -> Self {
        Self { stores_addr, http_client }
    }
}

impl ProductInfoRepo for ProductInfoRepoImpl {
    fn get_store_id(&self, product_id: i32) -> RepoFuture<i32> {
        Box::new(
            future::ok(())
                .and_then({
                    let http_client = self.http_client.clone();
                    let stores_addr = self.stores_addr.clone();
                    move |_| http_client.request::<ProductInfo>(Get, format!("{}/products/{}", stores_addr, product_id), None, None)
                })
                .and_then({
                    let http_client = self.http_client.clone();
                    let stores_addr = self.stores_addr.clone();
                    move |product_info| {
                        http_client.request::<BaseProductInfo>(
                            Get,
                            format!("{}/base_products/{}", stores_addr, product_info.base_product_id),
                            None,
                            None,
                        )
                    }
                })
                .map_err(|e| RepoError::Connection { reason: e.to_string() })
                .map(|base_product_info| base_product_info.store_id),
        )
    }
}
