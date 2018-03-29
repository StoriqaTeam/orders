use tokio_postgres::rows::Row;

#[derive(Clone, Debug)]
pub struct NewProduct {
    pub user_id: i32,
    pub product_id: i32,
    pub quantity: i32,
}

#[derive(Clone, Debug)]
pub struct Product {
    pub id: i32,
    pub user_id: i32,
    pub product_id: i32,
    pub quantity: i32,
}

impl From<Row> for Product {
    fn from(row: Row) -> Self {
        Self {
            id: row.get("id"),
            user_id: row.get("user_id"),
            product_id: row.get("product_id"),
            quantity: row.get("quantity"),
        }
    }
}
