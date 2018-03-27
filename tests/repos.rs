extern crate orders_lib as lib;

extern crate bb8;
extern crate bb8_postgres;
extern crate futures;
extern crate tokio_core;
extern crate tokio_postgres;

use bb8_postgres::PostgresConnectionManager;
use futures::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_core::reactor::{Core, Remote};
use tokio_postgres::TlsMode;

use lib::config;
use lib::models::*;
use lib::repos::*;

fn prepare_db(remote: Remote) -> Box<Future<Item = bb8::Pool<PostgresConnectionManager>, Error = tokio_postgres::Error>> {
    let config = config::Config::new().unwrap();
    let manager = PostgresConnectionManager::new(config.dsn.clone(), || TlsMode::None).unwrap();

    bb8::Pool::builder().min_idle(Some(10)).build(manager, remote)
}

#[test]
fn test_products_repo() {
    let mut core = Core::new().unwrap();
    let remote = core.remote();
    let pool = Arc::new(core.run(prepare_db(remote)).unwrap());

    let repo = ProductsRepoImpl::new(pool);

    let user_id = 1234;

    let set_a = (5555, 9000);
    let set_b = (5555, 9010);
    let set_c = (4444, 8000);

    // Clear user cart before starting
    assert_eq!(Cart::default(), core.run(repo.clear_cart(user_id)).unwrap());

    // Add the first product
    assert_eq!(
        Cart {
            products: vec![set_a].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.set_item(user_id, set_a.0, set_a.1)).unwrap()
    );

    // Check DB contents
    assert_eq!(
        Cart {
            products: vec![set_a].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.get_cart(user_id)).unwrap()
    );

    // Amend the first product
    assert_eq!(
        Cart {
            products: vec![set_a, set_b].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.set_item(user_id, set_b.0, set_b.1)).unwrap()
    );

    // Add the last product
    assert_eq!(
        Cart {
            products: vec![set_a, set_b, set_c].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.set_item(user_id, set_c.0, set_c.1)).unwrap()
    );

    // Check DB contents
    assert_eq!(
        Cart {
            products: vec![set_a, set_b, set_c].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.get_cart(user_id)).unwrap()
    );

    // Delete the last item
    assert_eq!(
        Cart {
            products: vec![set_a, set_b].into_iter().collect::<HashMap<i32, i32>>(),
        },
        core.run(repo.delete_item(user_id, set_c.0)).unwrap()
    );

    // Clear user cart
    assert_eq!(Cart::default(), core.run(repo.clear_cart(user_id)).unwrap());
}
