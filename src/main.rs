
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

extern crate jon_listen;

use std::sync::Arc;

use jon_listen::start_up;
use jon_listen::settings::Settings;

fn main() {
    pretty_env_logger::init().unwrap();

    info!("Starting jon-listen app...");

    let settings = Settings::load();

    start_up(Arc::new(settings));
}
