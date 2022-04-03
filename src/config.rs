#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub port: Option<u16>,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
