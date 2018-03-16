use failure::Error;
use futures::prelude::*;
use futures_state_stream::*;
use std::sync::{Arc, Mutex};
use tokio_postgres;
use tokio_postgres::transaction::Transaction;

use errors;
use errors::*;
use models;
use models::*;
use types;
use types::*;

pub type RepoFuture<T> = Box<Future<Item = T, Error = RepoError>>;

pub trait ProductsRepo {
    fn get_cart(&self, user_id: i64) -> RepoFuture<models::Cart>;
    fn set_item(&self, user_id: i64, product_id: i64, quantity: i64) -> RepoFuture<()>;
    fn delete_item(&self, user_id: i64, product_id: i64) -> RepoFuture<()>;
    fn clear_cart(&self, user_id: i64) -> RepoFuture<()>;
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
    fn get_cart(&self, user_id: i64) -> RepoFuture<models::Cart> {
        let out = Arc::new(Mutex::new(models::Cart::default()));
        Box::new(
            self.db_pool
                .run(move |conn| {
                    conn.prepare("SELECT * FROM cart_items WHERE user_id = $1")
                        .and_then({
                            let out = out.clone();
                            move |(s, c)| {
                                c.query(&s, &[&user_id]).for_each({
                                    let out = out.clone();
                                    move |row| {
                                        let product_id = row.get("product_id");
                                        let quantity = row.get("quantity");

                                        out.lock().unwrap().products.insert(product_id, quantity);
                                    }
                                })
                            }
                        })
                        .map({
                            let out = out.clone();
                            move |c| {
                                let g = out.lock().unwrap();
                                ((*g).clone(), c)
                            }
                        })
                })
                .map_err(RepoError::from),
        )
    }

    fn set_item(&self, user_id: i64, product_id: i64, quantity: i64) -> RepoFuture<()> {
        Box::new(
            self.db_pool
                .run(move |conn| {
                    conn.prepare(
                        "
                        INSERT INTO cart_items (user_id, product_id, quantity)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (user_id, product_id)
                        DO UPDATE SET quantity = $3
                        ;",
                    ).and_then(move |(s, c)| c.execute(&s, &[&user_id, &product_id, &quantity]))
                })
                .map(|v| ())
                .map_err(RepoError::from),
        )
    }

    fn delete_item(&self, user_id: i64, product_id: i64) -> RepoFuture<()> {
        Box::new(
            self.db_pool
                .run(move |conn| {
                    println!("Acquired connection");
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1 AND product_id = $2;")
                        .and_then(move |(s, c)| c.execute(&s, &[&user_id, &product_id]))
                })
                .map(|v| ())
                .map_err(RepoError::from),
        )
    }

    fn clear_cart(&self, user_id: i64) -> RepoFuture<()> {
        Box::new(
            self.db_pool
                .run(move |conn| {
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1;")
                        .and_then(move |(s, c)| c.execute(&s, &[&user_id]))
                })
                .map(|_| ())
                .map_err(RepoError::from),
        )
    }
}
