use super::*;

use stq_db::repo::*;
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

pub struct UpsertCartProduct {
    pub inner: NewCartProduct,
}

impl Inserter for UpsertCartProduct {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        self.inner
            .into_insert_builder(table)
            .with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = $2")
    }
}

impl From<NewCartProduct> for UpsertCartProduct {
    fn from(v: NewCartProduct) -> Self {
        Self { inner: v }
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

impl From<CartProduct> for (CartProductId, NewCartProduct) {
    fn from(product: CartProduct) -> Self {
        (
            product.id,
            NewCartProduct {
                user_id: product.user_id,
                product_id: product.product_id,
                quantity: product.quantity,
                selected: product.selected,
                store_id: product.store_id,
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
    pub product_id: Option<i32>,
    pub user_id: Option<i32>,
    pub selected: Option<bool>,
    pub store_id: Option<i32>,
}

impl Filter for CartProductMask {
    fn into_filtered_operation_builder(self, op: FilteredOperation, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(op, table);

        if let Some(id) = self.id {
            b = b.with_arg(ID_COLUMN, id);
        }

        if let Some(product_id) = self.product_id {
            b = b.with_arg(PRODUCT_ID_COLUMN, product_id);
        }

        if let Some(user_id) = self.user_id {
            b = b.with_arg(USER_ID_COLUMN, user_id);
        }

        if let Some(selected) = self.selected {
            b = b.with_arg(SELECTED_COLUMN, selected);
        }

        if let Some(store_id) = self.store_id {
            b = b.with_arg(STORE_ID_COLUMN, store_id);
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

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(FilteredOperation::Select, table));

        if let Some(selected) = data.selected {
            b = b.with_value(SELECTED_COLUMN, selected);
        }

        if let Some(quantity) = data.quantity {
            b = b.with_value(QUANTITY_COLUMN, quantity);
        }

        b
    }
}
