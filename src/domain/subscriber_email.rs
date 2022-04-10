use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use validator::validate_email;

#[derive(AsRef, Debug, Serialize, Deserialize, Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email.", s))
        }
    }
}
