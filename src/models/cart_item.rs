use either::Either;
use stq_db::statement::*;
use stq_types::*;
use tokio_postgres::rows::Row;
use uuid::Uuid;

const ID_COLUMN: &str = "id";
const PRODUCT_ID_COLUMN: &str = "product_id";
const QUANTITY_COLUMN: &str = "quantity";
const SELECTED_COLUMN: &str = "selected";
const COMMENT_COLUMN: &str = "comment";
const STORE_ID_COLUMN: &str = "store_id";

const USER_ID_COLUMN: &str = "user_id";
const SESSION_ID_COLUMN: &str = "session_id";

#[derive(Clone, Debug)]
pub struct CartItemUser {
    pub id: CartItemId,
    pub user_id: UserId,
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

#[derive(Clone, Debug)]
pub struct CartItemSession {
    pub id: CartItemId,
    pub session_id: SessionId,
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

impl From<CartItemUser> for CartItem {
    fn from(v: CartItemUser) -> Self {
        Self {
            id: v.id,
            customer: CartCustomer::User(v.user_id),
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
            comment: v.comment,
            store_id: v.store_id,
        }
    }
}

impl From<CartItemSession> for CartItem {
    fn from(v: CartItemSession) -> Self {
        Self {
            id: v.id,
            customer: CartCustomer::Anonymous(v.session_id),
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
            comment: v.comment,
            store_id: v.store_id,
        }
    }
}

impl Inserter for CartItemUser {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(COMMENT_COLUMN, self.comment)
            .with_arg(ID_COLUMN, self.id.0)
            .with_arg(PRODUCT_ID_COLUMN, self.product_id.0)
            .with_arg(QUANTITY_COLUMN, self.quantity.0)
            .with_arg(SELECTED_COLUMN, self.selected)
            .with_arg(STORE_ID_COLUMN, self.store_id.0)
            .with_arg(USER_ID_COLUMN, self.user_id.0)
    }
}

impl Inserter for CartItemSession {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        InsertBuilder::new(table)
            .with_arg(COMMENT_COLUMN, self.comment)
            .with_arg(ID_COLUMN, self.id.0)
            .with_arg(PRODUCT_ID_COLUMN, self.product_id.0)
            .with_arg(QUANTITY_COLUMN, self.quantity.0)
            .with_arg(SELECTED_COLUMN, self.selected)
            .with_arg(STORE_ID_COLUMN, self.store_id.0)
            .with_arg(SESSION_ID_COLUMN, self.session_id.0)
    }
}

impl CartItemUser {
    pub fn new(user_id: UserId, product_id: ProductId, store_id: StoreId) -> Self {
        Self {
            user_id,
            product_id,
            store_id,

            id: CartItemId::new(),
            quantity: Quantity(1),
            selected: true,
            comment: String::new(),
        }
    }
}

