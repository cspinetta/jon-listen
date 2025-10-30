use config::{Config, Environment, File};
use log::info;
use serde;
use serde::de::Deserializer;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

pub trait DeserializeWith: Sized {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(deserialize_with = "ProtocolType::deserialize_with")]
    pub protocol: ProtocolType,
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
pub enum ProtocolType {
    TCP,
    UDP,
}

impl DeserializeWith for ProtocolType {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "TCP" => Ok(ProtocolType::TCP),
            "UDP" => Ok(ProtocolType::UDP),
            _ => Err(serde::de::Error::custom(
                "error trying to deserialize protocol config",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq, Clone)]
pub enum RotationPolicyType {
    ByDuration,
    ByDay,
}

impl DeserializeWith for RotationPolicyType {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "ByDuration" => Ok(RotationPolicyType::ByDuration),
            "ByDay" => Ok(RotationPolicyType::ByDay),
            _ => Err(serde::de::Error::custom(
                "error trying to deserialize rotation policy config",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RotationPolicyConfig {
    pub count: i32,
    #[serde(deserialize_with = "RotationPolicyType::deserialize_with")]
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
        let run_mode = env::var("RUN_MODE").unwrap_or("development".into());

        let builder = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("APP").separator("_"));

        let config = builder.build().expect("Failed to build configuration");

        info!("Debug: {:?}", config.get_bool("debug"));

        let settings: Settings = config
            .try_deserialize()
            .expect("Failed to deserialize settings");
        info!("Settings: {:?}", settings);
        settings
    }
}
