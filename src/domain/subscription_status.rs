#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Pending,
    Confirmed,
    Failed,
}
