pub type UserId = i32;
pub type ProductId = i32;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StoreId(pub i32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetterPayload<T> {
    pub value: T,
}
