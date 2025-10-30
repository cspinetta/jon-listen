use log::info;
use std::sync::Arc;

use jon_listen::settings::Settings;
use jon_listen::App;

fn main() {
    pretty_env_logger::init();

    info!("Starting jon-listen app...");

    let settings = Settings::load();

    App::start_up(Arc::new(settings));
}
