#[derive(Clone, Copy, Debug, Default, Display, Eq, From, FromStr, Into, PartialEq, Hash, Serialize, Deserialize)]
pub struct UserId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, From, FromStr, Into, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProductId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, From, FromStr, Into, PartialEq, Hash, Serialize, Deserialize)]
pub struct StoreId(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, Eq, From, FromStr, Into, PartialEq, Hash, Serialize, Deserialize)]
pub struct Quantity(pub i32);

#[derive(Clone, Copy, Debug, Default, Display, From, FromStr, Into, PartialEq, Serialize, Deserialize)]
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
