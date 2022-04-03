use std::net::Ipv4Addr;

const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() -> zero2prod::Result {
    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    zero2prod::bind(&(Ipv4Addr::LOCALHOST, config.port.unwrap_or(DEFAULT_PORT)).into()).await
}
