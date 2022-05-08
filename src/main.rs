use std::net::Ipv4Addr;

use tracing::info;
use zero2prod::App;

#[tokio::main]
async fn main() {
    zero2prod::telemetry::init(env!("CARGO_PKG_NAME"), std::io::stdout);

    let address = (Ipv4Addr::LOCALHOST, 8000).into();
    let config = zero2prod::Config::builder()
        .address(address)
        .base_url(format!("http://{}/", address).parse().unwrap())
        .merge_env()
        .expect("invalid configuration in environment")
        .build()
        .expect("failed to read configuration");

    let app = App::new(config);
    let server = app.serve().await.expect("failed to serve app");

    info!("Listening on {}", server.local_addr());
    server.await.expect("error while running server")
}
