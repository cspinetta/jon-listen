use chrono::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, mpsc};

use crate::error::FileWriterError;
use crate::metrics::messages;
use crate::settings::FileWriterConfig;
use crate::settings::RotationPolicyType;
use crate::writer::file_rotation::FileRotation;
use crate::writer::metrics;
use crate::writer::rotation_policy::{RotationByDay, RotationByDuration, RotationPolicy};
use log::{debug, info};

pub struct FileWriter {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    file: File,
    pub tx: mpsc::Sender<FileWriterCommand>,
    rx: mpsc::Receiver<FileWriterCommand>,
    file_config: FileWriterConfig,
}

impl FileWriter {
    pub async fn new(
        buffer_bound: usize,
        file_config: FileWriterConfig,
    ) -> Result<Self, FileWriterError> {
        let file_dir_path = file_config.filedir.clone();
        let mut file_path = file_dir_path.clone();
        file_path.push(file_config.filename.clone());
        let file = Self::open_file(&file_path, file_config.formatting.startingmsg, true).await?;

        let (tx, rx) = mpsc::channel(buffer_bound);

        Ok(FileWriter {
            file_dir_path,
            file_path,
            file_name: file_config.filename.clone(),
            file,
            tx,
            rx,
            file_config,
        })
    }

    pub async fn start(
        &mut self,
        shutdown_rx: broadcast::Receiver<()>,
        rotation_shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), FileWriterError> {
        info!("File writer starting");
        let rotation_policy: Box<dyn RotationPolicy> = match self.file_config.rotation.policy {
            RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(
                Duration::from_secs(self.file_config.rotation.duration.unwrap()),
            )),
            RotationPolicyType::ByDay => Box::new(RotationByDay::new()),
        };
        let file_rotation = FileRotation::new(
            self.file_dir_path.clone(),
            self.file_path.clone(),
            self.file_name.clone(),
            self.file_config.rotation.count,
            rotation_policy,
            self.tx.clone(),
        );
        let rotation_handle = file_rotation.start_async(rotation_shutdown_rx);
        let mut shutdown_rx = shutdown_rx;

        // Run listen_commands and rotation concurrently, wait for shutdown
        tokio::select! {
            result = Self::listen_commands_internal(self, &mut shutdown_rx) => {
                result?;
            }
            result = rotation_handle => {
                result
                    .map_err(|e| FileWriterError::OtherError(format!("Rotation task join error: {:?}", e)))?
                    .map_err(|e| FileWriterError::OtherError(format!("Rotation error: {}", e)))?;
            }
        }

        info!("File writer shutting down gracefully");
        Ok(())
    }

    pub(crate) async fn listen_commands_internal(
        &mut self,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) -> Result<(), FileWriterError> {
        let mut count = 0;
        loop {
            tokio::select! {
                command = self.rx.recv() => {
                    match command {
                        Some(mut cmd) => {
                            // Note: tokio::sync::mpsc::Receiver doesn't expose len() for queue depth tracking
                            // To track queue depth, we would need a wrapper that counts sends/receives
                            debug!("Command received: {:?}", cmd);
                            match cmd {
                                FileWriterCommand::WriteDebug(id, value, i) => {
                                    count += 1;
                                    info!(
                                        "WriteDebug - {} - Count in FileWriter: {} - In Server: {}",
                                        id, count, i
                                    );
                                    self.write(value.as_slice()).await?
                                }
                                FileWriterCommand::Write(ref value)
                                    if value.last().map(|x| x.eq(&b'\n')).unwrap_or(false) =>
                                {
                                    self.write(value).await?
                                }
                                FileWriterCommand::Write(ref mut value) => {
                                    value.push(b'\n');
                                    self.write(value.as_slice()).await?
                                }
                                FileWriterCommand::Rename(new_path) => Self::rotate_internal(self, new_path).await?,
                            }
                        }
                        None => {
                            // Channel closed - return error immediately
                            return Err(FileWriterError::ChannelClosed);
                        }
                    }
                }
                result = shutdown_rx.recv() => {
                    match result {
                        Ok(_) => {
                            info!("FileWriter received shutdown signal");
                            break;
                        }
                        Err(_) => {
                            // Shutdown channel closed - continue processing commands
                            // This shouldn't happen in normal operation, but handle gracefully
                            continue;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Public method for testing only
    pub async fn listen_commands(
        &mut self,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) -> Result<(), FileWriterError> {
        Self::listen_commands_internal(self, shutdown_rx).await
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<(), FileWriterError> {
        let timer = metrics::file_write::WriteTimer::start();
        let result = Self::write_with(&mut self.file, buf).await;
        timer.finish();
        if result.is_ok() {
            messages::written();
        }
        result
    }

    async fn write_with(file: &mut File, buf: &[u8]) -> Result<(), FileWriterError> {
        debug!("Writing to file {:?}", file);
        file.write_all(buf)
            .await
            .map_err(FileWriterError::WriteError)?;
        Ok(())
    }

    async fn open_file(
        filepath: &PathBuf,
        with_starting_msg: bool,
        keep_content: bool,
    ) -> Result<File, FileWriterError> {
        let starting_msg = format!(
            "Starting {} at {}\n",
            filepath.to_string_lossy(),
            Local::now().to_rfc2822()
        );
        info!("Opening file {:?}", filepath);
        info!("{}", starting_msg);
        let mut options = OpenOptions::new();
        let options = if keep_content {
            options.append(true)
        } else {
            options.write(true)
        };

        let mut file =
            options
                .create(true)
                .open(filepath)
                .await
                .map_err(|e| FileWriterError::FileOpen {
                    path: filepath.clone(),
                    source: e,
                })?;
        if with_starting_msg {
            Self::write_with(&mut file, starting_msg.as_bytes()).await?;
        }
        Ok(file)
    }

    pub(crate) async fn rotate_internal(
        &mut self,
        new_path: PathBuf,
    ) -> Result<(), FileWriterError> {
        tokio::fs::rename(self.file_path.clone(), new_path.clone())
            .await
            .map_err(|e| FileWriterError::RenameError {
                from: self.file_path.clone(),
                to: new_path.clone(),
                source: e,
            })?;

        let ending_msg = format!(
            "Ending log as {} at {}\n",
            new_path.as_path().to_string_lossy(),
            Local::now().to_rfc2822()
        );
        info!("File rename successfully. {}", ending_msg);
        if self.file_config.formatting.endingmsg {
            self.write(ending_msg.as_bytes()).await?;
        }
        self.file = Self::open_file(
            &self.file_path.clone(),
            self.file_config.formatting.startingmsg,
            false,
        )
        .await?;
        Ok(())
    }

    /// Public method for testing only
    pub async fn rotate(&mut self, new_path: PathBuf) -> Result<(), FileWriterError> {
        Self::rotate_internal(self, new_path).await
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileWriterCommand {
    Write(Vec<u8>),
    Rename(PathBuf),
    WriteDebug(String, Vec<u8>, i32),
}
