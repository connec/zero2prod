use std::net::Ipv4Addr;

const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() -> zero2prod::Result {
    zero2prod::bind(&(Ipv4Addr::LOCALHOST, DEFAULT_PORT).into()).await
}
