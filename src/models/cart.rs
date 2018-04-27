use std::collections::HashMap;

pub struct CartItemInfo {}

pub type Cart = HashMap<i32, i32>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpsertCart {
    pub quantity: i32,
}
