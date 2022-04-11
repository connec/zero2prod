CREATE TABLE subscription_tokens (
  id uuid NOT NULL PRIMARY KEY,
  subscriber_id uuid NOT NULL REFERENCES subscriptions (id)
);
