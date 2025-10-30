pub mod listener;
pub mod settings;
pub mod writer;

use std::sync::Arc;

use listener::Listener;
use settings::Settings;
use writer::file_writer::FileWriter;

// use std::borrow::Borrow; // not needed

pub struct App;

impl App {
    pub fn start_up(settings: Arc<Settings>) {
        let mut file_writer = FileWriter::new(settings.buffer_bound, settings.filewriter.clone());

        let conn_threads = Listener::start(settings.clone(), file_writer.tx.clone());

        let _ = file_writer.start();
        for t in conn_threads {
            t.join().unwrap();
        }
    }
}
