#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductInfo {
    pub base_product_id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseProductInfo {
    pub store_id: i32,
}
