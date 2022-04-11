use crate::helpers::TestApp;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_422() {
    let app = TestApp::spawn().await;

    let response = reqwest::get(app.base_url.join("/subscriptions/confirm").unwrap())
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 422);
}
