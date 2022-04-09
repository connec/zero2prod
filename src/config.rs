use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use reqwest::Url;
use sqlx::postgres::PgConnectOptions;

use crate::domain::SubscriberEmail;

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

    email_base_url: Url,

    #[serde(deserialize_with = "subscriber_email_from_string")]
    email_sender: SubscriberEmail,

    email_authorization_token: String,

    #[serde(rename = "email_send_timeout_ms", deserialize_with = "duration_ms")]
    email_send_timeout: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I>(iter: I) -> Result<Self, envy::Error>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        envy::from_iter(iter)
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

    pub fn email_base_url(&self) -> &Url {
        &self.email_base_url
    }

    pub fn email_sender(&self) -> &SubscriberEmail {
        &self.email_sender
    }

    pub fn email_authorization_token(&self) -> &str {
        &self.email_authorization_token
    }

    pub fn email_send_timeout(&self) -> Duration {
        self.email_send_timeout
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

fn subscriber_email_from_string<'de, D>(deserializer: D) -> Result<SubscriberEmail, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let email: String = serde::Deserialize::deserialize(deserializer)?;
    SubscriberEmail::parse(email).map_err(serde::de::Error::custom)
}

fn duration_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis: u64 = serde::Deserialize::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}
