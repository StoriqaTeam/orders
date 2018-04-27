use std::collections::HashMap;
use super::*;

pub struct CartItemInfo {}

pub type Cart = HashMap<i32, i32>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpsertCart {
    pub quantity: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: i32,
    pub quantity: i32,
}

impl From<Product> for CartItem {
    fn from(v: Product) -> Self {
        Self {
            product_id: v.product_id,
            quantity: v.quantity,
        }
    }
}
