extern crate orders_lib;

fn main() {
    let config = orders_lib::config::Config::new()
        .expect("Failed to load service configuration. Please check your 'config' folder");
    orders_lib::start_server(config);
}
