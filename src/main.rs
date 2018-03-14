extern crate carts_lib;

fn main() {
    let config = carts_lib::Config::from_vars(std::env::vars()).unwrap();

    carts_lib::start_server(config);
}