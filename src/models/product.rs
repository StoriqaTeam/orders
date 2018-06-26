use super::*;

use stq_db::statement::*;
use tokio_postgres::rows::Row;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize)]
pub struct CartItemId(pub i32);

const ID_COLUMN: &'static str = "id";
const USER_ID_COLUMN: &'static str = "user_id";
const PRODUCT_ID_COLUMN: &'static str = "product_id";
const QUANTITY_COLUMN: &'static str = "quantity";
const SELECTED_COLUMN: &'static str = "selected";
const COMMENT_COLUMN: &'static str = "comment";
const STORE_ID_COLUMN: &'static str = "store_id";

#[derive(Clone, Debug)]
pub struct NewCartProduct {
    pub user_id: UserId,
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

impl NewCartProduct {
    pub fn new(user_id: UserId, product_id: ProductId, store_id: StoreId) -> Self {
        NewCartProduct {
            user_id,
            product_id,
            store_id,

            quantity: Quantity(1),
            selected: true,
            comment: String::new(),
        }
    }
}

impl Inserter for NewCartProduct {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(USER_ID_COLUMN, self.user_id.0)
            .with_arg(PRODUCT_ID_COLUMN, self.product_id.0)
            .with_arg(QUANTITY_COLUMN, self.quantity.0)
            .with_arg(SELECTED_COLUMN, self.selected)
            .with_arg(COMMENT_COLUMN, self.comment)
            .with_arg(STORE_ID_COLUMN, self.store_id.0)
    }
}

#[derive(Clone, Debug)]
pub enum CartProductInserter {
    Incrementer(NewCartProduct),
    CollisionNoOp(NewCartProduct),
}

impl Inserter for CartProductInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        use self::CartProductInserter::*;

        match self {
            Incrementer(data) => data.into_insert_builder(table)
                .with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = cart_items.quantity + 1"),
            CollisionNoOp(data) => data.into_insert_builder(table)
                .with_extra("ON CONFLICT (user_id, product_id) DO NOTHING"),
        }
    }
}

/// Base unit of user's product selection
#[derive(Clone, Debug)]
pub struct CartProduct {
    pub id: CartItemId,
    pub user_id: UserId,
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

impl CartProduct {
    pub fn decompose(self) -> (CartItemId, NewCartProduct) {
        (
            self.id,
            NewCartProduct {
                user_id: self.user_id,
                product_id: self.product_id,
                quantity: self.quantity,
                selected: self.selected,
                comment: self.comment,
                store_id: self.store_id,
            },
        )
    }
}

impl From<Row> for CartProduct {
    fn from(row: Row) -> Self {
        Self {
            id: CartItemId(row.get(ID_COLUMN)),
            user_id: UserId(row.get(USER_ID_COLUMN)),
            product_id: ProductId(row.get(PRODUCT_ID_COLUMN)),
            quantity: Quantity(row.get(QUANTITY_COLUMN)),
            selected: row.get(SELECTED_COLUMN),
            comment: row.get(COMMENT_COLUMN),
            store_id: StoreId(row.get(STORE_ID_COLUMN)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartProductMask {
    pub id: Option<Uuid>,
    pub user_id: Option<Range<UserId>>,
    pub product_id: Option<Range<ProductId>>,
    pub quantity: Option<Range<Quantity>>,
    pub selected: Option<bool>,
    pub comment: Option<String>,
    pub store_id: Option<Range<StoreId>>,
}

impl Filter for CartProductMask {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v);
        }

        if let Some(v) = self.product_id {
            b = b.with_filter::<i32, _>(PRODUCT_ID_COLUMN, v.convert());
        }

        if let Some(v) = self.user_id {
            b = b.with_filter::<i32, _>(USER_ID_COLUMN, v.convert());
        }

        if let Some(v) = self.selected {
            b = b.with_filter(SELECTED_COLUMN, v);
        }

        if let Some(v) = self.comment {
            b = b.with_filter(COMMENT_COLUMN, v);
        }

        if let Some(v) = self.store_id {
            b = b.with_filter::<i32, _>(STORE_ID_COLUMN, v.convert());
        }

        b
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartProductUpdateData {
    pub quantity: Option<Quantity>,
    pub selected: Option<bool>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CartProductUpdater {
    pub mask: CartProductMask,
    pub data: CartProductUpdateData,
}

impl Updater for CartProductUpdater {
    fn into_update_builder(self, table: &'static str) -> UpdateBuilder {
        let Self { mask, data } = self;

        let mut b = UpdateBuilder::from(mask.into_filtered_operation_builder(table));

        if let Some(v) = data.selected {
            b = b.with_value(SELECTED_COLUMN, v);
        }

        if let Some(v) = data.quantity {
            b = b.with_value(QUANTITY_COLUMN, v.0);
        }

        if let Some(v) = data.comment {
            b = b.with_value(COMMENT_COLUMN, v);
        }

        b
    }
}
