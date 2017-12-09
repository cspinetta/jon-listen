use std::env;
use config::{Config, File, Environment};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub filedir: PathBuf,
    pub filename: String,
    pub rotations: i32,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub threads: i32,
    pub server: Server,
    pub file: FileConfig,
}

impl Settings {
    pub fn new() -> Self {
        let mut s = Config::new();

        // Start off by merging in the "default" configuration file
        s.merge(File::with_name("config/default")).unwrap();

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional_
        let env = env::var("RUN_MODE").unwrap_or("development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false)).unwrap();

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false)).unwrap();

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("app")).unwrap();

        // Now that we're done, let's access our configuration
        info!("Debug: {:?}", s.get_bool("debug"));
//        info!("database: {:?}", s.get::<String>("database.url"));

        // You can deserialize (and thus freeze) the entire configuration as
        s.deserialize().unwrap()
    }
}
