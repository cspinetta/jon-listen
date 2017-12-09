
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


pub struct FileWriter {
    file_dir_path: PathBuf,
    file_path: PathBuf,
    file_name: String,
    max_files: i32,
    file: Arc<Mutex<File>>,
    file_date: Arc<Mutex<Date<Local>>>,
}

impl FileWriter {

    pub fn new(file_dir_path: PathBuf, file_name: String, max_files: i32) -> Self {
        let file_date = Arc::new(Mutex::new(Local::today()));

        let mut file_path = file_dir_path.clone();
        file_path.push(file_name.clone());

        let file = Arc::new(Mutex::new(Self::open_file(&file_path)));
        FileWriter { file_dir_path, file_path, file_name, max_files, file, file_date }
    }

    pub fn write(&self, buf: Arc<&[u8]>) {
        let mut file: Arc<Mutex<File>> = self.file.clone();
        let mut file = file.lock().unwrap();
        (*file).write(buf.as_ref()).unwrap();
        (*file).flush().unwrap();
        (*file).sync_data().unwrap();
    }

    fn open_file(filepath: &PathBuf) -> File {
        info!("Opening file {:?}", filepath);
        let mut options = OpenOptions::new();
        options.append(true).create(true).open(filepath).unwrap()
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
            match self.rotate() {
                Err(err) => {
                    error!("failed trying to rename the file. Reason: {}", String::from(err));
                    thread::sleep_ms(10);
//                        sleep_for_test();
                },
                Ok(name) => {
                    info!("file rename successfully. It was save as {:?}", name);
                    *file_date = Local::now();
//                        sleep_for_test();
                }
            }
        }
    }

    fn rotate(&self) -> Result<PathBuf, RotateError> {

//        let files: Vec<PathBuf> = glob(&files_query)?.flat_map(|x| x).collect();
        let files: Vec<PathBuf> = FileWriter::search_files(self.file_path.clone())?;

        let new_path = if files.len() >= self.max_files as usize {
            self.oldest_file(&files)?
        } else {
            self.next_path(&files)?
        };

        let mut file = self.file.lock()
            .map_err(|x| RotateError::OtherError(format!("Failed trying to get lock. Reason: {}", x)))?;

        fs::rename(self.file_path.clone(), new_path.clone())
            .map(|_| {
                *file = Self::open_file(&self.file_path.clone());
            })
            .map(|_| new_path.clone())
            .map_err(|e| RotateError::OtherError(format!("Failed trying to rename the file {:?} to {:?}. Reason: {}", self.file_path.clone(), new_path, e)))
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

