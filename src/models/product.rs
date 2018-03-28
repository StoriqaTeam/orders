use tokio_postgres::rows::Row;

#[derive(Clone, Debug)]
pub struct NewProduct {
    user_id: i32,
    product_id: i32,
    quantity: i32,
}

#[derive(Clone, Debug)]
pub struct Product {
    id: i32,
    user_id: i32,
    product_id: i32,
    quantity: i32,
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
