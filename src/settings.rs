use std::env;
use config::{Config, File, Environment};
use std::path::PathBuf;
use serde;
use serde::de::Deserializer;
use serde::Deserialize;
use config::Source;

pub trait DeserializeWith: Sized {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>;
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
pub enum RotationPolicyType {
    ByDuration,
    ByDay
}

impl DeserializeWith for RotationPolicyType {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "ByDuration" => Ok(RotationPolicyType::ByDuration),
            "ByDay" => Ok(RotationPolicyType::ByDay),
            _ => Err(serde::de::Error::custom("error trying to deserialize rotation policy config"))
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RotationPolicyConfig {
    pub count: i32,
    #[serde(deserialize_with="RotationPolicyType::deserialize_with")]
    pub policy: RotationPolicyType,
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FormattingConfig {
    pub startingmsg: bool,
    pub endingmsg: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileWriterConfig {
    pub filedir: PathBuf,
    pub filename: String,
    pub rotation: RotationPolicyConfig,
    pub formatting: FormattingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub debug: bool,
    pub threads: i32,
    pub buffer_bound: usize,
    pub server: ServerConfig,
    pub filewriter: FileWriterConfig,
}

impl Settings {

    pub fn load() -> Self {
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
        debug!("Provided settings:  {:?}", s.collect());
//        info!("database: {:?}", s.get::<String>("database.url"));

        // You can deserialize (and thus freeze) the entire configuration as
        let settings = s.deserialize().unwrap();
        info!("Settings: {:?}", settings);
        settings
    }
}
