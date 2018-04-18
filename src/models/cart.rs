use std::collections::HashMap;

pub struct CartItemInfo {}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cart {
    pub products: HashMap<i32, i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpsertCart {
    pub quantity: i32,
}
