use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use sqlx::postgres::PgConnectOptions;

const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 8000;

#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(default, deserialize_with = "ip_addr_from_str")]
    address: Option<IpAddr>,

    #[serde(default)]
    port: Option<u16>,

    #[serde(rename = "database_url", deserialize_with = "database_url_from_str")]
    database: PgConnectOptions,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from((
            self.address.unwrap_or(DEFAULT_ADDRESS),
            self.port.unwrap_or(DEFAULT_PORT),
        ))
    }

    pub fn database_options(&self) -> PgConnectOptions {
        self.database.clone()
    }

    pub fn database_options_with_database(&self, name: &str) -> PgConnectOptions {
        self.database.clone().database(name)
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