impl CartItemSession {
    pub fn new(session_id: SessionId, product_id: ProductId, store_id: StoreId) -> Self {
        Self {
            session_id,
            product_id,
            store_id,

            id: CartItemId::new(),
            quantity: Quantity(1),
            selected: true,
            comment: String::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum CartItemMergeStrategy {
    Standard,
    Replacer,
    Incrementer,
    CollisionNoOp,
}

#[derive(Clone, Debug)]
pub struct CartItemInserter {
    pub strategy: CartItemMergeStrategy,
    pub data: CartItem,
}

pub fn split_cart_item(v: CartItem) -> Either<CartItemUser, CartItemSession> {
    use self::CartCustomer::*;

    match v.customer {
        User(user_id) => Either::Left(CartItemUser {
            user_id,
            id: v.id,
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
            comment: v.comment,
            store_id: v.store_id,
        }),
        Anonymous(session_id) => Either::Right(CartItemSession {
            session_id,
            id: v.id,
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
            comment: v.comment,
            store_id: v.store_id,
        }),
    }
}

impl Inserter for CartItemInserter {
    fn into_insert_builder(self, _table: &'static str) -> InsertBuilder {
        unreachable!()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartItemUpdateData {
    pub quantity: Option<Quantity>,
    pub selected: Option<bool>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CartItemUserInserter {
    pub strategy: CartItemMergeStrategy,
    pub data: CartItemUser,
}

#[derive(Clone, Debug)]
pub struct CartItemSessionInserter {
    pub strategy: CartItemMergeStrategy,
    pub data: CartItemSession,
}

impl Inserter for CartItemUserInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        use self::CartItemMergeStrategy::*;

        let b = self.data.into_insert_builder(table);

        match self.strategy {
            Standard => b,
            Replacer => b.with_extra(
                "\
                 ON CONFLICT (user_id, product_id) DO UPDATE SET \
                 comment = EXCLUDED.comment, \
                 id = EXCLUDED.id, \
                 product_id = EXCLUDED.product_id, \
                 quantity = EXCLUDED.quantity, \
                 selected = EXCLUDED.selected, \
                 store_id = EXCLUDED.store_id, \
                 user_id = EXCLUDED.user_id\
                 ",
            ),
            Incrementer => b.with_extra("ON CONFLICT (user_id, product_id) DO UPDATE SET quantity = cart_items.quantity + 1"),
            CollisionNoOp => b.with_extra("ON CONFLICT (user_id, product_id) DO NOTHING"),
        }
    }
}

impl Inserter for CartItemSessionInserter {
    fn into_insert_builder(self, table: &'static str) -> InsertBuilder {
        use self::CartItemMergeStrategy::*;

        let b = self.data.into_insert_builder(table);

        match self.strategy {
            Standard => b,
            Replacer => b.with_extra(
                "\
                 ON CONFLICT (session_id, product_id) DO UPDATE SET \
                 comment = EXCLUDED.comment, \
                 id = EXCLUDED.id, \
                 product_id = EXCLUDED.product_id, \
                 quantity = EXCLUDED.quantity, \
                 selected = EXCLUDED.selected, \
                 store_id = EXCLUDED.store_id, \
                 session_id = EXCLUDED.session_id\
                 ",
            ),
            Incrementer => b.with_extra("ON CONFLICT (session_id, product_id) DO UPDATE SET quantity = cart_items.quantity + 1"),
            CollisionNoOp => b.with_extra("ON CONFLICT (session_id, product_id) DO NOTHING"),
        }
    }
}

impl From<Row> for CartItemUser {
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

impl From<Row> for CartItemSession {
    fn from(row: Row) -> Self {
        Self {
            id: CartItemId(row.get(ID_COLUMN)),
            session_id: SessionId(row.get(SESSION_ID_COLUMN)),
            product_id: ProductId(row.get(PRODUCT_ID_COLUMN)),
            quantity: Quantity(row.get(QUANTITY_COLUMN)),
            selected: row.get(SELECTED_COLUMN),
            comment: row.get(COMMENT_COLUMN),
            store_id: StoreId(row.get(STORE_ID_COLUMN)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartItemMetaFilter {
    pub id: Option<Uuid>,
    pub product_id: Option<Range<ProductId>>,
    pub quantity: Option<Range<Quantity>>,
    pub selected: Option<bool>,
    pub comment: Option<String>,
    pub store_id: Option<Range<StoreId>>,
}

impl CartItemMetaFilter {
    pub fn write_into_filtered_operation_builder(self, mut b: FilteredOperationBuilder) -> FilteredOperationBuilder {
        if let Some(v) = self.id {
            b = b.with_filter(ID_COLUMN, v);
        }

        if let Some(v) = self.product_id {
            b = b.with_filter::<i32, _>(PRODUCT_ID_COLUMN, v.convert());
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

impl Filter for CartItemMetaFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let b = FilteredOperationBuilder::new(table);

        self.write_into_filtered_operation_builder(b)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartItemFilter {
    pub meta_filter: CartItemMetaFilter,
    pub customer: Option<CartCustomer>,
}

impl Filter for CartItemFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        if let Some(customer) = self.customer {
            match customer {
                CartCustomer::User(user_id) => CartItemUserFilter {
                    meta_filter: self.meta_filter,
                    user_id: Some(user_id),
                }.into_filtered_operation_builder(table),
                CartCustomer::Anonymous(session_id) => CartItemSessionFilter {
                    meta_filter: self.meta_filter,
                    session_id: Some(session_id),
                }.into_filtered_operation_builder(table),
            }
        } else {
            self.meta_filter.into_filtered_operation_builder(table)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartItemUserFilter {
    pub meta_filter: CartItemMetaFilter,
    pub user_id: Option<UserId>,
}

impl From<CartItemMetaFilter> for CartItemUserFilter {
    fn from(meta_filter: CartItemMetaFilter) -> Self {
        Self {
            meta_filter,
            user_id: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CartItemSessionFilter {
    pub meta_filter: CartItemMetaFilter,
    pub session_id: Option<SessionId>,
}

impl From<CartItemMetaFilter> for CartItemSessionFilter {
    fn from(meta_filter: CartItemMetaFilter) -> Self {
        Self {
            meta_filter,
            session_id: None,
        }
    }
}

impl Filter for CartItemUserFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.user_id {
            b = b.with_filter(USER_ID_COLUMN, v.0);
        }

        b = self.meta_filter.write_into_filtered_operation_builder(b);

        b
    }
}

impl Filter for CartItemSessionFilter {
    fn into_filtered_operation_builder(self, table: &'static str) -> FilteredOperationBuilder {
        let mut b = FilteredOperationBuilder::new(table);

        if let Some(v) = self.session_id {
            b = b.with_filter(SESSION_ID_COLUMN, v.0);
        }

        b = self.meta_filter.write_into_filtered_operation_builder(b);

        b
    }
}

#[derive(Clone, Debug)]
pub struct CartItemUpdater<F> {
    pub filter: F,
    pub data: CartItemUpdateData,
}

impl<F> Updater for CartItemUpdater<F>
where
    F: Filter,
{
    fn into_update_builder(self, table: &'static str) -> UpdateBuilder {
        let Self { filter, data } = self;

        let mut b = UpdateBuilder::from(filter.into_filtered_operation_builder(table));

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

pub type CartItemUserUpdater = CartItemUpdater<CartItemUserFilter>;
pub type CartItemSessionUpdater = CartItemUpdater<CartItemSessionFilter>;
