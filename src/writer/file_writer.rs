
use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

use std::thread::{self, JoinHandle};
use std::path::PathBuf;
use std::fs;
use std::time::Duration;
use chrono::prelude::*;

use std::borrow::BorrowMut;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};

use ::settings::FileWriterConfig;
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
    file_config: FileWriterConfig,
}

impl FileWriter {

    pub fn new(buffer_bound: usize, file_config: FileWriterConfig) -> Self {
        let file_dir_path = file_config.filedir.clone();
        let mut file_path = file_dir_path.clone();
        file_path.push(file_config.filename.clone());
        let file = Self::open_file(&file_path, file_config.formatting.startingmsg, true).unwrap();

        let (tx, rx) = sync_channel(buffer_bound);

        FileWriter { file_dir_path, file_path, file_name: file_config.filename.clone(), file, tx, rx, file_config }
    }

    pub fn start(&mut self) -> Result<(), String> {
        info!("File writer starting");
        let rotation_policy: Box<RotationPolicy> = match self.file_config.rotation.policy {
            RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(Duration::from_secs(self.file_config.rotation.duration.unwrap()))),
            RotationPolicyType::ByDay => Box::new(RotationByDay::new())
        };
        let file_rotation = FileRotation::new(
            self.file_dir_path.clone(),self.file_path.clone(),
              self.file_name.clone(), self.file_config.rotation.count, rotation_policy, self.tx.clone());
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
        Self::write_with(self.file.borrow_mut(), buf)
    }

    fn write_with(file: &mut File, buf: &[u8]) -> Result<(), String> {
        debug!("Writing to file {:?}", file);
        file.write(buf)
            .map_err(|e| format!("Failed trying to write to the log file. Reason: {}", e))?;
        Ok(())
    }

    fn open_file(filepath: &PathBuf, with_starting_msg: bool, keep_content: bool) -> Result<File, String> {
        let starting_msg = format!("Starting {} at {}\n", filepath.to_string_lossy(), Local::now().to_rfc2822());
        info!("Opening file {:?}", filepath);
        info!("{}", starting_msg);
        let mut options = OpenOptions::new();
        let mut options = if keep_content { options.append(true) } else { options.write(true) };

        let mut file = options.create(true).open(filepath)
            .expect(format!("Open the log file {:?}", filepath).as_ref());
        if with_starting_msg {
            Self::write_with(file.borrow_mut(), starting_msg.as_bytes());
        }
        Ok(file)
    }

    fn rotate(&mut self, new_path: PathBuf) -> Result<(), String> {
        fs::rename(self.file_path.clone(), new_path.clone())
            .map_err(|e| format!("Failed trying to rename the file {:?} to {:?}. Reason: {}", self.file_path.clone(), new_path, e))
            .and_then(|_| {
                let ending_msg = format!("Ending log as {} at {}\n", new_path.as_path().to_string_lossy(), Local::now().to_rfc2822());
                info!("File rename successfully. {}", ending_msg);
                if self.file_config.formatting.endingmsg {
                    self.write(ending_msg.as_bytes())?;
                }
                self.file = Self::open_file(&self.file_path.clone(), self.file_config.formatting.startingmsg, false)?;
                Ok(())
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileWriterCommand {
    Write(Vec<u8>),
    Rename(PathBuf),
    WriteDebug(String, Vec<u8>, i32),
}
