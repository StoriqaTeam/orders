#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CartItem {
    pub user_id: i64,
    pub product_id: i64,
}
