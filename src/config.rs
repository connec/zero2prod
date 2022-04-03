use sqlx::postgres::PgConnectOptions;

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(rename = "database_url", deserialize_with = "database_url_from_str")]
    pub database: PgConnectOptions,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}

fn database_url_from_str<'de, D>(deserializer: D) -> Result<PgConnectOptions, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let database_url: String = serde::Deserialize::deserialize(deserializer)?;
    database_url.parse().map_err(serde::de::Error::custom)
}
