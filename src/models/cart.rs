use super::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "quantity")]
pub struct Quantity(pub i32);

fn return_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartItemInfo {
    pub quantity: Quantity,
    #[serde(default = "return_true")]
    pub selected: bool,
    pub store_id: StoreId,
}

pub type Cart = HashMap<ProductId, CartItemInfo>;

pub type CartProductQuantityPayload = SetterPayload<Quantity>;
pub type CartProductSelectionPayload = SetterPayload<bool>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartProductIncrementPayload {
    pub store_id: StoreId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartMergePayload {
    pub user_from: UserId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConvertCartPayload {
    pub comment: String,
    pub receiver_name: String,
    pub address: AddressFull,
}

/// Model for vectorized cart
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub store_id: StoreId,
}

impl From<(ProductId, CartItemInfo)> for CartItem {
    fn from(v: (ProductId, CartItemInfo)) -> Self {
        Self {
            product_id: v.0,
            quantity: v.1.quantity,
            selected: v.1.selected,
            store_id: v.1.store_id,
        }
    }
}

impl From<CartProduct> for CartItem {
    fn from(v: CartProduct) -> Self {
        Self {
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
            store_id: v.store_id,
        }
    }
}

impl From<CartProduct> for (ProductId, CartItemInfo) {
    fn from(v: CartProduct) -> (ProductId, CartItemInfo) {
        (
            v.product_id,
            CartItemInfo {
                quantity: v.quantity,
                selected: v.selected,
                store_id: v.store_id,
            },
        )
    }
}
