use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;

use crate::domain::subscriber_email::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    sender: String,
    base_url: String,
    timeout: Duration,
    sendgrid_api_key: Secret<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Personalization<'a> {
    #[serde(borrow)]
    pub to: Vec<Email<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct Email<'a> {
    pub email: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct Content<'a> {
    pub r#type: &'a str,
    // See https://github.com/serde-rs/serde/issues/1413#issuecomment-494892266
    pub value: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
pub struct SendEmailRequest<'a> {
    pub personalizations: Vec<Personalization<'a>>,
    pub from: Email<'a>,
    pub subject: &'a str,
    pub content: Vec<Content<'a>>,
}

impl EmailClient {
    pub fn new(
        sender: &str,
        base_url: &str,
        timeout: Duration,
        sendgrid_api_key: Secret<String>,
    ) -> Self {
        Self {
            http_client: Client::new(),
            sender: sender.to_owned(),
            base_url: base_url.to_owned(),
            timeout,
            sendgrid_api_key,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        text_content: &str,
    ) -> anyhow::Result<()> {
        let url = format!("{}/mail/send", &self.base_url);
        let request = SendEmailRequest {
            subject,
            from: Email {
                email: &self.sender,
            },
            personalizations: vec![Personalization {
                to: vec![Email {
                    email: recipient.as_ref(),
                }],
            }],
            content: vec![Content {
                value: Cow::Borrowed(text_content),
                r#type: "text/plain",
            }],
        };
        self.http_client
            .post(&url)
            .bearer_auth(self.sendgrid_api_key.expose_secret())
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
