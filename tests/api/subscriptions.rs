use axum::http::StatusCode;

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("http://{}/subscriptions", app.addr))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_status("valid", StatusCode::OK, response).await;
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let bodies = vec![
        ("missing the email", "name=le%20guin"),
        ("missing the name", "email=ursula_le_guin%40gmail.com"),
        ("missing both name and email", ""),
    ];

    for (problem, body) in bodies {
        let response = client
            .post(format!("http://{}/subscriptions", app.addr))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request");

        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let bodies = vec![
        ("empty name", "name=&email=ursula_le_guin%40gmail.com"),
        ("empty email", "name=Ursula&email="),
        ("invalid email", "name=Ursula&email=definitely-not-an-email"),
    ];

    for (problem, body) in bodies {
        let response = client
            .post(format!("http://{}/subscriptions", app.addr))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request");

        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
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
