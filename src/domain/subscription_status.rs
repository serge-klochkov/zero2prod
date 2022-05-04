use std::fmt::{Display, Formatter};

#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl Display for SubscriptionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
