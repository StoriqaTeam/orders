use tokio_postgres::rows::Row;

macro_rules! ID_COLUMN {
    () => {
        "id"
    };
}
macro_rules! USER_ID_COLUMN {
    () => {
        "user_id"
    };
}
macro_rules! PRODUCT_ID_COLUMN {
    () => {
        "product_id"
    };
}
macro_rules! QUANTITY_COLUMN {
    () => {
        "quantity"
    };
}

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
            id: row.get(ID_COLUMN!()),
            user_id: row.get(USER_ID_COLUMN!()),
            product_id: row.get(PRODUCT_ID_COLUMN!()),
            quantity: row.get(QUANTITY_COLUMN!()),
        }
    }
}
