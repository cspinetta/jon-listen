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
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

fn default_max_connections() -> usize {
    1000
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum BackpressurePolicy {
    Block,   // Block message ingestion until space is available
    Discard, // Discard messages when channel is full
}

impl DeserializeWith for BackpressurePolicy {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "Block" => Ok(BackpressurePolicy::Block),
            "Discard" => Ok(BackpressurePolicy::Discard),
            _ => Err(serde::de::Error::custom(
                "error trying to deserialize backpressure policy config. Must be 'Block' or 'Discard'",
            )),
        }
    }
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
    #[serde(
        default = "default_backpressure_policy",
        deserialize_with = "BackpressurePolicy::deserialize_with"
    )]
    pub backpressure_policy: BackpressurePolicy,
}

fn default_backpressure_policy() -> BackpressurePolicy {
    BackpressurePolicy::Discard
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub debug: bool,
    pub threads: i32,
    pub buffer_bound: usize,
    pub server: ServerConfig,
    pub filewriter: FileWriterConfig,
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

fn default_metrics_port() -> u16 {
    9090
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let run_mode = env::var("RUN_MODE").unwrap_or("development".into());

        let builder = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("APP").separator("_"));

        let config = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build configuration: {}", e))?;

        info!("Debug: {:?}", config.get_bool("debug"));

        let settings: Settings = config
            .try_deserialize()
            .map_err(|e| anyhow::anyhow!("Failed to deserialize settings: {}", e))?;
        info!("Settings: {:?}", settings);
        Ok(settings)
    }
}
