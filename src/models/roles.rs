use models::*;

pub enum Role {
    StoreOwner(StoreId),
    Superadmin,
}
