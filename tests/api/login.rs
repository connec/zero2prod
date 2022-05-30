use axum::http::{header::LOCATION, StatusCode};

use crate::helpers::TestApp;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = TestApp::spawn().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });
    let response = app.post_login(&login_body).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[LOCATION], "/login");

    let login_page = app.get_login().await;
    assert!(login_page.contains("Authentication failed"));

    let login_page = app.get_login().await;
    assert!(!login_page.contains("Authentication failed"));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = TestApp::spawn().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[LOCATION], "/admin/dashboard");

    let dashboard = app.get_admin_dashboard().await;
    assert!(dashboard.contains(&format!("Welcome {}", app.test_user.username)));
}
