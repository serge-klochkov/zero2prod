#[derive(sqlx::Type)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Pending,
    Confirmed,
    Failed,
}
