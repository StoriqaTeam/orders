extern crate orders_lib;
extern crate stq_logging;

fn main() {
    let config = orders_lib::config::Config::new().expect("Can't load app config!");

    // Prepare sentry integration
    let _sentry = orders_lib::sentry_integration::init(config.sentry.as_ref());

    // Prepare logger
    stq_logging::init(config.graylog.as_ref());

    orders_lib::start_delivered_orders_loader(config);
}
