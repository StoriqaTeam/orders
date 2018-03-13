use failure::Error;

use types;
use types::*;

#[derive(Debug, Fail)]
pub enum RepoError {

}

pub trait ProductsRepo {
    fn add(user_id: i64, product_id: i64) -> Box<Future<Item=(), Error=RepoError>>;
}

pub struct ProductsRepoImpl<'a> {
    pub db_conn: &'a DbConnection,
}