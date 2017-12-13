
use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

use std::sync::Arc;
use std::thread;
use std::path::PathBuf;
use std::fs;
use std::time::Duration;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};

use ::settings::Settings;
use writer::file_rotation::FileRotation;
use writer::rotation_policy::RotationByDuration;


const BUFFER_BOUND: usize = 1000;

pub struct FileWriter {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    max_files: i32,
    file: File,
    pub tx: SyncSender<FileWriterCommand>,
    rx: Receiver<FileWriterCommand>,
    settings: Arc<Settings>,
}

impl FileWriter {

    pub fn new(file_dir_path: PathBuf, file_name: String, max_files: i32, settings: Arc<Settings>) -> Self {
        let mut file_path = file_dir_path.clone();
        file_path.push(file_name.clone());
        let file = Self::open_file(&file_path);

        let (tx, rx) = sync_channel(BUFFER_BOUND);

        FileWriter { file_dir_path, file_path, file_name, max_files, file, tx, rx, settings }
    }

    pub fn start(&mut self) -> Result<(), String> {
        info!("File writer starting");
        let rotation_policy = RotationByDuration::new(Duration::from_secs(10));
        let file_rotation = FileRotation::new(
            self.file_dir_path.clone(),self.file_path.clone(),
              self.file_name.clone(), self.max_files, Box::new(rotation_policy), self.tx.clone());
        let rotation_handle = thread::spawn(move || {
            file_rotation.start_rotation();
        });
        self.listen_commands();
        rotation_handle.join();
        Ok(())
    }

    fn listen_commands(&mut self) -> Result<(), String> {
        let mut count = 0;
        loop {
            let command = self.rx.recv()
                .map_err(|e| format!("Error getting file-write-command from channel: {}", e))?;
            debug!("Command received: {:?}", command);
            match command {
                FileWriterCommand::WriteDebug(id, value, i) => {
                    count += 1;
                    info!("WriteDebug - {} - Count in FileWriter: {} - In Server: {}", id, count, i);
                    self.write(value.as_slice())?
                },
                FileWriterCommand::Write(value) => self.write(value.as_slice())?,
                FileWriterCommand::Rename(new_path) => self.rotate(new_path)?,
            }
        }
        Ok(())
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), String> {
        debug!("Writing to file {:?}", self.file);
        self.file.write(buf)
            .map_err(|e| format!("Failed trying to write to the log file. Reason: {}", e))?;
        Ok(())
    }

    fn open_file(filepath: &PathBuf) -> File {
        info!("Opening file {:?}", filepath);
        let mut options = OpenOptions::new();
        options.append(true).create(true).open(filepath)
            .expect(format!("Open the log file {:?}", filepath).as_ref())
    }

    fn rotate(&mut self, new_path: PathBuf) -> Result<(), String> {
        fs::rename(self.file_path.clone(), new_path.clone())
            .map(|_| {
                self.file = Self::open_file(&self.file_path.clone());
            })
            .map_err(|e| format!("Failed trying to rename the file {:?} to {:?}. Reason: {}", self.file_path.clone(), new_path, e))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileWriterCommand {
    Write(Vec<u8>),
    Rename(PathBuf),
    WriteDebug(String, Vec<u8>, i32),
}
