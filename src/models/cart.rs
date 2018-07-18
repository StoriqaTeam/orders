use std::collections::HashMap;

use stq_types::*;

use super::*;

fn return_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartItemInfo {
    pub quantity: Quantity,
    #[serde(default = "return_true")]
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

pub type Cart = HashMap<ProductId, CartItemInfo>;

pub type CartProductQuantityPayload = SetterPayload<Quantity>;
pub type CartProductSelectionPayload = SetterPayload<bool>;
pub type CartProductCommentPayload = SetterPayload<String>;

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
    pub conversion_id: Option<ConversionId>,
    pub customer_id: UserId,
    pub receiver_name: String,
    #[serde(flatten)]
    pub address: AddressFull,
    pub prices: HashMap<ProductId, ProductSellerPrice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConvertCartRevertPayload {
    pub conversion_id: ConversionId,
}

/// Model for vectorized cart
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: ProductId,
    pub quantity: Quantity,
    pub selected: bool,
    pub comment: String,
    pub store_id: StoreId,
}

impl CartItem {
    pub fn into_meta(self) -> (ProductId, CartItemInfo) {
        (
            self.product_id,
            CartItemInfo {
                quantity: self.quantity,
                selected: self.selected,
                comment: self.comment,
                store_id: self.store_id,
            },
        )
    }
}

impl From<(ProductId, CartItemInfo)> for CartItem {
    fn from(v: (ProductId, CartItemInfo)) -> Self {
        Self {
            product_id: v.0,
            quantity: v.1.quantity,
            selected: v.1.selected,
            comment: v.1.comment,
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
            comment: v.comment,
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
                comment: v.comment,
                store_id: v.store_id,
            },
        )
    }
}
