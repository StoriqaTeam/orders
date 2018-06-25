#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "user_id")]
pub struct UserId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "product_id")]
pub struct ProductId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "store_id")]
pub struct StoreId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "quantity")]
pub struct Quantity(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, FromStr, PartialEq, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "product_price")]
pub struct ProductPrice(pub f64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetterPayload<T> {
    pub value: T,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValueContainer<T> {
    pub value: T,
}

impl<T> From<T> for ValueContainer<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}
