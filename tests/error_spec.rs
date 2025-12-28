use jon_listen::error::{FileWriterError, RotationError};
use std::io;
use std::path::PathBuf;

#[test]
fn test_file_writer_error_display_file_open() {
    let path = PathBuf::from("/test/path.log");
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
    let error = FileWriterError::FileOpen {
        path: path.clone(),
        source: io_error,
    };

    let display = format!("{}", error);
    assert!(display.contains("file"));
    assert!(display.contains("open") || display.contains("Open"));
    assert!(display.contains("/test/path.log") || display.contains("path.log"));
}

#[test]
fn test_file_writer_error_display_write_error() {
    let io_error = io::Error::new(io::ErrorKind::WriteZero, "Write zero bytes");
    let error = FileWriterError::WriteError(io_error);

    let display = format!("{}", error);
    assert!(display.contains("write") || display.contains("Write"));
}

#[test]
fn test_file_writer_error_display_rename_error() {
    let from = PathBuf::from("/test/old.log");
    let to = PathBuf::from("/test/new.log");
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let error = FileWriterError::RenameError {
        from: from.clone(),
        to: to.clone(),
        source: io_error,
    };

    let display = format!("{}", error);
    assert!(display.contains("rename") || display.contains("Rename"));
    assert!(display.contains("/test/old.log") || display.contains("old.log"));
    assert!(display.contains("/test/new.log") || display.contains("new.log"));
}

#[test]
fn test_file_writer_error_display_channel_closed() {
    let error = FileWriterError::ChannelClosed;
    let display = format!("{}", error);
    assert!(display.contains("channel") || display.contains("Channel"));
    assert!(display.contains("closed") || display.contains("Closed"));
}

#[test]
fn test_rotation_error_display_io_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let error = RotationError::IOError(io_error);

    let display = format!("{}", error);
    assert!(display.contains("IO") || display.contains("io") || display.contains("error"));
}

#[test]
fn test_rotation_error_display_invalid_file() {
    let error = RotationError::InvalidFile("invalid filename".to_string());
    let display = format!("{}", error);
    assert!(display.contains("invalid") || display.contains("Invalid"));
    assert!(display.contains("filename") || display.contains("file"));
}

#[test]
fn test_rotation_error_display_regex_error() {
    let error = RotationError::RegexError("regex failed".to_string());
    let display = format!("{}", error);
    assert!(display.contains("regex") || display.contains("Regex"));
    assert!(display.contains("failed"));
}

#[test]
fn test_rotation_error_display_channel_send_error() {
    let error = RotationError::ChannelSendError("send failed".to_string());
    let display = format!("{}", error);
    assert!(
        display.contains("RenameCommand") || display.contains("send") || display.contains("Send")
    );
    assert!(display.contains("send failed"));
}

#[test]
fn test_rotation_error_display_other_error() {
    let error = RotationError::OtherError("some error".to_string());
    let display = format!("{}", error);
    assert!(display.contains("error") || display.contains("Error"));
    assert!(display.contains("some error"));
}

#[test]
fn test_rotation_error_from_io_error() {
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
    let rotation_error: RotationError = io_error.into();

    match rotation_error {
        RotationError::IOError(_) => {}
        _ => panic!("Expected IOError variant"),
    }
}

#[test]
fn test_file_writer_error_from_io_error() {
    let io_error = io::Error::new(io::ErrorKind::WriteZero, "Write zero");
    let file_writer_error: FileWriterError = io_error.into();

    match file_writer_error {
        FileWriterError::WriteError(_) => {}
        _ => panic!("Expected WriteError variant"),
    }
}

#[test]
fn test_rotation_error_error_chain() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let rotation_error: RotationError = io_error.into();

    // Test that error can be converted to anyhow::Error
    let anyhow_error: anyhow::Error = rotation_error.into();
    let display = format!("{}", anyhow_error);
    assert!(!display.is_empty());
}

#[test]
fn test_file_writer_error_error_chain() {
    let io_error = io::Error::new(io::ErrorKind::WriteZero, "Write zero");
    let file_writer_error: FileWriterError = io_error.into();

    // Test that error can be converted to anyhow::Error
    let anyhow_error: anyhow::Error = file_writer_error.into();
    let display = format!("{}", anyhow_error);
    assert!(!display.is_empty());
}

#[test]
fn test_rotation_error_debug_format() {
    let error = RotationError::InvalidFile("test".to_string());
    let debug = format!("{:?}", error);
    assert!(debug.contains("InvalidFile"));
    assert!(debug.contains("test"));
}

#[test]
fn test_file_writer_error_debug_format() {
    let error = FileWriterError::ChannelClosed;
    let debug = format!("{:?}", error);
    assert!(debug.contains("ChannelClosed"));
}
