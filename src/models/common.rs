#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "user_id")]
pub struct UserId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "product_id")]
pub struct ProductId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, FromStr, PartialEq, Hash, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "store_id")]
pub struct StoreId(pub i32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetterPayload<T> {
    pub value: T,
}
