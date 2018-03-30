use futures::prelude::*;

use errors::*;

pub type ServiceFuture<T> = Box<Future<Item = T, Error = RepoError>>;
