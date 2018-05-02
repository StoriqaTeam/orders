use std::collections::HashMap;
use super::*;

pub type ProductId = i32;

fn return_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartItemInfo {
    pub quantity: i32,
    #[serde(default = "return_true")]
    pub selected: bool,
}

pub type Cart = HashMap<ProductId, CartItemInfo>;

pub type CartProductQuantityPayload = SetterPayload<i32>;
pub type CartProductSelectionPayload = SetterPayload<bool>;

/// Model for vectorized cart
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: i32,
    pub quantity: i32,
    pub selected: bool,
}

impl From<(ProductId, CartItemInfo)> for CartItem {
    fn from(v: (ProductId, CartItemInfo)) -> Self {
        Self {
            product_id: v.0,
            quantity: v.1.quantity,
            selected: v.1.selected,
        }
    }
}

impl From<CartProduct> for CartItem {
    fn from(v: CartProduct) -> Self {
        Self {
            product_id: v.product_id,
            quantity: v.quantity,
            selected: v.selected,
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
            },
        )
    }
}
