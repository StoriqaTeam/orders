use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cart {
    pub products: HashMap<i32, i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UpsertCart {
    pub quantity: i32,
}
