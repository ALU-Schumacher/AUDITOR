use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(serde::Deserialize)]
pub struct Settings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_record_prefix")]
    pub record_prefix: String,
    #[serde(default = "default_string")]
    pub site_id: String,
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_record_prefix() -> String {
    "slurm".to_string()
}

fn default_string() -> String {
    "none".to_string()
}

/// Loads the configuration from a file `configuration.{yaml,json,toml,...}`
#[tracing::instrument(name = "Loading configuration")]
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path
        .join("configuration")
        .join("slurm-epilog-collector");

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base")).required(true));
    let settings = match std::env::args().nth(1) {
        Some(file) => settings.add_source(config::File::from(file.as_ref()).required(false)),
        None => settings,
    };
    let settings = settings.add_source(config::Environment::with_prefix("auditor").separator("__"));
    // settings.merge(config::File::from(configuration_directory.join("base")).required(false))?;

    // match std::env::args().nth(1) {
    //     Some(file) => {
    //         settings.merge(config::File::from(file.as_ref()).required(false))?;
    //     }
    //     None => (),
    // }

    // settings.merge(config::Environment::with_prefix("auditor").separator("_"))?;

    match settings.build() {
        Ok(config) => config.try_deserialize(),
        Err(e) => Err(e),
    }
}
