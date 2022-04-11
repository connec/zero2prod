mod confirm;

use axum::http::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::TestApp;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = TestApp::spawn().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body).await;

    assert_status("valid", StatusCode::OK, response).await;
}

#[tokio::test]
async fn subscribe_persists_a_new_subscriber() {
    let app = TestApp::spawn().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.post_subscriptions(body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.status, "pending");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let app = TestApp::spawn().await;
    let bodies = vec![
        ("missing the email", "name=le%20guin"),
        ("missing the name", "email=ursula_le_guin%40gmail.com"),
        ("missing both name and email", ""),
    ];

    for (problem, body) in bodies {
        let response = app.post_subscriptions(body).await;
        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_invalid() {
    let app = TestApp::spawn().await;
    let bodies = vec![
        ("empty name", "name=&email=ursula_le_guin%40gmail.com"),
        ("empty email", "name=Ursula&email="),
        ("invalid email", "name=Ursula&email=definitely-not-an-email"),
    ];

    for (problem, body) in bodies {
        let response = app.post_subscriptions(body).await;

        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = TestApp::spawn().await;
    let body = "name=le%20guin&email=ursula_le_guin@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    assert_eq!(links.html, links.text);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = TestApp::spawn().await;
    let body = "name=le%20guin&email=ursula_le_guin@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    let response = reqwest::get(links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_confirms_a_subscriber() {
    let app = TestApp::spawn().await;
    let body = "name=le%20guin&email=ursula_le_guin@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = app.get_confirmation_links(email_request);

    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

async fn assert_status(problem: &str, expected: StatusCode, response: reqwest::Response) {
    assert_eq!(
        expected,
        response.status(),
        "did not get {} when the payload was {problem}
got {}: {:?}",
        expected,
        response.status(),
        response.text().await,
    );
}
