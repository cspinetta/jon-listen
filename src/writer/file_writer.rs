
use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

use std::thread::{self, JoinHandle};
use std::path::PathBuf;
use std::fs;
use std::time::Duration;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};

use ::settings::FileConfig;
use ::settings::RotationPolicyType;
use writer::file_rotation::FileRotation;
use writer::rotation_policy::{RotationPolicy, RotationByDuration, RotationByDay};


pub struct FileWriter {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    file: File,
    pub tx: SyncSender<FileWriterCommand>,
    rx: Receiver<FileWriterCommand>,
    file_config: FileConfig,
}

impl FileWriter {

    pub fn new(buffer_bound: usize, file_config: FileConfig) -> Self {
        let file_dir_path = file_config.filedir.clone();
        let mut file_path = file_dir_path.clone();
        file_path.push(file_config.filename.clone());
        let file = Self::open_file(&file_path);

        let (tx, rx) = sync_channel(buffer_bound);

        FileWriter { file_dir_path, file_path, file_name: file_config.filename.clone(), file, tx, rx, file_config }
    }

    pub fn start(&mut self) -> Result<(), String> {
        info!("File writer starting");
        let rotation_policy: Box<RotationPolicy> = match self.file_config.rotation_policy_type {
            RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(Duration::from_secs(self.file_config.duration.unwrap()))),
            RotationPolicyType::ByDay => Box::new(RotationByDay::new())
        };
        let file_rotation = FileRotation::new(
            self.file_dir_path.clone(),self.file_path.clone(),
              self.file_name.clone(), self.file_config.rotations, rotation_policy, self.tx.clone());
        let rotation_handle: JoinHandle<Result<(), String>> = file_rotation.start_async();
        self.listen_commands()?;
        rotation_handle.join().unwrap_or_else(|e| Err(format!("Failed trying to join. Reason: {:?}", e)))?;
        Ok(())
    }

    pub fn start_async(mut self) -> JoinHandle<Result<(), String>> {
        thread::spawn(move || {
            self.start()
        })
    }

    fn listen_commands(&mut self) -> Result<(), String> {
        let mut count = 0;
        loop {
            let mut command = self.rx.recv()
                .map_err(|e| format!("Error getting file-write-command from channel: {}", e))?;
            debug!("Command received: {:?}", command);
            match command {
                FileWriterCommand::WriteDebug(id, value, i) => {
                    count += 1;
                    info!("WriteDebug - {} - Count in FileWriter: {} - In Server: {}", id, count, i);
                    self.write(value.as_slice())?
                },
                FileWriterCommand::Write(ref value) if value.last().map(|x| x.eq(&('\n' as u8))).unwrap_or(false) => {
                    self.write(value)?
                },
                FileWriterCommand::Write(ref mut value) => {
                    let value: &mut Vec<u8> = value.as_mut();
                    value.push('\n' as u8);
                    self.write((value).as_slice())?
                },
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
                info!("File rename successfully. It was saved as {:?}", new_path);
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
