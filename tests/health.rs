use std::net::{Ipv4Addr, SocketAddr};

#[tokio::test]
async fn health_works() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("http://{addr}/subscriptions"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let addr = spawn_app().await;
    let client = reqwest::Client::new();
    let bodies = vec![
        ("missing the email", "name=le%20guin"),
        ("missing the name", "email=ursula_le_guin%40gmail.com"),
        ("missing both name and email", ""),
    ];

    for (problem, body) in bodies {
        let response = client
            .post(format!("http://{addr}/subscriptions"))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request");

        assert_eq!(
            422,
            response.status().as_u16(),
            "did not get 422 Unprocessable Entity when the payload was {problem}"
        );
    }
}

async fn spawn_app() -> SocketAddr {
    let server = zero2prod::bind(&(Ipv4Addr::LOCALHOST, 0).into());
    let addr = server.local_addr();

    tokio::spawn(server);

    addr
}
