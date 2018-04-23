use std::collections::HashMap;

pub struct CartItemInfo {}

pub type CartProducts = HashMap<i32, i32>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cart {
    pub products: CartProducts,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpsertCart {
    pub quantity: i32,
}
