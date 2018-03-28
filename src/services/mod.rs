use futures::prelude::*;
use futures_state_stream::*;
use std::sync::{Arc, Mutex};
use tokio_postgres;

use errors::*;
use log;
use models::*;
use repos::*;
use types::*;

pub type ServiceFuture<T> = Box<Future<Item = T, Error = RepoError>>;

pub trait CartService {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart>;
    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart>;
    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart>;
}

pub struct CartServiceImpl {
    db_pool: DbPool,
    repo_factory: Arc<Fn(RepoConnection) -> Box<ProductRepo>>,
}

impl CartServiceImpl {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool, repo_factory: Arc::new(|conn| Box::new(ProductRepoImpl::new(conn))) }
    }
}

impl CartServiceImpl {
    fn _get_cart(
        conn: tokio_postgres::Connection,
        user_id: i32,
    ) -> Box<Future<Item = (Cart, tokio_postgres::Connection), Error = (tokio_postgres::Error, tokio_postgres::Connection)>> {
        let out = Arc::new(Mutex::new(Cart::default()));

        Box::new(
            conn.prepare("SELECT * FROM cart_items WHERE user_id = $1")
                .and_then({
                    let out = out.clone();
                    move |(statement, conn)| {
                        conn.query(&statement, &[&user_id]).for_each({
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
                    move |conn| {
                        let guard = out.lock().unwrap();
                        ((*guard).clone(), conn)
                    }
                }),
        )
    }
}

impl CartService for CartServiceImpl {
    fn get_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Getting cart for user {}.", user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    Self::_get_cart(conn, user_id)
                })
                .map_err(RepoError::from),
        )
    }

    fn set_item(&self, user_id: i32, product_id: i32, quantity: i32) -> ServiceFuture<Cart> {
        debug!("Setting item {} to quantity {} in cart for user {}.", product_id, quantity, user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare(
                        "
                        INSERT INTO cart_items (user_id, product_id, quantity)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (user_id, product_id)
                        DO UPDATE SET quantity = $3
                        ;",
                    ).and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id, &product_id, &quantity]))
                        .map(|(_, conn)| conn)
                        .and_then(move |conn| Self::_get_cart(conn, user_id))
                })
                .map_err(RepoError::from),
        )
    }

    fn delete_item(&self, user_id: i32, product_id: i32) -> ServiceFuture<Cart> {
        debug!("Deleting item {} for user {}", product_id, user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1 AND product_id = $2;")
                        .and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id, &product_id]))
                        .map(|(_, conn)| conn)
                        .and_then(move |conn| Self::_get_cart(conn, user_id))
                })
                .map_err(RepoError::from),
        )
    }

    fn clear_cart(&self, user_id: i32) -> ServiceFuture<Cart> {
        debug!("Clearing cart for user {}", user_id);
        Box::new(
            self.db_pool
                .run(move |conn| {
                    log::acquired_db_connection(&conn);
                    conn.prepare("DELETE FROM cart_items WHERE user_id = $1;")
                        .and_then(move |(statement, conn)| conn.execute(&statement, &[&user_id]))
                        .map(|(_, conn)| conn)
                        .and_then(move |conn| Self::_get_cart(conn, user_id))
                })
                .map_err(RepoError::from),
        )
    }
}
