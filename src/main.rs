extern crate orders_lib as lib;

fn main() {
    let config = lib::Config::new().expect("Failed to load service configuration. Please check your 'config' folder");
    lib::start_server(config, None, || ());
}
