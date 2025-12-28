use glob::{GlobError, PatternError};
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Application-level errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to create FileWriter: {0}")]
    FileWriterCreation(String),

    #[error("Listener failed: {0}")]
    ListenerFailure(#[from] io::Error),

    #[error("FileWriter task failed: {0}")]
    FileWriterTaskFailure(String),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Signal handler installation failed: {0}")]
    SignalHandlerInstallation(String),
}

/// FileWriter domain errors
#[derive(Error, Debug)]
pub enum FileWriterError {
    #[error("Failed to open file {path}: {source}")]
    FileOpen { path: PathBuf, source: io::Error },

    #[error("Failed to write to file: {0}")]
    WriteError(#[from] io::Error),

    #[error("Failed to rename file from {from} to {to}: {source}")]
    RenameError {
        from: PathBuf,
        to: PathBuf,
        source: io::Error,
    },

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Other error: {0}")]
    OtherError(String),
}

/// File rotation domain errors
#[derive(Error, Debug)]
pub enum RotationError {
    #[error("Regex error: {0}")]
    RegexError(String),

    #[error("Invalid file: {0}")]
    InvalidFile(String),

    #[error("IO error: {0}")]
    IOError(#[from] io::Error),

    #[error("Other error: {0}")]
    OtherError(String),

    #[error("Search files error: {0}")]
    SearchFilesError(String),

    #[error("Error sending RenameCommand: {0}")]
    ChannelSendError(String),
}

// Implement From traits for RotationError
impl From<PatternError> for RotationError {
    fn from(error: PatternError) -> Self {
        RotationError::SearchFilesError(error.to_string())
    }
}

impl From<GlobError> for RotationError {
    fn from(error: GlobError) -> Self {
        RotationError::SearchFilesError(error.to_string())
    }
}

// Note: anyhow automatically implements From<E> for anyhow::Error
// where E: std::error::Error + Send + Sync + 'static
// So FileWriterError and RotationError can be converted automatically
