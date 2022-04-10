use crate::helpers::TestApp;

#[tokio::test]
async fn health_works() {
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.base_url.join("/health").unwrap())
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
