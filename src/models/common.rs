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
