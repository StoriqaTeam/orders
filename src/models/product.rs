use super::*;

use stq_db::statement::*;
use tokio_postgres::rows::Row;

pub type CartProductId = i32;

const ID_COLUMN: &'static str = "id";
const USER_ID_COLUMN: &'static str = "user_id";
const PRODUCT_ID_COLUMN: &'static str = "product_id";
const QUANTITY_COLUMN: &'static str = "quantity";
const SELECTED_COLUMN: &'static str = "selected";
const STORE_ID_COLUMN: &'static str = "store_id";

#[derive(Clone, Debug)]
pub struct NewCartProduct {
    pub user_id: CartProductId,
    pub product_id: i32,
    pub quantity: i32,
    pub selected: bool,
    pub store_id: i32,
}

impl Inserter for NewCartProduct {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(USER_ID_COLUMN, self.user_id)
            .with_arg(PRODUCT_ID_COLUMN, self.product_id)
            .with_arg(QUANTITY_COLUMN, self.quantity)
            .with_arg(SELECTED_COLUMN, self.selected)
            .with_arg(STORE_ID_COLUMN, self.store_id)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CartProductInserter {
    Upserter(NewCartProduct),
    CollisionNoOp(NewCartProduct),
}

impl Inserter for CartProductInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        use self::CartProductInserter::*;

        match self {
            Upserter(data) => data.into_insert_builder(table)
                .with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = $2"),
            CollisionNoOp(data) => data.into_insert_builder(table)
                .with_extra("ON CONFLICT (user_id, product_id) DO NOTHING"),
        }
    }
}

/// Base unit of user's product selection
#[derive(Clone, Debug)]
pub struct CartProduct {
    pub id: CartProductId,
    pub user_id: i32,
    pub product_id: ProductId,
    pub quantity: i32,
    pub selected: bool,
    pub store_id: i32,
}

impl CartProduct {
    pub fn decompose(self) -> (CartProductId, NewCartProduct) {
        (
            self.id,
            NewCartProduct {
                user_id: self.user_id,
                product_id: self.product_id,
                quantity: self.quantity,
                selected: self.selected,
                store_id: self.store_id,
            },
        )
    }
}

impl From<Row> for CartProduct {
    fn from(row: Row) -> Self {
        Self {
            id: row.get(ID_COLUMN),
            user_id: row.get(USER_ID_COLUMN),
            product_id: row.get(PRODUCT_ID_COLUMN),
            quantity: row.get(QUANTITY_COLUMN),
            selected: row.get(SELECTED_COLUMN),
            store_id: row.get(STORE_ID_COLUMN),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartProductMask {
    pub id: Option<CartProductId>,
    pub user_id: Option<Range<i32>>,
    pub product_id: Option<Range<i32>>,
    pub quantity: Option<Range<i32>>,
    pub selected: Option<bool>,
    pub store_id: Option<Range<i32>>,
}

impl Filter for CartProductMask {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v);
        }

        if let Some(v) = self.product_id {
            b = b.with_filter(PRODUCT_ID_COLUMN, v);
        }

        if let Some(v) = self.user_id {
            b = b.with_filter(USER_ID_COLUMN, v);
        }

        if let Some(v) = self.selected {
            b = b.with_filter(SELECTED_COLUMN, v);
        }

        if let Some(v) = self.store_id {
            b = b.with_filter(STORE_ID_COLUMN, v);
        }

        b
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartProductUpdateData {
    pub quantity: Option<i32>,
    pub selected: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct CartProductUpdate {
    pub mask: CartProductMask,
    pub data: CartProductUpdateData,
}

impl Updater for CartProductUpdate {
    fn into_update_builder(self, table: &'static str) -> UpdateBuilder {
        let Self { mask, data } = self;

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(table));

        if let Some(selected) = data.selected {
            b = b.with_value(SELECTED_COLUMN, selected);
        }

        if let Some(quantity) = data.quantity {
            b = b.with_value(QUANTITY_COLUMN, quantity);
        }

        b
    }
}
