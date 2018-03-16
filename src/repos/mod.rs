use failure::Error;
use futures::prelude::*;

use errors;
use errors::*;
use models;
use models::*;
use types;
use types::*;

pub type RepoFuture<T> = Box<Future<Item = T, Error = RepoError>>;

pub trait ProductsRepo {
    fn add(&self, item: CartItem) -> RepoFuture<()>;
    fn clear(&self, user_id: DeleteCart) -> RepoFuture<()>;
}

pub struct ProductsRepoImpl {
    db_pool: DbPool,
}

impl ProductsRepoImpl {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}

impl ProductsRepo for ProductsRepoImpl {
    fn add(&self, item: CartItem) -> RepoFuture<()> {
        println!("Adding started");
        println!("State: {:?}", self.db_pool.state());
        Box::new(
            self.db_pool
                .run(move |conn| {
                    println!("Acquired connection");
                    conn.prepare("INSERT INTO cart_items (user_id, product) VALUES ($1, $2);")
                        .and_then(move |(s, c)| c.execute(&s, &[&item.user_id, &item.product_id]))
                })
                .map(|v| ())
                .map_err(RepoError::from),
        )
    }

    fn clear(&self, args: DeleteCart) -> RepoFuture<()> {
        Box::new(
            self.db_pool
                .run(move |conn| {
                    println!("Acquired connection");
                    conn.prepare("DELETE FROM cart_items WHERE user_id=$1;")
                        .and_then(move |(s, c)| c.execute(&s, &[&args.user_id]))
                })
                .map(|v| ())
                .map_err(RepoError::from),
        )
    }
}
