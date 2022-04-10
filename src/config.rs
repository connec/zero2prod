use std::{net::SocketAddr, time::Duration};

use reqwest::Url;
use sqlx::postgres::PgConnectOptions;

use crate::domain::SubscriberEmail;

pub struct Config {
    pub(crate) address: SocketAddr,
    pub(crate) database_options: PgConnectOptions,
    pub(crate) email_base_url: Url,
    pub(crate) email_sender: SubscriberEmail,
    pub(crate) email_authorization_token: String,
    pub(crate) email_send_timeout: Duration,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    pub fn database_options(&self) -> PgConnectOptions {
        self.database_options.clone()
    }

    pub fn with_database(mut self, database: &str) -> Self {
        self.database_options = self.database_options.database(database);
        self
    }
}

#[derive(Default, serde::Deserialize)]
pub struct ConfigBuilder {
    #[serde(default, deserialize_with = "parse_optional")]
    address: Option<SocketAddr>,

    #[serde(default, rename = "database_url", deserialize_with = "parse_optional")]
    database_options: Option<PgConnectOptions>,

    #[serde(default, deserialize_with = "parse_optional")]
    email_base_url: Option<Url>,

    #[serde(default, deserialize_with = "parse_optional")]
    email_sender: Option<SubscriberEmail>,

    #[serde(default)]
    email_authorization_token: Option<String>,

    #[serde(
        default,
        rename = "email_send_timeout_ms",
        deserialize_with = "parse_millis_optional"
    )]
    email_send_timeout: Option<Duration>,
}

impl ConfigBuilder {
    pub fn address(mut self, address: SocketAddr) -> Self {
        self.address = Some(address);
        self
    }

    pub fn database_options(mut self, database_options: PgConnectOptions) -> Self {
        self.database_options = Some(database_options);
        self
    }

    pub fn email_base_url(mut self, email_base_url: Url) -> Self {
        self.email_base_url = Some(email_base_url);
        self
    }

    pub fn email_sender(mut self, email_sender: SubscriberEmail) -> Self {
        self.email_sender = Some(email_sender);
        self
    }

    pub fn email_authorization_token(mut self, email_authorization_token: String) -> Self {
        self.email_authorization_token = Some(email_authorization_token);
        self
    }

    pub fn email_send_timeout(mut self, email_send_timeout: Duration) -> Self {
        self.email_send_timeout = Some(email_send_timeout);
        self
    }

    pub fn build(self) -> Result<Config, envy::Error> {
        // Get any overrides from the environment
        let overrides: Self = envy::from_env()?;

        Ok(Config {
            address: overrides
                .address
                .or(self.address)
                .ok_or(envy::Error::MissingValue("address"))?,
            database_options: overrides
                .database_options
                .or(self.database_options)
                .ok_or(envy::Error::MissingValue("database_url"))?,
            email_base_url: overrides
                .email_base_url
                .or(self.email_base_url)
                .ok_or(envy::Error::MissingValue("email_base_url"))?,
            email_sender: overrides
                .email_sender
                .or(self.email_sender)
                .ok_or(envy::Error::MissingValue("email_sender"))?,
            email_authorization_token: overrides
                .email_authorization_token
                .or(self.email_authorization_token)
                .ok_or(envy::Error::MissingValue("email_authorization_token"))?,
            email_send_timeout: overrides
                .email_send_timeout
                .or(self.email_send_timeout)
                .ok_or(envy::Error::MissingValue("email_send_timeout_ms"))?,
        })
    }
}

fn parse_optional<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    s.map(|s| s.parse().map_err(serde::de::Error::custom))
        .transpose()
}

fn parse_millis_optional<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis: Option<u64> = serde::Deserialize::deserialize(deserializer)?;
    Ok(millis.map(Duration::from_millis))
}
