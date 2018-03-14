use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cart {
    pub products: HashMap<i64, i64>,
}
