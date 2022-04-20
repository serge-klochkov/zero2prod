CREATE TABLE subscription_tokens(
    subscription_token TEXT NOT NULL PRIMARY KEY,
    subscriber_id uuid NOT NULL REFERENCES subscriptions (id)
);