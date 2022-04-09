#![allow(dead_code)]

use std::{sync::Arc, time::Duration};

use reqwest::Url;

use crate::domain::SubscriberEmail;

#[derive(Clone)]
pub struct EmailClient {
    inner: Arc<EmailClientInner>,
}

struct EmailClientInner {
    http_client: reqwest::Client,
    base_url: Url,
    sender: SubscriberEmail,
    authorization_token: String,
}

impl EmailClient {
    pub fn new(
        base_url: Url,
        sender: SubscriberEmail,
        authorization_token: String,
        timeout: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(EmailClientInner {
                http_client: reqwest::Client::builder().timeout(timeout).build().unwrap(),
                base_url,
                sender,
                authorization_token,
            }),
        }
    }

    pub(crate) async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let inner = &self.inner;

        let url = inner.base_url.join("/email").unwrap();
        let body = SendEmailRequest {
            from: inner.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            text_body: text_content,
            html_body: html_content,
        };
        inner
            .http_client
            .post(url)
            .header("X-Postmark-Server-Token", &inner.authorization_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claim::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use wiremock::{
        matchers::{any, header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::domain::SubscriberEmail;

    use super::EmailClient;

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("content-type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailRequestMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_ok!(result);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(result);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(result);
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url.parse().unwrap(),
            email(),
            Faker.fake(),
            Duration::from_millis(200),
        )
    }

    struct SendEmailRequestMatcher;

    impl wiremock::Match for SendEmailRequestMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }
}
