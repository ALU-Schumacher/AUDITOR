use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(serde::Deserialize, Debug)]
pub struct AuditorSettings {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

fn default_addr() -> String {
    "127.0.0.1".to_string()
}

pub fn get_configuration() -> Result<AuditorSettings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("benches").join("configuration");

    let base_yaml_path = configuration_directory.join("bench.yaml");

    let mut settings = config::Config::builder();

    settings = settings.add_source(config::File::from(base_yaml_path).required(true));

    settings.build()?.try_deserialize()
}
