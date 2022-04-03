use std::net::{Ipv4Addr, SocketAddr};

#[tokio::test]
async fn health_works() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/health", addr))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn spawn_app() -> SocketAddr {
    let server = zero2prod::bind(&(Ipv4Addr::LOCALHOST, 0).into());
    let addr = server.local_addr();

    tokio::spawn(server);

    addr
}
