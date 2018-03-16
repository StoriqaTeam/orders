use std::collections::HashMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cart {
    pub products: HashMap<i64, i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetProductParams {
    pub quantity: i64,
}