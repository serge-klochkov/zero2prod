use crate::config::CONFIG;
use reqwest::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::subscriber_email::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    sender: String,
    base_url: String,
    timeout: Duration,
}

#[derive(Serialize, Deserialize)]
struct Personalization<'a> {
    #[serde(borrow)]
    to: Vec<Email<'a>>,
}

#[derive(Serialize, Deserialize)]
struct Email<'a> {
    email: &'a str,
}

#[derive(Serialize, Deserialize)]
struct Content<'a> {
    r#type: &'a str,
    value: &'a str,
}

#[derive(Serialize, Deserialize)]
struct SendEmailRequest<'a> {
    personalizations: Vec<Personalization<'a>>,
    from: Email<'a>,
    subject: &'a str,
    content: Vec<Content<'a>>,
}

impl EmailClient {
    pub fn new(sender: &str, base_url: &str, timeout: Duration) -> Self {
        Self {
            http_client: Client::new(),
            sender: sender.to_owned(),
            base_url: base_url.to_owned(),
            timeout,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
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
                value: text_content,
                r#type: "text/plain",
            }],
        };
        self.http_client
            .post(&url)
            .bearer_auth(&CONFIG.sendgrid_api_key.expose_secret())
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;
        Ok(())
    }
}
