use secrecy::ExposeSecret;
use secrecy::Secret;
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(serde::Deserialize, Debug)]
pub struct ParquetToAuditorSettings {
    pub file_path: String,
    pub db_username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl ParquetToAuditorSettings {
    pub fn to_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db_username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
        )
    }
}

pub fn get_configuration() -> Result<ParquetToAuditorSettings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    let base_yaml_path = configuration_directory.join("config.yaml");

    println!("Reading config from: {base_yaml_path:?}");

    let mut settings = config::Config::builder();

    settings = settings
        .add_source(config::File::from(base_yaml_path).required(true))
        .add_source(config::Environment::default());

    settings.build()?.try_deserialize()
}
