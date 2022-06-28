use serde_aux::field_attributes::deserialize_number_from_string;
use std::collections::HashMap;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_port")]
    pub port: u16,
    pub components: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_min_priority")]
    pub min_priority: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_max_priority")]
    pub max_priority: u64,
    pub group_mapping: HashMap<String, Vec<String>>,
    #[serde(default = "default_command")]
    pub command: String,
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_min_priority() -> u64 {
    0
}

fn default_max_priority() -> u64 {
    1024
}

fn default_command() -> String {
    "/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}".to_string()
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
#[tracing::instrument(name = "Loading configuration")]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration").join("priority-plugin");

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(false));
    let settings = match std::env::args().nth(1) {
        Some(file) => settings.add_source(
            config::File::from(file.as_ref())
                .required(false)
                .format(config::FileFormat::Yaml),
        ),
        None => settings,
    };
    let settings = settings.add_source(
        config::Environment::with_prefix("AUDITOR")
            .separator("__")
            .prefix_separator("_"),
    );

    settings.build()?.try_deserialize()
}
