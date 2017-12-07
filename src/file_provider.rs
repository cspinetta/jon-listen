

use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

pub struct FileProvider {
    filepath: String,
    file: Arc<Mutex<File>>,
}

impl FileProvider {

    pub fn new(filepath: String) -> Self {
        let file = Arc::new(Mutex::new(Self::handle_file(&filepath)));
        FileProvider { filepath, file }
    }

    pub fn get(&self) -> Arc<Mutex<File>> {
        self.file.clone()
    }

    fn handle_file(filepath: &str) -> File {
        info!("Opening file {}", filepath);
        let mut options = OpenOptions::new();
        options.append(true).create(true).open(filepath).unwrap()
    }
}
