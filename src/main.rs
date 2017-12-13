
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

extern crate jon_listen;

use std::sync::Arc;

use jon_listen::start_server;
use jon_listen::settings::Settings;

fn main() {
    pretty_env_logger::init().unwrap();

    info!("Starting jon-listen app...");

    let settings = Settings::new();

    start_server(Arc::new(settings));
}
