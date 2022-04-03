mod config;
mod routes;
mod server;

type Tx = axum_sqlx_tx::Tx<sqlx::Postgres>;

pub use self::{
    config::Config,
    server::{bind, Result, Server},
};
