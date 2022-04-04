use std::net::IpAddr;

use sqlx::postgres::PgConnectOptions;

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default, deserialize_with = "ip_addr_from_str")]
    pub address: Option<IpAddr>,

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

fn ip_addr_from_str<'de, D>(deserializer: D) -> Result<Option<IpAddr>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ip_addr: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    ip_addr
        .map(|ip_addr| ip_addr.parse().map_err(serde::de::Error::custom))
        .transpose()
}

fn database_url_from_str<'de, D>(deserializer: D) -> Result<PgConnectOptions, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let database_url: String = serde::Deserialize::deserialize(deserializer)?;
    database_url.parse().map_err(serde::de::Error::custom)
}
