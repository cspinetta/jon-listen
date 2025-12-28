use chrono::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration as TokioDuration};

use regex::Regex;

use glob::glob;

use crate::error::RotationError;
use crate::writer::file_writer::FileWriterCommand;
use crate::writer::metrics;
use crate::writer::rotation_policy::RotationPolicy;
use log::{error, info};
use tokio::sync::{broadcast, mpsc};

pub struct FileRotation {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    max_files: i32,
    rotation_policy: Box<dyn RotationPolicy>,
    tx_file_writer: mpsc::Sender<FileWriterCommand>,
}

impl FileRotation {
    pub fn new(
        file_dir_path: PathBuf,
        file_path: PathBuf,
        file_name: String,
        max_files: i32,
        rotation_policy: Box<dyn RotationPolicy>,
        tx_file_writer: mpsc::Sender<FileWriterCommand>,
    ) -> Self {
        FileRotation {
            file_dir_path,
            file_path,
            file_name,
            max_files,
            rotation_policy,
            tx_file_writer,
        }
    }

    pub async fn start(
        &self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), RotationError> {
        let mut last_rotation = Local::now(); // FIXME: get modified of the current file
        loop {
            info!("loop rotate...");
            let time_for_rotate = self.rotation_policy.next_rotation(last_rotation);
            let now = Local::now();
            if time_for_rotate.gt(&now) {
                let dur_to_rotate = time_for_rotate.signed_duration_since(now).to_std().unwrap();
                info!("Sleep and wait {:?} for the time to rotate", dur_to_rotate);

                // Convert std::time::Duration to tokio::time::Duration preserving full precision
                let tokio_dur = TokioDuration::from_nanos(dur_to_rotate.as_nanos() as u64);
                tokio::select! {
                    _ = sleep(tokio_dur) => {
                        // Sleep completed, continue to rotation check
                    }
                    _ = shutdown_rx.recv() => {
                        info!("FileRotation received shutdown signal");
                        break;
                    }
                }
            } else {
                info!("it's the time to rotate: {}", &now);
                match self.request_rotate().await {
                    Err(err) => {
                        error!("Failed trying to rename the file. Reason: {}", err);
                        metrics::rotation::error();
                        tokio::select! {
                            _ = sleep(TokioDuration::from_secs(1)) => {
                                // Sleep completed, continue loop
                            }
                            _ = shutdown_rx.recv() => {
                                info!("FileRotation received shutdown signal");
                                break;
                            }
                        }
                    }
                    Ok(new_path) => {
                        info!("File rename requested. It will be saved as {:?}", new_path);
                        metrics::rotation::event();
                        last_rotation = now;
                    }
                }
            }
        }
        info!("FileRotation shutting down gracefully");
        Ok(())
    }

    pub fn start_async(
        self,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> tokio::task::JoinHandle<Result<(), RotationError>> {
        // Now that start() is async, we can just spawn it directly
        tokio::spawn(async move { self.start(shutdown_rx).await })
    }

    /// Public method for testing only
    pub async fn request_rotate(&self) -> Result<PathBuf, RotationError> {
        let files = Self::search_files(self.file_path.clone()).await?;

        let new_path = if files.len() >= self.max_files as usize {
            self.oldest_file(&files).await?
        } else {
            self.next_path(&files)?
        };

        // Now fully async - can use .await directly
        self.tx_file_writer
            .send(FileWriterCommand::Rename(new_path.clone()))
            .await
            .map_err(|e| {
                RotationError::ChannelSendError(format!("Error sending RenameCommand: {:?}", e))
            })?;

        Ok(new_path)
    }

    /// Public method for testing only
    pub async fn search_files(path: PathBuf) -> Result<Vec<PathBuf>, RotationError> {
        let files_query = path.to_str().ok_or(RotationError::OtherError(format!(
            "Impossible get file path from {:?}",
            &path
        )))?;
        let files_query = format!("{}.*", files_query);

        // glob is a blocking library, so we use spawn_blocking to run it on a blocking thread pool
        let files_query_clone = files_query.clone();
        let files = tokio::task::spawn_blocking(move || {
            let mut files: Vec<PathBuf> = vec![];
            for result in glob(&files_query_clone)? {
                files.push(result?);
            }
            Ok::<Vec<PathBuf>, RotationError>(files)
        })
        .await
        .map_err(|e| RotationError::OtherError(format!("Task join error: {:?}", e)))??;

        Ok(files)
    }

    /// Public method for testing only
    pub async fn oldest_file(&self, files: &Vec<PathBuf>) -> Result<PathBuf, RotationError> {
        info!("Getting oldest log file from {:?}", files);
        let mut default_file = self.file_dir_path.clone();
        default_file.push(format!("{}.0", self.file_name));

        // Use async metadata operations
        let mut oldest_opt: Option<(PathBuf, SystemTime)> = None;
        for file in files.iter() {
            let metadata = tokio::fs::metadata(file).await?;
            let modified = metadata.modified()?;
            if let Some((_, oldest_time)) = &oldest_opt {
                if modified < *oldest_time {
                    oldest_opt = Some((file.clone(), modified));
                }
            } else {
                oldest_opt = Some((file.clone(), modified));
            }
        }

        let oldest = oldest_opt.map(|(path, _)| path).unwrap_or(default_file);

        // If the file doesn't exist (empty list case), return the default path without canonicalizing
        if !tokio::fs::try_exists(&oldest).await.unwrap_or(false) {
            return Ok(oldest);
        }

        tokio::fs::canonicalize(&oldest)
            .await
            .map_err(RotationError::from)
    }

    /// Public method for testing only
    pub fn next_path(&self, files: &Vec<PathBuf>) -> Result<PathBuf, RotationError> {
        info!(
            "Getting next name of log file to use. Current files: {:?}",
            files
        );
        let re =
            Regex::new(r".*(\d+)$").map_err(|e| RotationError::RegexError(format!("{}", e)))?;
        let mut next_id = 0;
        for file in files.iter() {
            let filename_x = file.file_name().and_then(|fname| fname.to_str()).ok_or(
                RotationError::InvalidFile(format!("invalid file: {:?}", file)),
            )?;
            // Skip files that don't match the pattern (no numeric suffix)
            if let Some(captures) = re.captures(filename_x) {
                if let Some(digit_match) = captures.get(1) {
                    if let Ok(file_id) = digit_match.as_str().parse::<i32>() {
                        if file_id >= next_id {
                            next_id = file_id + 1
                        }
                    }
                }
            }
            // If file doesn't match pattern, skip it (no error)
        }
        Ok(Path::new(&format!("{}.{}", self.file_path.to_str().unwrap(), next_id)).to_path_buf())
    }
}

#[allow(dead_code)]
fn system_time_to_date_time(t: SystemTime) -> DateTime<Utc> {
    let (sec, nsec) = match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => {
            // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        }
    };
    Utc.timestamp_opt(sec, nsec).single().unwrap()
}
