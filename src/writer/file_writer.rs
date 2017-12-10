
use std::ops::Try;

use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use std::io;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::prelude::*;

use regex;
use regex::Regex;
use time;

use glob::glob;
use glob::PatternError;
use glob::GlobResult;
use glob::GlobError;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver, RecvError};


const BUFFER_BOUND: usize = 1000;

pub struct FileWriter {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    max_files: i32,
    file: File,
    pub tx: SyncSender<FileWriterCommand>,
    rx: Receiver<FileWriterCommand>,
}

impl FileWriter {

    pub fn new(file_dir_path: PathBuf, file_name: String, max_files: i32) -> Self {
        let mut file_path = file_dir_path.clone();
        file_path.push(file_name.clone());
        let file = Self::open_file(&file_path);

        let (tx, rx) = sync_channel(BUFFER_BOUND);

        FileWriter { file_dir_path, file_path, file_name, max_files, file, tx, rx }
    }

    pub fn start(&mut self) -> Result<(), String> {
        info!("File writer starting");
        let file_rotation = FileRotation::new(
            self.file_dir_path.clone(),self.file_path.clone(),
              self.file_name.clone(), self.max_files, self.tx.clone());
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
            let command = self.rx.recv().unwrap();
            debug!("Command received: {:?}", command);
            count += 1;
            match command {
                FileWriterCommand::WriteDebug(id, value, i) => {
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
        self.file.write(buf).unwrap();
        self.file.flush().unwrap();
        self.file.sync_data().unwrap();
        Ok(())
    }

    fn open_file(filepath: &PathBuf) -> File {
        info!("Opening file {:?}", filepath);
        let mut options = OpenOptions::new();
        options.append(true).create(true).open(filepath).unwrap()
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

struct FileRotation {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    max_files: i32,
    tx_file_writer: SyncSender<FileWriterCommand>
}

impl FileRotation {

    fn new(file_dir_path: PathBuf, file_path: PathBuf, file_name: String, max_files: i32,
           tx_file_writer: SyncSender<FileWriterCommand>) -> Self {
        FileRotation { file_dir_path, file_path, file_name, max_files, tx_file_writer}
    }

    pub fn start_rotation(&self) {
        let mut file_date = Local::now(); // FIXME: get modified of the current file
        loop {
            self.loop_rotate(&mut file_date);
        }
    }

    fn loop_rotate(&self, file_date: &mut DateTime<Local>) {
        info!("loop rotate...");
        let today = Local::today();
        if file_date.date().eq(&today) {
            info!("I'm in the same day: {}", today);
            let dur_until_midnight = until_next_midnight(Local::now());
            info!("next day is {:?}", dur_until_midnight);
            thread::sleep(dur_until_midnight);
        } else {
            info!("it's a new day: {}", &today);
            match self.request_rotate() {
                Err(err) => {
                    error!("failed trying to rename the file. Reason: {}", String::from(err));
                    thread::sleep_ms(10);
                },
                Ok(new_path) => {
                    info!("file rename successfully. It was save as {:?}", new_path);
                    *file_date = Local::now();
                }
            }
        }
    }

    fn request_rotate(&self) -> Result<PathBuf, RotateError> {

        let files: Vec<PathBuf> = Self::search_files(self.file_path.clone())?;

        let new_path = if files.len() >= self.max_files as usize {
            self.oldest_file(&files)?
        } else {
            self.next_path(&files)?
        };

        self.tx_file_writer.send(FileWriterCommand::Rename(new_path.clone()))
            .map_err(|e| RotateError::OtherError(format!("Error sending RenameCommand: {:?}", e)))?;

        Ok(new_path)
    }

    fn search_files(path: PathBuf) -> Result<Vec<PathBuf>, RotateError> {

        let files_query = path.to_str().ok_or(RotateError::OtherError(format!("Impossible get file path from {:?}", &path)))?;
        let files_query = format!("{}.*", files_query);

        let mut files: Vec<PathBuf> = vec![];
        for result in glob(&files_query)? {
            files.push(result?);
        }
        Ok(files)
    }

    fn oldest_file(&self, files: &Vec<PathBuf>) -> Result<PathBuf, io::Error> {
        info!("Getting oldest log file from {:?}", files);
        let mut default_file = self.file_dir_path.clone();
        default_file.push(format!("{}.0", self.file_name));
        let now = SystemTime::now();
        let oldest = files.iter().min_by(|x, y| {
            let modified_x = x.metadata().unwrap().modified().unwrap();
            let modified_y = y.metadata().unwrap().modified().unwrap();
            let ordering = modified_x.cmp(&modified_y);
//            info!("Iterating files. Ordering: {:?}. modified_x: {:?} ||| modified_y: {:?}", ordering, modified_x, modified_y);
            ordering
        }).unwrap_or(&default_file).canonicalize()?;
        Ok(oldest.clone())
    }

    fn next_path(&self, files: &Vec<PathBuf>) -> Result<PathBuf, RotateError> {
        info!("Getting next name of log file to use. Current files: {:?}", files);
        let re = Regex::new(r".*(\d+)$").map_err(|e| RotateError::RegexError(format!("{}", e)))?;
        let mut n = 0;
        for file in files.iter() {
            let filename_x = file.file_name().and_then(|fname| fname.to_str()).ok_or(RotateError::InvalidFile(format!("invalid file: {:?}", file)))?;
            let n_x = re.captures(filename_x)
                .ok_or(RotateError::RegexError(format!("digit not found in {}", filename_x)))
                .and_then(|captures| {
                    captures.get(1)
                        .ok_or(RotateError::RegexError(format!("It was impossible to capture first group of regex to get the number of file {}", filename_x)))
                })
                .and_then(|digit| {
                    digit.as_str().parse::<i32>()
                        .map_err(|e| RotateError::RegexError(format!("Impossible parse {:?} as integer. Reason: {}", digit, e)))
                })?;
//            info!("Digit of file found: {}", n_x);
            if n_x >= n { n = n_x + 1 }
        }
        Ok(Path::new(&format!("{}.{}", self.file_path.to_str().unwrap(), n)).to_path_buf())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RotateError {
    RegexError(String),
    InvalidFile(String),
    IOError(String),
    OtherError(String),
    SearchFilesError(String),
}

impl From<RotateError> for String {
    fn from(error: RotateError) -> Self {
        format!("{:?}", error)
    }
}

impl From<io::Error> for RotateError {
    fn from(error: io::Error) -> Self {
        RotateError::IOError(error.to_string())
    }
}

impl From<PatternError> for RotateError {
    fn from(error: PatternError) -> Self {
        RotateError::SearchFilesError(error.to_string())
    }
}

impl From<GlobError> for RotateError {
    fn from(error: GlobError) -> Self {
        RotateError::SearchFilesError(error.to_string())
    }
}

fn sleep_for_test() {
    let time_ms = 5000;
    info!("Sleeping for {}ms", time_ms);
    thread::sleep_ms(time_ms);
    info!("Waking up.....");
}

fn system_time_to_date_time(t: SystemTime) -> DateTime<Utc> {
    let (sec, nsec) = match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => { // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        },
    };
    Utc.timestamp(sec, nsec)
}

// see: https://stackoverflow.com/q/47708305/3392786
fn until_next_midnight(from: DateTime<Local>) -> Duration {
    let tomorrow_midnight = (from + time::Duration::days(1)).date().and_hms(0, 0, 0);
    let duration = tomorrow_midnight.signed_duration_since(from).to_std().unwrap();
    println!("Duration between {:?} and {:?}: {:?}", from, tomorrow_midnight, duration);
    duration
}

