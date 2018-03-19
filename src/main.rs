extern crate orders_lib;

fn main() {
    let config = orders_lib::Config::from_vars(std::env::vars()).unwrap();
    orders_lib::start_server(config);
}
