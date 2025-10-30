use log::info;
use std::sync::Arc;

use jon_listen::settings::Settings;
use jon_listen::App;

fn main() {
    pretty_env_logger::init();

    info!("Starting jon-listen app...");

    let settings = Settings::load();

    let settings = Arc::new(settings);
    std::thread::spawn({
        let settings = settings.clone();
        move || {
            App::start_up(settings);
        }
    });

    // Exit the whole process on Ctrl+C to ensure non-async threads are terminated
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    });
    std::process::exit(0);
}
