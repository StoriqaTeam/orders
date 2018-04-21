pub mod product;
pub use self::product::*;

pub mod order;
pub use self::order::*;

pub mod product_info;
pub use self::product_info::*;

pub use errors::RepoError;

use futures::prelude::*;
use stq_db::connection::*;

pub type RepoFuture<T> = Box<Future<Item = T, Error = RepoError>>;

pub type RepoConnection = BoxedConnection<RepoError>;
pub type RepoConnectionFuture<T> = ConnectionFuture<T, RepoError>;
