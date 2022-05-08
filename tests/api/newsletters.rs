use axum::http::StatusCode;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{ConfirmationLinks, TestApp};

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = TestApp::spawn().await;

    let response = reqwest::Client::new()
        .post(app.base_url.join("/newsletters").unwrap())
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            },
        }))
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        "Basic realm=\"publish\""
    );
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = TestApp::spawn().await;
    let password = Uuid::new_v4().to_string();
    assert_ne!(
        app.test_user.password, password,
        "generated the same UUID twice!"
    );

    let response = reqwest::Client::new()
        .post(app.base_url.join("/newsletters").unwrap())
        .basic_auth(&app.test_user.username, Some(&password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            },
        }))
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        "Basic realm=\"zero2prod\""
    );
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = TestApp::spawn().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });
    let response = app.post_newsletters(&body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = TestApp::spawn().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });
    let response = app.post_newsletters(&body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = TestApp::spawn().await;

    let test_cases = vec![
        (
            "missing title",
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                },
            }),
        ),
        (
            "missing content",
            serde_json::json!({"title": "Newsletter!"}),
        ),
    ];
    for (problem, body) in test_cases {
        let response = app.post_newsletters(&body).await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "the API did not fail with {} when the payload was {}",
            StatusCode::UNPROCESSABLE_ENTITY,
            problem
        );
    }
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.text)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
